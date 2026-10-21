use std::path::PathBuf;

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, Frame, RichText, Sense, Stroke, TextureHandle, Ui,
    Vec2,
};

use crate::category::{category, Category};
use crate::convert::{self, Ring};
use crate::ffmpeg::Ffmpeg;
use crate::paths::{file_size, human_size};
use crate::settings::Settings;
use crate::ui::theme;

pub enum HomeEvent {
    OpenFiles,
    Clear,
    Settings,
    Welcome,
    Convert(String),
    Compress,
    Resize,
    Remove(usize),
    InstallFfmpeg,
}

pub fn show(
    ui: &mut Ui,
    settings: &Settings,
    files: &[PathBuf],
    thumb: Option<&TextureHandle>,
    hover_drop: bool,
) -> Option<HomeEvent> {
    let mut event = None;
    let ffmpeg = Ffmpeg::resolve(settings).ok();
    let rings = convert::build_rings(files, settings);
    let cat = files.first().and_then(category);

    ui.vertical(|ui| {
        if event.is_none() {
            event = top_bar(ui, settings, ffmpeg.as_ref());
        }
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            if event.is_none() {
                event = drop_column(&mut cols[0], files, hover_drop, cat);
            }
            if event.is_none() {
                event = action_column(
                    &mut cols[1],
                    files,
                    &rings,
                    cat,
                    ffmpeg.is_some(),
                    thumb,
                );
            }
        });
    });
    event
}

fn top_bar(ui: &mut Ui, settings: &Settings, ffmpeg: Option<&Ffmpeg>) -> Option<HomeEvent> {
    let mut event = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("DACT")
                    .size(22.0)
                    .color(theme::COPPER)
                    .family(FontFamily::Proportional)
                    .strong(),
            );
            ui.label(
                RichText::new(crate::i18n::home_tagline())
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if chip(ui, crate::i18n::how_to_use()).clicked() {
                event = Some(HomeEvent::Welcome);
            }
            if chip(ui, crate::i18n::settings_title()).clicked() {
                event = Some(HomeEvent::Settings);
            }
            let (label, color) = if let Some(ff) = ffmpeg {
                (
                    format!(
                        "FFmpeg  {}",
                        ff.exe
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| crate::i18n::ffmpeg_ready_chip().into())
                    ),
                    theme::GOOD,
                )
            } else {
                (format!("FFmpeg  {}", crate::i18n::ffmpeg_missing_chip()), theme::BAD)
            };
            ui.label(RichText::new(label).size(12.0).color(color));
            if ffmpeg.is_none() && chip(ui, crate::i18n::get_ffmpeg()).clicked() {
                event = Some(HomeEvent::InstallFfmpeg);
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(crate::i18n::chord(settings.trigger.label()))
                    .size(12.0)
                    .color(theme::COPPER_HOT),
            );
        });
    });
    ui.add_space(6.0);
    let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), Sense::hover());
    ui.painter()
        .rect_filled(bar, 0.0, theme::COPPER.gamma_multiply(0.55));
    event
}

fn drop_column(
    ui: &mut Ui,
    files: &[PathBuf],
    hover_drop: bool,
    cat: Option<Category>,
) -> Option<HomeEvent> {
    let mut event = None;
    ui.set_min_width(300.0);
    Frame::new()
        .fill(if hover_drop {
            Color32::from_rgb(36, 30, 22)
        } else {
            theme::PANEL
        })
        .stroke(Stroke::new(
            1.0_f32,
            if hover_drop { theme::COPPER } else { theme::LINE },
        ))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_height(420.0);
            ui.label(
                RichText::new(if files.is_empty() {
                    crate::i18n::drop_files_here()
                } else {
                    crate::i18n::queue()
                })
                .size(16.0)
                .color(theme::TEXT),
            );
            ui.label(
                RichText::new(crate::i18n::drop_kinds())
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if copper_button(ui, crate::i18n::open_files()).clicked() {
                    event = Some(HomeEvent::OpenFiles);
                }
                if !files.is_empty() && quiet_button(ui, crate::i18n::clear()).clicked() {
                    event = Some(HomeEvent::Clear);
                }
            });
            ui.add_space(10.0);
            if files.is_empty() {
                ui.add_space(48.0);
                ui.label(
                    RichText::new(crate::i18n::empty_formats_images())
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
                ui.label(
                    RichText::new(crate::i18n::empty_formats_heic())
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
                ui.label(
                    RichText::new(crate::i18n::empty_formats_media())
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
                ui.label(
                    RichText::new(crate::i18n::empty_formats_docs())
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            } else {
                if let Some(c) = cat {
                    ui.label(
                        RichText::new(category_label(c))
                            .size(12.0)
                            .color(theme::COPPER),
                    );
                    ui.add_space(6.0);
                }
                for (i, path) in files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(
                                path.file_name()
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_default(),
                            )
                            .color(theme::TEXT)
                            .size(13.0),
                        );
                        ui.label(
                            RichText::new(human_size(file_size(path)))
                                .color(theme::TEXT_DIM)
                                .size(11.0),
                        );
                        if quiet_button(ui, "×").clicked() {
                            event = Some(HomeEvent::Remove(i));
                        }
                    });
                }
            }
        });
    event
}

