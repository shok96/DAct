use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui::{self, TextureHandle, ViewportBuilder, ViewportCommand, ViewportId};

use crate::convert::{self, ActionExtra, MenuItem};
use crate::ipc::IpcMessage;
use crate::report::{self, ResultCard};
use crate::settings::Settings;
use crate::ui::home::{self, HomeEvent};
use crate::ui::radial::{RadialMenu, SIZE};
use crate::ui::theme;
use crate::ui::toasts::ToastStack;
use crate::ui::tray::{Tray, TrayCmd};
use crate::ui::trigger::{TriggerEvent, Watcher};
use crate::ui::windows::{CompressWin, ResizeWin, SettingsWin, WelcomeWin};
use crate::APP_NAME;

enum JobDone {
    Convert {
        toast: usize,
        result: std::result::Result<ResultCard, String>,
    },
    Progress {
        toast: usize,
        fraction: f32,
        detail: String,
    },
    Ffmpeg {
        toast: usize,
        result: std::result::Result<PathBuf, String>,
        retry: Option<(String, Vec<PathBuf>)>,
    },
}

pub struct DactApp {
    settings: Settings,
    watcher: Watcher,
    tray: Option<Tray>,
    radial: RadialMenu,
    toasts: ToastStack,
    settings_win: SettingsWin,
    welcome: WelcomeWin,
    compress: Option<CompressWin>,
    resize: Option<ResizeWin>,
    ipc_rx: Receiver<IpcMessage>,
    job_tx: Sender<JobDone>,
    job_rx: Receiver<JobDone>,
    last_tick: Instant,
    queue: Vec<PathBuf>,
    thumb: Option<TextureHandle>,
    thumb_src: Option<PathBuf>,
    enabled: bool,
    hidden: bool,
    exit: bool,
    tray_hinted: bool,
    restore_size: egui::Vec2,
    restore_pos: Option<egui::Pos2>,
    toast_live: bool,
    radial_live: bool,
}

