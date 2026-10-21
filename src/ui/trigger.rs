use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eframe::egui::{Context, Pos2};

use crate::settings::TriggerKey;

#[derive(Debug, Clone)]
pub enum TriggerEvent {
    Show { pos: Pos2, files: Vec<PathBuf> },
    Files(Vec<PathBuf>),
    ToggleRing,
    Hide { activate: bool },
}

pub struct Watcher {
    rx: Receiver<TriggerEvent>,
    cursor: Arc<Mutex<Pos2>>,
    enabled: Arc<AtomicBool>,
    key: Arc<Mutex<TriggerKey>>,
    menu_visible: Arc<AtomicBool>,
}

impl Watcher {
    pub fn start(ctx: Context, trigger: TriggerKey) -> Self {
        let (tx, rx) = mpsc::channel();
        let cursor = Arc::new(Mutex::new(Pos2::ZERO));
        let enabled = Arc::new(AtomicBool::new(true));
        let key = Arc::new(Mutex::new(trigger));
        let menu_visible = Arc::new(AtomicBool::new(false));
        thread::Builder::new()
            .name("dact-input".into())
            .spawn({
                let cursor = Arc::clone(&cursor);
                let enabled = Arc::clone(&enabled);
                let key = Arc::clone(&key);
                let menu_visible = Arc::clone(&menu_visible);
                move || run_loop(ctx, tx, cursor, enabled, key, menu_visible)
            })
            .ok();
        Self {
            rx,
            cursor,
            enabled,
            key,
            menu_visible,
        }
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        crate::ui::mac_input::set_enabled(on);
    }

    pub fn set_key(&self, key: TriggerKey) {
        if let Ok(mut g) = self.key.lock() {
            *g = key;
        }
        #[cfg(target_os = "macos")]
        crate::ui::mac_input::set_trigger(key);
    }

    pub fn set_menu_visible(&self, visible: bool) {
        self.menu_visible.store(visible, Ordering::Relaxed);
    }

    pub fn cursor(&self) -> Pos2 {
        self.cursor.lock().map(|g| *g).unwrap_or(Pos2::ZERO)
    }

