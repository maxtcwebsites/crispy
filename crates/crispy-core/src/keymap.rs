//! Platform-neutral key codes.
//!
//! Every key travels over the wire as a USB HID usage (`page << 16 | usage`). HID usages
//! describe the *physical* key position, so the receiving computer applies its own
//! keyboard layout - exactly like a USB keyboard plugged straight into it.

use serde::{Deserialize, Serialize};

pub const PAGE_KEYBOARD: u32 = 0x07;
pub const PAGE_CONSUMER: u32 = 0x0C;
const NONE: u16 = 0xFFFF;

pub const fn kb(usage: u32) -> u32 {
    (PAGE_KEYBOARD << 16) | usage
}
pub const fn consumer(usage: u32) -> u32 {
    (PAGE_CONSUMER << 16) | usage
}

pub const CTRL_L: u32 = kb(0xE0);
pub const SHIFT_L: u32 = kb(0xE1);
pub const ALT_L: u32 = kb(0xE2);
pub const META_L: u32 = kb(0xE3);
pub const CTRL_R: u32 = kb(0xE4);
pub const SHIFT_R: u32 = kb(0xE5);
pub const ALT_R: u32 = kb(0xE6);
pub const META_R: u32 = kb(0xE7);
pub const CAPS_LOCK: u32 = kb(0x39);
pub const NUM_LOCK: u32 = kb(0x53);
pub const PAUSE: u32 = kb(0x48);
pub const PRINT_SCREEN: u32 = kb(0x46);
pub const ESCAPE: u32 = kb(0x29);

