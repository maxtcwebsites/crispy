//! The shared arrangement of every device's displays, and the geometry of moving between them.
//!
//! Each device keeps its own OS arrangement of monitors (in its native coordinates). The shared
//! [`Layout`] only stores where each *device* sits on a common canvas, so the internal monitor
//! arrangement is always whatever the OS says it is. Units on the canvas are each device's native
//! cursor units (points on macOS, pixels on Windows) which mirrors how macOS' own display
//! arrangement panel sizes displays.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::types::{bounds_of, DeviceId, Direction, Rect, Screen};

/// How far inside the target screen the cursor lands, so that it does not immediately bounce back.
pub const LANDING_INSET: f64 = 2.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

/// The arrangement every peer agrees on. Conflicts are resolved by (rev, author): the highest wins,
/// so concurrent edits on two machines converge without a coordinator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Layout {
    pub rev: u64,
    pub author: DeviceId,
    pub positions: BTreeMap<DeviceId, Pos>,
}

impl Layout {
    pub fn supersedes(&self, other: &Layout) -> bool {
        (self.rev, &self.author) > (other.rev, &other.author)
    }

    /// Give every device that has no position yet a sensible spot to the right of the existing
    /// arrangement. Returns true when something was placed.
    pub fn place_missing(&mut self, devices: &[(DeviceId, Rect)]) -> bool {
        let mut missing: Vec<&(DeviceId, Rect)> =
            devices.iter().filter(|(id, _)| !self.positions.contains_key(id)).collect();
        if missing.is_empty() {
            return false;
        }
        missing.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, bounds) in missing {
            let placed = devices
                .iter()
                .filter_map(|(oid, b)| self.positions.get(oid).map(|p| Rect::new(p.x, p.y, b.w, b.h)))
                .reduce(|a, b| a.union(&b));
            let pos = match placed {
                None => Pos { x: 0.0, y: 0.0 },
                Some(u) => Pos { x: u.right(), y: (u.y + (u.h - bounds.h) / 2.0).round() },
            };
            self.positions.insert(id.clone(), pos);
        }
        true
    }
}

/// A screen positioned on the shared canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldScreen {
    pub device: DeviceId,
    pub local: Rect,
    pub global: Rect,
}

/// Where the cursor should appear after crossing an edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Landing {
    pub device: DeviceId,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum EdgeMapping {
    /// The cursor keeps its position along the edge, as if the screens were one big desk.
    #[default]
    Direct,
    /// The position along the edge is scaled, so the whole edge maps onto the whole neighbour edge.
    Proportional,
}

/// The result of moving a cursor that Crispy tracks itself (the cursor on a remote device).
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// The cursor moved to a point inside one of the device's screens.
    Moved(f64, f64),
    /// The movement left every screen; `x, y` is the clamped point and `dirs` the edges it pushed
    /// against, most significant first.
    Edge { x: f64, y: f64, dirs: Vec<Direction> },
}

#[derive(Default, Debug, Clone)]
pub struct World {
    pub screens: Vec<WorldScreen>,
    offsets: HashMap<DeviceId, (f64, f64)>,
}

impl World {
    pub fn build(layout: &Layout, devices: &[(DeviceId, Vec<Screen>)]) -> World {
        let mut w = World::default();
        for (id, screens) in devices {
            if screens.is_empty() {
                continue;
            }
            let Some(pos) = layout.positions.get(id) else { continue };
            let b = bounds_of(screens);
            let (ox, oy) = (pos.x - b.x, pos.y - b.y);
            w.offsets.insert(id.clone(), (ox, oy));
            for s in screens {
                let local = s.rect();
                w.screens.push(WorldScreen { device: id.clone(), local, global: local.offset(ox, oy) });
            }
        }
        w
    }

    pub fn has_device(&self, id: &str) -> bool {
        self.offsets.contains_key(id)
    }

    pub fn to_global(&self, device: &str, x: f64, y: f64) -> Option<(f64, f64)> {
        self.offsets.get(device).map(|(ox, oy)| (x + ox, y + oy))
    }

