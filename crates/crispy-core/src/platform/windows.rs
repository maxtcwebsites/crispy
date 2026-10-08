//! Windows integration: low-level keyboard/mouse hooks for capture, SendInput for injection.

#![allow(clippy::upper_case_acronyms)]

use std::collections::HashSet;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use windows_sys::Win32::Foundation::{BOOL, LPARAM, LRESULT, POINT, RECT, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{Inject, Platform};
use crate::capture::{CaptureCore, InputEvent, Verdict};
use crate::keymap::{hid_from_windows, windows_from_hid, WinKey};
use crate::types::{MouseButton, Screen};

/// Tag on every event Crispy injects, so our own hooks can recognise and ignore them.
const MAGIC: usize = 0x4352_5350; // "CRSP"

static CORE: RwLock<Option<Arc<CaptureCore>>> = RwLock::new(None);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static LAST_X: AtomicI32 = AtomicI32::new(i32::MIN);
static LAST_Y: AtomicI32 = AtomicI32::new(i32::MIN);
static KEYS_DOWN: Mutex<Option<HashSet<u32>>> = Mutex::new(None);
static CURSORS_HIDDEN: AtomicBool = AtomicBool::new(false);

pub struct WindowsPlatform {
    wheel_rest: Mutex<(f64, f64)>,
}

impl WindowsPlatform {
    pub fn new() -> WindowsPlatform {
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        // A previous crash may have left the system cursors blanked.
        restore_cursors();
        WindowsPlatform { wheel_rest: Mutex::new((0.0, 0.0)) }
    }
}

/// Undo anything that could leave the desktop in a bad state. Safe to call at any time.
pub fn emergency_restore() {
    restore_cursors();
}

fn restore_cursors() {
    unsafe {
        SystemParametersInfoW(SPI_SETCURSORS, 0, null_mut(), 0);
    }
    CURSORS_HIDDEN.store(false, Ordering::SeqCst);
}

fn hide_cursors() {
    if CURSORS_HIDDEN.swap(true, Ordering::SeqCst) {
        return;
    }
    // Replace every system cursor with a fully transparent one (AND mask 1s, XOR mask 0s).
    const IDS: [u32; 14] = [
        OCR_NORMAL, OCR_IBEAM, OCR_WAIT, OCR_CROSS, OCR_UP, OCR_SIZENWSE, OCR_SIZENESW, OCR_SIZEWE, OCR_SIZENS,
        OCR_SIZEALL, OCR_NO, OCR_HAND, OCR_APPSTARTING, 32651, // OCR_HELP
    ];
    let and = [0xFFu8; 32 * 32 / 8];
    let xor = [0u8; 32 * 32 / 8];
    unsafe {
        let hinst = GetModuleHandleW(null());
        for id in IDS {
            let blank = CreateCursor(hinst, 0, 0, 32, 32, and.as_ptr().cast(), xor.as_ptr().cast());
            if !blank.is_null() {
                // SetSystemCursor takes ownership of (and destroys) the handle.
                SetSystemCursor(blank, id);
            }
        }
    }
}

fn cursor_pos() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

fn set_cursor(x: f64, y: f64) {
    unsafe { SetCursorPos(x.round() as i32, y.round() as i32) };
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = &*(lparam as *const KBDLLHOOKSTRUCT);
        if k.dwExtraInfo != MAGIC {
            if let Some(core) = CORE.read().as_ref() {
                let msg = wparam as u32;
                let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                let extended = k.flags & LLKHF_EXTENDED != 0;
                // AltGr sends a synthetic Left Ctrl (scan 0x21D); NumLock + navigation keys send fake
                // extended shifts. Neither is a real key press: never forward them.
                let fake = k.scanCode == 0x21D
                    || (matches!(k.vkCode, 0x10 | 0xA0 | 0xA1) && extended);
                if fake {
                    if core.is_grabbing() {
                        return 1;
                    }
                } else if let Some(hid) = hid_from_windows(k.vkCode, k.scanCode, extended) {
                    let repeat = {
                        let mut keys = KEYS_DOWN.lock();
                        let set = keys.get_or_insert_with(HashSet::new);
                        if down { !set.insert(hid) } else { set.remove(&hid); false }
                    };
                    if core.key(hid, down, repeat) == Verdict::Swallow {
                        return 1;
                    }
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let m = &*(lparam as *const MSLLHOOKSTRUCT);
        if m.dwExtraInfo != MAGIC {
            if let Some(core) = CORE.read().as_ref() {
                let wheel = ((m.mouseData >> 16) as u16 as i16) as f64;
                let xbutton = if (m.mouseData >> 16) as u16 == XBUTTON1 { MouseButton::Back } else { MouseButton::Forward };
                let verdict = match wparam as u32 {
                    WM_MOUSEMOVE => {
                        if core.is_grabbing() {
                            // The cursor is parked; how far it *would* have moved is the delta.
                            let (cx, cy) = cursor_pos();
                            core.motion(cx as f64, cy as f64, (m.pt.x - cx) as f64, (m.pt.y - cy) as f64)
                        } else {
                            let lx = LAST_X.swap(m.pt.x, Ordering::Relaxed);
                            let ly = LAST_Y.swap(m.pt.y, Ordering::Relaxed);
                            let (dx, dy) = if lx == i32::MIN { (0, 0) } else { (m.pt.x - lx, m.pt.y - ly) };
                            core.motion(m.pt.x as f64, m.pt.y as f64, dx as f64, dy as f64)
                        }
                    }
                    WM_LBUTTONDOWN => core.button(MouseButton::Left, true),
                    WM_LBUTTONUP => core.button(MouseButton::Left, false),
                    WM_RBUTTONDOWN => core.button(MouseButton::Right, true),
                    WM_RBUTTONUP => core.button(MouseButton::Right, false),
                    WM_MBUTTONDOWN => core.button(MouseButton::Middle, true),
                    WM_MBUTTONUP => core.button(MouseButton::Middle, false),
                    WM_XBUTTONDOWN => core.button(xbutton, true),
                    WM_XBUTTONUP => core.button(xbutton, false),
                    WM_MOUSEWHEEL => core.wheel(0.0, wheel / 120.0, wheel.abs() % 120.0 != 0.0),
                    WM_MOUSEHWHEEL => core.wheel(wheel / 120.0, 0.0, wheel.abs() % 120.0 != 0.0),
                    _ => Verdict::Pass,
                };
                if verdict == Verdict::Swallow {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn hook_thread(core: Arc<CaptureCore>) {
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
        HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
        let hmod = GetModuleHandleW(null());
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0);
        let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0);
        if kb.is_null() || ms.is_null() {
            core.emit(InputEvent::CaptureFailed("Windows refused to install the input hooks".into()));
            if !kb.is_null() {
                UnhookWindowsHookEx(kb);
            }
            if !ms.is_null() {
                UnhookWindowsHookEx(ms);
            }
            HOOK_THREAD.store(0, Ordering::SeqCst);
            return;
        }
        core.emit(InputEvent::CaptureRunning);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(kb);
        UnhookWindowsHookEx(ms);
        HOOK_THREAD.store(0, Ordering::SeqCst);
    }
}

fn send_inputs(inputs: &[INPUT]) {
    if inputs.is_empty() {
        return;
    }
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

fn mouse_input(dx: i32, dy: i32, data: u32, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: data, dwFlags: flags, time: 0, dwExtraInfo: MAGIC },
        },
    }
}

fn key_input(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: MAGIC } },
    }
}