/// (HID usage on the keyboard page, Windows scan code (0xE0xx = extended), macOS virtual key code, DOM `code`, label)
#[rustfmt::skip]
const KEYBOARD: &[(u32, u16, u16, &str, &str)] = &[
    (0x04, 0x001E, 0x00, "KeyA", "A"), (0x05, 0x0030, 0x0B, "KeyB", "B"), (0x06, 0x002E, 0x08, "KeyC", "C"),
    (0x07, 0x0020, 0x02, "KeyD", "D"), (0x08, 0x0012, 0x0E, "KeyE", "E"), (0x09, 0x0021, 0x03, "KeyF", "F"),
    (0x0A, 0x0022, 0x05, "KeyG", "G"), (0x0B, 0x0023, 0x04, "KeyH", "H"), (0x0C, 0x0017, 0x22, "KeyI", "I"),
    (0x0D, 0x0024, 0x26, "KeyJ", "J"), (0x0E, 0x0025, 0x28, "KeyK", "K"), (0x0F, 0x0026, 0x25, "KeyL", "L"),
    (0x10, 0x0032, 0x2E, "KeyM", "M"), (0x11, 0x0031, 0x2D, "KeyN", "N"), (0x12, 0x0018, 0x1F, "KeyO", "O"),
    (0x13, 0x0019, 0x23, "KeyP", "P"), (0x14, 0x0010, 0x0C, "KeyQ", "Q"), (0x15, 0x0013, 0x0F, "KeyR", "R"),
    (0x16, 0x001F, 0x01, "KeyS", "S"), (0x17, 0x0014, 0x11, "KeyT", "T"), (0x18, 0x0016, 0x20, "KeyU", "U"),
    (0x19, 0x002F, 0x09, "KeyV", "V"), (0x1A, 0x0011, 0x0D, "KeyW", "W"), (0x1B, 0x002D, 0x07, "KeyX", "X"),
    (0x1C, 0x0015, 0x10, "KeyY", "Y"), (0x1D, 0x002C, 0x06, "KeyZ", "Z"),
    (0x1E, 0x0002, 0x12, "Digit1", "1"), (0x1F, 0x0003, 0x13, "Digit2", "2"), (0x20, 0x0004, 0x14, "Digit3", "3"),
    (0x21, 0x0005, 0x15, "Digit4", "4"), (0x22, 0x0006, 0x17, "Digit5", "5"), (0x23, 0x0007, 0x16, "Digit6", "6"),
    (0x24, 0x0008, 0x1A, "Digit7", "7"), (0x25, 0x0009, 0x1C, "Digit8", "8"), (0x26, 0x000A, 0x19, "Digit9", "9"),
    (0x27, 0x000B, 0x1D, "Digit0", "0"),
    (0x28, 0x001C, 0x24, "Enter", "Return"), (0x29, 0x0001, 0x35, "Escape", "Esc"),
    (0x2A, 0x000E, 0x33, "Backspace", "Backspace"), (0x2B, 0x000F, 0x30, "Tab", "Tab"),
    (0x2C, 0x0039, 0x31, "Space", "Space"), (0x2D, 0x000C, 0x1B, "Minus", "-"), (0x2E, 0x000D, 0x18, "Equal", "="),
    (0x2F, 0x001A, 0x21, "BracketLeft", "["), (0x30, 0x001B, 0x1E, "BracketRight", "]"),
    (0x31, 0x002B, 0x2A, "Backslash", "\\"), (0x33, 0x0027, 0x29, "Semicolon", ";"),
    (0x34, 0x0028, 0x27, "Quote", "'"), (0x35, 0x0029, 0x32, "Backquote", "`"), (0x36, 0x0033, 0x2B, "Comma", ","),
    (0x37, 0x0034, 0x2F, "Period", "."), (0x38, 0x0035, 0x2C, "Slash", "/"),
    (0x39, 0x003A, 0x39, "CapsLock", "Caps Lock"),
    (0x3A, 0x003B, 0x7A, "F1", "F1"), (0x3B, 0x003C, 0x78, "F2", "F2"), (0x3C, 0x003D, 0x63, "F3", "F3"),
    (0x3D, 0x003E, 0x76, "F4", "F4"), (0x3E, 0x003F, 0x60, "F5", "F5"), (0x3F, 0x0040, 0x61, "F6", "F6"),
    (0x40, 0x0041, 0x62, "F7", "F7"), (0x41, 0x0042, 0x64, "F8", "F8"), (0x42, 0x0043, 0x65, "F9", "F9"),
    (0x43, 0x0044, 0x6D, "F10", "F10"), (0x44, 0x0057, 0x67, "F11", "F11"), (0x45, 0x0058, 0x6F, "F12", "F12"),
    (0x46, 0xE037, NONE, "PrintScreen", "Print Screen"), (0x47, 0x0046, NONE, "ScrollLock", "Scroll Lock"),
    (0x48, 0x0045, NONE, "Pause", "Pause"),
    (0x49, 0xE052, 0x72, "Insert", "Insert"), (0x4A, 0xE047, 0x73, "Home", "Home"),
    (0x4B, 0xE049, 0x74, "PageUp", "Page Up"), (0x4C, 0xE053, 0x75, "Delete", "Delete"),
    (0x4D, 0xE04F, 0x77, "End", "End"), (0x4E, 0xE051, 0x79, "PageDown", "Page Down"),
    (0x4F, 0xE04D, 0x7C, "ArrowRight", "→"), (0x50, 0xE04B, 0x7B, "ArrowLeft", "←"),
    (0x51, 0xE050, 0x7D, "ArrowDown", "↓"), (0x52, 0xE048, 0x7E, "ArrowUp", "↑"),
    (0x53, 0xE045, 0x47, "NumLock", "Num Lock"), (0x54, 0xE035, 0x4B, "NumpadDivide", "Num /"),
    (0x55, 0x0037, 0x43, "NumpadMultiply", "Num *"), (0x56, 0x004A, 0x4E, "NumpadSubtract", "Num -"),
    (0x57, 0x004E, 0x45, "NumpadAdd", "Num +"), (0x58, 0xE01C, 0x4C, "NumpadEnter", "Num Enter"),
    (0x59, 0x004F, 0x53, "Numpad1", "Num 1"), (0x5A, 0x0050, 0x54, "Numpad2", "Num 2"),
    (0x5B, 0x0051, 0x55, "Numpad3", "Num 3"), (0x5C, 0x004B, 0x56, "Numpad4", "Num 4"),
    (0x5D, 0x004C, 0x57, "Numpad5", "Num 5"), (0x5E, 0x004D, 0x58, "Numpad6", "Num 6"),
    (0x5F, 0x0047, 0x59, "Numpad7", "Num 7"), (0x60, 0x0048, 0x5B, "Numpad8", "Num 8"),
    (0x61, 0x0049, 0x5C, "Numpad9", "Num 9"), (0x62, 0x0052, 0x52, "Numpad0", "Num 0"),
    (0x63, 0x0053, 0x41, "NumpadDecimal", "Num ."), (0x64, 0x0056, 0x0A, "IntlBackslash", "§"),
    (0x65, 0xE05D, 0x6E, "ContextMenu", "Menu"), (0x67, 0x0059, 0x51, "NumpadEqual", "Num ="),
    (0x68, 0x0064, 0x69, "F13", "F13"), (0x69, 0x0065, 0x6B, "F14", "F14"), (0x6A, 0x0066, 0x71, "F15", "F15"),
    (0x6B, 0x0067, 0x6A, "F16", "F16"), (0x6C, 0x0068, 0x40, "F17", "F17"), (0x6D, 0x0069, 0x4F, "F18", "F18"),
    (0x6E, 0x006A, 0x50, "F19", "F19"), (0x6F, 0x006B, 0x5A, "F20", "F20"), (0x70, 0x006C, NONE, "F21", "F21"),
    (0x71, 0x006D, NONE, "F22", "F22"), (0x72, 0x006E, NONE, "F23", "F23"), (0x73, 0x0076, NONE, "F24", "F24"),
    (0x85, 0x007E, 0x5F, "NumpadComma", "Num ,"), (0x87, 0x0073, 0x5E, "IntlRo", "Ro"),
    (0x88, 0x0070, NONE, "KanaMode", "Kana"), (0x89, 0x007D, 0x5D, "IntlYen", "¥"),
    (0x8A, 0x0079, NONE, "Convert", "Convert"), (0x8B, 0x007B, NONE, "NonConvert", "NonConvert"),
    (0x90, 0x0072, 0x68, "Lang1", "Lang1"), (0x91, 0x0071, 0x66, "Lang2", "Lang2"),
    (0xE0, 0x001D, 0x3B, "ControlLeft", "Ctrl"), (0xE1, 0x002A, 0x38, "ShiftLeft", "Shift"),
    (0xE2, 0x0038, 0x3A, "AltLeft", "Alt"), (0xE3, 0xE05B, 0x37, "MetaLeft", "Meta"),
    (0xE4, 0xE01D, 0x3E, "ControlRight", "Right Ctrl"), (0xE5, 0x0036, 0x3C, "ShiftRight", "Right Shift"),
    (0xE6, 0xE038, 0x3D, "AltRight", "Right Alt"), (0xE7, 0xE05C, 0x36, "MetaRight", "Right Meta"),
];

