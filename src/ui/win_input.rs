//! Windows input that still works when DAct is unfocused, plus Explorer files.
//! The radial overlay is marked WS_EX_NOACTIVATE so it does not cancel OLE drag.

use std::path::PathBuf;
use std::sync::Mutex;

use eframe::egui::{Pos2, Rect, Vec2};

use windows::core::{Interface, w};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LBUTTON, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    IShellFolderViewDual, IShellWindows, IWebBrowser2, ShellWindows,
};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMSBT_NONE, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateEllipticRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject,
    GetMonitorInfoW, MonitorFromPoint, SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    RGN_OR,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetAncestor, GetClassNameW, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetShellWindow, GetWindowLongW, GetWindowTextW, IsWindow, SetForegroundWindow,
    SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow, WindowFromPoint, GA_ROOT,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, HWND_TOP, HWND_TOPMOST, LWA_COLORKEY, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOWNOACTIVATE,
    WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

use crate::settings::TriggerKey;

pub const RADIAL_TITLE: &str = "DAct Dial";
pub const TOAST_TITLE: &str = "DAct Notices";

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub lbtn: bool,
    pub trigger: bool,
    pub esc: bool,
    pub pos: Pos2,
}

pub fn init_thread() {
    unsafe {
        let _ = OleInitialize(None);
    }
}

pub fn sample(trigger: TriggerKey) -> Sample {
    let vk = match trigger {
        TriggerKey::Shift => VK_SHIFT,
        TriggerKey::Ctrl => VK_CONTROL,
        TriggerKey::Alt => VK_MENU,
    };
    Sample {
        lbtn: down(VK_LBUTTON.0),
        trigger: down(vk.0),
        esc: down(VK_ESCAPE.0),
        pos: cursor(),
    }
}

fn down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

pub fn cursor() -> Pos2 {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    Pos2::new(pt.x as f32, pt.y as f32)
}

/// Files Explorer is dragging or has selected under the cursor.
pub fn explorer_files() -> Vec<PathBuf> {
    match explorer_files_inner() {
        Ok(paths) if !paths.is_empty() => paths,
        _ => Vec::new(),
    }
}

fn explorer_files_inner() -> windows::core::Result<Vec<PathBuf>> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
        let count = windows.Count()?;
        let target = explorer_hwnd_under_cursor();
        let fg = GetForegroundWindow();
        let shell = GetShellWindow();

        let mut under = Vec::new();
        let mut foreground = Vec::new();
        let mut any = Vec::new();

        for i in 0..count {
            let Ok(disp) = windows.Item(&vt_i4(i)) else {
                continue;
            };
            let Ok(browser) = disp.cast::<IWebBrowser2>() else {
                continue;
            };
            let Ok(handle) = browser.HWND() else {
                continue;
            };
            let hwnd = HWND(handle.0 as *mut core::ffi::c_void);
            let files = files_from_browser(&browser).unwrap_or_default();
            if files.is_empty() {
                continue;
            }
            if hwnd_eq(hwnd, target) || is_child_of(target, hwnd) {
                under.extend(files);
            } else if hwnd_eq(hwnd, fg) || hwnd_eq(hwnd, shell) {
                foreground.extend(files);
            } else {
                any.extend(files);
            }
        }

        if !under.is_empty() {
            return Ok(dedup(under));
        }
        if !foreground.is_empty() {
            return Ok(dedup(foreground));
        }
        Ok(dedup(any))
    }
}

fn files_from_browser(browser: &IWebBrowser2) -> windows::core::Result<Vec<PathBuf>> {
    unsafe {
        let doc = browser.Document()?;
        let view = doc.cast::<IShellFolderViewDual>()?;
        let items = view.SelectedItems()?;
        let n = items.Count()?;
        let mut out = Vec::new();
        for i in 0..n {
            let Ok(item) = items.Item(&vt_i4(i)) else {
                continue;
            };
            let Ok(path) = item.Path() else {
                continue;
            };
            let path = PathBuf::from(path.to_string());
            if path.is_file() {
                out.push(path);
            }
        }
        Ok(out)
    }
}