impl DactApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        ipc_rx: Receiver<IpcMessage>,
        initial_files: Vec<PathBuf>,
        quiet: bool,
    ) -> Self {
        theme::apply(&cc.egui_ctx);
        let tray = Tray::new(settings.trigger.label()).ok();
        let watcher = Watcher::start(cc.egui_ctx.clone(), settings.trigger);
        let (job_tx, job_rx) = mpsc::channel();
        let mut welcome = WelcomeWin::default();
        let mut toasts = ToastStack::default();
        let hidden = quiet && settings.welcomed && initial_files.is_empty();
        if !settings.welcomed {
            welcome.open = true;
        } else if !quiet && initial_files.is_empty() {
            toasts.info(
                "DAct is ready",
                format!(
                    "Drop a file here, or hold {} and press the left mouse button. Close the window — DAct stays in the tray.",
                    settings.trigger.label()
                ),
            );
        }
        let mut app = Self {
            settings,
            watcher,
            tray,
            radial: RadialMenu::default(),
            toasts,
            settings_win: SettingsWin::default(),
            welcome,
            compress: None,
            resize: None,
            ipc_rx,
            job_tx,
            job_rx,
            last_tick: Instant::now(),
            queue: Vec::new(),
            thumb: None,
            thumb_src: None,
            enabled: true,
            hidden,
            exit: false,
            tray_hinted: false,
            restore_size: egui::vec2(980.0, 680.0),
            restore_pos: None,
            toast_live: false,
            radial_live: false,
        };
        if !initial_files.is_empty() {
            app.set_queue(initial_files, &cc.egui_ctx);
        }
        app
    }

    fn set_queue(&mut self, files: Vec<PathBuf>, ctx: &egui::Context) {
        self.queue = files;
        self.refresh_thumb(ctx);
    }

    fn add_files(&mut self, files: Vec<PathBuf>, ctx: &egui::Context) {
        for f in files {
            if f.is_file() && !self.queue.contains(&f) {
                self.queue.push(f);
            }
        }
        self.refresh_thumb(ctx);
    }

    fn refresh_thumb(&mut self, ctx: &egui::Context) {
        let Some(path) = self.queue.first().cloned() else {
            self.thumb = None;
            self.thumb_src = None;
            return;
        };
        if self.thumb_src.as_ref() == Some(&path) && self.thumb.is_some() {
            return;
        }
        self.thumb = load_thumb(ctx, &path);
        self.thumb_src = Some(path);
    }

    fn submit(&mut self, action: String, srcs: Vec<PathBuf>, extra: ActionExtra) {
        let names = if srcs.len() == 1 {
            srcs[0]
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            format!("{} files", srcs.len())
        };
        let title = working_label(&action);
        if let Some(tray) = &self.tray {
            tray.set_activity(Some(format!("{title} {names}")));
        }
        let toast = self.toasts.working(&title, &names);
        let settings = self.settings.clone();
        let tx = self.job_tx.clone();
        std::thread::Builder::new()
            .name("dact-job".into())
            .spawn(move || {
                let progress_tx = tx.clone();
                let result = convert::run_action_with_progress(
                    &action,
                    &srcs,
                    &settings,
                    &extra,
                    Some(&mut |p| {
                        let _ = progress_tx.send(JobDone::Progress {
                            toast,
                            fraction: p.fraction(),
                            detail: p.detail(),
                        });
                    }),
                )
                .map(|outs| {
                    let note = extra
                        .quality
                        .map(|q| format!("quality {q}"))
                        .unwrap_or_default();
                    report::summarize(&action, &srcs, &outs, &note)
                })
                .map_err(|e| e.to_string());
                let _ = tx.send(JobDone::Convert { toast, result });
            })
            .ok();
    }

    fn activate(&mut self, item: MenuItem, paths: Vec<PathBuf>) {
        if item.key == "__close__" {
            self.radial.close();
            return;
        }
        self.radial.close();
        if item.key == "tool:Compress" {
            self.compress = Some(CompressWin::new(paths));
            return;
        }
        if item.key == "tool:Resize" {
            self.resize = Some(ResizeWin::new(paths));
            return;
        }
        self.convert_or_fetch_ffmpeg(item.key, paths);
    }

    fn convert_or_fetch_ffmpeg(&mut self, action: String, paths: Vec<PathBuf>) {
        let needs_ff = action.starts_with("media:") || action.starts_with("extract:");
        if needs_ff && crate::ffmpeg::Ffmpeg::resolve(&self.settings).is_err() {
            self.start_ffmpeg_install(Some((action, paths)));
            return;
        }
        self.submit(action, paths, ActionExtra::default());
    }

    fn start_ffmpeg_install(&mut self, retry: Option<(String, Vec<PathBuf>)>) {
        if crate::ffmpeg::Ffmpeg::resolve(&self.settings).is_ok() {
            if let Some((action, paths)) = retry {
                self.submit(action, paths, ActionExtra::default());
            } else {
                self.toasts.info(
                    "FFmpeg is ready",
                    "Video and audio conversion is available.",
                );
            }
            return;
        }
        if let Some(tray) = &self.tray {
            tray.set_activity(Some("Getting FFmpeg…".into()));
        }
        let toast = self.toasts.working(
            "Getting FFmpeg…",
            "Downloading a static build into this user folder.",
        );
        let tx = self.job_tx.clone();
        std::thread::Builder::new()
            .name("dact-ffmpeg".into())
            .spawn(move || {
                let result = crate::ffmpeg::install()
                    .map_err(|e| e.to_string());
                let _ = tx.send(JobDone::Ffmpeg {
                    toast,
                    result,
                    retry,
                });
            })
            .ok();
    }

    fn remember_window(&mut self, ctx: &egui::Context) {
        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            if rect.width() > 200.0 && rect.height() > 200.0 {
                self.restore_size = rect.size();
            }
        }
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            if rect.min.x > -10_000.0 && rect.width() > 200.0 {
                self.restore_pos = Some(rect.min);
            }
        }
    }

    fn reveal(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(egui::vec2(820.0, 560.0)));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(self.restore_size));
        if let Some(pos) = self.restore_pos {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
        }
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        #[cfg(windows)]
        crate::ui::win_input::restore_main_window();
    }

    fn hide_to_tray(&mut self, ctx: &egui::Context, cancel_close: bool) {
        if self.exit {
            return;
        }
        self.remember_window(ctx);
        self.hidden = true;
        if cancel_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }
        // Keep the HWND shown. A hidden winit window stops the message loop,
        // which kills the tray menu and the Explorer dial.
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(egui::vec2(1.0, 1.0)));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(8.0, 8.0)));
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(-32000.0, -32000.0)));
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
        #[cfg(windows)]
        crate::ui::win_input::park_main_window();
        if !self.tray_hinted {
            self.tray_hinted = true;
            self.toasts.info(
                "DAct is in the tray",
                "Conversions keep running. Open the window from the tray icon. Quit only from there.",
            );
        }
    }

    fn clear_activity(&self) {
        if let Some(tray) = &self.tray {
            tray.set_activity(None);
        }
    }

    fn handle_home(&mut self, event: HomeEvent, ctx: &egui::Context) {
        match event {
            HomeEvent::OpenFiles => {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    self.add_files(files, ctx);
                }
            }
            HomeEvent::Clear => {
                self.queue.clear();
                self.thumb = None;
                self.thumb_src = None;
            }
            HomeEvent::Settings => self.settings_win.open = true,
            HomeEvent::Welcome => self.welcome.open = true,
            HomeEvent::Convert(key) => self.convert_or_fetch_ffmpeg(key, self.queue.clone()),
            HomeEvent::InstallFfmpeg => self.start_ffmpeg_install(None),
            HomeEvent::Compress => {
                self.compress = Some(CompressWin::new(self.queue.clone()));
            }
            HomeEvent::Resize => {
                self.resize = Some(ResizeWin::new(self.queue.clone()));
            }
            HomeEvent::Remove(i) => {
                if i < self.queue.len() {
                    self.queue.remove(i);
                    self.thumb_src = None;
                    self.refresh_thumb(ctx);
                }
            }
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let hovered: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .hovered_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !hovered.is_empty() && self.radial.visible && hovered != self.radial.paths {
            let rings = convert::build_rings(&hovered, &self.settings);
            self.radial.load_paths(hovered, rings);
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if dropped.is_empty() || !self.radial.visible {
            return;
        }
        if dropped != self.radial.paths {
            let rings = convert::build_rings(&dropped, &self.settings);
            self.radial.load_paths(dropped.clone(), rings);
        }
        if let crate::ui::radial::Hit::Slice(i) = self.radial.hover {
            if let Some(item) = self.radial.items().get(i).cloned() {
                let paths = self.radial.paths.clone();
                self.activate(item, paths);
            }
        }
    }
}