/// Media / system keys: (consumer usage, Windows virtual key, Windows scan code, macOS NX key type, DOM code, label)
#[rustfmt::skip]
const MEDIA: &[(u32, u16, u16, i32, &str, &str)] = &[
    (0xCD, 0xB3, 0xE022, 16, "MediaPlayPause", "Play/Pause"),
    (0xB5, 0xB0, 0xE019, 17, "MediaTrackNext", "Next Track"),
    (0xB6, 0xB1, 0xE010, 18, "MediaTrackPrevious", "Previous Track"),
    (0xB7, 0xB2, 0xE024, -1, "MediaStop", "Stop"),
    (0xE2, 0xAD, 0xE020, 7, "AudioVolumeMute", "Mute"),
    (0xE9, 0xAF, 0xE030, 0, "AudioVolumeUp", "Volume Up"),
    (0xEA, 0xAE, 0xE02E, 1, "AudioVolumeDown", "Volume Down"),
    (0x6F, 0x00, 0x0000, 2, "BrightnessUp", "Brightness Up"),
    (0x70, 0x00, 0x0000, 3, "BrightnessDown", "Brightness Down"),
];

/// How a key should be synthesised on Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinKey {
    /// Inject by scan code (layout independent, what games and most apps expect).
    Scan { scan: u16, extended: bool },
    /// Inject by virtual key (keys whose scan codes are awkward to synthesise).
    Vk { vk: u16, extended: bool },
}

