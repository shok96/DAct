//! macOS modifiers and click swallow.
//!
//! Option is `kCGEventFlagMaskAlternate`, not Alt. Control+click is turned
//! into a right-click by the window server; the event tap eats that click
//! so Finder does not open its menu.

use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;

use eframe::egui::Pos2;

use crate::settings::TriggerKey;

const HID_STATE: u32 = 1;
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALTERNATE: u64 = 0x0008_0000;
const ESC_KEY: u16 = 53;

const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const TAP_DISABLED_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_USER: u32 = 0xFFFF_FFFF;

const TAP_HID: u32 = 0;
const TAP_HEAD: u32 = 0;
const TAP_DEFAULT: u32 = 0;

const TRIGGER_SHIFT: u8 = 0;
const TRIGGER_CTRL: u8 = 1;
const TRIGGER_ALT: u8 = 2;

static ENABLED: AtomicBool = AtomicBool::new(true);
static TRIGGER: AtomicU8 = AtomicU8::new(TRIGGER_ALT);
static TAP: std::sync::atomic::AtomicPtr<c_void> = std::sync::atomic::AtomicPtr::new(ptr::null_mut());

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: u32) -> u64;
    fn CGEventSourceButtonState(state_id: u32, button: u32) -> bool;
    fn CGEventSourceKeyState(state_id: u32, key: u16) -> bool;
    fn CGEventCreate(source: *mut c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CGEventGetFlags(event: *mut c_void) -> u64;
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
        refcon: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *mut c_void);
    fn CFRunLoopRun();
    fn CFRelease(cf: *mut c_void);
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
    static kCFBooleanTrue: *const c_void;
    static kCFAllocatorDefault: *const c_void;
    static kCFRunLoopCommonModes: *mut c_void;
    fn CFDictionaryCreateMutable(
        allocator: *mut c_void,
        capacity: isize,
        key_call_backs: *const c_void,
        value_call_backs: *const c_void,
    ) -> *mut c_void;
    fn CFDictionarySetValue(dict: *mut c_void, key: *const c_void, value: *const c_void);
}

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub lbtn: bool,
    pub trigger: bool,
    pub esc: bool,
    pub pos: Pos2,
}

pub fn init(trigger: TriggerKey) {
    set_trigger(trigger);
    prompt_accessibility();
    thread::Builder::new()
        .name("dact-mac-tap".into())
        .spawn(run_tap)
        .ok();
}

pub fn set_trigger(key: TriggerKey) {
    let v = match key {
        TriggerKey::Shift => TRIGGER_SHIFT,
        TriggerKey::Ctrl => TRIGGER_CTRL,
        TriggerKey::Alt => TRIGGER_ALT,
    };
    TRIGGER.store(v, Ordering::Relaxed);
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn sample(trigger: TriggerKey) -> Sample {
    set_trigger(trigger);
    let flags = unsafe { CGEventSourceFlagsState(HID_STATE) };
    let left = unsafe { CGEventSourceButtonState(HID_STATE, 0) };
    let right = unsafe { CGEventSourceButtonState(HID_STATE, 1) };
    let trigger_down = match trigger {
        TriggerKey::Shift => flags & FLAG_SHIFT != 0,
        TriggerKey::Ctrl => flags & FLAG_CONTROL != 0,
        TriggerKey::Alt => flags & FLAG_ALTERNATE != 0,
    };
    // Control+click is delivered as a right-click. Count either button.
    let lbtn = left || (matches!(trigger, TriggerKey::Ctrl) && right);
    Sample {
        lbtn,
        trigger: trigger_down,
        esc: unsafe { CGEventSourceKeyState(HID_STATE, ESC_KEY) },
        pos: cursor(),
    }
}

fn cursor() -> Pos2 {
    unsafe {
        let ev = CGEventCreate(ptr::null_mut());
        if ev.is_null() {
            return Pos2::ZERO;
        }
        let p = CGEventGetLocation(ev);
        CFRelease(ev);
        Pos2::new(p.x as f32, p.y as f32)
    }
}

fn prompt_accessibility() {
    unsafe {
        let dict = CFDictionaryCreateMutable(
            kCFAllocatorDefault as *mut c_void,
            1,
            ptr::null(),
            ptr::null(),
        );
        if dict.is_null() {
            return;
        }
        CFDictionarySetValue(dict, kAXTrustedCheckOptionPrompt, kCFBooleanTrue);
        let _ = AXIsProcessTrustedWithOptions(dict as *const c_void);
        CFRelease(dict);
    }
}

fn run_tap() {
    let mask = (1u64 << LEFT_DOWN)
        | (1u64 << LEFT_UP)
        | (1u64 << RIGHT_DOWN)
        | (1u64 << RIGHT_UP);
    unsafe {
        let tap = CGEventTapCreate(
            TAP_HID,
            TAP_HEAD,
            TAP_DEFAULT,
            mask,
            tap_callback,
            ptr::null_mut(),
        );
        if tap.is_null() {
            return;
        }
        TAP.store(tap, Ordering::SeqCst);
        let src = CFMachPortCreateRunLoopSource(kCFAllocatorDefault as *mut c_void, tap, 0);
        if src.is_null() {
            CFRelease(tap);
            TAP.store(ptr::null_mut(), Ordering::SeqCst);
            return;
        }
        CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        CFRunLoopRun();
    }
}

extern "C" fn tap_callback(
    proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    _refcon: *mut c_void,
) -> *mut c_void {
    if etype == TAP_DISABLED_TIMEOUT || etype == TAP_DISABLED_USER {
        let tap = TAP.load(Ordering::SeqCst);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
        }
        let _ = proxy;
        return event;
    }
    if should_swallow(etype, event) {
        ptr::null_mut()
    } else {
        event
    }
}

fn should_swallow(etype: u32, event: *mut c_void) -> bool {
    if !ENABLED.load(Ordering::Relaxed) || event.is_null() {
        return false;
    }
    if !matches!(etype, LEFT_DOWN | LEFT_UP | RIGHT_DOWN | RIGHT_UP) {
        return false;
    }
    let flags = unsafe { CGEventGetFlags(event) };
    let want = match TRIGGER.load(Ordering::Relaxed) {
        TRIGGER_SHIFT => FLAG_SHIFT,
        TRIGGER_CTRL => FLAG_CONTROL,
        _ => FLAG_ALTERNATE,
    };
    flags & want != 0
}
