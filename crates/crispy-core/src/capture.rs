//! The decision logic that runs *inside* the OS input hooks.
//!
//! OS hooks must answer instantly ("let this event through" or "swallow it"), so this layer is
//! tiny and lock-light. It forwards every physical event to the engine over a channel and decides
//! pass/swallow from a single atomic flag, plus two pieces of bookkeeping that keep keys from ever
//! getting stuck:
//!
//! * keys and buttons that were already down on this computer when the cursor left are still
//!   released *here* (the local OS saw them go down, so it must see them go up), and
//! * hotkeys are swallowed on both their press and their release.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::keymap::{modifier_of, Hotkey, ModState};
use crate::types::MouseButton;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    LockToScreen,
    BringHome,
    NextDevice,
    PauseSharing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// Pointer motion. While local, (x, y) is the new cursor position; while grabbing only the
    /// deltas are meaningful.
    Motion { x: f64, y: f64, dx: f64, dy: f64 },
    Button { button: MouseButton, down: bool },
    /// Scroll in wheel notches (fractional for high-resolution devices).
    Wheel { dx: f64, dy: f64, continuous: bool },
    Key { hid: u32, down: bool, repeat: bool },
    Hotkey(HotkeyAction),
    /// The OS hook stopped (e.g. permissions revoked) or could not start.
    CaptureFailed(String),
    CaptureRunning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Swallow,
}

#[derive(Default)]
struct State {
    hotkeys: Vec<(HotkeyAction, Hotkey)>,
    mods: ModState,
    swallowed_keys: HashSet<u32>,
    local_keys: HashSet<u32>,
    local_buttons: u8,
    passthrough_keys: HashSet<u32>,
    passthrough_buttons: u8,
}

pub struct CaptureCore {
    grabbing: AtomicBool,
    tx: mpsc::UnboundedSender<InputEvent>,
    state: Mutex<State>,
}

impl CaptureCore {
    pub fn new(tx: mpsc::UnboundedSender<InputEvent>) -> CaptureCore {
        CaptureCore { grabbing: AtomicBool::new(false), tx, state: Mutex::new(State::default()) }
    }

    pub fn emit(&self, ev: InputEvent) {
        let _ = self.tx.send(ev);
    }

    pub fn is_grabbing(&self) -> bool {
        self.grabbing.load(Ordering::Acquire)
    }

    pub fn set_hotkeys(&self, hotkeys: Vec<(HotkeyAction, Hotkey)>) {
        self.state.lock().hotkeys = hotkeys.into_iter().filter(|(_, h)| h.is_set()).collect();
    }

    /// Start or stop sending this computer's input elsewhere.
    pub fn set_grab(&self, grab: bool) {
        let mut s = self.state.lock();
        if grab {
            s.passthrough_keys = s.local_keys.clone();
            s.passthrough_buttons = s.local_buttons;
        } else {
            s.passthrough_keys.clear();
            s.passthrough_buttons = 0;
        }
        self.grabbing.store(grab, Ordering::Release);
    }

    pub fn key(&self, hid: u32, down: bool, repeat: bool) -> Verdict {
        let mut s = self.state.lock();
        let before = s.mods;
        s.mods.update(hid, down);

        if s.swallowed_keys.contains(&hid) {
            if !down {
                s.swallowed_keys.remove(&hid);
            }
            return Verdict::Swallow;
        }
        if down && !repeat && modifier_of(hid).is_none() {
            if let Some(action) = s.hotkeys.iter().find(|(_, h)| h.matches(hid, &before)).map(|(a, _)| *a) {
                s.swallowed_keys.insert(hid);
                drop(s);
                self.emit(InputEvent::Hotkey(action));
                return Verdict::Swallow;
            }
        }

        if self.is_grabbing() {
            if !down && s.passthrough_keys.remove(&hid) {
                s.local_keys.remove(&hid);
                drop(s);
                // The other computer may have pressed it too (held modifiers travel with the cursor).
                self.emit(InputEvent::Key { hid, down, repeat });
                return Verdict::Pass;
            }
            drop(s);
            self.emit(InputEvent::Key { hid, down, repeat });
            Verdict::Swallow
        } else {
            if down {
                s.local_keys.insert(hid);
            } else {
                s.local_keys.remove(&hid);
            }
            drop(s);
            self.emit(InputEvent::Key { hid, down, repeat });
            Verdict::Pass
        }
    }