fn explorer_hwnd_under_cursor() -> HWND {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
        let hit = WindowFromPoint(pt);
        if hit.is_invalid() {
            return GetForegroundWindow();
        }
        let root = GetAncestor(hit, GA_ROOT);
        if root.is_invalid() {
            hit
        } else {
            root
        }
    }
}

fn is_child_of(child: HWND, ancestor: HWND) -> bool {
    if child.is_invalid() || ancestor.is_invalid() {
        return false;
    }
    unsafe {
        let mut cur = child;
        for _ in 0..12 {
            if hwnd_eq(cur, ancestor) {
                return true;
            }
            if is_explorer_class(cur) && hwnd_eq(cur, ancestor) {
                return true;
            }
            let next = GetAncestor(cur, GA_ROOT);
            if hwnd_eq(next, cur) || next.is_invalid() {
                break;
            }
            cur = next;
        }
    }
    false
}

fn is_explorer_class(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    if n <= 0 {
        return false;
    }
    let name = String::from_utf16_lossy(&buf[..n as usize]);
    matches!(
        name.as_str(),
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    )
}

fn hwnd_eq(a: HWND, b: HWND) -> bool {
    a.0 == b.0 && !a.is_invalid()
}

fn vt_i4(n: i32) -> VARIANT {
    let mut v = VARIANT::default();
    unsafe {
        let inner = &mut *v.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = n;
    }
    v
}

fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

struct OverlayCache {
    hwnd: isize,
    ghosted: bool,
    w: i32,
    h: i32,
    rgn: u64,
    placed: Option<(i32, i32, i32, i32)>,
}

const EMPTY_CACHE: OverlayCache = OverlayCache {
    hwnd: 0,
    ghosted: false,
    w: 0,
    h: 0,
    rgn: 0,
    placed: None,
};

static RADIAL_CACHE: Mutex<OverlayCache> = Mutex::new(EMPTY_CACHE);
static TOAST_CACHE: Mutex<OverlayCache> = Mutex::new(EMPTY_CACHE);