impl eframe::App for DactApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = self.last_tick.elapsed().as_secs_f32().min(0.05);
        self.last_tick = Instant::now();

        while let Ok(msg) = self.ipc_rx.try_recv() {
            if !msg.files.is_empty() {
                self.add_files(msg.files, ctx);
                self.reveal(ctx);
            } else if msg.show {
                self.reveal(ctx);
                self.toasts.info(
                    "DAct is already running",
                    format!(
                        "It is in the tray. Hold {} and press the left mouse button, or open the window from the tray icon.",
                        self.settings.trigger.label()
                    ),
                );
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.exit {
            if self.tray.is_some() {
                if self.hidden {
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                } else {
                    self.hide_to_tray(ctx, true);
                }
            }
        } else if ctx.input(|i| i.viewport().minimized.unwrap_or(false))
            && !self.hidden
            && !self.exit
            && self.tray.is_some()
        {
            self.hide_to_tray(ctx, false);
        }

        if let Some(tray) = &self.tray {
            for cmd in tray.poll() {
                match cmd {
                    TrayCmd::Show => self.reveal(ctx),
                    TrayCmd::OpenFile => {
                        self.reveal(ctx);
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            self.add_files(files, ctx);
                        }
                    }
                    TrayCmd::Settings => {
                        self.reveal(ctx);
                        self.settings_win.open = true;
                    }
                    TrayCmd::Welcome => {
                        self.reveal(ctx);
                        self.welcome.open = true;
                    }
                    TrayCmd::ToggleEnabled(on) => {
                        self.enabled = on;
                        self.watcher.set_enabled(on);
                    }
                    TrayCmd::Quit => {
                        self.exit = true;
                        crate::ipc::cleanup();
                        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
        }

        while let Ok(done) = self.job_rx.try_recv() {
            match done {
                JobDone::Convert { toast, result } => {
                    self.clear_activity();
                    match result {
                        Ok(card) => self.toasts.finish_ok(toast, card),
                        Err(e) => self.toasts.finish_err(toast, "Couldn't finish", e),
                    }
                }
                JobDone::Progress {
                    toast,
                    fraction,
                    detail,
                } => {
                    if let Some(tray) = &self.tray {
                        tray.set_activity(Some(format!(
                            "{}% — {detail}",
                            (fraction * 100.0).round() as u32
                        )));
                    }
                    self.toasts.set_progress(toast, fraction, detail);
                }
                JobDone::Ffmpeg {
                    toast,
                    result,
                    retry,
                } => match result {
                    Ok(path) => {
                        self.clear_activity();
                        self.settings.ffmpeg_path = path.display().to_string();
                        let _ = self.settings.save();
                        self.toasts.finish_ok(
                            toast,
                            ResultCard {
                                title: "FFmpeg ready".into(),
                                detail: path.display().to_string(),
                                good: "Video and audio conversion is available.".into(),
                                outs: vec![],
                                srcs: vec![],
                            },
                        );
                        if let Some((action, paths)) = retry {
                            self.submit(action, paths, ActionExtra::default());
                        }
                    }
                    Err(e) => {
                        self.clear_activity();
                        self.toasts.finish_err(toast, "Couldn't get FFmpeg", e);
                    }
                },
            }
        }

        self.watcher
            .set_menu_visible(self.radial.visible && !self.radial.click_mode);
        if self.radial.visible && !self.radial.click_mode {
            self.radial.hover_from_screen(self.watcher.cursor());
        }
        let mut pending_hide = None;
        for ev in self.watcher.poll() {
            match ev {
                TriggerEvent::Show { pos, files } => {
                    self.radial.open_at(pos, None, vec![]);
                    if !files.is_empty() {
                        let rings = convert::build_rings(&files, &self.settings);
                        self.radial.load_paths(files, rings);
                    }
                }
                TriggerEvent::ToggleRing => self.radial.next_ring(),
                TriggerEvent::Hide { activate } => pending_hide = Some(activate),
            }
        }

        let hover_drop = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let dropped_main: Vec<PathBuf> = ctx.input(|i| {
            if self.radial.visible {
                vec![]
            } else {
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect()
            }
        });
        if !dropped_main.is_empty() {
            self.add_files(dropped_main, ctx);
            if self.hidden {
                self.reveal(ctx);
            }
        }

        if self.hidden && self.restore_pos.is_none() && !self.exit {
            self.hide_to_tray(ctx, false);
        }

        if self.hidden {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |_| {});
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(theme::INK)
                        .inner_margin(egui::Margin::symmetric(22, 18)),
                )
                .show(ctx, |ui| {
                    let event = home::show(
                        ui,
                        &self.settings,
                        &self.queue,
                        self.thumb.as_ref(),
                        hover_drop,
                    );
                    if let Some(event) = event {
                        self.handle_home(event, ctx);
                    }
                });
        }

        if !self.hidden && self.settings_win.show(ctx, &mut self.settings) {
            self.watcher.set_key(self.settings.trigger);
            if let Some(tray) = &self.tray {
                tray.set_trigger_label(self.settings.trigger.label());
            }
            let _ = self.settings.save();
        }
        if !self.hidden {
            self.welcome
                .show(ctx, self.settings.trigger.label(), &mut self.settings);
        }

        let compress_job = self
            .compress
            .as_mut()
            .filter(|_| !self.hidden)
            .and_then(|win| win.show(ctx).map(|q| (win.paths.clone(), q)));
        if let Some((paths, q)) = compress_job {
            self.submit(
                "tool:Compress".into(),
                paths,
                ActionExtra {
                    quality: Some(q),
                    ..Default::default()
                },
            );
        }
        if self.compress.as_ref().is_some_and(|w| !w.open) {
            self.compress = None;
        }

        let resize_job = self
            .resize
            .as_mut()
            .filter(|_| !self.hidden)
            .and_then(|win| win.show(ctx).map(|v| (win.paths.clone(), v)));
        if let Some((paths, (mode, value))) = resize_job {
            self.submit(
                "tool:Resize".into(),
                paths,
                ActionExtra {
                    resize: Some((mode, value)),
                    ..Default::default()
                },
            );
        }
        if self.resize.as_ref().is_some_and(|w| !w.open) {
            self.resize = None;
        }

        self.toasts.tick();
        let toast_id = ViewportId::from_hash_of("toasts");
        let radial_id = ViewportId::from_hash_of("radial");
        if self.toasts.is_empty() {
            if self.toast_live {
                ctx.send_viewport_cmd_to(toast_id, ViewportCommand::Visible(false));
                ctx.send_viewport_cmd_to(
                    toast_id,
                    ViewportCommand::OuterPosition(egui::pos2(-32000.0, -32000.0)),
                );
                ctx.send_viewport_cmd_to(toast_id, ViewportCommand::InnerSize(egui::vec2(1.0, 1.0)));
                #[cfg(windows)]
                crate::ui::win_input::hide_toast_overlay();
                self.toast_live = false;
            }
        } else {
            let size = self.toasts.viewport_size();
            let pos = crate::ui::notice_anchor(size, ctx.pixels_per_point());
            ctx.show_viewport_immediate(
                toast_id,
                ViewportBuilder::default()
                    .with_title(crate::ui::toast_overlay_title())
                    .with_decorations(false)
                    .with_close_button(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_taskbar(false)
                    .with_active(false)
                    .with_inner_size([size.x, size.y])
                    .with_position(pos),
                |ctx, _class| {
                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
                        pos[0], pos[1],
                    )));
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(
                        egui::WindowLevel::AlwaysOnTop,
                    ));
                    ctx.send_viewport_cmd(ViewportCommand::EnableButtons {
                        close: false,
                        minimized: false,
                        maximize: false,
                    });
                    #[cfg(windows)]
                    {
                        crate::ui::win_input::place_toast_overlay(
                            pos[0] as i32,
                            pos[1] as i32,
                            size.x as i32,
                            size.y as i32,
                        );
                        crate::ui::win_input::harden_toast_overlay(&self.toasts.card_heights());
                    }
                    egui::Area::new(egui::Id::new("dact-toasts"))
                        .fixed_pos(egui::pos2(0.0, 0.0))
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            self.toasts.show_list(ui);
                        });
                },
            );
            self.toast_live = true;
        }

        if !self.radial.visible {
            if self.radial_live {
                ctx.send_viewport_cmd_to(radial_id, ViewportCommand::Visible(false));
                #[cfg(windows)]
                crate::ui::win_input::hide_radial_overlay();
                self.radial_live = false;
            }
        } else {
            let pos = self.radial.pos - egui::vec2(SIZE * 0.5, SIZE * 0.5);
            ctx.show_viewport_immediate(
                radial_id,
                ViewportBuilder::default()
                    .with_title(crate::ui::radial_overlay_title())
                    .with_decorations(false)
                    .with_close_button(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_taskbar(false)
                    .with_active(false)
                    .with_drag_and_drop(true)
                    .with_inner_size([SIZE, SIZE])
                    .with_position([pos.x, pos.y]),
                |ctx, _class| {
                    ctx.send_viewport_cmd(ViewportCommand::EnableButtons {
                        close: false,
                        minimized: false,
                        maximize: false,
                    });
                    #[cfg(windows)]
                    crate::ui::win_input::harden_radial_overlay();
                    egui::Area::new(egui::Id::new("dact-dial"))
                        .fixed_pos(egui::pos2(0.0, 0.0))
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            ui.set_min_size(egui::vec2(SIZE, SIZE));
                            if let Some((item, paths, _)) = self.radial.show(ui, dt) {
                                self.activate(item, paths);
                            }
                        });
                    self.handle_drops(ctx);
                    if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                    {
                        self.radial.close();
                    }
                },
            );
            self.radial_live = true;
        }

        if let Some(activate) = pending_hide {
            if self.radial.visible && !self.radial.click_mode {
                if activate && !self.radial.paths.is_empty() {
                    if let crate::ui::radial::Hit::Slice(i) = self.radial.hover {
                        if let Some(item) = self.radial.items().get(i).cloned() {
                            let paths = self.radial.paths.clone();
                            self.activate(item, paths);
                        }
                    }
                }
                if self.radial.visible && !self.radial.click_mode {
                    self.radial.close();
                }
            }
        }

        if self.radial.visible {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.toasts.needs_spin() {
            ctx.request_repaint_after(Duration::from_millis(50));
        } else if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        } else {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::ipc::cleanup();
    }
}