    pub fn device_screens<'a>(&'a self, device: &'a str) -> impl Iterator<Item = &'a WorldScreen> + 'a {
        self.screens.iter().filter(move |s| s.device == device)
    }

    /// The device screen containing the point, or the closest one.
    pub fn screen_at(&self, device: &str, x: f64, y: f64) -> Option<&WorldScreen> {
        let mine = || self.screens.iter().filter(|s| s.device == device);
        mine().find(|s| s.local.contains(x, y)).or_else(|| {
            mine().min_by(|a, b| a.local.distance_sq(x, y).partial_cmp(&b.local.distance_sq(x, y)).unwrap())
        })
    }

    /// Is the edge of `device`'s screen at (x, y) open, i.e. does the device itself have no screen
    /// right behind it in that direction? (If it does, the OS moves the cursor there itself.)
    pub fn edge_is_open(&self, device: &str, x: f64, y: f64, dir: Direction) -> bool {
        let (px, py) = match dir {
            Direction::Left => (x - 1.0, y),
            Direction::Right => (x + 1.0, y),
            Direction::Up => (x, y - 1.0),
            Direction::Down => (x, y + 1.0),
        };
        !self.device_screens(device).any(|s| s.local.contains(px, py))
    }

    /// Find the screen of another device that sits beyond `from` in `dir`, and where to land on it.
    /// `x, y` is the point (in `from.device` local coordinates) where the cursor hit the edge.
    pub fn neighbor(
        &self,
        from: &WorldScreen,
        dir: Direction,
        x: f64,
        y: f64,
        mapping: EdgeMapping,
    ) -> Option<Landing> {
        let (gx, gy) = self.to_global(&from.device, x, y)?;
        let fr = from.global;
        let along = if horizontal(dir) { gy } else { gx };
        let (from_lo, from_hi) = if horizontal(dir) { (fr.y, fr.bottom()) } else { (fr.x, fr.right()) };

        let mut candidates: Vec<(f64, f64, &WorldScreen)> = Vec::new();
        for s in self.screens.iter().filter(|s| s.device != from.device) {
            let g = s.global;
            let (cx, cy) = g.center();
            let (beyond, gap) = match dir {
                Direction::Right => (cx > fr.right(), g.x - fr.right()),
                Direction::Left => (cx < fr.x, fr.x - g.right()),
                Direction::Down => (cy > fr.bottom(), g.y - fr.bottom()),
                Direction::Up => (cy < fr.y, fr.y - g.bottom()),
            };
            if !beyond {
                continue;
            }
            let (lo, hi) = if horizontal(dir) { (g.y, g.bottom()) } else { (g.x, g.right()) };
            if hi.min(from_hi) - lo.max(from_lo) <= 0.0 {
                continue; // not facing this edge at all
            }
            let along_dist = if along < lo { lo - along } else if along >= hi { along - hi + 1.0 } else { 0.0 };
            candidates.push((gap.max(0.0), along_dist, s));
        }
        let min_gap = candidates.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
        let (_, _, target) = candidates
            .into_iter()
            .filter(|c| c.0 <= min_gap + 2.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())?;

        let g = target.global;
        let (lo, hi) = if horizontal(dir) { (g.y, g.bottom()) } else { (g.x, g.right()) };
        let mapped = match mapping {
            EdgeMapping::Direct => along,
            EdgeMapping::Proportional => {
                let t = ((along - from_lo) / (from_hi - from_lo).max(1.0)).clamp(0.0, 1.0);
                lo + t * (hi - lo)
            }
        };
        let mapped = mapped.clamp(lo, hi - 1.0);
        let (lx, ly) = match dir {
            Direction::Right => (g.x + LANDING_INSET, mapped),
            Direction::Left => (g.right() - 1.0 - LANDING_INSET, mapped),
            Direction::Down => (mapped, g.y + LANDING_INSET),
            Direction::Up => (mapped, g.bottom() - 1.0 - LANDING_INSET),
        };
        let (ox, oy) = self.offsets[&target.device];
        Some(Landing { device: target.device.clone(), x: lx - ox, y: ly - oy })
    }

    /// Move a cursor that Crispy tracks itself across a device's screens, imitating how the OS
    /// moves a real cursor: free inside screens, sliding along edges, stopping at open edges.
    pub fn step(&self, device: &str, x: f64, y: f64, dx: f64, dy: f64) -> Step {
        let rects: Vec<Rect> = self.device_screens(device).map(|s| s.local).collect();
        step_within(&rects, x, y, dx, dy)
    }
}

