//! macOS integration: a Quartz event tap for capture, CGEventPost for injection.
//!
//! Requires the Accessibility permission (System Settings → Privacy & Security → Accessibility).
//! Without it the tap cannot be created; Crispy keeps retrying so it starts working the moment the
//! user flips the switch, without a restart.

#![allow(non_upper_case_globals, non_snake_case)]

use std::ffi::{c_char, c_void, CString};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};

use super::{Inject, PermissionState, Platform};
use crate::capture::{CaptureCore, InputEvent, Verdict};
use crate::keymap::{self, hid_from_mac, hid_from_mac_media, mac_from_hid, mac_media_from_hid};
use crate::types::{MouseButton, Screen};

type CGEventRef = *mut c_void;
type CGEventSourceRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CGDirectDisplayID = u32;
type CGEventMask = u64;
type CGEventFlags = u64;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

type CGEventTapCallBack = unsafe extern "C" fn(*mut c_void, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: CGEventMask, cb: CGEventTapCallBack, info: *mut c_void) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetLocation(e: CGEventRef) -> CGPoint;
    fn CGEventGetIntegerValueField(e: CGEventRef, field: u32) -> i64;
    fn CGEventGetDoubleValueField(e: CGEventRef, field: u32) -> f64;
    fn CGEventSetIntegerValueField(e: CGEventRef, field: u32, value: i64);
    fn CGEventGetFlags(e: CGEventRef) -> CGEventFlags;
    fn CGEventSetFlags(e: CGEventRef, flags: CGEventFlags);
    fn CGEventSetType(e: CGEventRef, t: u32);
    fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
    fn CGEventCreateMouseEvent(source: CGEventSourceRef, t: u32, pos: CGPoint, button: u32) -> CGEventRef;
    fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, keycode: u16, down: bool) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent2(source: CGEventSourceRef, units: u32, count: u32, w1: i32, w2: i32, w3: i32) -> CGEventRef;
    fn CGEventPost(tap: u32, e: CGEventRef);
    fn CGEventSourceCreate(state: i32) -> CGEventSourceRef;
    fn CGEventSourceSetLocalEventsSuppressionInterval(source: CGEventSourceRef, seconds: f64);
    fn CGEventSourceFlagsState(state: i32) -> CGEventFlags;
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    fn CGAssociateMouseAndMouseCursorPosition(connected: u32) -> i32;
    fn CGDisplayHideCursor(display: CGDirectDisplayID) -> i32;
    fn CGDisplayShowCursor(display: CGDirectDisplayID) -> i32;
    fn CGMainDisplayID() -> CGDirectDisplayID;
    fn CGGetActiveDisplayList(max: u32, displays: *mut CGDirectDisplayID, count: *mut u32) -> i32;
    fn CGDisplayBounds(d: CGDirectDisplayID) -> CGRect;
    fn CGDisplayCopyDisplayMode(d: CGDirectDisplayID) -> *mut c_void;
    fn CGDisplayModeGetPixelWidth(mode: *mut c_void) -> usize;
    fn CGDisplayModeRelease(mode: *mut c_void);
    fn CGDisplayIsBuiltin(d: CGDirectDisplayID) -> u32;
    fn CGPreflightPostEventAccess() -> bool;
    fn CGRequestPostEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(alloc: *const c_void, port: CFMachPortRef, order: isize) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRun();
    fn CFRelease(cf: CFTypeRef);
    fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, encoding: u32) -> CFStringRef;
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: isize,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> CFDictionaryRef;
    static kCFRunLoopCommonModes: CFStringRef;
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> u8;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(service: u32, owning_task: u32, kind: u32, connect: *mut u32) -> i32;
    fn IOServiceClose(connect: u32) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOHIDSetModifierLockState(handle: u32, selector: i32, state: bool) -> i32;
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

extern "C" {
    static mach_task_self_: u32;
}