pub fn is_media(hid: u32) -> bool {
    hid >> 16 == PAGE_CONSUMER
}

fn keyboard_entry(hid: u32) -> Option<&'static (u32, u16, u16, &'static str, &'static str)> {
    if hid >> 16 != PAGE_KEYBOARD {
        return None;
    }
    let usage = hid & 0xFFFF;
    KEYBOARD.iter().find(|e| e.0 == usage)
}

fn media_entry(hid: u32) -> Option<&'static (u32, u16, u16, i32, &'static str, &'static str)> {
    if !is_media(hid) {
        return None;
    }
    let usage = hid & 0xFFFF;
    MEDIA.iter().find(|e| e.0 == usage)
}

const VK_CANCEL: u32 = 0x03;
const VK_PAUSE: u32 = 0x13;
const VK_SNAPSHOT: u32 = 0x2C;
const VK_NUMLOCK: u32 = 0x90;

/// Translate a Windows low-level keyboard hook event into a HID usage.
pub fn hid_from_windows(vk: u32, scan: u32, extended: bool) -> Option<u32> {
    match vk {
        VK_PAUSE | VK_CANCEL => return Some(PAUSE),
        VK_NUMLOCK => return Some(NUM_LOCK),
        VK_SNAPSHOT => return Some(PRINT_SCREEN),
        _ => {}
    }
    if let Some(m) = MEDIA.iter().find(|m| m.1 as u32 == vk && m.1 != 0) {
        return Some(consumer(m.0));
    }
    if scan != 0 {
        let code = (scan & 0xFF) as u16 | if extended { 0xE000 } else { 0 };
        if let Some(e) = KEYBOARD.iter().find(|e| e.1 == code) {
            return Some(kb(e.0));
        }
    }
    // A handful of keys arrive without a usable scan code (synthetic input from other tools).
    let fallback = match vk {
        0x5B => 0xE3, // VK_LWIN
        0x5C => 0xE7, // VK_RWIN
        0x5D => 0x65, // VK_APPS
        0xA0 => 0xE1,
        0xA1 => 0xE5,
        0xA2 => 0xE0,
        0xA3 => 0xE4,
        0xA4 => 0xE2,
        0xA5 => 0xE6,
        _ => return None,
    };
    Some(kb(fallback))
}

pub fn windows_from_hid(hid: u32) -> Option<WinKey> {
    match hid {
        PAUSE => return Some(WinKey::Vk { vk: VK_PAUSE as u16, extended: false }),
        NUM_LOCK => return Some(WinKey::Vk { vk: VK_NUMLOCK as u16, extended: true }),
        PRINT_SCREEN => return Some(WinKey::Vk { vk: VK_SNAPSHOT as u16, extended: true }),
        _ => {}
    }
    if let Some(m) = media_entry(hid) {
        return (m.1 != 0).then_some(WinKey::Vk { vk: m.1, extended: true });
    }
    keyboard_entry(hid).map(|e| WinKey::Scan { scan: e.1 & 0xFF, extended: e.1 & 0xE000 == 0xE000 })
}

pub fn hid_from_mac(keycode: u16) -> Option<u32> {
    KEYBOARD.iter().find(|e| e.2 == keycode && e.2 != NONE).map(|e| kb(e.0))
}

pub fn mac_from_hid(hid: u32) -> Option<u16> {
    keyboard_entry(hid).map(|e| e.2).filter(|&k| k != NONE)
}

/// macOS media keys are "system defined" events identified by an NX key type.
pub fn mac_media_from_hid(hid: u32) -> Option<i32> {
    media_entry(hid).map(|m| m.3).filter(|&t| t >= 0)
}

pub fn hid_from_mac_media(nx_key_type: i32) -> Option<u32> {
    MEDIA.iter().find(|m| m.3 == nx_key_type).map(|m| consumer(m.0))
}

pub fn hid_from_dom_code(code: &str) -> Option<u32> {
    KEYBOARD
        .iter()
        .find(|e| e.3 == code)
        .map(|e| kb(e.0))
        .or_else(|| MEDIA.iter().find(|m| m.4 == code).map(|m| consumer(m.0)))
}