fn load_thumb(ctx: &egui::Context, path: &std::path::Path) -> Option<TextureHandle> {
    let img = image::open(path).ok()?.thumbnail(320, 200);
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture("dact-thumb", color, egui::TextureOptions::LINEAR))
}

fn working_label(action: &str) -> String {
    let (kind, name) = action.split_once(':').unwrap_or(("", action));
    match (kind, name) {
        ("convert", n) | ("media", n) => format!("Converting to {n}…"),
        ("extract", n) => format!("Extracting {n}…"),
        ("tool", "Compress") => "Compressing…".into(),
        ("tool", "Resize") => "Resizing…".into(),
        ("tool", "Rotate") => "Rotating…".into(),
        ("tool", "Split") => "Splitting…".into(),
        ("tool", "Merge") => "Merging…".into(),
        ("tool", "Extract") => "Extracting images…".into(),
        ("tool", "Metadata") => "Stripping metadata…".into(),
        _ => "Working…".into(),
    }
}

pub fn run_ui(initial_files: Vec<PathBuf>, quiet: bool) -> eframe::Result<()> {
    let settings = Settings::load();
    let (tx, rx) = mpsc::channel();
    if let Err(e) = crate::ipc::listen(tx) {
        eprintln!("IPC listen failed: {e}");
    }
    let start_visible = !quiet || !settings.welcomed || !initial_files.is_empty();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size([980.0, 680.0])
            .with_min_inner_size([820.0, 560.0])
            .with_decorations(true)
            .with_taskbar(start_visible)
            .with_transparent(true)
            .with_visible(true),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            Ok(Box::new(DactApp::new(
                cc,
                settings,
                rx,
                initial_files,
                quiet,
            )))
        }),
    )
}