// Event taps & locations
const kCGHIDEventTap: u32 = 0;
const kCGSessionEventTap: u32 = 1;
const kCGHeadInsertEventTap: u32 = 0;
const kCGEventTapOptionDefault: u32 = 0;
// Event types
const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const MOUSE_MOVED: u32 = 5;
const LEFT_DRAGGED: u32 = 6;
const RIGHT_DRAGGED: u32 = 7;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const SYSTEM_DEFINED: u32 = 14;
const SCROLL_WHEEL: u32 = 22;
const OTHER_DOWN: u32 = 25;
const OTHER_UP: u32 = 26;
const OTHER_DRAGGED: u32 = 27;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;
// Event fields
const F_CLICK_STATE: u32 = 1;
const F_BUTTON_NUMBER: u32 = 3;
const F_DELTA_X: u32 = 4;
const F_DELTA_Y: u32 = 5;
const F_AUTOREPEAT: u32 = 8;
const F_KEYCODE: u32 = 9;
const F_SCROLL_FIXED_1: u32 = 93;
const F_SCROLL_FIXED_2: u32 = 94;
const F_SCROLL_POINT_1: u32 = 96;
const F_SCROLL_POINT_2: u32 = 97;
const F_SCROLL_CONTINUOUS: u32 = 88;
const F_SOURCE_USER_DATA: u32 = 42;
// Flags
const FLAG_CAPS: u64 = 0x0001_0000;
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALT: u64 = 0x0008_0000;
const FLAG_COMMAND: u64 = 0x0010_0000;
const FLAG_NON_COALESCED: u64 = 0x0000_0100;
const DEVICE_MASKS: u64 = 0x207F;
const kCGEventSourceStateHIDSystemState: i32 = 1;
const kCFStringEncodingUTF8: u32 = 0x0800_0100;

/// Tag on every event Crispy posts, so the tap can tell them from real input.
const MAGIC: i64 = 0x4352_5350;
/// Trackpad pixels per wheel notch on the wire.
const PX_PER_NOTCH: f64 = 60.0;
/// Lines a Mac scrolls for one PC wheel notch (a PC notch is three lines).
const LINES_PER_NOTCH: f64 = 3.0;

static CORE: RwLock<Option<Arc<CaptureCore>>> = RwLock::new(None);
static TAP: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static TAP_STARTED: AtomicBool = AtomicBool::new(false);
static MOD_STATE: Mutex<u16> = Mutex::new(0);

fn ev_mask(types: &[u32]) -> CGEventMask {
    types.iter().fold(0, |m, &t| m | (1u64 << t))
}

/// Device-dependent bit for each modifier keycode (left/right aware).
fn modifier_bits(keycode: u16) -> Option<(u64, u64)> {
    Some(match keycode {
        0x38 => (0x02, FLAG_SHIFT),
        0x3C => (0x04, FLAG_SHIFT),
        0x3B => (0x01, FLAG_CONTROL),
        0x3E => (0x2000, FLAG_CONTROL),
        0x3A => (0x20, FLAG_ALT),
        0x3D => (0x40, FLAG_ALT),
        0x37 => (0x08, FLAG_COMMAND),
        0x36 => (0x10, FLAG_COMMAND),
        _ => return None,
    })
}

unsafe fn msg_send_ptr(obj: *mut c_void, sel: &str, arg: *mut c_void) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let sel = CString::new(sel).unwrap();
    f(obj, sel_registerName(sel.as_ptr()), arg)
}

unsafe fn class(name: &str) -> *mut c_void {
    let n = CString::new(name).unwrap();
    objc_getClass(n.as_ptr())
}

unsafe fn sel(name: &str) -> *mut c_void {
    let n = CString::new(name).unwrap();
    sel_registerName(n.as_ptr())
}

/// Decode a media key (play/pause, volume...) from an NX_SYSDEFINED event.
unsafe fn media_key(event: CGEventRef) -> Option<(i32, bool, bool)> {
    let pool = objc_autoreleasePoolPush();
    let ns = msg_send_ptr(class("NSEvent"), "eventWithCGEvent:", event);
    let mut out = None;
    if !ns.is_null() {
        let subtype_fn: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i16 =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let data_fn: unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        if subtype_fn(ns, sel("subtype")) == 8 {
            let data1 = data_fn(ns, sel("data1")) as i64;
            let key = ((data1 & 0xFFFF_0000) >> 16) as i32;
            let flags = data1 & 0xFFFF;
            let down = (flags & 0xFF00) >> 8 == 0x0A;
            out = Some((key, down, flags & 1 != 0));
        }
    }
    objc_autoreleasePoolPop(pool);
    out
}

