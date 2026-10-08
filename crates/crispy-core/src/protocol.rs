//! Everything two Crispy devices say to each other. Messages are bincode-encoded and travel
//! inside Noise-encrypted frames (see `net::session`).

use bincode::Options;
use serde::{Deserialize, Serialize};

use crate::layout::Layout;
use crate::types::{DeviceId, MouseButton, Os, Screen};

/// Bumped on incompatible changes. Peers with a different major protocol refuse to connect.
pub const PROTOCOL_VERSION: u32 = 1;

/// Upper bound for one decoded message (clipboard images are the largest).
pub const MAX_MESSAGE: u64 = 96 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hello {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub version: String,
    pub protocol: u32,
    /// TCP port the sender accepts connections on (for reconnecting later).
    pub port: u16,
    pub screens: Vec<Screen>,
    pub layout: Layout,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PairMsg {
    /// "I would like to pair" — sent by the device where the user clicked *Pair*.
    Request { name: String, os: Os },
    /// The responder shows a code to its user and sends its SPAKE2 message.
    Challenge { name: String, os: Os, spake: Vec<u8> },
    /// The initiator's user typed the code: SPAKE2 message plus key confirmation.
    Response { spake: Vec<u8>, confirm: Vec<u8> },
    /// Responder's key confirmation: both sides now trust each other.
    Confirm { confirm: Vec<u8> },
    Reject { reason: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClipItem {
    Text(String),
    Html(String),
    Rtf(String),
    Png(#[serde(with = "serde_bytes")] Vec<u8>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipboardData {
    pub items: Vec<ClipItem>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Purpose {
    /// Files the user sent explicitly; saved to the downloads folder.
    Files,
    /// Files copied to the clipboard; cached and placed on the receiver's clipboard.
    Clipboard,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    /// Relative path with `/` separators, e.g. `Photos/2024/beach.jpg`.
    pub path: String,
    pub size: u64,
    /// Unix seconds.
    pub mtime: i64,
    pub dir: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FileOffer {
    pub id: u64,
    pub purpose: Purpose,
    pub entries: Vec<FileEntry>,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Msg {
    Hello(Hello),
    Screens(Vec<Screen>),
    Layout(Layout),
    Rename(String),
    Ping(u64),
    Pong(u64),

    /// The sender's keyboard and mouse now drive the receiver; put the cursor at (x, y).
    /// `held` lists modifier keys already down; `caps_lock` asks the receiver to match.
    Enter { x: f64, y: f64, held: Vec<u32>, caps_lock: Option<bool> },
    /// The cursor left the receiver: release anything still pressed.
    Leave,
    MouseMove { x: f64, y: f64 },
    MouseRel { dx: f64, dy: f64 },
    Button { button: MouseButton, down: bool },
    /// Scroll amount in wheel notches (fractional for smooth scrolling).
    Wheel { dx: f64, dy: f64, continuous: bool },
    Key { hid: u32, down: bool, repeat: bool },
    /// "My own keyboard/mouse is being used — stop driving me." Also sent when someone else took over.
    TakeOver,

    Clipboard(ClipboardData),

    FileOffer(FileOffer),
    FileAnswer { id: u64, accept: bool },
    FileChunk {
        id: u64,
        file: u32,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    FileDone { id: u64 },
    FileCancel { id: u64, reason: String },

    Pair(PairMsg),
    /// The sender removed us from its trusted devices.
    Unpair,
}

impl Msg {
    /// Latency-sensitive messages jump ahead of bulk data (file chunks, clipboard images).
    pub fn is_bulk(&self) -> bool {
        matches!(self, Msg::FileChunk { .. } | Msg::FileOffer(_) | Msg::FileDone { .. } | Msg::Clipboard(_))
    }
}

fn codec() -> impl Options {
    bincode::DefaultOptions::new().with_limit(MAX_MESSAGE)
}

pub fn encode(msg: &Msg) -> Vec<u8> {
    codec().serialize(msg).expect("messages always serialize")
}

pub fn decode(bytes: &[u8]) -> anyhow::Result<Msg> {
    Ok(codec().deserialize(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_compact_input_events() {
        let msgs = vec![
            Msg::MouseMove { x: 1440.5, y: 22.0 },
            Msg::Key { hid: 0x07_0004, down: true, repeat: false },
            Msg::Enter { x: 2.0, y: 3.0, held: vec![0x07_00E1], caps_lock: Some(true) },
            Msg::FileChunk { id: 7, file: 0, data: vec![1, 2, 3] },
            Msg::Pair(PairMsg::Request { name: "Mac".into(), os: Os::Macos }),
        ];
        for m in msgs {
            assert_eq!(decode(&encode(&m)).unwrap(), m);
        }
        assert!(encode(&Msg::Key { hid: 0x07_0004, down: true, repeat: false }).len() <= 8);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(decode(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).is_err());
    }
}
