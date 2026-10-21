use std::f32::consts::TAU;
use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Mesh, Pos2, Sense, Shape, Stroke, Vec2,
};

use crate::convert::{MenuItem, Ring};
use crate::paths::{file_size, human_size};
use crate::ui::theme;

pub const SIZE: f32 = 420.0;
pub const R_OUT: f32 = 158.0;
pub const R_IN: f32 = 62.0;
pub const R_HUB: f32 = 52.0;
const GAP: f32 = 3.2_f32.to_radians();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    None,
    Hub,
    Slice(usize),
}

pub struct RadialMenu {
    pub visible: bool,
    pub click_mode: bool,
    pub got_drag: bool,
    pub pos: Pos2,
    pub paths: Vec<PathBuf>,
    pub rings: Vec<Ring>,
    pub ring_idx: usize,
    pub hover: Hit,
    pub opened_at: Instant,
    pub hub_armed: bool,
    springs: Vec<f32>,
    hub_spring: f32,
}

impl Default for RadialMenu {
    fn default() -> Self {
        Self {
            visible: false,
            click_mode: false,
            got_drag: false,
            pos: Pos2::ZERO,
            paths: vec![],
            rings: vec![],
            ring_idx: 0,
            hover: Hit::None,
            opened_at: Instant::now(),
            hub_armed: false,
            springs: vec![],
            hub_spring: 0.0,
        }
    }
}

impl RadialMenu {
    pub fn open_at(&mut self, pos: Pos2, paths: Option<Vec<PathBuf>>, rings: Vec<Ring>) {
        self.visible = true;
        self.click_mode = paths.is_some();
        self.got_drag = paths.is_some();
        self.pos = pos;
        self.paths = paths.unwrap_or_default();
        self.rings = rings;
        self.ring_idx = 0;
        self.hover = Hit::None;
        self.opened_at = Instant::now();
        self.hub_armed = false;
        self.springs = vec![0.0; self.items().len()];
        self.hub_spring = 0.0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.got_drag = false;
        self.paths.clear();
        self.rings.clear();
        self.hover = Hit::None;
    }

    pub fn items(&self) -> &[MenuItem] {
        self.rings
            .get(self.ring_idx)
            .map(|r| r.items.as_slice())
            .unwrap_or(&[])
    }

    pub fn next_ring(&mut self) {
        if self.rings.len() > 1 {
            self.ring_idx = (self.ring_idx + 1) % self.rings.len();
            self.hover = Hit::None;
            self.springs = vec![0.0; self.items().len()];
        }
    }

    pub fn load_paths(&mut self, paths: Vec<PathBuf>, rings: Vec<Ring>) {
        self.paths = paths;
        self.rings = rings;
        self.ring_idx = 0;
        self.got_drag = true;
        self.springs = vec![0.0; self.items().len()];
    }

