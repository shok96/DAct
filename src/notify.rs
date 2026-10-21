//! Optional process identity for Windows. Job notices are drawn as overlay cards.

/// Windows toast grouping. Call once at process start.
#[cfg(windows)]
pub fn set_app_id() {
    use windows::core::w;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(w!("DAct.Desktop"));
    }
}

#[cfg(not(windows))]
pub fn set_app_id() {}