unsafe fn post_media_key(nx_key: i32, down: bool) {
    let pool = objc_autoreleasePoolPush();
    type Other = unsafe extern "C" fn(*mut c_void, *mut c_void, u64, CGPoint, u64, f64, isize, *mut c_void, i16, isize, isize) -> *mut c_void;
    let make: Other = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let state: isize = if down { 0xA00 } else { 0xB00 };
    let ns = make(
        class("NSEvent"),
        sel("otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:"),
        14,
        CGPoint::default(),
        state as u64,
        0.0,
        0,
        null_mut(),
        8,
        ((nx_key as isize) << 16) | state,
        -1,
    );
    if !ns.is_null() {
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> CGEventRef =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let cg = get(ns, sel("CGEvent"));
        if !cg.is_null() {
            CGEventSetIntegerValueField(cg, F_SOURCE_USER_DATA, MAGIC);
            CGEventPost(kCGHIDEventTap, cg);
        }
    }
    objc_autoreleasePoolPop(pool);
}

unsafe extern "C" fn tap_callback(_proxy: *mut c_void, etype: u32, event: CGEventRef, _info: *mut c_void) -> CGEventRef {
    if etype == TAP_DISABLED_BY_TIMEOUT || etype == TAP_DISABLED_BY_USER_INPUT {
        let tap = TAP.load(Ordering::SeqCst);
        if !tap.is_null() {
            CGEventTapEnable(tap, true);
        }
        return event;
    }
    let core = match CORE.read().as_ref() {
        Some(c) => c.clone(),
        None => return event,
    };
    if CGEventGetIntegerValueField(event, F_SOURCE_USER_DATA) == MAGIC {
        return event;
    }
    let verdict = match etype {
        MOUSE_MOVED | LEFT_DRAGGED | RIGHT_DRAGGED | OTHER_DRAGGED => {
            let p = CGEventGetLocation(event);
            let dx = CGEventGetDoubleValueField(event, F_DELTA_X);
            let dy = CGEventGetDoubleValueField(event, F_DELTA_Y);
            core.motion(p.x, p.y, dx, dy)
        }
        LEFT_DOWN => core.button(MouseButton::Left, true),
        LEFT_UP => core.button(MouseButton::Left, false),
        RIGHT_DOWN => core.button(MouseButton::Right, true),
        RIGHT_UP => core.button(MouseButton::Right, false),
        OTHER_DOWN | OTHER_UP => {
            let button = match CGEventGetIntegerValueField(event, F_BUTTON_NUMBER) {
                2 => Some(MouseButton::Middle),
                3 => Some(MouseButton::Back),
                4 => Some(MouseButton::Forward),
                _ => None,
            };
            match button {
                Some(b) => core.button(b, etype == OTHER_DOWN),
                None => Verdict::Pass,
            }
        }
        SCROLL_WHEEL => {
            let continuous = CGEventGetIntegerValueField(event, F_SCROLL_CONTINUOUS) != 0;
            let (dy, dx) = if continuous {
                (
                    CGEventGetDoubleValueField(event, F_SCROLL_POINT_1) / PX_PER_NOTCH,
                    -CGEventGetDoubleValueField(event, F_SCROLL_POINT_2) / PX_PER_NOTCH,
                )
            } else {
                (
                    CGEventGetDoubleValueField(event, F_SCROLL_FIXED_1),
                    -CGEventGetDoubleValueField(event, F_SCROLL_FIXED_2),
                )
            };
            core.wheel(dx, dy, continuous)
        }
        KEY_DOWN | KEY_UP => {
            let keycode = CGEventGetIntegerValueField(event, F_KEYCODE) as u16;
            let repeat = CGEventGetIntegerValueField(event, F_AUTOREPEAT) != 0;
            match hid_from_mac(keycode) {
                Some(hid) => core.key(hid, etype == KEY_DOWN, repeat),
                None => if core.is_grabbing() { Verdict::Swallow } else { Verdict::Pass },
            }
        }
        FLAGS_CHANGED => {
            let keycode = CGEventGetIntegerValueField(event, F_KEYCODE) as u16;
            let flags = CGEventGetFlags(event);
            if keycode == 0x39 {
                // Caps Lock reports a single event per press: synthesise press + release.
                let v = core.key(keymap::CAPS_LOCK, true, false);
                core.key(keymap::CAPS_LOCK, false, false);
                v
            } else if let (Some((device_bit, generic)), Some(hid)) = (modifier_bits(keycode), hid_from_mac(keycode)) {
                let down = if flags & DEVICE_MASKS != 0 {
                    flags & device_bit != 0
                } else {
                    flags & generic != 0
                };
                core.key(hid, down, false)
            } else if core.is_grabbing() {
                Verdict::Swallow
            } else {
                Verdict::Pass
            }
        }
        SYSTEM_DEFINED => match media_key(event) {
            Some((key, down, repeat)) => match hid_from_mac_media(key) {
                Some(hid) => core.key(hid, down, repeat),
                None => Verdict::Pass,
            },
            None => Verdict::Pass,
        },
        _ => Verdict::Pass,
    };
    if verdict == Verdict::Swallow {
        null_mut()
    } else {
        event
    }
}

