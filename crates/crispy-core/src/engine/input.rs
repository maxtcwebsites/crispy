//! Who has the cursor: switching at screen edges, driving a peer, and being driven by one.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::Engine;
use crate::capture::{HotkeyAction, InputEvent};
use crate::keymap::{self, is_media};
use crate::layout::{corner_of, Corner, Landing, Step};
use crate::platform::Inject;
use crate::protocol::Msg;
use crate::state::{CaptureStatus, PointerEvent, UiEvent};
use crate::types::{DeviceId, Direction, MouseButton, Os, Screen};

#[derive(Clone, Debug, PartialEq)]
pub enum Focus {
    /// This computer's keyboard and mouse work on this computer.
    Local,
    /// This computer's keyboard and mouse drive `peer`; (x, y) is the cursor there.
    Remote { peer: DeviceId, x: f64, y: f64 },
}

#[derive(Default)]
pub(crate) struct InputState {
    /// Edge the cursor is currently resting against, and since when.
    edge: Option<(Direction, Instant)>,
    last_tap: Option<(Direction, Instant)>,
    /// A local switch is waiting for the edge delay to pass.
    pending_local: bool,
    /// Where the cursor left this computer, to come back to if the connection drops.
    exit_point: Option<(f64, f64)>,
    /// Buttons held while driving a peer (for "don't switch while dragging").
    remote_buttons: u8,
    wheel_rest: (f64, f64),
    /// Keys and buttons a peer is holding down *on this computer*.
    injected_keys: HashSet<u32>,
    injected_buttons: u8,
    takeover_motion: f64,
    last_pointer: Option<Instant>,
}

/// Native cursor units per logical point (macOS uses points; Windows uses physical pixels).
fn unit_scale(os: Os, screen: Option<&Screen>) -> f64 {
    if os.is_mac() {
        1.0
    } else {
        screen.map(|s| s.scale).filter(|s| *s > 0.0).unwrap_or(1.0)
    }
}

