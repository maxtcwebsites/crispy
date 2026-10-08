//! The snapshot the UI renders. Mirrors `src/lib/types.ts`.

use serde::Serialize;

use crate::layout::Layout;
use crate::platform::PermissionState;
use crate::types::{DeviceId, Os, Screen};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub version: String,
    pub fingerprint: String,
    pub screens: Vec<Screen>,
    pub addresses: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PeerStatus {
    Connected,
    Connecting,
    Offline,
    Nearby,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub paired: bool,
    pub status: PeerStatus,
    pub address: Option<String>,
    pub rtt_ms: Option<u32>,
    pub screens: Vec<Screen>,
    pub fingerprint: String,
    pub last_seen: Option<u64>,
    pub enabled: bool,
    pub version: Option<String>,
    pub controlling: bool,
    pub controlling_me: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FocusView {
    Local,
    #[serde(rename_all = "camelCase")]
    Remote { peer_id: DeviceId },
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    Pending,
    Awaiting,
    Active,
    Done,
    Failed,
    Cancelled,
    Declined,
}

impl TransferState {
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled | Self::Declined)
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Send,
    Receive,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferView {
    pub id: String,
    pub peer_id: DeviceId,
    pub peer_name: String,
    pub direction: Direction,
    pub purpose: &'static str,
    pub files: Vec<FileView>,
    pub file_count: usize,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub bytes_per_sec: f64,
    pub state: TransferState,
    pub error: Option<String>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub save_path: Option<String>,
    pub current_file: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureStatus {
    Running,
    Starting,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub accessibility: PermissionState,
    pub capture: CaptureStatus,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PairingStage {
    Connecting,
    EnterCode,
    ShowCode,
    Verifying,
    Success,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PairingRole {
    Initiator,
    Responder,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairingView {
    pub peer_id: DeviceId,
    pub peer_name: String,
    pub peer_os: Os,
    pub role: PairingRole,
    pub stage: PairingStage,
    pub code: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub id: String,
    pub level: &'static str,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub events_per_sec: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub uptime_secs: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub me: DeviceInfo,
    pub peers: Vec<PeerView>,
    pub layout: Layout,
    pub focus: FocusView,
    pub locked: bool,
    pub paused: bool,
    pub transfers: Vec<TransferView>,
    pub permissions: Permissions,
    pub pairing: Option<PairingView>,
    pub warnings: Vec<Warning>,
    pub stats: Stats,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointerEvent {
    pub device: DeviceId,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub kind: &'static str,
    pub title: String,
    pub body: String,
}

/// Everything the engine tells the app shell.
#[derive(Clone, Debug)]
pub enum UiEvent {
    State(Box<AppState>),
    Pointer(PointerEvent),
    Toast(Toast),
    /// Something needs the user's attention (pairing code, incoming files): show the window.
    Attention,
    /// Native notification (when the window may be hidden).
    Notify { title: String, body: String },
    /// Files finished arriving and the user wants the folder revealed.
    Reveal(std::path::PathBuf),
    /// Focus moved (tray icon/tooltip update): peer name or None for this computer.
    FocusChanged(Option<String>),
}