fn tap_thread() {
    let mask = ev_mask(&[
        LEFT_DOWN, LEFT_UP, RIGHT_DOWN, RIGHT_UP, MOUSE_MOVED, LEFT_DRAGGED, RIGHT_DRAGGED, KEY_DOWN, KEY_UP,
        FLAGS_CHANGED, SYSTEM_DEFINED, SCROLL_WHEEL, OTHER_DOWN, OTHER_UP, OTHER_DRAGGED,
    ]);
    let mut reported = false;
    loop {
        let tap = unsafe {
            CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap, kCGEventTapOptionDefault, mask, tap_callback, null_mut())
        };
        if tap.is_null() {
            if !reported {
                if let Some(core) = CORE.read().as_ref() {
                    core.emit(InputEvent::CaptureFailed(
                        "Crispy needs the Accessibility permission to share your keyboard and mouse.".into(),
                    ));
                }
                reported = true;
            }
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }
        TAP.store(tap, Ordering::SeqCst);
        unsafe {
            let source = CFMachPortCreateRunLoopSource(null(), tap, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
        }
        if let Some(core) = CORE.read().as_ref() {
            core.emit(InputEvent::CaptureRunning);
        }
        unsafe { CFRunLoopRun() };
        return;
    }
}

/// Let a background app hide the cursor (the same private switch Synergy and Barrier use).
fn allow_background_cursor_hiding() {
    unsafe {
        let conn_fn = libc::dlsym(libc::RTLD_DEFAULT, c"_CGSDefaultConnection".as_ptr());
        let set_fn = libc::dlsym(libc::RTLD_DEFAULT, c"CGSSetConnectionProperty".as_ptr());
        if conn_fn.is_null() || set_fn.is_null() {
            return;
        }
        let conn: extern "C" fn() -> i32 = std::mem::transmute(conn_fn);
        let set: extern "C" fn(i32, i32, CFStringRef, CFTypeRef) -> i32 = std::mem::transmute(set_fn);
        let key = CFStringCreateWithCString(null(), c"SetsCursorInBackground".as_ptr(), kCFStringEncodingUTF8);
        let c = conn();
        set(c, c, key, kCFBooleanTrue);
        CFRelease(key);
    }
}

struct Injector {
    source: usize,
    buttons: u8,
    pos: CGPoint,
    last_click: Option<(Instant, MouseButton, CGPoint)>,
    clicks: i64,
    scroll_rest: (f64, f64),
    double_click: Duration,
}

pub struct MacPlatform {
    inject: Mutex<Injector>,
    hidden: AtomicBool,
}

fn double_click_interval() -> Duration {
    unsafe {
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64 = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let secs = f(class("NSEvent"), sel("doubleClickInterval"));
        if secs.is_finite() && secs > 0.05 && secs < 5.0 {
            Duration::from_secs_f64(secs)
        } else {
            Duration::from_millis(500)
        }
    }
}