fn key_of(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn from_key(key: isize) -> HWND {
    HWND(key as *mut core::ffi::c_void)
}

fn live(key: isize) -> Option<HWND> {
    if key == 0 {
        return None;
    }
    let hwnd = from_key(key);
    unsafe {
        if IsWindow(Some(hwnd)).as_bool() {
            Some(hwnd)
        } else {
            None
        }
    }
}

fn remember(cache: &mut OverlayCache, hwnd: HWND) {
    let key = key_of(hwnd);
    if cache.hwnd != key {
        cache.hwnd = key;
        cache.ghosted = false;
        cache.w = 0;
        cache.h = 0;
        cache.rgn = 0;
        cache.placed = None;
    }
}

fn toast_hwnd(cache: &mut OverlayCache) -> Option<HWND> {
    if let Some(hwnd) = live(cache.hwnd) {
        if window_title(hwnd) == TOAST_TITLE {
            return Some(hwnd);
        }
    }
    unsafe {
        let hwnd = FindWindowW(None, w!("DAct Notices")).ok()?;
        if hwnd.is_invalid() {
            return None;
        }
        remember(cache, hwnd);
        Some(hwnd)
    }
}

fn radial_hwnd(cache: &mut OverlayCache) -> Option<HWND> {
    if let Some(hwnd) = live(cache.hwnd) {
        if is_dial_hwnd(hwnd) {
            return Some(hwnd);
        }
    }
    unsafe {
        let hwnd = FindWindowW(None, w!("DAct Dial")).ok()?;
        if hwnd.is_invalid() {
            return None;
        }
        remember(cache, hwnd);
        Some(hwnd)
    }
}

fn toast_rgn_key(w: i32, heights: &[i32]) -> u64 {
    let mut k = w as u64;
    for h in heights {
        k = k.wrapping_mul(16_777_619) ^ (*h as u32 as u64);
    }
    k ^ ((heights.len() as u64) << 32)
}

/// Strip Win11 chrome. No glass, no DWM plate — only painted pixels stay.
unsafe fn ghost_overlay(hwnd: HWND) {
    if hwnd.is_invalid() {
        return;
    }
    let old_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let style = (old_style | WS_POPUP.0 | WS_VISIBLE.0)
        & !(WS_CAPTION.0 | WS_SYSMENU.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0);
    if old_style != style {
        SetWindowLongW(hwnd, GWL_STYLE, style as i32);
    }
    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let want = (ex | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0 | WS_EX_LAYERED.0)
        & !WS_EX_APPWINDOW.0;
    if ex != want {
        SetWindowLongW(hwnd, GWL_EXSTYLE, want as i32);
    }
    let corner = DWMWCP_DONOTROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        (&raw const corner).cast(),
        std::mem::size_of_val(&corner) as u32,
    );
    let border = DWMWA_COLOR_NONE;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_BORDER_COLOR,
        (&raw const border).cast(),
        std::mem::size_of_val(&border) as u32,
    );
    let dark = 1i32;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        (&raw const dark).cast(),
        std::mem::size_of_val(&dark) as u32,
    );
    let backdrop = DWMSBT_NONE;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_SYSTEMBACKDROP_TYPE,
        (&raw const backdrop).cast(),
        std::mem::size_of_val(&backdrop) as u32,
    );
    let margins = MARGINS {
        cxLeftWidth: 0,
        cxRightWidth: 0,
        cyTopHeight: 0,
        cyBottomHeight: 0,
    };
    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    // Empty blur region punches a real hole: unpainted pixels are gone.
    let punch = CreateRectRgn(0, 0, 0, 0);
    let bb = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
        fEnable: true.into(),
        hRgnBlur: punch,
        fTransitionOnMaximized: false.into(),
    };
    let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_COLORKEY);
    if old_style != style || ex != want {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

unsafe fn ensure_ghost(hwnd: HWND, cache: &mut OverlayCache) {
    remember(cache, hwnd);
    if cache.ghosted {
        return;
    }
    ghost_overlay(hwnd);
    cache.ghosted = true;
}

unsafe fn clip_to_ellipse(hwnd: HWND, radius_over_size: f32, redraw: bool) {
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_err() {
        return;
    }
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    if w <= 8 || h <= 8 {
        return;
    }
    let cx = w / 2;
    let cy = h / 2;
    let r = ((w.min(h) as f32) * radius_over_size).round() as i32;
    let hrgn = CreateEllipticRgn(cx - r, cy - r, cx + r + 1, cy + r + 1);
    let _ = SetWindowRgn(hwnd, Some(hrgn), redraw);
}

unsafe fn clip_to_toast_cards(hwnd: HWND, heights: &[i32]) {
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_err() {
        return;
    }
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    if w <= 8 || h <= 8 {
        return;
    }
    const GAP: i32 = 10;
    if heights.len() <= 1 {
        let card_h = heights.first().copied().unwrap_or(h);
        let hrgn = CreateRoundRectRgn(0, 0, w + 1, card_h.min(h) + 1, 32, 32);
        let _ = SetWindowRgn(hwnd, Some(hrgn), true);
        return;
    }
    let combined = CreateRectRgn(0, 0, 0, 0);
    let mut y = 0i32;
    for (i, card_h) in heights.iter().enumerate() {
        let rgn = CreateRoundRectRgn(0, y, w + 1, y + *card_h + 1, 32, 32);
        let _ = CombineRgn(Some(combined), Some(combined), Some(rgn), RGN_OR);
        let _ = DeleteObject(rgn.into());
        y += *card_h;
        if i + 1 < heights.len() {
            y += GAP;
        }
    }
    let _ = SetWindowRgn(hwnd, Some(combined), true);
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if n > 0 {
        String::from_utf16_lossy(&buf[..n as usize])
    } else {
        String::new()
    }
}

fn is_dial_hwnd(hwnd: HWND) -> bool {
    let title = window_title(hwnd);
    title == RADIAL_TITLE
}

/// Keep the dial above Explorer without taking focus, and punch a real hole
/// through the HWND so only the painted circle is visible.
pub fn harden_radial_overlay() {
    let Ok(mut cache) = RADIAL_CACHE.lock() else {
        return;
    };
    let Some(hwnd) = radial_hwnd(&mut cache) else {
        return;
    };
    unsafe {
        ensure_ghost(hwnd, &mut cache);
        let mut rc = RECT::default();
        if GetClientRect(hwnd, &mut rc).is_err() {
            return;
        }
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        let changed = cache.w != w || cache.h != h;
        // Always clip last — SetWindowPos / winit reset the region.
        clip_to_ellipse(hwnd, 168.0 / 420.0, changed);
        cache.w = w;
        cache.h = h;
    }
}

pub fn harden_toast_overlay(card_heights: &[i32]) {
    let Ok(mut cache) = TOAST_CACHE.lock() else {
        return;
    };
    let Some(hwnd) = toast_hwnd(&mut cache) else {
        return;
    };
    unsafe {
        ensure_ghost(hwnd, &mut cache);
        let mut rc = RECT::default();
        if GetClientRect(hwnd, &mut rc).is_err() {
            return;
        }
        let w = rc.right - rc.left;
        let key = toast_rgn_key(w, card_heights);
        clip_to_toast_cards(hwnd, card_heights);
        cache.rgn = key;
    }
}

fn hide_hwnd(hwnd: HWND) {
    if hwnd.is_invalid() {
        return;
    }
    unsafe {
        let _ = SetWindowRgn(hwnd, None, true);
        let _ = ShowWindow(hwnd, SW_HIDE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            -32000,
            -32000,
            1,
            1,
            SWP_NOACTIVATE,
        );
    }
}

pub fn hide_toast_overlay() {
    let Ok(mut cache) = TOAST_CACHE.lock() else {
        return;
    };
    if let Some(hwnd) = toast_hwnd(&mut cache) {
        hide_hwnd(hwnd);
    }
    *cache = EMPTY_CACHE;
}

pub fn hide_radial_overlay() {
    let Ok(mut cache) = RADIAL_CACHE.lock() else {
        return;
    };
    if let Some(hwnd) = radial_hwnd(&mut cache) {
        hide_hwnd(hwnd);
    }
    *cache = EMPTY_CACHE;
}

fn find_main() -> Option<HWND> {
    unsafe {
        let hwnd = FindWindowW(None, w!("DAct")).ok()?;
        if hwnd.is_invalid() {
            None
        } else {
            Some(hwnd)
        }
    }
}

fn set_ex_style(hwnd: HWND, add: u32, remove: u32) {
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let want = (ex | add) & !remove;
        if ex != want {
            SetWindowLongW(hwnd, GWL_EXSTYLE, want as i32);
        }
    }
}