fn action_column(
    ui: &mut Ui,
    files: &[PathBuf],
    rings: &[Ring],
    cat: Option<Category>,
    has_ffmpeg: bool,
    thumb: Option<&TextureHandle>,
) -> Option<HomeEvent> {
    let mut event = None;
    Frame::new()
        .fill(theme::HUB)
        .stroke(Stroke::new(1.0_f32, theme::LINE))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_height(420.0);
            ui.set_min_width(360.0);
            if files.is_empty() {
                ui.label(RichText::new(crate::i18n::no_file()).size(16.0).color(theme::TEXT));
                ui.label(
                    RichText::new(crate::i18n::no_file_help())
                        .size(12.0)
                        .color(theme::TEXT_DIM),
                );
                return;
            }
            if crate::category::uniform(files).is_none() {
                ui.label(
                    RichText::new(crate::i18n::select_one_type())
                        .size(16.0)
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new(crate::i18n::select_one_type_help())
                    .size(12.0)
                    .color(theme::BAD),
                );
                return;
            }
            if files.len() > 1 {
                ui.label(
                    RichText::new(crate::i18n::same_type_n(files.len()))
                        .size(12.0)
                        .color(theme::COPPER),
                );
                ui.add_space(6.0);
            }
            if let Some(tex) = thumb {
                let size = tex.size_vec2();
                let scale = (260.0 / size.x).min(160.0 / size.y).min(1.0);
                ui.add(egui::Image::new(tex).max_size(size * scale));
                ui.add_space(8.0);
            }
            if matches!(cat, Some(Category::Audio | Category::Video)) && !has_ffmpeg {
                ui.label(
                    RichText::new(crate::i18n::need_ffmpeg())
                    .color(theme::BAD)
                    .size(13.0),
                );
                ui.add_space(6.0);
                if copper_button(ui, crate::i18n::get_ffmpeg()).clicked() {
                    event = Some(HomeEvent::InstallFfmpeg);
                }
                ui.add_space(8.0);
            }
            if matches!(cat, Some(Category::Image)) {
                if let Some(p) = files.first() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext.to_ascii_lowercase().as_str(), "heic" | "heif" | "avif")
                        && !has_ffmpeg
                    {
                        ui.label(
                            RichText::new(crate::i18n::heic_ffmpeg())
                                .color(theme::TEXT_DIM)
                                .size(12.0),
                        );
                    }
                }
            }
            for ring in rings {
                ui.label(
                    RichText::new(&ring.name)
                        .size(13.0)
                        .color(theme::COPPER)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for item in &ring.items {
                        let btn = ui.add(action_chip(&item.label));
                        if btn.clicked() {
                            if item.key == "tool:Compress" {
                                event = Some(HomeEvent::Compress);
                            } else if item.key == "tool:Resize" {
                                event = Some(HomeEvent::Resize);
                            } else {
                                event = Some(HomeEvent::Convert(item.key.clone()));
                            }
                        }
                        if !item.hint.is_empty() {
                            btn.on_hover_text(&item.hint);
                        }
                    }
                });
                ui.add_space(12.0);
            }
        });
    event
}

fn category_label(cat: Category) -> &'static str {
    crate::i18n::category_label(cat)
}

fn chip(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(12.0).color(theme::TEXT))
            .fill(theme::PANEL)
            .stroke(Stroke::new(1.0_f32, theme::LINE))
            .corner_radius(8.0),
    )
}

fn quiet_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).size(12.0).color(theme::TEXT_DIM)).frame(false))
}

fn copper_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(theme::INK).strong())
            .fill(theme::COPPER)
            .corner_radius(8.0)
            .min_size(Vec2::new(128.0, 32.0)),
    )
}

fn action_chip(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label.to_string()).size(13.0).color(theme::TEXT))
        .fill(Color32::from_rgb(32, 28, 24))
        .stroke(Stroke::new(1.0_f32, theme::COPPER.gamma_multiply(0.55)))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(72.0, 34.0))
}