    pub fn button(&self, button: MouseButton, down: bool) -> Verdict {
        let mut s = self.state.lock();
        let bit = button.bit();
        if self.is_grabbing() {
            if !down && s.passthrough_buttons & bit != 0 {
                s.passthrough_buttons &= !bit;
                s.local_buttons &= !bit;
                return Verdict::Pass;
            }
            drop(s);
            self.emit(InputEvent::Button { button, down });
            Verdict::Swallow
        } else {
            if down {
                s.local_buttons |= bit;
            } else {
                s.local_buttons &= !bit;
            }
            drop(s);
            self.emit(InputEvent::Button { button, down });
            Verdict::Pass
        }
    }

    pub fn motion(&self, x: f64, y: f64, dx: f64, dy: f64) -> Verdict {
        self.emit(InputEvent::Motion { x, y, dx, dy });
        if self.is_grabbing() {
            Verdict::Swallow
        } else {
            Verdict::Pass
        }
    }

    pub fn wheel(&self, dx: f64, dy: f64, continuous: bool) -> Verdict {
        self.emit(InputEvent::Wheel { dx, dy, continuous });
        if self.is_grabbing() {
            Verdict::Swallow
        } else {
            Verdict::Pass
        }
    }

    /// Mouse buttons currently held on this computer (bitmask of [`MouseButton::bit`]).
    pub fn local_buttons(&self) -> u8 {
        self.state.lock().local_buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{kb, CTRL_L, SHIFT_L};

    fn core() -> (CaptureCore, mpsc::UnboundedReceiver<InputEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (CaptureCore::new(tx), rx)
    }

    #[test]
    fn keys_held_before_grab_are_released_locally() {
        let (c, _rx) = core();
        assert_eq!(c.key(SHIFT_L, true, false), Verdict::Pass);
        c.set_grab(true);
        assert_eq!(c.key(kb(0x04), true, false), Verdict::Swallow);
        assert_eq!(c.key(SHIFT_L, false, false), Verdict::Pass, "local OS must see shift go up");
        assert_eq!(c.key(kb(0x04), false, false), Verdict::Swallow);
        assert_eq!(c.key(SHIFT_L, true, false), Verdict::Swallow);
    }

    #[test]
    fn buttons_held_before_grab_are_released_locally() {
        let (c, _rx) = core();
        c.button(MouseButton::Left, true);
        c.set_grab(true);
        assert_eq!(c.button(MouseButton::Right, true), Verdict::Swallow);
        assert_eq!(c.button(MouseButton::Left, false), Verdict::Pass);
        assert_eq!(c.local_buttons(), 0);
    }

    #[test]
    fn hotkeys_are_swallowed_and_reported() {
        let (c, mut rx) = core();
        c.set_hotkeys(vec![(HotkeyAction::LockToScreen, Hotkey::new(true, false, false, false, kb(0x0F)))]);
        assert_eq!(c.key(CTRL_L, true, false), Verdict::Pass);
        assert_eq!(c.key(kb(0x0F), true, false), Verdict::Swallow);
        assert_eq!(c.key(kb(0x0F), true, true), Verdict::Swallow);
        assert_eq!(c.key(kb(0x0F), false, false), Verdict::Swallow);
        assert_eq!(c.key(kb(0x0F), true, false), Verdict::Swallow, "fires again while ctrl held");
        let mut actions = 0;
        while let Ok(ev) = rx.try_recv() {
            if ev == InputEvent::Hotkey(HotkeyAction::LockToScreen) {
                actions += 1;
            }
        }
        assert_eq!(actions, 2);
        // Without ctrl it is just the letter L.
        c.key(kb(0x0F), false, false);
        c.key(CTRL_L, false, false);
        assert_eq!(c.key(kb(0x0F), true, false), Verdict::Pass);
    }
}
