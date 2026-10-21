pub mod app;
pub mod home;
pub mod icon;
pub mod radial;
pub mod theme;
pub mod toasts;
pub mod tray;
pub mod trigger;
pub mod windows;

#[cfg(windows)]
pub mod win_input;

pub fn radial_overlay_title() -> &'static str {
    #[cfg(windows)]
    {
        win_input::RADIAL_TITLE
    }
    #[cfg(not(windows))]
    {
        crate::APP_NAME
    }
}

pub fn toast_overlay_title() -> &'static str {
    #[cfg(windows)]
    {
        win_input::TOAST_TITLE
    }
    #[cfg(not(windows))]
    {
        "DAct Notices"
    }
}

pub fn notice_anchor(size: eframe::egui::Vec2, pixels_per_point: f32) -> [f32; 2] {
    #[cfg(windows)]
    {
        let area = win_input::work_area();
        // Same coordinate space as the radial overlay (GetCursorPos / monitor rects).
        let _ = pixels_per_point;
        [
            area.max.x - size.x - 16.0,
            area.max.y - size.y - 16.0,
        ]
    }
    #[cfg(not(windows))]
    {
        let _ = pixels_per_point;
        [1400.0 - size.x, 860.0 - size.y]
    }
}