impl MacPlatform {
    pub fn new() -> MacPlatform {
        allow_background_cursor_hiding();
        let source = unsafe {
            let s = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
            if !s.is_null() {
                // Don't freeze the real mouse for 250ms after each synthetic event.
                CGEventSourceSetLocalEventsSuppressionInterval(s, 0.0);
            }
            s as usize
        };
        MacPlatform {
            inject: Mutex::new(Injector {
                source,
                buttons: 0,
                pos: current_location(),
                last_click: None,
                clicks: 1,
                scroll_rest: (0.0, 0.0),
                double_click: double_click_interval(),
            }),
            hidden: AtomicBool::new(false),
        }
    }

    fn post(event: CGEventRef, flags: Option<CGEventFlags>) {
        if event.is_null() {
            return;
        }
        unsafe {
            if let Some(f) = flags {
                CGEventSetFlags(event, f | FLAG_NON_COALESCED);
            }
            CGEventSetIntegerValueField(event, F_SOURCE_USER_DATA, MAGIC);
            CGEventPost(kCGHIDEventTap, event);
            CFRelease(event);
        }
    }
}

fn current_location() -> CGPoint {
    unsafe {
        let e = CGEventCreate(null_mut());
        if e.is_null() {
            return CGPoint::default();
        }
        let p = CGEventGetLocation(e);
        CFRelease(e);
        p
    }
}

/// Modifier flags matching the keys Crispy is currently holding down on this Mac.
fn modifier_flags() -> CGEventFlags {
    let held = *MOD_STATE.lock();
    let mut flags = 0u64;
    for (i, keycode) in [0x38u16, 0x3C, 0x3B, 0x3E, 0x3A, 0x3D, 0x37, 0x36].iter().enumerate() {
        if held & (1 << i) != 0 {
            let (dev, generic) = modifier_bits(*keycode).unwrap();
            flags |= dev | generic;
        }
    }
    let caps = unsafe { CGEventSourceFlagsState(kCGEventSourceStateHIDSystemState) } & FLAG_CAPS;
    flags | caps
}

fn set_modifier(keycode: u16, down: bool) {
    let idx = [0x38u16, 0x3C, 0x3B, 0x3E, 0x3A, 0x3D, 0x37, 0x36].iter().position(|k| *k == keycode);
    if let Some(i) = idx {
        let mut s = MOD_STATE.lock();
        if down {
            *s |= 1 << i;
        } else {
            *s &= !(1 << i);
        }
    }
}

fn set_caps_lock(on: bool) {
    unsafe {
        let matching = IOServiceMatching(c"IOHIDSystem".as_ptr());
        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return;
        }
        let mut connect = 0u32;
        if IOServiceOpen(service, mach_task_self_, 1, &mut connect) == 0 {
            IOHIDSetModifierLockState(connect, 1, on);
            IOServiceClose(connect);
        }
        IOObjectRelease(service);
    }
}

