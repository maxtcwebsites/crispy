//! Small value types shared by every layer (engine, network, UI).

use serde::{Deserialize, Serialize};

pub type DeviceId = String;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Windows,
    Linux,
}

impl Os {
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }

    /// Windows and Linux share PC keyboard conventions; macOS is the odd one out.
    pub fn is_mac(self) -> bool {
        matches!(self, Os::Macos)
    }
}

/// A rectangle in some device's native cursor coordinate space
/// (points on macOS, physical pixels on Windows). Origin is top-left, y grows downwards.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    /// Clamp a point so that it lies inside the rectangle (inclusive of the last pixel).
    pub fn clamp(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x.clamp(self.x, (self.right() - 1.0).max(self.x)),
            y.clamp(self.y, (self.bottom() - 1.0).max(self.y)),
        )
    }
    pub fn union(&self, o: &Rect) -> Rect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect::new(x, y, self.right().max(o.right()) - x, self.bottom().max(o.bottom()) - y)
    }
    pub fn offset(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
    pub fn distance_sq(&self, x: f64, y: f64) -> f64 {
        let (cx, cy) = self.clamp(x, y);
        (cx - x).powi(2) + (cy - y).powi(2)
    }
}

/// One physical display attached to a device.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Backing scale factor (2.0 on Retina, 1.5 for 150% on Windows...).
    pub scale: f64,
    pub primary: bool,
}

impl Screen {
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

pub fn bounds_of(screens: &[Screen]) -> Rect {
    let mut it = screens.iter();
    match it.next() {
        None => Rect::new(0.0, 0.0, 1920.0, 1080.0),
        Some(first) => it.fold(first.rect(), |acc, s| acc.union(&s.rect())),
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl MouseButton {
    pub const ALL: [MouseButton; 5] = [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ];
    pub fn bit(self) -> u8 {
        match self {
            MouseButton::Left => 1,
            MouseButton::Right => 2,
            MouseButton::Middle => 4,
            MouseButton::Back => 8,
            MouseButton::Forward => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_clamp_and_contains() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert!(r.contains(0.0, 0.0));
        assert!(r.contains(99.5, 49.0));
        assert!(!r.contains(100.0, 10.0));
        assert_eq!(r.clamp(150.0, -5.0), (99.0, 0.0));
    }

    #[test]
    fn bounds_cover_all_screens() {
        let s = |x, y, w, h| Screen {
            id: "a".into(),
            name: "a".into(),
            x,
            y,
            width: w,
            height: h,
            scale: 1.0,
            primary: false,
        };
        let b = bounds_of(&[s(0.0, 0.0, 1920.0, 1080.0), s(-1280.0, 200.0, 1280.0, 1024.0)]);
        assert_eq!(b, Rect::new(-1280.0, 0.0, 3200.0, 1224.0));
    }
}