pub fn key_label(hid: u32) -> &'static str {
    keyboard_entry(hid)
        .map(|e| e.4)
        .or_else(|| media_entry(hid).map(|m| m.5))
        .unwrap_or("?")
}

/// The key table exported to the UI, so hotkey recording and display share one source of truth.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyInfo {
    pub hid: u32,
    pub code: &'static str,
    pub label: &'static str,
}

pub fn key_table() -> Vec<KeyInfo> {
    KEYBOARD
        .iter()
        .map(|e| KeyInfo { hid: kb(e.0), code: e.3, label: e.4 })
        .chain(MEDIA.iter().map(|m| KeyInfo { hid: consumer(m.0), code: m.4, label: m.5 }))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModKey {
    Ctrl,
    Shift,
    Alt,
    Meta,
}

/// Classify a HID usage as a modifier, returning the modifier and whether it is the right-hand key.
pub fn modifier_of(hid: u32) -> Option<(ModKey, bool)> {
    Some(match hid {
        CTRL_L => (ModKey::Ctrl, false),
        CTRL_R => (ModKey::Ctrl, true),
        SHIFT_L => (ModKey::Shift, false),
        SHIFT_R => (ModKey::Shift, true),
        ALT_L => (ModKey::Alt, false),
        ALT_R => (ModKey::Alt, true),
        META_L => (ModKey::Meta, false),
        META_R => (ModKey::Meta, true),
        _ => return None,
    })
}

pub fn modifier_hid(m: ModKey, right: bool) -> u32 {
    match (m, right) {
        (ModKey::Ctrl, false) => CTRL_L,
        (ModKey::Ctrl, true) => CTRL_R,
        (ModKey::Shift, false) => SHIFT_L,
        (ModKey::Shift, true) => SHIFT_R,
        (ModKey::Alt, false) => ALT_L,
        (ModKey::Alt, true) => ALT_R,
        (ModKey::Meta, false) => META_L,
        (ModKey::Meta, true) => META_R,
    }
}

/// Where Ctrl, Alt and Meta (⌘ / ⊞) should land on the other computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModifierMap {
    pub ctrl: ModKey,
    pub alt: ModKey,
    pub meta: ModKey,
}

impl Default for ModifierMap {
    fn default() -> Self {
        Self::identity()
    }
}

impl ModifierMap {
    pub const fn identity() -> Self {
        ModifierMap { ctrl: ModKey::Ctrl, alt: ModKey::Alt, meta: ModKey::Meta }
    }
    /// Ctrl ⇄ ⌘: shortcuts like copy/paste keep working with the muscle memory of the source keyboard.
    pub const fn swap_ctrl_meta() -> Self {
        ModifierMap { ctrl: ModKey::Meta, alt: ModKey::Alt, meta: ModKey::Ctrl }
    }

    pub fn apply(&self, hid: u32) -> u32 {
        match modifier_of(hid) {
            Some((ModKey::Ctrl, right)) => modifier_hid(self.ctrl, right),
            Some((ModKey::Alt, right)) => modifier_hid(self.alt, right),
            Some((ModKey::Meta, right)) => modifier_hid(self.meta, right),
            _ => hid,
        }
    }
}

/// A global shortcut, matched against physical keys on whichever computer you are typing on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
    pub key: u32,
}


impl Hotkey {
    pub fn new(ctrl: bool, shift: bool, alt: bool, meta: bool, key: u32) -> Self {
        Hotkey { ctrl, shift, alt, meta, key }
    }

    pub fn is_set(&self) -> bool {
        self.key != 0
    }

    /// Does pressing `key` with exactly the modifiers in `held` trigger this hotkey?
    pub fn matches(&self, key: u32, held: &ModState) -> bool {
        self.is_set()
            && self.key == key
            && self.ctrl == held.ctrl()
            && self.shift == held.shift()
            && self.alt == held.alt()
            && self.meta == held.meta()
    }
}

/// Tracks which modifier keys are physically down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModState(u8);

