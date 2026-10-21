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
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("Hotkey").color(theme::TEXT_DIM).size(12.0));
                ui.label(
                    RichText::new("Hold this key and press the left mouse button. Change it any time.")
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
                ui.label(RichText::new("Save results").color(theme::TEXT_DIM).size(12.0));
                if ui
                    .radio_value(&mut settings.output, OutputMode::NextToOriginal, "Next to the original")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .radio_value(&mut settings.output, OutputMode::Subfolder, "In a DAct subfolder")
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new("JPEG quality").color(theme::TEXT_DIM).size(12.0));
                if ui
                    .add(egui::Slider::new(&mut settings.jpg_quality, 40..=100).suffix("%"))
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Audio quality").color(theme::TEXT_DIM).size(12.0));
                ui.horizontal(|ui| {
                    for q in [AudioQuality::Low, AudioQuality::Medium, AudioQuality::High] {
                        let label = match q {
                            AudioQuality::Low => "Low",
                            AudioQuality::Medium => "Medium",
                            AudioQuality::High => "High",
                        };
                        if ui.selectable_label(settings.audio_quality == q, label).clicked() {
                            settings.audio_quality = q;
                            changed = true;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("FFmpeg path (optional)").color(theme::TEXT_DIM).size(12.0));
                ui.label(
                    RichText::new("Local binary only. Get FFmpeg on the home screen copies the official Windows build into this user folder — never on launch.")
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut settings.ffmpeg_path).changed() {
                        changed = true;
                    }
                    if ui.button("Browse").clicked() {
                        let mut dlg = rfd::FileDialog::new().set_title("FFmpeg executable");
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
                    .checkbox(&mut settings.autostart, "Start DAct when I sign in")
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
        egui::Window::new("How to use DAct")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("A copper dial instead of a glass disc.")
                        .color(theme::COPPER)
                        .size(16.0),
                );
                ui.add_space(8.0);
                ui.label("1. Select a file in Explorer, or start dragging it.");
                ui.label(format!("2. Hold {trigger} and press the left mouse button. The dial opens under the cursor."));
                ui.label("3. Keep the button down, move onto a slice, release. The original is never changed.");
                ui.label(format!("4. Tap {trigger} again to flip Convert ↔ Tools."));
                ui.label("5. Close or minimize this window — DAct stays in the tray. Quit only from the tray icon.");
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Progress and results appear as system notifications. DAct does not capture the desktop and does not phone home. FFmpeg is fetched only if you press Get FFmpeg.")
                        .color(theme::TEXT_DIM)
                        .size(12.0),
                );
                ui.add_space(10.0);
                if ui.button("Got it").clicked() {
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
        egui::Window::new("Compress")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Quality for the new copy. The original stays put.");
                ui.add(egui::Slider::new(&mut self.quality, 30..=95).text("quality"));
                if ui.button("Compress").clicked() {
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
        egui::Window::new("Resize")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (id, label) in [
                        ("pct", "Percent"),
                        ("long", "Long side"),
                        ("w", "Width"),
                        ("h", "Height"),
                    ] {
                        if ui.selectable_label(self.mode == id, label).clicked() {
                            self.mode = id.into();
                        }
                    }
                });
                ui.add(egui::Slider::new(&mut self.value, 16..=8000).text("value"));
                if ui.button("Resize").clicked() {
                    apply = Some((self.mode.clone(), self.value));
                }
            });
        if apply.is_some() {
            self.open = false;
        }
        apply
    }
}
