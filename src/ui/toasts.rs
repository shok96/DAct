use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use eframe::egui::{self, CornerRadius, FontFamily, Pos2, RichText, Sense, Stroke, Vec2};

use crate::report::ResultCard;
use crate::reveal;
use crate::ui::theme;

pub const CARD_W: f32 = 380.0;
const GAP: f32 = 10.0;

#[derive(Debug, Clone)]
enum ToastKind {
    Working {
        title: String,
        detail: String,
        fraction: Option<f32>,
    },
    Done(ResultCard),
    Error {
        title: String,
        detail: String,
    },
    Info {
        title: String,
        detail: String,
    },
}

#[derive(Debug, Clone)]
struct Toast {
    kind: ToastKind,
    born: Instant,
    closing: bool,
}

impl Toast {
    fn ttl(&self) -> Duration {
        match self.kind {
            ToastKind::Working { .. } => Duration::from_secs(24 * 3600),
            ToastKind::Done(_) => Duration::from_secs(14),
            _ => Duration::from_secs(8),
        }
    }

    fn height(&self) -> f32 {
        match &self.kind {
            ToastKind::Done(c) if !c.outs.is_empty() => 122.0,
            ToastKind::Working { fraction, .. } if fraction.is_some() => 108.0,
            _ => 92.0,
        }
    }
}

#[derive(Default)]
pub struct ToastStack {
    items: Vec<Toast>,
}

impl ToastStack {
    pub fn working(&mut self, title: impl Into<String>, detail: impl Into<String>) -> usize {
        self.items.push(Toast {
            kind: ToastKind::Working {
                title: title.into(),
                detail: detail.into(),
                fraction: None,
            },
            born: Instant::now(),
            closing: false,
        });
        self.items.len() - 1
    }

    pub fn set_progress(&mut self, idx: usize, fraction: f32, detail: impl Into<String>) {
        if let Some(Toast {
            kind: ToastKind::Working {
                detail: d,
                fraction: f,
                ..
            },
            ..
        }) = self.items.get_mut(idx)
        {
            *d = detail.into();
            *f = Some(fraction.clamp(0.0, 1.0));
        }
    }

    pub fn finish_ok(&mut self, idx: usize, card: ResultCard) {
        if let Some(t) = self.items.get_mut(idx) {
            t.kind = ToastKind::Done(card);
            t.born = Instant::now();
        }
    }

    pub fn finish_err(&mut self, idx: usize, title: impl Into<String>, detail: impl Into<String>) {
        if let Some(t) = self.items.get_mut(idx) {
            t.kind = ToastKind::Error {
                title: title.into(),
                detail: detail.into(),
            };
            t.born = Instant::now();
        }
    }

    pub fn info(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.items.push(Toast {
            kind: ToastKind::Info {
                title: title.into(),
                detail: detail.into(),
            },
            born: Instant::now(),
            closing: false,
        });
    }

    pub fn error(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.items.push(Toast {
            kind: ToastKind::Error {
                title: title.into(),
                detail: detail.into(),
            },
            born: Instant::now(),
            closing: false,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn needs_spin(&self) -> bool {
        self.items
            .iter()
            .any(|t| matches!(t.kind, ToastKind::Working { .. }))
    }

    pub fn tick(&mut self) {
        self.items
            .retain(|t| t.born.elapsed() < t.ttl() && !t.closing);
    }

    pub fn viewport_size(&self) -> Vec2 {
        let h = self.items.iter().map(|t| t.height()).sum::<f32>()
            + GAP * self.items.len().saturating_sub(1) as f32;
        Vec2::new(CARD_W, h.max(1.0))
    }

    pub fn card_heights(&self) -> Vec<i32> {
        self.items.iter().map(|t| t.height() as i32).collect()
    }

    pub fn show_list(&mut self, ui: &mut egui::Ui) {
        self.tick();
        let n = self.items.len();
        for (i, toast) in self.items.iter_mut().enumerate() {
            draw_toast(ui, toast, i);
            if i + 1 < n {
                ui.add_space(GAP);
            }
        }
    }
}

fn draw_toast(ui: &mut egui::Ui, toast: &mut Toast, idx: usize) {
    let h = toast.height();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(CARD_W, h), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(16), theme::PANEL);
    painter.rect_stroke(
        rect,
        CornerRadius::same(16),
        Stroke::new(1.0_f32, theme::LINE),
        egui::StrokeKind::Inside,
    );

    let icon = egui::Rect::from_center_size(
        Pos2::new(rect.min.x + 36.0, rect.min.y + 36.0),
        Vec2::splat(36.0),
    );
    match &toast.kind {
        ToastKind::Working { .. } => {
            let spin = toast.born.elapsed().as_secs_f32() * TAU;
            let c = icon.center();
            painter.circle_stroke(c, 13.0, Stroke::new(3.0_f32, theme::LINE));
            let a0 = spin;
            let a1 = spin + 1.9;
            let mut pts = Vec::new();
            for s in 0..=10 {
                let t = s as f32 / 10.0;
                let a = a0 + (a1 - a0) * t;
                pts.push(c + Vec2::new(a.cos(), a.sin()) * 13.0);
            }
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(3.0_f32, theme::COPPER),
            ));
        }
        ToastKind::Error { .. } => {
            painter.rect_filled(icon, CornerRadius::same(10), theme::BAD.gamma_multiply(0.25));
            painter.circle_filled(icon.center(), 5.0, theme::BAD);
        }
        ToastKind::Done(_) | ToastKind::Info { .. } => {
            painter.rect_filled(icon, CornerRadius::same(10), theme::COPPER.gamma_multiply(0.22));
            painter.circle_filled(icon.center(), 5.0, theme::COPPER);
        }
    }