/// Keep the HWND shown so winit still pumps messages (tray + radial),
/// but park it off-screen and off the taskbar.
pub fn park_main_window() {
    let Some(hwnd) = find_main() else {
        return;
    };
    set_ex_style(hwnd, WS_EX_TOOLWINDOW.0, WS_EX_APPWINDOW.0);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            -32000,
            -32000,
            8,
            8,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub fn work_area() -> Rect {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(mon, &mut info).as_bool() {
            let r = info.rcWork;
            return Rect::from_min_max(
                Pos2::new(r.left as f32, r.top as f32),
                Pos2::new(r.right as f32, r.bottom as f32),
            );
        }
    }
    Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0))
}

pub fn place_toast_overlay(x: i32, y: i32, w: i32, h: i32) {
    if w < 8 || h < 8 {
        return;
    }
    let Ok(mut cache) = TOAST_CACHE.lock() else {
        return;
    };
    if cache.placed == Some((x, y, w, h)) {
        return;
    }
    let Some(hwnd) = toast_hwnd(&mut cache) else {
        return;
    };
    unsafe {
        ensure_ghost(hwnd, &mut cache);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    cache.placed = Some((x, y, w, h));
    cache.rgn = 0;
}

pub fn restore_main_window() {
    let Some(hwnd) = find_main() else {
        return;
    };
    set_ex_style(hwnd, WS_EX_APPWINDOW.0, WS_EX_TOOLWINDOW.0);
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        let _ = SetForegroundWindow(hwnd);
    }
}