impl Platform for MacPlatform {
    fn screens(&self) -> Vec<Screen> {
        let mut ids = [0u32; 16];
        let mut count = 0u32;
        unsafe {
            if CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) != 0 {
                return vec![];
            }
        }
        let main = unsafe { CGMainDisplayID() };
        let mut external = 0;
        ids[..count as usize]
            .iter()
            .map(|&id| unsafe {
                let b = CGDisplayBounds(id);
                let mode = CGDisplayCopyDisplayMode(id);
                let scale = if mode.is_null() || b.size.width <= 0.0 {
                    1.0
                } else {
                    let px = CGDisplayModeGetPixelWidth(mode) as f64;
                    CGDisplayModeRelease(mode);
                    (px / b.size.width).max(1.0)
                };
                let name = if CGDisplayIsBuiltin(id) != 0 {
                    "Built-in Display".to_string()
                } else {
                    external += 1;
                    format!("Display {external}")
                };
                Screen {
                    id: id.to_string(),
                    name,
                    x: b.origin.x,
                    y: b.origin.y,
                    width: b.size.width,
                    height: b.size.height,
                    scale,
                    primary: id == main,
                }
            })
            .collect()
    }

    fn cursor_pos(&self) -> (f64, f64) {
        let p = current_location();
        (p.x, p.y)
    }

    fn start_capture(&self, core: Arc<CaptureCore>) -> anyhow::Result<()> {
        *CORE.write() = Some(core.clone());
        if TAP_STARTED.swap(true, Ordering::SeqCst) {
            if !TAP.load(Ordering::SeqCst).is_null() {
                core.emit(InputEvent::CaptureRunning);
            }
            return Ok(());
        }
        std::thread::Builder::new().name("crispy-event-tap".into()).spawn(tap_thread)?;
        Ok(())
    }

    fn grab_begin(&self) {
        unsafe {
            CGAssociateMouseAndMouseCursorPosition(0);
            if !self.hidden.swap(true, Ordering::SeqCst) {
                CGDisplayHideCursor(CGMainDisplayID());
            }
        }
    }

    fn grab_end(&self, x: f64, y: f64) {
        unsafe {
            CGWarpMouseCursorPosition(CGPoint { x, y });
            CGAssociateMouseAndMouseCursorPosition(1);
            if self.hidden.swap(false, Ordering::SeqCst) {
                CGDisplayShowCursor(CGMainDisplayID());
            }
        }
        self.inject.lock().pos = CGPoint { x, y };
    }

    fn warp(&self, x: f64, y: f64) {
        unsafe {
            CGWarpMouseCursorPosition(CGPoint { x, y });
            // Re-associating clears the short freeze macOS applies after a warp.
            CGAssociateMouseAndMouseCursorPosition(1);
        }
        self.inject.lock().pos = CGPoint { x, y };
    }

    fn inject(&self, ev: &Inject) {
        let mut st = self.inject.lock();
        let source = st.source as CGEventSourceRef;
        match *ev {
            Inject::MoveTo { x, y } | Inject::MoveBy { dx: x, dy: y } => {
                let (target, dx, dy) = if let Inject::MoveTo { .. } = ev {
                    (CGPoint { x, y }, x - st.pos.x, y - st.pos.y)
                } else {
                    let p = current_location();
                    (CGPoint { x: p.x + x, y: p.y + y }, x, y)
                };
                let (etype, button) = if st.buttons & MouseButton::Left.bit() != 0 {
                    (LEFT_DRAGGED, 0)
                } else if st.buttons & MouseButton::Right.bit() != 0 {
                    (RIGHT_DRAGGED, 1)
                } else if st.buttons != 0 {
                    (OTHER_DRAGGED, 2)
                } else {
                    (MOUSE_MOVED, 0)
                };
                unsafe {
                    let e = CGEventCreateMouseEvent(source, etype, target, button);
                    if !e.is_null() {
                        CGEventSetIntegerValueField(e, F_DELTA_X, dx.round() as i64);
                        CGEventSetIntegerValueField(e, F_DELTA_Y, dy.round() as i64);
                    }
                    Self::post(e, Some(modifier_flags()));
                }
                st.pos = target;
            }
            Inject::Button { button, down } => {
                let pos = current_location();
                if down {
                    let now = Instant::now();
                    let repeat = st.last_click.is_some_and(|(t, b, p)| {
                        b == button && now.duration_since(t) <= st.double_click && (p.x - pos.x).abs() < 5.0 && (p.y - pos.y).abs() < 5.0
                    });
                    st.clicks = if repeat { st.clicks + 1 } else { 1 };
                    st.last_click = Some((now, button, pos));
                    st.buttons |= button.bit();
                } else {
                    st.buttons &= !button.bit();
                }
                let (etype, number) = match (button, down) {
                    (MouseButton::Left, true) => (LEFT_DOWN, 0),
                    (MouseButton::Left, false) => (LEFT_UP, 0),
                    (MouseButton::Right, true) => (RIGHT_DOWN, 1),
                    (MouseButton::Right, false) => (RIGHT_UP, 1),
                    (MouseButton::Middle, d) => (if d { OTHER_DOWN } else { OTHER_UP }, 2),
                    (MouseButton::Back, d) => (if d { OTHER_DOWN } else { OTHER_UP }, 3),
                    (MouseButton::Forward, d) => (if d { OTHER_DOWN } else { OTHER_UP }, 4),
                };
                unsafe {
                    let e = CGEventCreateMouseEvent(source, etype, pos, number.min(2));
                    if !e.is_null() {
                        CGEventSetIntegerValueField(e, F_CLICK_STATE, st.clicks);
                        CGEventSetIntegerValueField(e, F_BUTTON_NUMBER, number as i64);
                    }
                    Self::post(e, Some(modifier_flags()));
                }
            }
            Inject::Wheel { dx, dy, continuous } => {
                let (unit, scale) = if continuous { (0u32, PX_PER_NOTCH) } else { (1u32, LINES_PER_NOTCH) };
                st.scroll_rest.0 += -dx * scale;
                st.scroll_rest.1 += dy * scale;
                let (wx, wy) = (st.scroll_rest.0.trunc(), st.scroll_rest.1.trunc());
                st.scroll_rest.0 -= wx;
                st.scroll_rest.1 -= wy;
                if wx != 0.0 || wy != 0.0 {
                    unsafe {
                        let e = CGEventCreateScrollWheelEvent2(source, unit, 2, wy as i32, wx as i32, 0);
                        if !e.is_null() && continuous {
                            CGEventSetIntegerValueField(e, F_SCROLL_CONTINUOUS, 1);
                        }
                        Self::post(e, Some(modifier_flags()));
                    }
                }
            }
            Inject::Key { hid, down, repeat } => {
                drop(st);
                if keymap::is_media(hid) {
                    if let Some(nx) = mac_media_from_hid(hid) {
                        unsafe { post_media_key(nx, down) };
                    }
                    return;
                }
                if hid == keymap::CAPS_LOCK {
                    if down {
                        let on = unsafe { CGEventSourceFlagsState(kCGEventSourceStateHIDSystemState) } & FLAG_CAPS != 0;
                        set_caps_lock(!on);
                    }
                    return;
                }
                let Some(keycode) = mac_from_hid(hid) else { return };
                unsafe {
                    let e = CGEventCreateKeyboardEvent(source, keycode, down);
                    if e.is_null() {
                        return;
                    }
                    if modifier_bits(keycode).is_some() {
                        set_modifier(keycode, down);
                        CGEventSetType(e, FLAGS_CHANGED);
                    } else if repeat {
                        CGEventSetIntegerValueField(e, F_AUTOREPEAT, 1);
                    }
                    Self::post(e, Some(modifier_flags()));
                }
            }
            Inject::SetCapsLock(on) => {
                drop(st);
                if self.caps_lock() != Some(on) {
                    set_caps_lock(on);
                }
            }
        }
    }

    fn precise_deltas(&self) -> bool {
        true
    }

    fn accessibility(&self) -> PermissionState {
        let trusted = unsafe { AXIsProcessTrusted() != 0 || CGPreflightPostEventAccess() };
        if trusted {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        }
    }

    fn request_accessibility(&self) {
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt as CFTypeRef];
            let values = [kCFBooleanTrue];
            let dict = CFDictionaryCreate(
                null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &kCFTypeDictionaryKeyCallBacks as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const c_void,
            );
            AXIsProcessTrustedWithOptions(dict);
            if !dict.is_null() {
                CFRelease(dict);
            }
            CGRequestPostEventAccess();
            CGRequestListenEventAccess();
        }
    }

    fn open_accessibility_settings(&self) {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }

    fn secure_input_active(&self) -> bool {
        unsafe { IsSecureEventInputEnabled() != 0 }
    }

    fn caps_lock(&self) -> Option<bool> {
        Some(unsafe { CGEventSourceFlagsState(kCGEventSourceStateHIDSystemState) } & FLAG_CAPS != 0)
    }
}

impl Drop for MacPlatform {
    fn drop(&mut self) {
        unsafe {
            CGAssociateMouseAndMouseCursorPosition(1);
            if self.hidden.swap(false, Ordering::SeqCst) {
                CGDisplayShowCursor(CGMainDisplayID());
            }
        }
    }
}
