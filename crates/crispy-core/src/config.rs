//! User settings and on-disk state. Every struct uses `#[serde(default)]`, so settings files
//! written by older or newer versions always load, and unknown fields are ignored.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::keymap::{kb, Hotkey, ModKey, ModifierMap};
use crate::layout::EdgeMapping;
use crate::types::Os;

pub const DEFAULT_PORT: u16 = 24_727;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub general: General,
    pub switching: Switching,
    pub mouse: Mouse,
    pub keyboard: Keyboard,
    pub hotkeys: Hotkeys,
    pub clipboard: Clipboard,
    pub files: Files,
    pub network: Network,
    pub advanced: Advanced,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct General {
    /// Empty = use the computer's own name.
    pub device_name: String,
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub sounds: bool,
    pub theme: Theme,
    /// Accent colour family, named after chip flavours.
    pub flavor: String,
    pub reduce_motion: bool,
    pub translucent_window: bool,
    pub onboarded: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            device_name: String::new(),
            launch_at_login: false,
            start_minimized: false,
            close_to_tray: true,
            notifications: true,
            sounds: false,
            theme: Theme::System,
            flavor: "classic".into(),
            reduce_motion: false,
            translucent_window: true,
            onboarded: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RequiredModifier {
    #[default]
    None,
    Shift,
    Ctrl,
    Alt,
    Meta,
}

impl RequiredModifier {
    pub fn key(self) -> Option<ModKey> {
        match self {
            RequiredModifier::None => None,
            RequiredModifier::Shift => Some(ModKey::Shift),
            RequiredModifier::Ctrl => Some(ModKey::Ctrl),
            RequiredModifier::Alt => Some(ModKey::Alt),
            RequiredModifier::Meta => Some(ModKey::Meta),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Corners {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_left: bool,
    pub bottom_right: bool,
}

impl Default for Corners {
    fn default() -> Self {
        Corners { top_left: true, top_right: true, bottom_left: true, bottom_right: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Switching {
    pub enabled: bool,
    /// The cursor must rest against the edge this long before switching.
    pub delay_ms: u32,
    /// Require hitting the edge twice in quick succession.
    pub double_tap: bool,
    pub double_tap_window_ms: u32,
    /// Size of the dead zone in each corner (0 = off).
    pub corner_size: u32,
    pub corners: Corners,
    /// Don't switch while a mouse button is held (e.g. while dragging a window).
    pub block_while_dragging: bool,
    pub required_modifier: RequiredModifier,
    /// Don't switch while a full-screen app or game is focused.
    pub block_fullscreen: bool,
    pub edge_mapping: EdgeMapping,
    /// Touching the mouse or keyboard of a computer that is being controlled hands control back to it.
    pub local_input_takes_over: bool,
}

impl Default for Switching {
    fn default() -> Self {
        Switching {
            enabled: true,
            delay_ms: 0,
            double_tap: false,
            double_tap_window_ms: 450,
            corner_size: 0,
            corners: Corners::default(),
            block_while_dragging: true,
            required_modifier: RequiredModifier::None,
            block_fullscreen: true,
            edge_mapping: EdgeMapping::Direct,
            local_input_takes_over: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Mouse {
    /// Multiplier for pointer movement on other computers.
    pub pointer_speed: f64,
    pub scroll_speed: f64,
    pub invert_vertical: bool,
    pub invert_horizontal: bool,
    /// Forward high-resolution scroll deltas (trackpads) instead of whole notches.
    pub smooth_scrolling: bool,
    /// Send raw movement instead of positions - for games that capture the pointer.
    pub relative_movement: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Mouse {
            pointer_speed: 1.0,
            scroll_speed: 1.0,
            invert_vertical: false,
            invert_horizontal: false,
            smooth_scrolling: true,
            relative_movement: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Keyboard {
    /// Applied when a Mac keyboard types on a Windows/Linux computer.
    pub mac_to_pc: ModifierMap,
    /// Applied when a Windows/Linux keyboard types on a Mac.
    pub pc_to_mac: ModifierMap,
    pub forward_media_keys: bool,
    /// Make Caps Lock on the other computer match this one when the cursor arrives.
    pub sync_caps_lock: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Keyboard {
            mac_to_pc: ModifierMap::swap_ctrl_meta(),
            pc_to_mac: ModifierMap::swap_ctrl_meta(),
            forward_media_keys: true,
            sync_caps_lock: true,
        }
    }
}

impl Keyboard {
    pub fn map_for(&self, from: Os, to: Os) -> ModifierMap {
        match (from.is_mac(), to.is_mac()) {
            (true, false) => self.mac_to_pc,
            (false, true) => self.pc_to_mac,
            _ => ModifierMap::identity(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotkeys {
    pub lock_to_screen: Hotkey,
    pub bring_home: Hotkey,
    pub next_device: Hotkey,
    pub pause_sharing: Hotkey,
}

impl Default for Hotkeys {
    fn default() -> Self {
        // Ctrl + Alt/⌥ + Shift + key: never used by either OS, reachable on every keyboard.
        Hotkeys {
            lock_to_screen: Hotkey::new(true, true, true, false, kb(0x0F)), // L
            bring_home: Hotkey::new(true, true, true, false, kb(0x0B)),     // H
            next_device: Hotkey::new(true, true, true, false, kb(0x11)),    // N
            pause_sharing: Hotkey::new(true, true, true, false, kb(0x13)),  // P
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardMode {
    /// Copy on one computer, paste on any other right away.
    #[default]
    Instant,
    /// Only hand the clipboard over when the cursor moves to another computer.
    OnSwitch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Clipboard {
    pub enabled: bool,
    pub mode: ClipboardMode,
    pub text: bool,
    pub rich_text: bool,
    pub images: bool,
    pub files: bool,
    pub max_image_mb: u32,
    pub max_files_mb: u32,
}

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            enabled: true,
            mode: ClipboardMode::Instant,
            text: true,
            rich_text: true,
            images: true,
            files: true,
            max_image_mb: 32,
            max_files_mb: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConflictPolicy {
    #[default]
    Rename,
    Overwrite,
    Skip,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Files {
    /// Empty = <Downloads>/Crispy.
    pub save_dir: String,
    pub auto_accept: bool,
    pub conflict: ConflictPolicy,
    pub reveal_when_done: bool,
    pub preserve_timestamps: bool,
    /// MB/s, 0 = unlimited.
    pub bandwidth_limit_mbps: u32,
}

impl Default for Files {
    fn default() -> Self {
        Files {
            save_dir: String::new(),
            auto_accept: true,
            conflict: ConflictPolicy::Rename,
            reveal_when_done: false,
            preserve_timestamps: true,
            bandwidth_limit_mbps: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Network {
    pub discovery: bool,
    pub port: u16,
    /// Addresses (host or host:port) to reach peers on networks that block broadcasts.
    pub manual_peers: Vec<String>,
    pub heartbeat_timeout_secs: u32,
}

impl Default for Network {
    fn default() -> Self {
        Network { discovery: true, port: DEFAULT_PORT, manual_peers: vec![], heartbeat_timeout_secs: 6 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Advanced {
    pub log_level: String,
    pub show_stats: bool,
}

impl Default for Advanced {
    fn default() -> Self {
        Advanced { log_level: "info".into(), show_stats: false }
    }
}

impl Settings {
    /// Keep values inside ranges the engine can work with, whatever the file or UI sent.
    pub fn sanitize(&mut self) {
        let m = &mut self.mouse;
        m.pointer_speed = if m.pointer_speed.is_finite() { m.pointer_speed.clamp(0.1, 5.0) } else { 1.0 };
        m.scroll_speed = if m.scroll_speed.is_finite() { m.scroll_speed.clamp(0.1, 10.0) } else { 1.0 };
        let s = &mut self.switching;
        s.delay_ms = s.delay_ms.min(5_000);
        s.double_tap_window_ms = s.double_tap_window_ms.clamp(100, 2_000);
        s.corner_size = s.corner_size.min(500);
        if self.network.port < 1024 {
            self.network.port = DEFAULT_PORT;
        }
        self.network.heartbeat_timeout_secs = self.network.heartbeat_timeout_secs.clamp(2, 60);
        self.general.device_name = self.general.device_name.trim().chars().take(64).collect();
        self.network.manual_peers.retain(|p| !p.trim().is_empty());
    }
}

/// Where everything lives on disk.
#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    /// `CRISPY_HOME` overrides the location, which lets two instances run side by side for testing.
    pub fn default_location() -> Paths {
        let root = std::env::var_os("CRISPY_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("Crispy")))
            .unwrap_or_else(|| PathBuf::from(".crispy"));
        Paths { root }
    }
    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    pub fn identity(&self) -> PathBuf {
        self.root.join("identity.json")
    }
    pub fn peers(&self) -> PathBuf {
        self.root.join("peers.json")
    }
    pub fn layout(&self) -> PathBuf {
        self.root.join("layout.json")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn clipboard_cache(&self) -> PathBuf {
        std::env::temp_dir().join("Crispy Clipboard")
    }
}

pub fn default_save_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("Crispy")
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("{} is unreadable ({e}); keeping a backup and starting fresh", path.display());
                let _ = std::fs::rename(path, path.with_extension("json.bak"));
                T::default()
            }
        },
        Err(_) => T::default(),
    }
}

/// Write atomically: a crash mid-write never leaves a truncated settings file behind.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_settings_fill_in_defaults() {
        let s: Settings = serde_json::from_str(r#"{"mouse":{"pointerSpeed":2.5},"unknown":1}"#).unwrap();
        assert_eq!(s.mouse.pointer_speed, 2.5);
        assert_eq!(s.mouse.scroll_speed, 1.0);
        assert!(s.clipboard.enabled);
        assert_eq!(s.network.port, DEFAULT_PORT);
    }

    #[test]
    fn sanitize_clamps() {
        let mut s = Settings::default();
        s.mouse.pointer_speed = f64::NAN;
        s.network.port = 80;
        s.general.device_name = "   Kitchen PC  ".into();
        s.sanitize();
        assert_eq!(s.mouse.pointer_speed, 1.0);
        assert_eq!(s.network.port, DEFAULT_PORT);
        assert_eq!(s.general.device_name, "Kitchen PC");
    }

    #[test]
    fn modifier_maps_only_apply_across_platforms() {
        let k = Keyboard::default();
        assert_eq!(k.map_for(Os::Macos, Os::Macos), ModifierMap::identity());
        assert_eq!(k.map_for(Os::Windows, Os::Linux), ModifierMap::identity());
        assert_eq!(k.map_for(Os::Macos, Os::Windows), ModifierMap::swap_ctrl_meta());
    }

    #[test]
    fn json_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x/settings.json");
        let mut s = Settings::default();
        s.files.auto_accept = false;
        save_json(&p, &s).unwrap();
        let back: Settings = load_json(&p);
        assert_eq!(back, s);
    }
}