pub fn step_within(rects: &[Rect], x: f64, y: f64, dx: f64, dy: f64) -> Step {
    let (nx, ny) = (x + dx, y + dy);
    if rects.iter().any(|r| r.contains(nx, ny)) {
        return Step::Moved(nx, ny);
    }
    let Some(cur) = rects
        .iter()
        .find(|r| r.contains(x, y))
        .or_else(|| rects.iter().min_by(|a, b| a.distance_sq(x, y).partial_cmp(&b.distance_sq(x, y)).unwrap()))
    else {
        return Step::Moved(x, y);
    };
    let (cx, cy) = cur.clamp(nx, ny);
    // Slide along an edge into a neighbouring screen of the same device when only one axis is blocked.
    if dx != 0.0 && rects.iter().any(|r| r != cur && r.contains(nx, cy)) {
        return Step::Moved(nx, cy);
    }
    if dy != 0.0 && rects.iter().any(|r| r != cur && r.contains(cx, ny)) {
        return Step::Moved(cx, ny);
    }
    let mut over: Vec<(f64, Direction)> = Vec::with_capacity(2);
    if nx >= cur.right() {
        over.push((nx - (cur.right() - 1.0), Direction::Right));
    } else if nx < cur.x {
        over.push((cur.x - nx, Direction::Left));
    }
    if ny >= cur.bottom() {
        over.push((ny - (cur.bottom() - 1.0), Direction::Down));
    } else if ny < cur.y {
        over.push((cur.y - ny, Direction::Up));
    }
    over.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    Step::Edge { x: cx, y: cy, dirs: over.into_iter().map(|o| o.1).collect() }
}

pub fn horizontal(dir: Direction) -> bool {
    matches!(dir, Direction::Left | Direction::Right)
}