    /// Hover from a screen-space cursor. Needed while Explorer owns the mouse.
    pub fn hover_from_screen(&mut self, screen: Pos2) {
        let origin = self.pos - Vec2::splat(SIZE * 0.5);
        let local = Pos2::new(screen.x - origin.x, screen.y - origin.y);
        let h = self.hit(local);
        if h != self.hover {
            if let Hit::Slice(_) = h {
                self.hub_armed = true;
            }
            self.hover = h;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        let n = self.items().len();
        self.springs.resize(n, 0.0);
        for i in 0..n {
            let target = match self.hover {
                Hit::Slice(h) if h == i => 1.0,
                _ => 0.0,
            };
            self.springs[i] += (target - self.springs[i]) * (1.0 - (-12.0 * dt).exp());
        }
        let ht = if self.hover == Hit::Hub { 1.0 } else { 0.0 };
        self.hub_spring += (ht - self.hub_spring) * (1.0 - (-10.0 * dt).exp());
    }

    pub fn hit(&self, local: Pos2) -> Hit {
        let c = Pos2::new(SIZE * 0.5, SIZE * 0.5);
        let d = local - c;
        let dist = d.length();
        if dist <= R_HUB + 4.0 {
            return Hit::Hub;
        }
        if dist > R_OUT + 28.0 || self.items().is_empty() {
            return Hit::None;
        }
        // Same frame as polar()/slice_angles: 0 = 3 o'clock, +angle CCW on screen
        // (Y is flipped). Index 0 sits at 12 o'clock and grows clockwise.
        let ang = ((-d.y).atan2(d.x) + TAU) % TAU;
        let n = self.items().len();
        let span = TAU / n as f32;
        // 12 o'clock is the center of slice 0; index grows clockwise.
        let from_top = (TAU * 0.25 - ang + TAU) % TAU;
        Hit::Slice(((from_top + span * 0.5) / span).floor() as usize % n)
    }

    pub fn hub_text(&self) -> String {
        if self.paths.is_empty() {
            return crate::i18n::drop_file().into();
        }
        if crate::category::uniform(&self.paths).is_none() {
            return crate::i18n::one_type_only().into();
        }
        if self.paths.len() > 1 {
            return crate::i18n::files_n(self.paths.len());
        }
        human_size(file_size(&self.paths[0]))
    }

    pub fn file_name(&self) -> String {
        if self.paths.len() == 1 {
            self.paths[0]
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else if self.paths.is_empty() {
            String::new()
        } else {
            self.rings
                .get(self.ring_idx)
                .map(|r| r.name.clone())
                .unwrap_or_default()
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        dt: f32,
        trigger: &str,
    ) -> Option<(MenuItem, Vec<PathBuf>, bool)> {
        self.tick(dt);
        // On macOS the first overlay frames can miss the mesh; skip the fade
        // so slices are fully opaque as soon as the surface is ready.
        let fade_ms = if cfg!(target_os = "macos") { 0.06 } else { 0.22 };
        let t = (self.opened_at.elapsed().as_secs_f32() / fade_ms).clamp(0.0, 1.0);
        let ease = if cfg!(target_os = "macos") {
            1.0
        } else {
            1.0 - (1.0 - t).powi(3)
        };
        let scale = if cfg!(target_os = "macos") {
            1.0
        } else {
            0.82 + 0.18 * ease
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(SIZE), Sense::click_and_drag());
        let center = rect.center();
        if self.click_mode {
            let pointer = ui.ctx().pointer_latest_pos();
            if let Some(pos) = pointer {
                if rect.contains(pos) {
                    let local = Pos2::new(pos.x - rect.min.x, pos.y - rect.min.y);
                    let h = self.hit(local);
                    if h != self.hover {
                        if let Hit::Slice(_) = h {
                            self.hub_armed = true;
                        }
                        self.hover = h;
                    }
                } else {
                    self.hover = Hit::None;
                }
            }
        }

        paint_dial(ui.painter(), center, self, scale, ease, trigger);

        let mut activate = None;
        if self.click_mode && response.clicked() {
            match self.hover {
                Hit::Hub => self.next_ring(),
                Hit::Slice(i) => {
                    if let Some(item) = self.items().get(i).cloned() {
                        activate = Some((item, self.paths.clone(), true));
                    }
                }
                Hit::None => activate = Some((dummy_close(), vec![], true)),
            }
        }
        if response.secondary_clicked() {
            activate = Some((dummy_close(), vec![], true));
        }
        activate
    }
}

fn dummy_close() -> MenuItem {
    MenuItem {
        label: String::new(),
        hint: String::new(),
        key: "__close__".into(),
    }
}

fn paint_dial(
    painter: &egui::Painter,
    center: Pos2,
    menu: &RadialMenu,
    scale: f32,
    fade: f32,
    trigger: &str,
) {
    let items = menu.items();
    let n = items.len();

    painter.circle_stroke(
        center,
        (R_OUT + 8.0) * scale,
        Stroke::new(1.2_f32, theme::COPPER.gamma_multiply(0.55 * fade)),
    );

    // Engraved ticks
    for i in 0..24 {
        let a = i as f32 / 24.0 * TAU - TAU * 0.25;
        let inner = if i % 3 == 0 { R_OUT + 6.0 } else { R_OUT + 10.0 };
        let p0 = polar(center, inner * scale, a);
        let p1 = polar(center, (R_OUT + 14.0) * scale, a);
        painter.line_segment(
            [p0, p1],
            Stroke::new(
                if i % 3 == 0 { 1.4_f32 } else { 0.8_f32 },
                theme::LINE.gamma_multiply(fade * 0.9),
            ),
        );
    }

    if n == 0 {
        painter.circle_stroke(
            center,
            R_OUT * scale,
            Stroke::new(1.2_f32, theme::COPPER.gamma_multiply(0.35 * fade)),
        );
    }

    for i in 0..n {
        let spring = menu.springs.get(i).copied().unwrap_or(0.0);
        let (a0, a1) = slice_angles(n, i);
        let grow = 1.0 + 0.045 * spring;
        let fill = lerp_color(
            theme::PANEL,
            theme::COPPER,
            0.08 + 0.82 * spring,
        )
        .gamma_multiply(fade);
        let stroke = Stroke::new(
            1.1 + spring,
            lerp_color(theme::LINE, theme::COPPER_HOT, spring).gamma_multiply(fade),
        );
        paint_sector(
            painter,
            center,
            R_IN * scale,
            R_OUT * scale * grow,
            a0,
            a1,
            fill,
            stroke,
        );

        let mid = (a0 + a1) * 0.5;
        let label_r = (R_IN + R_OUT) * 0.5 * scale * grow;
        let pos = polar(center, label_r, mid);
        let item = &items[i];
        let color = if spring > 0.45 {
            theme::INK
        } else {
            theme::TEXT.gamma_multiply(fade)
        };
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            &item.label,
            FontId::new(13.0 + 1.5 * spring, FontFamily::Proportional),
            color,
        );
    }

    let hub_r = (R_HUB + 3.0 * menu.hub_spring) * scale;
    painter.circle_filled(center, hub_r + 3.0, theme::HUB.gamma_multiply(fade));
    painter.circle_stroke(
        center,
        hub_r,
        Stroke::new(
            1.6 + menu.hub_spring,
            lerp_color(theme::COPPER, theme::COPPER_HOT, menu.hub_spring).gamma_multiply(fade),
        ),
    );
    painter.circle_stroke(
        center,
        hub_r * 0.72,
        Stroke::new(0.8_f32, theme::LINE.gamma_multiply(fade * 0.8)),
    );

    let ring_name = menu
        .rings
        .get(menu.ring_idx)
        .map(|r| r.name.as_str())
        .unwrap_or("DAct");
    painter.text(
        center + Vec2::new(0.0, -16.0),
        Align2::CENTER_CENTER,
        ring_name,
        FontId::new(11.0, FontFamily::Proportional),
        theme::COPPER.gamma_multiply(fade),
    );
    painter.text(
        center + Vec2::new(0.0, 2.0),
        Align2::CENTER_CENTER,
        menu.hub_text(),
        FontId::new(15.0, FontFamily::Proportional),
        theme::TEXT.gamma_multiply(fade),
    );

    let chord = crate::i18n::chord(trigger);
    let hint_owned = match menu.hover {
        Hit::Slice(i) => items.get(i).map(|it| it.hint.clone()).unwrap_or_default(),
        _ if menu.file_name().is_empty() => chord,
        _ => String::new(),
    };
    if !hint_owned.is_empty() {
        painter.text(
            center + Vec2::new(0.0, 20.0),
            Align2::CENTER_CENTER,
            hint_owned,
            FontId::new(10.0, FontFamily::Proportional),
            theme::TEXT_DIM.gamma_multiply(fade),
        );
    } else if !menu.file_name().is_empty() && menu.paths.len() == 1 {
        let name = truncate(&menu.file_name(), 18);
        painter.text(
            center + Vec2::new(0.0, 20.0),
            Align2::CENTER_CENTER,
            name,
            FontId::new(10.0, FontFamily::Proportional),
            theme::TEXT_DIM.gamma_multiply(fade),
        );
    }
}

fn slice_angles(n: usize, i: usize) -> (f32, f32) {
    let span = TAU / n as f32;
    let start = TAU * 0.25 + span * 0.5 - (i as f32 + 1.0) * span;
    (start + GAP * 0.5, start + span - GAP * 0.5)
}

fn polar(center: Pos2, r: f32, a: f32) -> Pos2 {
    center + Vec2::new(a.cos(), -a.sin()) * r
}

#[allow(clippy::too_many_arguments)]
fn paint_sector(
    painter: &egui::Painter,
    center: Pos2,
    r_in: f32,
    r_out: f32,
    a0: f32,
    a1: f32,
    fill: Color32,
    stroke: Stroke,
) {
    let steps = ((a1 - a0).abs() / 0.07).ceil().max(8.0) as usize;
    let mut mesh = Mesh::default();
    let mut rim = Vec::new();
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let a = a0 + (a1 - a0) * t;
        let o = polar(center, r_out, a);
        let i = polar(center, r_in, a);
        let io = mesh.vertices.len() as u32;
        let ii = io + 1;
        mesh.colored_vertex(o, fill);
        mesh.colored_vertex(i, fill);
        if s > 0 {
            let prev_o = io - 2;
            let prev_i = io - 1;
            mesh.add_triangle(prev_o, io, prev_i);
            mesh.add_triangle(prev_i, io, ii);
        }
        rim.push(o);
    }
    for s in (0..=steps).rev() {
        let t = s as f32 / steps as f32;
        let a = a0 + (a1 - a0) * t;
        rim.push(polar(center, r_in, a));
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::closed_line(rim, stroke));
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r(), t),
        lerp(a.g(), b.g(), t),
        lerp(a.b(), b.b(), t),
        lerp(a.a(), b.a(), t),
    )
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let take: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{take}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{MenuItem, Ring};

    fn menu_with(n: usize) -> RadialMenu {
        let mut menu = RadialMenu::default();
        menu.rings = vec![Ring {
            name: "t".into(),
            items: (0..n)
                .map(|i| MenuItem {
                    label: i.to_string(),
                    hint: String::new(),
                    key: i.to_string(),
                })
                .collect(),
        }];
        menu
    }

    #[test]
    fn hit_matches_painted_slice() {
        let menu = menu_with(8);
        let c = Pos2::new(SIZE * 0.5, SIZE * 0.5);
        let r = (R_IN + R_OUT) * 0.5;
        for i in 0..8 {
            let (a0, a1) = super::slice_angles(8, i);
            let mid = (a0 + a1) * 0.5;
            let local = super::polar(c, r, mid);
            assert_eq!(menu.hit(local), Hit::Slice(i), "center of slice {i}");
        }
    }
}
