//! An in-memory platform: used by tests to simulate a user at a computer, and as the fallback on
//! operating systems Crispy has no native integration for.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{Inject, Platform};
use crate::capture::{CaptureCore, Verdict};
use crate::types::{bounds_of, MouseButton, Screen};

pub fn screen(x: f64, y: f64, w: f64, h: f64) -> Screen {
    Screen {
        id: format!("{x}:{y}"),
        name: format!("Display {w}×{h}"),
        x,
        y,
        width: w,
        height: h,
        scale: 1.0,
        primary: x == 0.0 && y == 0.0,
    }
}

#[derive(Default)]
struct State {
    screens: Vec<Screen>,
    cursor: (f64, f64),
    hidden: bool,
    injected: Vec<Inject>,
    core: Option<Arc<CaptureCore>>,
    caps: bool,
}

pub struct MockPlatform {
    state: Mutex<State>,
}

impl MockPlatform {
    pub fn new(screens: Vec<Screen>) -> Arc<MockPlatform> {
        let c = bounds_of(&screens).center();
        Arc::new(MockPlatform { state: Mutex::new(State { screens, cursor: c, ..Default::default() }) })
    }

    fn core(&self) -> Arc<CaptureCore> {
        self.state.lock().core.clone().expect("capture not started")
    }

    /// Simulate the user moving the physical mouse. Returns whether the OS let it through.
    pub fn user_move(&self, dx: f64, dy: f64) -> Verdict {
        let core = self.core();
        if core.is_grabbing() {
            let (x, y) = self.cursor_pos();
            return core.motion(x, y, dx, dy);
        }
        let (x, y) = {
            let s = self.state.lock();
            let (nx, ny) = (s.cursor.0 + dx, s.cursor.1 + dy);
            if s.screens.iter().any(|sc| sc.rect().contains(nx, ny)) {
                (nx, ny)
            } else {
                // Clamp like an OS would, to the screen the cursor is on.
                let cur = s.screens.iter().find(|sc| sc.rect().contains(s.cursor.0, s.cursor.1)).unwrap_or(&s.screens[0]);
                cur.rect().clamp(nx, ny)
            }
        };
        let v = core.motion(x, y, dx, dy);
        if v == Verdict::Pass {
            self.state.lock().cursor = (x, y);
        }
        v
    }

    pub fn user_key(&self, hid: u32, down: bool) -> Verdict {
        self.core().key(hid, down, false)
    }

    pub fn user_button(&self, button: MouseButton, down: bool) -> Verdict {
        self.core().button(button, down)
    }

    pub fn take_injected(&self) -> Vec<Inject> {
        std::mem::take(&mut self.state.lock().injected)
    }

    pub fn cursor_hidden(&self) -> bool {
        self.state.lock().hidden
    }

    pub fn set_screens(&self, screens: Vec<Screen>) {
        self.state.lock().screens = screens;
    }
}

impl Platform for MockPlatform {
    fn screens(&self) -> Vec<Screen> {
        self.state.lock().screens.clone()
    }
    fn cursor_pos(&self) -> (f64, f64) {
        self.state.lock().cursor
    }
    fn start_capture(&self, core: Arc<CaptureCore>) -> anyhow::Result<()> {
        core.emit(crate::capture::InputEvent::CaptureRunning);
        self.state.lock().core = Some(core);
        Ok(())
    }
    fn grab_begin(&self) {
        self.state.lock().hidden = true;
    }
    fn grab_end(&self, x: f64, y: f64) {
        let mut s = self.state.lock();
        s.hidden = false;
        s.cursor = (x, y);
    }
    fn warp(&self, x: f64, y: f64) {
        self.state.lock().cursor = (x, y);
    }
    fn inject(&self, ev: &Inject) {
        let mut s = self.state.lock();
        match ev {
            Inject::MoveTo { x, y } => s.cursor = (*x, *y),
            Inject::SetCapsLock(on) => s.caps = *on,
            _ => {}
        }
        s.injected.push(ev.clone());
    }
    fn precise_deltas(&self) -> bool {
        true
    }
    fn caps_lock(&self) -> Option<bool> {
        Some(self.state.lock().caps)
    }
}