/// Is the point in a corner zone of the given edge? Returns which corner, if any.
pub fn corner_of(screen: &Rect, x: f64, y: f64, dir: Direction, size: f64) -> Option<Corner> {
    if size <= 0.0 {
        return None;
    }
    let near_start = |v: f64, lo: f64| v < lo + size;
    let near_end = |v: f64, hi: f64| v >= hi - size;
    match dir {
        Direction::Left | Direction::Right => {
            let left = dir == Direction::Left;
            if near_start(y, screen.y) {
                Some(if left { Corner::TopLeft } else { Corner::TopRight })
            } else if near_end(y, screen.bottom()) {
                Some(if left { Corner::BottomLeft } else { Corner::BottomRight })
            } else {
                None
            }
        }
        Direction::Up | Direction::Down => {
            let top = dir == Direction::Up;
            if near_start(x, screen.x) {
                Some(if top { Corner::TopLeft } else { Corner::BottomLeft })
            } else if near_end(x, screen.right()) {
                Some(if top { Corner::TopRight } else { Corner::BottomRight })
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: f64, y: f64, w: f64, h: f64) -> Screen {
        Screen { id: format!("{x},{y}"), name: String::new(), x, y, width: w, height: h, scale: 1.0, primary: x == 0.0 && y == 0.0 }
    }

    /// A 1440x900 Mac on the left, a 1920x1080 Windows PC on the right with a second monitor.
    fn desk() -> (Layout, Vec<(DeviceId, Vec<Screen>)>) {
        let devices = vec![
            ("mac".to_string(), vec![screen(0.0, 0.0, 1440.0, 900.0)]),
            ("win".to_string(), vec![screen(0.0, 0.0, 1920.0, 1080.0), screen(1920.0, 0.0, 1920.0, 1080.0)]),
        ];
        let mut layout = Layout::default();
        layout.positions.insert("mac".into(), Pos { x: 0.0, y: 90.0 });
        layout.positions.insert("win".into(), Pos { x: 1440.0, y: 0.0 });
        (layout, devices)
    }

    #[test]
    fn crossing_right_lands_on_windows_with_direct_mapping() {
        let (layout, devices) = desk();
        let world = World::build(&layout, &devices);
        let mac = world.screen_at("mac", 1439.0, 450.0).unwrap().clone();
        assert!(world.edge_is_open("mac", 1439.0, 450.0, Direction::Right));
        let landing = world.neighbor(&mac, Direction::Right, 1439.0, 450.0, EdgeMapping::Direct).unwrap();
        assert_eq!(landing, Landing { device: "win".into(), x: LANDING_INSET, y: 540.0 });
    }

    #[test]
    fn proportional_mapping_stretches_along_the_edge() {
        let (layout, devices) = desk();
        let world = World::build(&layout, &devices);
        let mac = world.screen_at("mac", 1439.0, 0.0).unwrap().clone();
        let bottom = world.neighbor(&mac, Direction::Right, 1439.0, 899.0, EdgeMapping::Proportional).unwrap();
        assert!(bottom.y > 1070.0, "{bottom:?}");
        let top = world.neighbor(&mac, Direction::Right, 1439.0, 0.0, EdgeMapping::Proportional).unwrap();
        assert_eq!(top.y, 0.0);
    }

    #[test]
    fn windows_internal_edge_is_not_open() {
        let (layout, devices) = desk();
        let world = World::build(&layout, &devices);
        assert!(!world.edge_is_open("win", 1919.0, 500.0, Direction::Right));
        assert!(world.edge_is_open("win", 0.0, 500.0, Direction::Left));
        let win = world.screen_at("win", 0.0, 500.0).unwrap().clone();
        let back = world.neighbor(&win, Direction::Left, 0.0, 500.0, EdgeMapping::Direct).unwrap();
        assert_eq!(back, Landing { device: "mac".into(), x: 1440.0 - 1.0 - LANDING_INSET, y: 410.0 });
    }

    #[test]
    fn no_neighbor_above() {
        let (layout, devices) = desk();
        let world = World::build(&layout, &devices);
        let win = world.screen_at("win", 100.0, 0.0).unwrap().clone();
        assert_eq!(world.neighbor(&win, Direction::Up, 100.0, 0.0, EdgeMapping::Direct), None);
    }

    #[test]
    fn edge_overlap_outside_cursor_still_connects_to_nearest() {
        // Windows screen sits lower than the Mac: hitting the Mac's top-right still lands at the top of Windows.
        let (mut layout, devices) = desk();
        layout.positions.insert("win".into(), Pos { x: 1440.0, y: 500.0 });
        let world = World::build(&layout, &devices);
        let mac = world.screen_at("mac", 1439.0, 10.0).unwrap().clone();
        let l = world.neighbor(&mac, Direction::Right, 1439.0, 10.0, EdgeMapping::Direct).unwrap();
        assert_eq!((l.device.as_str(), l.y), ("win", 0.0));
    }

    #[test]
    fn virtual_cursor_moves_slides_and_stops() {
        let (layout, devices) = desk();
        let world = World::build(&layout, &devices);
        assert_eq!(world.step("win", 1900.0, 500.0, 50.0, 0.0), Step::Moved(1950.0, 500.0));
        assert_eq!(
            world.step("win", 3830.0, 500.0, 50.0, 0.0),
            Step::Edge { x: 3839.0, y: 500.0, dirs: vec![Direction::Right] }
        );
        assert_eq!(
            world.step("win", 10.0, 10.0, -30.0, -5.0),
            Step::Edge { x: 0.0, y: 5.0, dirs: vec![Direction::Left] }
        );
    }

    #[test]
    fn diagonal_slide_into_offset_monitor() {
        // Second monitor sits lower; moving right+down from the first lands inside it.
        let rects = [Rect::new(0.0, 0.0, 100.0, 100.0), Rect::new(100.0, 50.0, 100.0, 100.0)];
        assert_eq!(step_within(&rects, 95.0, 20.0, 10.0, 0.0), Step::Edge { x: 99.0, y: 20.0, dirs: vec![Direction::Right] });
        assert_eq!(step_within(&rects, 95.0, 60.0, 10.0, 0.0), Step::Moved(105.0, 60.0));
        assert_eq!(step_within(&rects, 150.0, 60.0, 0.0, -20.0), Step::Edge { x: 150.0, y: 50.0, dirs: vec![Direction::Up] });
        assert_eq!(step_within(&rects, 120.0, 140.0, -30.0, -100.0), Step::Moved(90.0, 40.0));
    }

    #[test]
    fn placement_and_conflict_resolution() {
        let mut l = Layout::default();
        let devices = vec![("b".to_string(), Rect::new(0.0, 0.0, 1920.0, 1080.0)), ("a".to_string(), Rect::new(0.0, 0.0, 1440.0, 900.0))];
        assert!(l.place_missing(&devices));
        assert_eq!(l.positions["a"], Pos { x: 0.0, y: 0.0 });
        assert_eq!(l.positions["b"], Pos { x: 1440.0, y: -90.0 });
        assert!(!l.place_missing(&devices));

        let mine = Layout { rev: 3, author: "a".into(), ..Default::default() };
        let theirs = Layout { rev: 3, author: "b".into(), ..Default::default() };
        assert!(theirs.supersedes(&mine));
        assert!(!mine.supersedes(&theirs));
    }

    #[test]
    fn corners() {
        let r = Rect::new(0.0, 0.0, 1000.0, 800.0);
        assert_eq!(corner_of(&r, 999.0, 5.0, Direction::Right, 20.0), Some(Corner::TopRight));
        assert_eq!(corner_of(&r, 999.0, 400.0, Direction::Right, 20.0), None);
        assert_eq!(corner_of(&r, 3.0, 0.0, Direction::Up, 20.0), Some(Corner::TopLeft));
    }
}