/// Map a pixel to SendInput's 0..65535 virtual-desktop space, aiming at the middle of the pixel
/// so rounding can never land on a neighbour.
fn normalize(p: f64, origin: i32, size: i32) -> i32 {
    let size = size.max(1) as f64;
    (((p.floor() - origin as f64) + 0.5) * 65536.0 / size).clamp(0.0, 65535.0) as i32
}

unsafe extern "system" fn monitor_enum(hmon: HMONITOR, _hdc: HDC, _rect: *mut RECT, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<HMONITOR>);
    list.push(hmon);
    TRUE
}

impl Platform for WindowsPlatform {
    fn screens(&self) -> Vec<Screen> {
        let mut monitors: Vec<HMONITOR> = Vec::new();
        unsafe {
            EnumDisplayMonitors(null_mut(), null(), Some(monitor_enum), &mut monitors as *mut _ as LPARAM);
        }
        let mut out = Vec::new();
        for (i, hmon) in monitors.into_iter().enumerate() {
            unsafe {
                let mut info: MONITORINFOEXW = std::mem::zeroed();
                info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                if GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) == 0 {
                    continue;
                }
                let r = info.monitorInfo.rcMonitor;
                let (mut dx, mut dy) = (96u32, 96u32);
                let scale = if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) == 0 { dx as f64 / 96.0 } else { 1.0 };
                let len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
                let device = String::from_utf16_lossy(&info.szDevice[..len]);
                out.push(Screen {
                    id: device.trim_start_matches("\\\\.\\").to_string(),
                    name: format!("Display {}", i + 1),
                    x: r.left as f64,
                    y: r.top as f64,
                    width: (r.right - r.left) as f64,
                    height: (r.bottom - r.top) as f64,
                    scale,
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                });
            }
        }
        out.sort_by(|a, b| b.primary.cmp(&a.primary).then(a.x.partial_cmp(&b.x).unwrap()));
        out
    }

    fn cursor_pos(&self) -> (f64, f64) {
        let (x, y) = cursor_pos();
        (x as f64, y as f64)
    }

    fn start_capture(&self, core: Arc<CaptureCore>) -> anyhow::Result<()> {
        *CORE.write() = Some(core.clone());
        if HOOK_THREAD.load(Ordering::SeqCst) != 0 {
            core.emit(InputEvent::CaptureRunning);
            return Ok(());
        }
        std::thread::Builder::new().name("crispy-input-hooks".into()).spawn(move || hook_thread(core))?;
        Ok(())
    }

    fn grab_begin(&self) {
        // Park the cursor in the middle of the primary display, away from every edge.
        let screens = self.screens();
        if let Some(p) = screens.iter().find(|s| s.primary).or(screens.first()) {
            let (cx, cy) = p.rect().center();
            set_cursor(cx, cy);
        }
        hide_cursors();
    }

    fn grab_end(&self, x: f64, y: f64) {
        set_cursor(x, y);
        LAST_X.store(x.round() as i32, Ordering::Relaxed);
        LAST_Y.store(y.round() as i32, Ordering::Relaxed);
        restore_cursors();
    }

    fn warp(&self, x: f64, y: f64) {
        set_cursor(x, y);
        LAST_X.store(x.round() as i32, Ordering::Relaxed);
        LAST_Y.store(y.round() as i32, Ordering::Relaxed);
    }

    fn inject(&self, ev: &Inject) {
        match *ev {
            Inject::MoveTo { x, y } => {
                let (vx, vy) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
                let (vw, vh) = unsafe { (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
                let flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
                send_inputs(&[mouse_input(normalize(x, vx, vw), normalize(y, vy, vh), 0, flags)]);
            }
            Inject::MoveBy { dx, dy } => {
                send_inputs(&[mouse_input(dx.round() as i32, dy.round() as i32, 0, MOUSEEVENTF_MOVE)]);
            }
            Inject::Button { button, down } => {
                let (flags, data) = match (button, down) {
                    (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
                    (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
                    (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
                    (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
                    (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
                    (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
                    (MouseButton::Back, true) => (MOUSEEVENTF_XDOWN, XBUTTON1 as u32),
                    (MouseButton::Back, false) => (MOUSEEVENTF_XUP, XBUTTON1 as u32),
                    (MouseButton::Forward, true) => (MOUSEEVENTF_XDOWN, XBUTTON2 as u32),
                    (MouseButton::Forward, false) => (MOUSEEVENTF_XUP, XBUTTON2 as u32),
                };
                send_inputs(&[mouse_input(0, 0, data, flags)]);
            }
            Inject::Wheel { dx, dy, .. } => {
                // Accumulate sub-unit remainders so slow trackpad scrolling still moves.
                let mut rest = self.wheel_rest.lock();
                rest.0 += dx * 120.0;
                rest.1 += dy * 120.0;
                let (wx, wy) = (rest.0.trunc(), rest.1.trunc());
                rest.0 -= wx;
                rest.1 -= wy;
                let mut inputs = Vec::new();
                if wy != 0.0 {
                    inputs.push(mouse_input(0, 0, wy as i32 as u32, MOUSEEVENTF_WHEEL));
                }
                if wx != 0.0 {
                    inputs.push(mouse_input(0, 0, wx as i32 as u32, MOUSEEVENTF_HWHEEL));
                }
                send_inputs(&inputs);
            }
            Inject::Key { hid, down, .. } => {
                let up = if down { 0 } else { KEYEVENTF_KEYUP };
                match windows_from_hid(hid) {
                    Some(WinKey::Scan { scan, extended }) => {
                        let ext = if extended { KEYEVENTF_EXTENDEDKEY } else { 0 };
                        send_inputs(&[key_input(0, scan, KEYEVENTF_SCANCODE | ext | up)]);
                    }
                    Some(WinKey::Vk { vk, extended }) => {
                        let ext = if extended { KEYEVENTF_EXTENDEDKEY } else { 0 };
                        send_inputs(&[key_input(vk, 0, ext | up)]);
                    }
                    None => {}
                }
            }
            Inject::SetCapsLock(on) => {
                if self.caps_lock() != Some(on) {
                    send_inputs(&[key_input(VK_CAPITAL, 0x3A, 0), key_input(VK_CAPITAL, 0x3A, KEYEVENTF_KEYUP)]);
                }
            }
        }
    }

    fn precise_deltas(&self) -> bool {
        false
    }

    fn fullscreen_active(&self) -> bool {
        let mut state = 0;
        unsafe {
            if SHQueryUserNotificationState(&mut state) != 0 {
                return false;
            }
        }
        matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
    }

    fn caps_lock(&self) -> Option<bool> {
        Some(unsafe { GetKeyState(VK_CAPITAL as i32) } & 1 != 0)
    }
}

impl Drop for WindowsPlatform {
    fn drop(&mut self) {
        let tid = HOOK_THREAD.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
        }
        restore_cursors();
    }
}
