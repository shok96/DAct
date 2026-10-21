use eframe::egui::{self, RichText};

use crate::settings::{AudioQuality, OutputMode, Settings, TriggerKey};
use crate::ui::theme;

#[derive(Default)]
pub struct SettingsWin {
    pub open: bool,
}

impl SettingsWin {
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        let mut open = self.open;
        egui::Window::new(crate::i18n::settings_title())
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(crate::i18n::settings_hotkey()).color(theme::TEXT_DIM).size(12.0));
                ui.label(
                    RichText::new(crate::i18n::settings_hotkey_help(settings.trigger.label()))
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
                ui.horizontal(|ui| {
                    for key in [TriggerKey::Shift, TriggerKey::Ctrl, TriggerKey::Alt] {
                        let on = settings.trigger == key;
                        let btn = ui.add(
                            egui::Button::new(
                                RichText::new(key.label())
                                    .color(if on { theme::INK } else { theme::TEXT })
                                    .strong(),
                            )
                            .fill(if on { theme::COPPER } else { theme::PANEL })
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if on { theme::COPPER } else { theme::LINE },
                            )),
                        );
                        if btn.clicked() {
                            settings.trigger = key;
                            changed = true;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new(crate::i18n::settings_save()).color(theme::TEXT_DIM).size(12.0));
                if ui
                    .radio_value(&mut settings.output, OutputMode::NextToOriginal, crate::i18n::settings_next_to())
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .radio_value(&mut settings.output, OutputMode::Subfolder, crate::i18n::settings_subfolder())
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new(crate::i18n::settings_jpeg()).color(theme::TEXT_DIM).size(12.0));
                if ui
                    .add(egui::Slider::new(&mut settings.jpg_quality, 40..=100).suffix("%"))
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new(crate::i18n::settings_audio()).color(theme::TEXT_DIM).size(12.0));
                ui.horizontal(|ui| {
                    for q in [AudioQuality::Low, AudioQuality::Medium, AudioQuality::High] {
                        let label = match q {
                            AudioQuality::Low => crate::i18n::quality_low(),
                            AudioQuality::Medium => crate::i18n::quality_medium(),
                            AudioQuality::High => crate::i18n::quality_high(),
                        };
                        if ui.selectable_label(settings.audio_quality == q, label).clicked() {
                            settings.audio_quality = q;
                            changed = true;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new(crate::i18n::settings_ffmpeg()).color(theme::TEXT_DIM).size(12.0));
                ui.label(
                    RichText::new(crate::i18n::settings_ffmpeg_help())
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut settings.ffmpeg_path).changed() {
                        changed = true;
                    }
                    if ui.button(crate::i18n::browse()).clicked() {
                        let mut dlg = rfd::FileDialog::new().set_title(crate::i18n::ffmpeg_exe_title());
                        #[cfg(windows)]
                        {
                            dlg = dlg.add_filter("FFmpeg", &["exe"]);
                        }
                        if let Some(path) = dlg.pick_file() {
                            settings.ffmpeg_path = path.display().to_string();
                            changed = true;
                        }
                    }
                });
                ui.add_space(8.0);
                if ui
                    .checkbox(&mut settings.autostart, crate::i18n::start_at_login())
                    .changed()
                {
                    changed = true;
                    let _ = crate::autostart::set_enabled(settings.autostart);
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("DAct {}", crate::VERSION))
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
            });
        self.open = open;
        changed
    }
}

#[derive(Default)]
pub struct WelcomeWin {
    pub open: bool,
}

impl WelcomeWin {
    pub fn show(&mut self, ctx: &egui::Context, trigger: &str, settings: &mut Settings) {
        if !self.open {
            return;
        }
        let mut dismiss = false;
        egui::Window::new(crate::i18n::welcome_title())
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(crate::i18n::welcome_lead())
                        .color(theme::COPPER)
                        .size(16.0),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::welcome_step1());
                ui.label(crate::i18n::welcome_step2(trigger));
                ui.label(crate::i18n::welcome_step3());
                ui.label(crate::i18n::welcome_step4(trigger));
                ui.label(crate::i18n::welcome_step5());
                ui.add_space(8.0);
                ui.label(
                    RichText::new(crate::i18n::welcome_privacy())
                        .color(theme::TEXT_DIM)
                        .size(12.0),
                );
                ui.add_space(10.0);
                if ui.button(crate::i18n::got_it()).clicked() {
                    settings.welcomed = true;
                    let _ = settings.save();
                    dismiss = true;
                }
            });
        if dismiss {
            self.open = false;
        }
    }
}

pub struct CompressWin {
    pub open: bool,
    pub paths: Vec<std::path::PathBuf>,
    pub quality: u8,
}

impl CompressWin {
    pub fn new(paths: Vec<std::path::PathBuf>) -> Self {
        Self {
            open: true,
            paths,
            quality: 72,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<u8> {
        if !self.open {
            return None;
        }
        let mut apply = None;
        egui::Window::new(crate::i18n::compress_title())
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(crate::i18n::compress_help());
                ui.add(egui::Slider::new(&mut self.quality, 30..=95).text(crate::i18n::compress_slider()));
                if ui.button(crate::i18n::tool_compress()).clicked() {
                    apply = Some(self.quality);
                }
            });
        if apply.is_some() {
            self.open = false;
        }
        apply
    }
}

pub struct ResizeWin {
    pub open: bool,
    pub paths: Vec<std::path::PathBuf>,
    pub mode: String,
    pub value: u32,
}

impl ResizeWin {
    pub fn new(paths: Vec<std::path::PathBuf>) -> Self {
        Self {
            open: true,
            paths,
            mode: "long".into(),
            value: 1280,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<(String, u32)> {
        if !self.open {
            return None;
        }
        let mut apply = None;
        egui::Window::new(crate::i18n::resize_title())
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (id, label) in [
                        ("pct", crate::i18n::resize_pct()),
                        ("long", crate::i18n::resize_long()),
                        ("w", crate::i18n::resize_w()),
                        ("h", crate::i18n::resize_h()),
                    ] {
                        if ui.selectable_label(self.mode == id, label).clicked() {
                            self.mode = id.into();
                        }
                    }
                });
                ui.add(egui::Slider::new(&mut self.value, 16..=8000).text(crate::i18n::resize_value()));
                if ui.button(crate::i18n::tool_resize()).clicked() {
                    apply = Some((self.mode.clone(), self.value));
                }
            });
        if apply.is_some() {
            self.open = false;
        }
        apply
    }
}