    pub fn poll(&self) -> Vec<TriggerEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

fn run_loop(
    ctx: Context,
    tx: Sender<TriggerEvent>,
    cursor: Arc<Mutex<Pos2>>,
    enabled: Arc<AtomicBool>,
    key: Arc<Mutex<TriggerKey>>,
    menu_visible: Arc<AtomicBool>,
) {
    #[cfg(windows)]
    crate::ui::win_input::init_thread();

    #[cfg(target_os = "macos")]
    crate::ui::mac_input::init(key.lock().map(|g| *g).unwrap_or(TriggerKey::Alt));

    #[cfg(all(not(windows), not(target_os = "macos")))]
    let device = device_query::DeviceState::new();

    let mut trigger_was = false;
    let mut chord_was = false;
    let mut awaiting = false;
    #[cfg(target_os = "macos")]
    let finder_cache: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    #[cfg(target_os = "macos")]
    let finder_busy = Arc::new(AtomicBool::new(false));
    #[cfg(target_os = "macos")]
    let mut last_finder = std::time::Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(std::time::Instant::now);

    loop {
        let trigger_key = key.lock().map(|g| *g).unwrap_or(TriggerKey::Alt);
        let sample = read_sample(
            #[cfg(all(not(windows), not(target_os = "macos")))]
            &device,
            trigger_key,
        );
        if let Ok(mut g) = cursor.lock() {
            *g = sample.pos;
        }

        let visible = menu_visible.load(Ordering::Relaxed);
        let chord = sample.trigger && sample.lbtn;
        let mut event = None;

        if visible {
            awaiting = false;
            if sample.esc {
                event = Some(TriggerEvent::Hide { activate: false });
            } else if !sample.lbtn {
                event = Some(TriggerEvent::Hide { activate: true });
            } else if sample.trigger && !trigger_was {
                event = Some(TriggerEvent::ToggleRing);
            }
        } else if awaiting {
            if sample.esc {
                event = Some(TriggerEvent::Hide { activate: false });
                awaiting = false;
            } else if !sample.lbtn {
                event = Some(TriggerEvent::Hide { activate: true });
                awaiting = false;
            }
        } else if enabled.load(Ordering::Relaxed) && chord && !chord_was {
            awaiting = true;
            event = Some(TriggerEvent::Show {
                pos: sample.pos,
                files: current_files(
                    #[cfg(target_os = "macos")]
                    &finder_cache,
                ),
            });
        }

        #[cfg(target_os = "macos")]
        {
            let due = last_finder.elapsed() >= Duration::from_millis(450);
            if (sample.trigger && !trigger_was) || (due && enabled.load(Ordering::Relaxed)) {
                if prefetch_finder(
                    Arc::clone(&finder_cache),
                    Arc::clone(&finder_busy),
                    tx.clone(),
                    ctx.clone(),
                ) {
                    last_finder = std::time::Instant::now();
                }
            }
        }

        trigger_was = sample.trigger;
        chord_was = chord;

        if let Some(ev) = event {
            let _ = tx.send(ev);
            ctx.request_repaint();
        } else if visible || awaiting {
            ctx.request_repaint();
        }

        thread::sleep(Duration::from_millis(8));
    }
}

struct Sample {
    lbtn: bool,
    trigger: bool,
    esc: bool,
    pos: Pos2,
}

fn read_sample(
    #[cfg(all(not(windows), not(target_os = "macos")))] device: &device_query::DeviceState,
    trigger: TriggerKey,
) -> Sample {
    #[cfg(windows)]
    {
        let s = crate::ui::win_input::sample(trigger);
        return Sample {
            lbtn: s.lbtn,
            trigger: s.trigger,
            esc: s.esc,
            pos: s.pos,
        };
    }
    #[cfg(target_os = "macos")]
    {
        let s = crate::ui::mac_input::sample(trigger);
        return Sample {
            lbtn: s.lbtn,
            trigger: s.trigger,
            esc: s.esc,
            pos: s.pos,
        };
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        use device_query::{DeviceQuery, Keycode};
        let mouse = device.get_mouse();
        let keys = device.get_keys();
        let lbtn = mouse.button_pressed.get(1).copied().unwrap_or(false);
        let trigger_down = match trigger {
            TriggerKey::Shift => {
                keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift)
            }
            TriggerKey::Ctrl => {
                keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl)
            }
            TriggerKey::Alt => keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt),
        };
        Sample {
            lbtn,
            trigger: trigger_down,
            esc: keys.contains(&Keycode::Escape),
            pos: Pos2::new(mouse.coords.0 as f32, mouse.coords.1 as f32),
        }
    }
}

fn current_files(
    #[cfg(target_os = "macos")] cache: &Arc<Mutex<Vec<PathBuf>>>,
) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        crate::ui::win_input::explorer_files()
    }
    #[cfg(target_os = "macos")]
    {
        cache
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.is_file())
            .collect()
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn prefetch_finder(
    cache: Arc<Mutex<Vec<PathBuf>>>,
    busy: Arc<AtomicBool>,
    tx: Sender<TriggerEvent>,
    ctx: Context,
) -> bool {
    if busy.swap(true, Ordering::SeqCst) {
        return false;
    }
    thread::Builder::new()
        .name("dact-finder".into())
        .spawn(move || {
            let files = query_finder_selection();
            if let Ok(mut g) = cache.lock() {
                *g = files.clone();
            }
            if !files.is_empty() {
                let _ = tx.send(TriggerEvent::Files(files));
                ctx.request_repaint();
            }
            busy.store(false, Ordering::SeqCst);
        })
        .is_ok()
}

#[cfg(target_os = "macos")]
fn query_finder_selection() -> Vec<PathBuf> {
    let script = r#"
tell application "System Events"
    try
        set frontApp to name of first application process whose frontmost is true
    on error
        return ""
    end try
end tell
if frontApp is not "Finder" then return ""
tell application "Finder"
    set theSel to selection
    if (count of theSel) is 0 then return ""
    set out to ""
    repeat with f in theSel
        try
            set out to out & POSIX path of (f as alias) & linefeed
        end try
    end repeat
    return out
end tell
"#;
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}