    if let ToastKind::Working {
        fraction: Some(f), ..
    } = &toast.kind
    {
        let track = egui::Rect::from_min_size(
            Pos2::new(rect.min.x + 16.0, rect.max.y - 16.0),
            Vec2::new(CARD_W - 32.0, 5.0),
        );
        painter.rect_filled(track, 3.0, theme::LINE);
        painter.rect_filled(
            egui::Rect::from_min_size(track.min, Vec2::new(track.width() * *f, 5.0)),
            3.0,
            theme::COPPER,
        );
    }

    let (title, detail, good, outs) = match &toast.kind {
        ToastKind::Working { title, detail, .. } => {
            (title.as_str(), detail.as_str(), "", None)
        }
        ToastKind::Done(c) => (
            c.title.as_str(),
            c.detail.as_str(),
            c.good.as_str(),
            Some(c.outs.clone()),
        ),
        ToastKind::Error { title, detail } | ToastKind::Info { title, detail } => {
            (title.as_str(), detail.as_str(), "", None)
        }
    };

    let text_rect = egui::Rect::from_min_max(
        Pos2::new(rect.min.x + 64.0, rect.min.y + 12.0),
        Pos2::new(rect.max.x - 36.0, rect.max.y - 10.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .color(theme::TEXT)
                    .size(15.0)
                    .family(FontFamily::Proportional)
                    .strong(),
            );
            if !good.is_empty() {
                let galley = ui.fonts(|f| {
                    f.layout_no_wrap(
                        good.to_string(),
                        egui::FontId::new(11.0, FontFamily::Proportional),
                        theme::INK,
                    )
                });
                let chip = ui.allocate_exact_size(
                    galley.size() + Vec2::new(12.0, 6.0),
                    Sense::hover(),
                );
                ui.painter()
                    .rect_filled(chip.0, 8.0, theme::GOOD);
                ui.painter().galley(
                    chip.0.center() - galley.size() * 0.5,
                    galley,
                    theme::INK,
                );
            }
        });
        ui.label(RichText::new(detail).color(theme::TEXT_DIM).size(12.0));
        if let Some(outs) = outs {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if pill(ui, crate::i18n::open()).clicked() {
                    if let Some(p) = outs.first() {
                        reveal::open_file(p);
                    }
                }
                if pill(ui, crate::i18n::show_in_folder()).clicked() {
                    if let Some(p) = outs.first() {
                        reveal::reveal(p);
                    }
                }
                if pill(ui, crate::i18n::undo()).clicked() {
                    reveal::trash_paths(&outs);
                    toast.closing = true;
                }
            });
        }
    });

    let close = egui::Rect::from_center_size(
        Pos2::new(rect.max.x - 18.0, rect.min.y + 18.0),
        Vec2::splat(18.0),
    );
    let close_r = ui.interact(close, ui.id().with(("toast-x", idx)), Sense::click());
    ui.painter().text(
        close.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        theme::label_font(16.0),
        if close_r.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    );
    if close_r.clicked() {
        toast.closing = true;
    }
}

fn pill(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(theme::TEXT).size(12.0))
            .fill(theme::LINE)
            .corner_radius(14.0)
            .min_size(Vec2::new(0.0, 26.0)),
    )
}