impl Engine {
    pub(crate) fn on_input(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::CaptureRunning => {
                self.capture_status = CaptureStatus::Running;
                self.capture_detail = None;
                self.dirty = true;
            }
            InputEvent::CaptureFailed(detail) => {
                self.capture_status = CaptureStatus::Failed;
                self.capture_detail = Some(detail);
                self.return_home();
                self.dirty = true;
            }
            InputEvent::Hotkey(a) => self.on_hotkey(a),
            InputEvent::Key { hid, down, repeat } => {
                self.mods.update(hid, down);
                if let Focus::Remote { peer, .. } = &self.focus {
                    if is_media(hid) && !self.settings.keyboard.forward_media_keys {
                        self.platform.inject(&Inject::Key { hid, down, repeat });
                        return;
                    }
                    let peer = peer.clone();
                    self.events_this_sec += 1;
                    self.send_to(&peer, Msg::Key { hid, down, repeat });
                }
            }
            InputEvent::Button { button, down } => {
                if let Focus::Remote { peer, .. } = &self.focus {
                    let peer = peer.clone();
                    if down {
                        self.input.remote_buttons |= button.bit();
                    } else {
                        self.input.remote_buttons &= !button.bit();
                    }
                    self.events_this_sec += 1;
                    self.send_to(&peer, Msg::Button { button, down });
                } else if down && self.controlled_by.is_some() && self.settings.switching.local_input_takes_over {
                    self.take_back_control();
                }
            }
            InputEvent::Motion { x, y, dx, dy } => {
                if matches!(self.focus, Focus::Remote { .. }) {
                    self.remote_motion(dx, dy);
                } else {
                    self.local_motion(x, y, dx, dy);
                }
            }
            InputEvent::Wheel { dx, dy, continuous } => {
                if let Focus::Remote { peer, .. } = &self.focus {
                    let peer = peer.clone();
                    let m = &self.settings.mouse;
                    let mut dx = dx * m.scroll_speed * if m.invert_horizontal { -1.0 } else { 1.0 };
                    let mut dy = dy * m.scroll_speed * if m.invert_vertical { -1.0 } else { 1.0 };
                    let mut continuous = continuous;
                    if !m.smooth_scrolling {
                        // Whole notches only: accumulate the fractions.
                        self.input.wheel_rest.0 += dx;
                        self.input.wheel_rest.1 += dy;
                        dx = self.input.wheel_rest.0.trunc();
                        dy = self.input.wheel_rest.1.trunc();
                        self.input.wheel_rest.0 -= dx;
                        self.input.wheel_rest.1 -= dy;
                        continuous = false;
                        if dx == 0.0 && dy == 0.0 {
                            return;
                        }
                    }
                    self.events_this_sec += 1;
                    self.send_to(&peer, Msg::Wheel { dx, dy, continuous });
                }
            }
        }
    }

    pub(crate) fn on_hotkey(&mut self, action: HotkeyAction) {
        match action {
            HotkeyAction::LockToScreen => {
                self.locked = !self.locked;
                let (title, body) = if self.locked {
                    ("Cursor locked", "The cursor stays on this screen until you unlock it.")
                } else {
                    ("Cursor unlocked", "Move across a shared edge to switch computers.")
                };
                self.toast("info", title, body);
            }
            HotkeyAction::BringHome => self.return_home(),
            HotkeyAction::NextDevice => self.cycle_device(),
            HotkeyAction::PauseSharing => {
                self.paused = !self.paused;
                if self.paused {
                    self.return_home();
                    self.toast("info", "Sharing paused", "Your keyboard and mouse stay on this computer.");
                } else {
                    self.toast("info", "Sharing resumed", "Move across a shared edge to switch computers.");
                }
            }
        }
        self.dirty = true;
    }

    fn switching_allowed(&self) -> bool {
        self.settings.switching.enabled && !self.locked && !self.paused && self.controlled_by.is_none()
    }

    /// Does the edge delay / double-tap / modifier / dragging policy allow switching now?
    fn edge_gate(&mut self, dir: Direction, dragging: bool) -> bool {
        let s = &self.settings.switching;
        let now = Instant::now();
        let arrived = match self.input.edge {
            Some((d, _)) if d == dir => false,
            _ => {
                self.input.edge = Some((dir, now));
                true
            }
        };
        if s.block_while_dragging && dragging {
            return false;
        }
        if let Some(m) = s.required_modifier.key() {
            if !self.mods.has(m) {
                return false;
            }
        }
        if s.double_tap {
            if arrived {
                let window = Duration::from_millis(s.double_tap_window_ms as u64);
                let second = self.input.last_tap.is_some_and(|(d, t)| d == dir && now.duration_since(t) <= window);
                if second {
                    self.input.last_tap = None;
                } else {
                    self.input.last_tap = Some((dir, now));
                    self.input.edge = None; // must leave the edge and come back
                    return false;
                }
            } else {
                return false;
            }
        }
        let delay = Duration::from_millis(s.delay_ms as u64);
        let since = self.input.edge.map(|(_, t)| t).unwrap_or(now);
        now.duration_since(since) >= delay
    }

    fn corner_blocked(&self, screen: &crate::types::Rect, x: f64, y: f64, dir: Direction) -> bool {
        let s = &self.settings.switching;
        match corner_of(screen, x, y, dir, s.corner_size as f64) {
            Some(Corner::TopLeft) => s.corners.top_left,
            Some(Corner::TopRight) => s.corners.top_right,
            Some(Corner::BottomLeft) => s.corners.bottom_left,
            Some(Corner::BottomRight) => s.corners.bottom_right,
            None => false,
        }
    }

    fn can_drive(&self, peer: &str) -> bool {
        self.peers.contains_key(peer) && self.trust.peers.get(peer).is_some_and(|p| p.enabled)
    }

    fn local_motion(&mut self, x: f64, y: f64, dx: f64, dy: f64) {
        if self.controlled_by.is_some() {
            if self.settings.switching.local_input_takes_over {
                self.input.takeover_motion += dx.abs() + dy.abs();
                if self.input.takeover_motion > 6.0 {
                    self.take_back_control();
                }
            }
            return;
        }
        if !self.peers.is_empty() {
            self.emit_pointer(self.my_id.clone(), x, y);
        }
        self.try_local_switch(x, y, dx, dy);
    }

    fn try_local_switch(&mut self, x: f64, y: f64, dx: f64, dy: f64) {
        self.input.pending_local = false;
        if !self.switching_allowed() || self.peers.is_empty() {
            self.input.edge = None;
            return;
        }
        let me = self.my_id.clone();
        let Some(screen) = self.world.screen_at(&me, x, y).cloned() else { return };
        let r = screen.local;
        let precise = self.platform.precise_deltas();
        // With exact deltas (macOS) the cursor must be pushing outwards; otherwise (Windows) reaching
        // the last pixel is the push, as long as the cursor isn't moving away from it.
        let mut dirs = Vec::with_capacity(2);
        if x <= r.x && (dx < 0.0 || (!precise && dx <= 0.0)) {
            dirs.push(Direction::Left);
        }
        if x >= r.right() - 1.0 && (dx > 0.0 || (!precise && dx >= 0.0)) {
            dirs.push(Direction::Right);
        }
        if y <= r.y && (dy < 0.0 || (!precise && dy <= 0.0)) {
            dirs.push(Direction::Up);
        }
        if y >= r.bottom() - 1.0 && (dy > 0.0 || (!precise && dy >= 0.0)) {
            dirs.push(Direction::Down);
        }
        if dirs.is_empty() {
            self.input.edge = None;
            return;
        }
        for dir in dirs {
            if !self.world.edge_is_open(&me, x, y, dir) || self.corner_blocked(&r, x, y, dir) {
                continue;
            }
            let Some(landing) = self.world.neighbor(&screen, dir, x, y, self.settings.switching.edge_mapping) else { continue };
            if !self.can_drive(&landing.device) {
                continue;
            }
            let dragging = self.capture.local_buttons() != 0;
            if !self.edge_gate(dir, dragging) {
                if self.settings.switching.delay_ms > 0 && !self.settings.switching.double_tap {
                    self.input.pending_local = true;
                }
                return;
            }
            if self.settings.switching.block_fullscreen && self.platform.fullscreen_active() {
                return;
            }
            self.input.exit_point = Some((x, y));
            self.enter(landing);
            return;
        }
    }

    /// Re-check a delayed switch even when the mouse isn't moving (Windows stops sending events at the edge).
    pub(crate) fn input_tick(&mut self) {
        if self.input.pending_local && matches!(self.focus, Focus::Local) {
            let (x, y) = self.platform.cursor_pos();
            self.try_local_switch(x, y, 0.0, 0.0);
        }
    }

    fn remote_motion(&mut self, dx: f64, dy: f64) {
        let Focus::Remote { peer, x, y } = self.focus.clone() else { return };
        let me_scale = unit_scale(self.my_os, self.screens.iter().find(|s| s.primary).or(self.screens.first()));
        let peer_os = self.trust.peers.get(&peer).map(|p| p.os).unwrap_or(Os::Windows);
        let target = self.world.screen_at(&peer, x, y).map(|s| s.local);
        let peer_screen = self.peer_screens(&peer).into_iter().find(|s| target.is_some_and(|t| t == s.rect()));
        let k = self.settings.mouse.pointer_speed * unit_scale(peer_os, peer_screen.as_ref()) / me_scale;
        let (dx, dy) = (dx * k, dy * k);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.events_this_sec += 1;
        let relative = self.settings.mouse.relative_movement;
        match self.world.step(&peer, x, y, dx, dy) {
            Step::Moved(nx, ny) => {
                self.input.edge = None;
                self.move_remote(&peer, nx, ny, dx, dy, relative);
            }
            Step::Edge { x: cx, y: cy, dirs } => {
                if !self.locked && !self.paused && self.settings.switching.enabled {
                    if let Some(screen) = self.world.screen_at(&peer, cx, cy).cloned() {
                        for dir in dirs {
                            if self.corner_blocked(&screen.local, cx, cy, dir) {
                                continue;
                            }
                            let Some(landing) = self.world.neighbor(&screen, dir, cx, cy, self.settings.switching.edge_mapping) else {
                                continue;
                            };
                            if landing.device != self.my_id && !self.can_drive(&landing.device) {
                                continue;
                            }
                            let dragging = self.input.remote_buttons != 0;
                            if !self.edge_gate(dir, dragging) {
                                break;
                            }
                            if landing.device == self.my_id {
                                self.come_back(landing.x, landing.y);
                            } else {
                                self.send_to(&peer, Msg::Leave);
                                self.enter(landing);
                            }
                            return;
                        }
                    }
                }
                self.move_remote(&peer, cx, cy, dx, dy, relative);
            }
        }
    }

    fn move_remote(&mut self, peer: &DeviceId, x: f64, y: f64, dx: f64, dy: f64, relative: bool) {
        self.focus = Focus::Remote { peer: peer.clone(), x, y };
        if relative {
            self.send_to(peer, Msg::MouseRel { dx, dy });
        } else {
            self.send_to(peer, Msg::MouseMove { x, y });
        }
        self.emit_pointer(peer.clone(), x, y);
    }

    fn peer_screens(&self, peer: &str) -> Vec<Screen> {
        self.peers.get(peer).and_then(|c| self.conns.get(c)).map(|c| c.screens.clone()).unwrap_or_default()
    }

    /// Hand the keyboard and mouse to the device at `landing`.
    fn enter(&mut self, landing: Landing) {
        let Landing { device, x, y } = landing;
        let was_local = matches!(self.focus, Focus::Local);
        let held: Vec<u32> = self.mods.held_keys().into_iter().filter(|k| keymap::modifier_of(*k).is_some()).collect();
        let caps = if self.settings.keyboard.sync_caps_lock { self.platform.caps_lock() } else { None };
        if was_local {
            self.capture.set_grab(true);
            self.platform.grab_begin();
        }
        self.input.edge = None;
        self.input.remote_buttons = 0;
        self.focus = Focus::Remote { peer: device.clone(), x, y };
        self.send_to(&device, Msg::Enter { x, y, held, caps_lock: caps });
        self.on_focus_moved_to_peer(&device);
        let name = self.peer_name(&device);
        let _ = self.ui.send(UiEvent::FocusChanged(Some(name)));
        self.emit_pointer(device, x, y);
        self.dirty = true;
    }

    /// The cursor came back to this computer at (x, y).
    fn come_back(&mut self, x: f64, y: f64) {
        if let Focus::Remote { peer, .. } = std::mem::replace(&mut self.focus, Focus::Local) {
            self.send_to(&peer, Msg::Leave);
        }
        self.capture.set_grab(false);
        self.platform.grab_end(x, y);
        self.input.edge = None;
        self.input.remote_buttons = 0;
        let _ = self.ui.send(UiEvent::FocusChanged(None));
        self.emit_pointer(self.my_id.clone(), x, y);
        self.dirty = true;
    }

    /// Bring the cursor home: to where it left, or the middle of the main display.
    pub(crate) fn return_home(&mut self) {
        if !matches!(self.focus, Focus::Remote { .. }) {
            return;
        }
        let (x, y) = self.home_point();
        self.come_back(x, y);
    }

    fn home_point(&self) -> (f64, f64) {
        let primary = self.screens.iter().find(|s| s.primary).or(self.screens.first());
        match (self.input.exit_point, primary) {
            (Some((x, y)), _) if self.screens.iter().any(|s| s.rect().contains(x, y)) => {
                // Step back from the edge so we don't bounce straight out again.
                let s = self.screens.iter().find(|s| s.rect().contains(x, y)).unwrap().rect();
                let inset = 24.0f64.min(s.w / 4.0).min(s.h / 4.0);
                (x.clamp(s.x + inset, s.right() - inset), y.clamp(s.y + inset, s.bottom() - inset))
            }
            (_, Some(p)) => p.rect().center(),
            _ => (0.0, 0.0),
        }
    }

    fn cycle_device(&mut self) {
        let mut order = vec![self.my_id.clone()];
        order.extend(super::sorted_ids(
            self.peers.keys().filter(|id| self.can_drive(id)).filter_map(|id| self.trust.peers.get(id).map(|p| (&p.id, &p.name))),
        ));
        if order.len() < 2 {
            return;
        }
        let current = match &self.focus {
            Focus::Local => self.my_id.clone(),
            Focus::Remote { peer, .. } => peer.clone(),
        };
        let idx = order.iter().position(|d| *d == current).unwrap_or(0);
        let next = order[(idx + 1) % order.len()].clone();
        if next == self.my_id {
            self.return_home();
            return;
        }
        let Some(center) = self.peer_screens(&next).iter().find(|s| s.primary).or(self.peer_screens(&next).first()).map(|s| s.rect().center())
        else {
            return;
        };
        if let Focus::Remote { peer, .. } = &self.focus {
            let peer = peer.clone();
            self.send_to(&peer, Msg::Leave);
        } else {
            self.input.exit_point = Some(self.platform.cursor_pos());
        }
        self.enter(Landing { device: next, x: center.0, y: center.1 });
    }

    pub(crate) fn emit_pointer(&mut self, device: DeviceId, x: f64, y: f64) {
        let now = Instant::now();
        if self.input.last_pointer.is_some_and(|t| now.duration_since(t) < Duration::from_millis(33)) {
            return;
        }
        self.input.last_pointer = Some(now);
        let _ = self.ui.send(UiEvent::Pointer(PointerEvent { device, x, y }));
    }

    // ------------------------------------------------------------------------------------------
    // Being driven by a peer
    // ------------------------------------------------------------------------------------------

    pub(crate) fn on_control_msg(&mut self, from: &DeviceId, msg: Msg) {
        match msg {
            Msg::Enter { x, y, held, caps_lock } => {
                // Someone else's keyboard and mouse are in use now: stop driving anyone ourselves.
                if let Focus::Remote { peer, .. } = self.focus.clone() {
                    self.send_to(&peer, Msg::Leave);
                    self.focus = Focus::Local;
                    self.capture.set_grab(false);
                    let (cx, cy) = self.clamp_local(x, y);
                    self.platform.grab_end(cx, cy);
                    let _ = self.ui.send(UiEvent::FocusChanged(None));
                }
                if let Some(prev) = self.controlled_by.clone() {
                    if prev != *from {
                        self.release_remote_input();
                        self.send_to(&prev, Msg::TakeOver);
                    }
                }
                self.controlled_by = Some(from.clone());
                self.input.takeover_motion = 0.0;
                let (cx, cy) = self.clamp_local(x, y);
                self.platform.inject(&Inject::MoveTo { x: cx, y: cy });
                let map = self.key_map_from(from);
                for k in held {
                    let k = map.apply(k);
                    if self.input.injected_keys.insert(k) {
                        self.platform.inject(&Inject::Key { hid: k, down: true, repeat: false });
                    }
                }
                if let Some(on) = caps_lock {
                    self.platform.inject(&Inject::SetCapsLock(on));
                }
                self.emit_pointer(self.my_id.clone(), cx, cy);
                self.dirty = true;
            }
            Msg::Leave => {
                if self.controlled_by.as_deref() == Some(from.as_str()) {
                    self.release_remote_input();
                    self.controlled_by = None;
                    self.on_peer_left_us(from);
                    self.dirty = true;
                }
            }
            Msg::TakeOver => {
                if matches!(&self.focus, Focus::Remote { peer, .. } if peer == from) {
                    let (x, y) = self.home_point();
                    self.focus = Focus::Local;
                    self.capture.set_grab(false);
                    self.platform.grab_end(x, y);
                    let _ = self.ui.send(UiEvent::FocusChanged(None));
                    self.dirty = true;
                }
            }
            other => {
                if self.controlled_by.as_deref() != Some(from.as_str()) {
                    return;
                }
                self.events_this_sec += 1;
                match other {
                    Msg::MouseMove { x, y } => {
                        let (cx, cy) = self.clamp_local(x, y);
                        self.platform.inject(&Inject::MoveTo { x: cx, y: cy });
                        self.emit_pointer(self.my_id.clone(), cx, cy);
                    }
                    Msg::MouseRel { dx, dy } => self.platform.inject(&Inject::MoveBy { dx, dy }),
                    Msg::Button { button, down } => {
                        let bit = button.bit();
                        if down {
                            self.input.injected_buttons |= bit;
                        } else if self.input.injected_buttons & bit == 0 {
                            return;
                        } else {
                            self.input.injected_buttons &= !bit;
                        }
                        self.platform.inject(&Inject::Button { button, down });
                    }
                    Msg::Wheel { dx, dy, continuous } => self.platform.inject(&Inject::Wheel { dx, dy, continuous }),
                    Msg::Key { hid, down, repeat } => {
                        let k = self.key_map_from(from).apply(hid);
                        if down {
                            self.input.injected_keys.insert(k);
                        } else if !self.input.injected_keys.remove(&k) {
                            return; // never pressed here: nothing to release
                        }
                        self.platform.inject(&Inject::Key { hid: k, down, repeat });
                    }
                    _ => {}
                }
            }
        }
    }

    fn key_map_from(&self, from: &str) -> keymap::ModifierMap {
        let their_os = self.trust.peers.get(from).map(|p| p.os).unwrap_or(self.my_os);
        self.settings.keyboard.map_for(their_os, self.my_os)
    }

    fn clamp_local(&self, x: f64, y: f64) -> (f64, f64) {
        if self.screens.iter().any(|s| s.rect().contains(x, y)) {
            return (x, y);
        }
        self.screens
            .iter()
            .map(|s| s.rect())
            .min_by(|a, b| a.distance_sq(x, y).partial_cmp(&b.distance_sq(x, y)).unwrap())
            .map(|r| r.clamp(x, y))
            .unwrap_or((x, y))
    }

    /// Let go of every key and button a peer was holding on this computer.
    pub(crate) fn release_remote_input(&mut self) {
        for k in std::mem::take(&mut self.input.injected_keys) {
            self.platform.inject(&Inject::Key { hid: k, down: false, repeat: false });
        }
        let buttons = std::mem::take(&mut self.input.injected_buttons);
        for b in MouseButton::ALL {
            if buttons & b.bit() != 0 {
                self.platform.inject(&Inject::Button { button: b, down: false });
            }
        }
    }

    /// The person at this computer touched its own mouse: take control back from the peer.
    fn take_back_control(&mut self) {
        if let Some(peer) = self.controlled_by.take() {
            self.release_remote_input();
            self.send_to(&peer, Msg::TakeOver);
            self.input.takeover_motion = 0.0;
            self.dirty = true;
        }
    }

    /// A connection dropped: make sure nothing is left grabbed or held down.
    pub(crate) fn input_peer_gone(&mut self, peer: &str) {
        if matches!(&self.focus, Focus::Remote { peer: p, .. } if p == peer) {
            let (x, y) = self.home_point();
            self.focus = Focus::Local;
            self.capture.set_grab(false);
            self.platform.grab_end(x, y);
            let _ = self.ui.send(UiEvent::FocusChanged(None));
        }
        if self.controlled_by.as_deref() == Some(peer) {
            self.release_remote_input();
            self.controlled_by = None;
        }
    }
}
