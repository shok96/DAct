use eframe::egui::{self, Color32, FontFamily, FontId, Stroke};

/// Copper on dark. Overlay HWNDs are clipped; unpainted pixels stay clear.
pub const INK: Color32 = Color32::from_rgb(11, 12, 16);
pub const PANEL: Color32 = Color32::from_rgb(20, 22, 28);
pub const LINE: Color32 = Color32::from_rgb(42, 46, 56);
pub const COPPER: Color32 = Color32::from_rgb(212, 137, 74);
pub const COPPER_HOT: Color32 = Color32::from_rgb(240, 176, 106);
pub const EMBER: Color32 = Color32::from_rgb(232, 93, 58);
pub const TEXT: Color32 = Color32::from_rgb(243, 237, 228);
pub const TEXT_DIM: Color32 = Color32::from_rgb(154, 145, 134);
pub const GOOD: Color32 = Color32::from_rgb(124, 184, 154);
pub const BAD: Color32 = Color32::from_rgb(224, 122, 106);
pub const HUB: Color32 = Color32::from_rgb(16, 17, 22);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.window_fill = PANEL;
    style.visuals.panel_fill = INK;
    style.visuals.extreme_bg_color = INK;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 31, 38);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 34, 28);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, COPPER_HOT);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(56, 40, 28);
    style.visuals.selection.bg_fill = COPPER.gamma_multiply(0.45);
    style.visuals.widgets.inactive.corner_radius = 6.0.into();
    style.visuals.widgets.hovered.corner_radius = 6.0.into();
    style.visuals.window_corner_radius = 12.0.into();
    style.visuals.window_stroke = Stroke::new(1.0_f32, LINE);
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}

pub fn label_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
