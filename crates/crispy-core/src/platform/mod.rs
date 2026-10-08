//! Operating-system integration: displays, global input capture and input injection.

use std::sync::Arc;

use serde::Serialize;

use crate::capture::CaptureCore;
use crate::types::{MouseButton, Screen};

#[cfg(target_os = "macos")]
mod macos;
pub mod mock;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    Granted,
    Denied,
    NotRequired,
}

/// Synthetic input to perform on this computer on behalf of a remote keyboard/mouse.
#[derive(Clone, Debug, PartialEq)]
pub enum Inject {
    MoveTo { x: f64, y: f64 },
    MoveBy { dx: f64, dy: f64 },
    Button { button: MouseButton, down: bool },
    /// Wheel notches; `continuous` for trackpad-style high resolution scrolling.
    Wheel { dx: f64, dy: f64, continuous: bool },
    Key { hid: u32, down: bool, repeat: bool },
    SetCapsLock(bool),
}

pub trait Platform: Send + Sync + 'static {
    /// All active displays in native cursor coordinates.
    fn screens(&self) -> Vec<Screen>;
    fn cursor_pos(&self) -> (f64, f64);
    /// Install the global input hooks. Events flow into `core`; must not block.
    fn start_capture(&self, core: Arc<CaptureCore>) -> anyhow::Result<()>;
    /// The cursor just left for another computer: hide it and keep it parked.
    fn grab_begin(&self);
    /// The cursor came back: show it again at (x, y).
    fn grab_end(&self, x: f64, y: f64);
    fn warp(&self, x: f64, y: f64);
    fn inject(&self, ev: &Inject);
    /// True when mouse events carry exact deltas (macOS); false when deltas are derived from
    /// positions and stop at screen edges (Windows hooks).
    fn precise_deltas(&self) -> bool;
    fn accessibility(&self) -> PermissionState {
        PermissionState::NotRequired
    }
    fn request_accessibility(&self) {}
    fn open_accessibility_settings(&self) {}
    /// A full-screen app (game, presentation, video) has focus.
    fn fullscreen_active(&self) -> bool {
        false
    }
    /// macOS Secure Keyboard Entry blocks keyboard capture (password fields, some terminals).
    fn secure_input_active(&self) -> bool {
        false
    }
    fn caps_lock(&self) -> Option<bool> {
        None
    }
}

/// Undo global side effects (e.g. hidden system cursors on Windows). Safe to call from a panic hook.
pub fn emergency_restore() {
    #[cfg(target_os = "windows")]
    windows::emergency_restore();
}

/// The real platform for this OS (a mock on platforms Crispy doesn't support natively).
pub fn native() -> Arc<dyn Platform> {
    #[cfg(target_os = "windows")]
    return Arc::new(windows::WindowsPlatform::new());
    #[cfg(target_os = "macos")]
    return Arc::new(macos::MacPlatform::new());
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return mock::MockPlatform::new(vec![mock::screen(0.0, 0.0, 1920.0, 1080.0)]);
}