impl ModState {
    pub fn update(&mut self, hid: u32, down: bool) {
        if let Some((m, right)) = modifier_of(hid) {
            let bit = 1u8 << ((m as u8) * 2 + right as u8);
            if down {
                self.0 |= bit;
            } else {
                self.0 &= !bit;
            }
        }
    }
    fn any(&self, m: ModKey) -> bool {
        self.0 & (0b11 << ((m as u8) * 2)) != 0
    }
    pub fn ctrl(&self) -> bool {
        self.any(ModKey::Ctrl)
    }
    pub fn shift(&self) -> bool {
        self.any(ModKey::Shift)
    }
    pub fn alt(&self) -> bool {
        self.any(ModKey::Alt)
    }
    pub fn meta(&self) -> bool {
        self.any(ModKey::Meta)
    }
    pub fn has(&self, m: ModKey) -> bool {
        self.any(m)
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    /// The HID usages of every modifier currently held.
    pub fn held_keys(&self) -> Vec<u32> {
        [CTRL_L, CTRL_R, SHIFT_L, SHIFT_R, ALT_L, ALT_R, META_L, META_R]
            .into_iter()
            .filter(|&k| {
                let (m, right) = modifier_of(k).unwrap();
                self.0 & (1u8 << ((m as u8) * 2 + right as u8)) != 0
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tables_have_no_duplicate_native_codes() {
        let mut win = HashSet::new();
        let mut mac = HashSet::new();
        let mut usages = HashSet::new();
        for e in KEYBOARD {
            assert!(usages.insert(e.0), "duplicate usage {:#x}", e.0);
            assert!(win.insert(e.1), "duplicate windows scan {:#x}", e.1);
            if e.2 != NONE {
                assert!(mac.insert(e.2), "duplicate mac keycode {:#x}", e.2);
            }
        }
    }

    #[test]
    fn round_trips_through_every_platform() {
        for e in KEYBOARD {
            let hid = kb(e.0);
            if let Some(m) = mac_from_hid(hid) {
                assert_eq!(hid_from_mac(m), Some(hid));
            }
            match windows_from_hid(hid).unwrap() {
                WinKey::Scan { scan, extended } => {
                    assert_eq!(hid_from_windows(0, scan as u32, extended), Some(hid), "{}", e.4)
                }
                WinKey::Vk { .. } => {}
            }
            assert_eq!(hid_from_dom_code(e.3), Some(hid));
        }
    }

    #[test]
    fn windows_special_keys() {
        // Pause and Num Lock share scan code 0x45 and are only told apart by the virtual key.
        assert_eq!(hid_from_windows(0x13, 0x45, false), Some(PAUSE));
        assert_eq!(hid_from_windows(0x90, 0x45, true), Some(NUM_LOCK));
        assert_eq!(hid_from_windows(0xB3, 0x22, true), Some(consumer(0xCD)));
        assert_eq!(hid_from_windows(0x41, 0x1E, false), Some(kb(0x04)));
        assert_eq!(hid_from_windows(0x5B, 0x5B, true), Some(META_L));
    }

    #[test]
    fn mac_command_maps_to_windows_key() {
        assert_eq!(hid_from_mac(0x37), Some(META_L));
        assert_eq!(windows_from_hid(META_L), Some(WinKey::Scan { scan: 0x5B, extended: true }));
    }

    #[test]
    fn modifier_swap() {
        let m = ModifierMap::swap_ctrl_meta();
        assert_eq!(m.apply(CTRL_L), META_L);
        assert_eq!(m.apply(META_R), CTRL_R);
        assert_eq!(m.apply(ALT_L), ALT_L);
        assert_eq!(m.apply(kb(0x04)), kb(0x04));
    }

    #[test]
    fn hotkey_matching_ignores_side() {
        let hk = Hotkey::new(true, false, true, false, kb(0x0F));
        let mut s = ModState::default();
        s.update(CTRL_R, true);
        s.update(ALT_L, true);
        assert!(hk.matches(kb(0x0F), &s));
        s.update(SHIFT_L, true);
        assert!(!hk.matches(kb(0x0F), &s));
        assert_eq!(s.held_keys(), vec![CTRL_R, SHIFT_L, ALT_L]);
    }
}
