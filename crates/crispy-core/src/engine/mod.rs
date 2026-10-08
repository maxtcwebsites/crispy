//! The engine: a single actor task that owns all state.
//!
//! Every computer runs the same engine; there is no server and no client. Whichever computer's
//! physical keyboard and mouse are in use is "in charge" at that moment and drives the others.
//! Everything (OS input, network messages, UI commands, timers) arrives as a message on one task,
//! so there is no shared mutable state and no locking in the decision logic.

mod clip;
mod input;
mod peers;
mod transfers;

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch, Notify};

use crate::capture::{CaptureCore, HotkeyAction, InputEvent};
use crate::clipboard::{ClipContent, ClipboardBackend};
use crate::config::{self, load_json, save_json, Paths, Settings};
use crate::files::TransferEvent;
use crate::identity::{Identity, TrustStore};
use crate::keymap::ModState;
use crate::layout::{Layout, Pos, World};
use crate::net::discovery::{self, Beacon, DiscoveryConfig};
use crate::net::session::{Handshaked, SessionEvent, Traffic};
use crate::platform::Platform;
use crate::state::*;
use crate::types::{now_ms, DeviceId, Os, Screen};

pub use input::Focus;
use peers::{Conn, Intent, Nearby, Pairing};
use transfers::Transfer;

/// Requests from the app shell / UI.
pub enum Command {
    GetState(oneshot::Sender<AppState>),
    GetSettings(oneshot::Sender<Settings>),
    SetSettings(Box<Settings>, oneshot::Sender<Settings>),
    GetInfo(oneshot::Sender<EngineInfo>),
    PairStart(DeviceId),
    PairSubmit(String),
    PairCancel,
    Unpair(DeviceId),
    SetPeerEnabled(DeviceId, bool),
    SetLayout(BTreeMap<DeviceId, Pos>),
    SendFiles(DeviceId, Vec<PathBuf>),
    CancelTransfer(String),
    AnswerTransfer(String, bool),
    ClearTransfers,
    ToggleLock,
    TogglePause,
    RequestPermissions,
    OpenPermissionSettings,
    ConnectAddress(String),
    RegenerateIdentity,
    Shutdown(oneshot::Sender<()>),
}

#[derive(Clone, Debug)]
pub struct EngineInfo {
    pub save_dir: PathBuf,
    pub log_dir: PathBuf,
    pub port: u16,
}

pub struct EngineOptions {
    pub paths: Paths,
    pub platform: Arc<dyn Platform>,
    pub clipboard: Arc<dyn ClipboardBackend>,
    pub version: String,
    /// Bind to this TCP port instead of the configured one (0 = any free port; for tests).
    pub port_override: Option<u16>,
    /// Run LAN discovery (tests turn it off).
    pub discovery: bool,
}

/// A cheap, cloneable handle used by the app shell to talk to the engine.
#[derive(Clone)]
pub struct EngineHandle {
    tx: mpsc::UnboundedSender<Command>,
    pub port: u16,
}

impl EngineHandle {
    pub fn send(&self, c: Command) {
        let _ = self.tx.send(c);
    }

    async fn ask<T>(&self, make: impl FnOnce(oneshot::Sender<T>) -> Command) -> Option<T> {
        let (t, r) = oneshot::channel();
        self.send(make(t));
        r.await.ok()
    }

    pub async fn state(&self) -> Option<AppState> {
        self.ask(Command::GetState).await
    }
    pub async fn settings(&self) -> Option<Settings> {
        self.ask(Command::GetSettings).await
    }
    pub async fn set_settings(&self, s: Settings) -> Option<Settings> {
        self.ask(|t| Command::SetSettings(Box::new(s), t)).await
    }
    pub async fn info(&self) -> Option<EngineInfo> {
        self.ask(Command::GetInfo).await
    }
    pub async fn shutdown(&self) {
        let _ = self.ask(Command::Shutdown).await;
    }
}

/// Messages the engine sends itself from helper tasks.
pub(crate) enum Internal {
    Handshaked { h: Box<Handshaked>, intent: Intent },
    ConnectFailed { intent: Intent, error: String },
    Resolved { address: String, addr: Option<SocketAddr> },
    Beacon(Box<Beacon>, SocketAddr),
    ClipboardChanged,
    ClipboardRead(Option<ClipContent>),
    ClipboardWritten { key: Option<u64> },
    ManifestReady { tid: u64, result: Result<(Vec<crate::protocol::FileEntry>, Vec<PathBuf>), String> },
}

pub(crate) struct Engine {
    paths: Paths,
    platform: Arc<dyn Platform>,
    clip: Arc<dyn ClipboardBackend>,
    version: String,
    default_name: String,
    my_os: Os,

    settings: Settings,
    identity: Arc<RwLock<Arc<Identity>>>,
    my_id: DeviceId,
    trust: TrustStore,
    layout: Layout,
    world: World,
    screens: Vec<Screen>,
    listen_port: u16,
    port_warning: Option<String>,
    /// This device's LAN addresses (refreshed every few seconds; shown in the UI).
    addresses: Vec<String>,

    capture: Arc<CaptureCore>,
    capture_status: CaptureStatus,
    capture_detail: Option<String>,

    next_conn: u64,
    conns: HashMap<u64, Conn>,
    /// The active, trusted session for each connected peer.
    peers: HashMap<DeviceId, u64>,
    nearby: HashMap<DeviceId, Nearby>,
    /// Outstanding or recent connection attempts: when they started.
    connecting: HashMap<DeviceId, Instant>,
    pairing: Option<Pairing>,
    /// Where we reached devices we tried to pair with (for retrying devices added by address).
    pair_addresses: HashMap<DeviceId, (SocketAddr, String, Os)>,
    last_pair_request: Option<Instant>,

    focus: Focus,
    controlled_by: Option<DeviceId>,
    input: input::InputState,
    mods: ModState,
    locked: bool,
    paused: bool,

    clip_state: clip::ClipState,
    transfers: BTreeMap<u64, Transfer>,
    next_tid: u64,

    ui: mpsc::UnboundedSender<UiEvent>,
    internal_tx: mpsc::UnboundedSender<Internal>,
    session_tx: mpsc::UnboundedSender<SessionEvent>,
    transfer_tx: mpsc::UnboundedSender<TransferEvent>,
    traffic: Arc<Traffic>,
    discovery_cfg: watch::Sender<DiscoveryConfig>,
    discovery_poke: Arc<Notify>,

    dirty: bool,
    last_emit: Instant,
    started: Instant,
    events_this_sec: u32,
    stats: Stats,
    ticks: u64,
}

fn default_device_name() -> String {
    let n = whoami::devicename();
    if n.trim().is_empty() {
        whoami::fallible::hostname().unwrap_or_else(|_| "My Computer".into())
    } else {
        n
    }
}

async fn bind_listener(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port)).await
}

/// Start the engine on the current Tokio runtime.
pub async fn start(opts: EngineOptions, ui: mpsc::UnboundedSender<UiEvent>) -> anyhow::Result<EngineHandle> {
    let EngineOptions { paths, platform, clipboard, version, port_override, discovery } = opts;
    std::fs::create_dir_all(&paths.root)?;
    let mut settings: Settings = load_json(&paths.settings());
    settings.sanitize();
    let identity = Identity::load_or_create(&paths.identity())?;
    let trust = TrustStore::load(&paths.peers());
    let layout: Layout = load_json(&paths.layout());
    crate::files::clean_cache(&paths.clipboard_cache());

    // Prefer the configured port; fall back to the next few if something else holds it.
    let wanted = port_override.unwrap_or(settings.network.port);
    let mut listener = None;
    let mut port_warning = None;
    let candidates: Vec<u16> = if wanted == 0 { vec![0] } else { (0..10).map(|i| wanted.saturating_add(i)).collect() };
    for p in candidates {
        match bind_listener(p).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => tracing::warn!("port {p} unavailable: {e}"),
        }
    }
    let listener = listener.ok_or_else(|| anyhow::anyhow!("no free network port near {wanted}"))?;
    let listen_port = listener.local_addr()?.port();
    if wanted != 0 && listen_port != wanted {
        port_warning = Some(format!("Port {wanted} is in use, so Crispy is using {listen_port}."));
    }

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (input_tx, input_rx) = mpsc::unbounded_channel();
    let (internal_tx, internal_rx) = mpsc::unbounded_channel();
    let (session_tx, session_rx) = mpsc::unbounded_channel();
    let (transfer_tx, transfer_rx) = mpsc::unbounded_channel();

    let capture = Arc::new(CaptureCore::new(input_tx));
    let my_id = identity.id();
    let identity = Arc::new(RwLock::new(Arc::new(identity)));
    let default_name = default_device_name();
    let screens = platform.screens();

    let beacon = Beacon {
        magic: discovery::MAGIC.into(),
        v: crate::protocol::PROTOCOL_VERSION,
        id: my_id.clone(),
        name: String::new(),
        os: Os::current(),
        port: listen_port,
        key: String::new(),
        reply: false,
    };
    let (discovery_cfg, discovery_rx) = watch::channel(DiscoveryConfig {
        beacon,
        broadcast: settings.network.discovery && discovery,
        manual: vec![],
        port: settings.network.port,
    });
    let discovery_poke = Arc::new(Notify::new());

    let mut engine = Engine {
        paths,
        platform,
        clip: clipboard,
        version,
        default_name,
        my_os: Os::current(),
        settings,
        identity,
        my_id,
        trust,
        layout,
        world: World::default(),
        screens,
        listen_port,
        port_warning,
        addresses: vec![],
        capture,
        capture_status: CaptureStatus::Starting,
        capture_detail: None,
        next_conn: 1,
        conns: HashMap::new(),
        peers: HashMap::new(),
        nearby: HashMap::new(),
        connecting: HashMap::new(),
        pairing: None,
        pair_addresses: HashMap::new(),
        last_pair_request: None,
        focus: Focus::Local,
        controlled_by: None,
        input: Default::default(),
        mods: ModState::default(),
        locked: false,
        paused: false,
        clip_state: Default::default(),
        transfers: BTreeMap::new(),
        next_tid: (rand::random::<u32>() as u64) << 16,
        ui,
        internal_tx,
        session_tx,
        transfer_tx,
        traffic: Arc::new(Traffic::default()),
        discovery_cfg,
        discovery_poke,
        dirty: true,
        last_emit: Instant::now(),
        started: Instant::now(),
        events_this_sec: 0,
        stats: Stats::default(),
        ticks: 0,
    };
    engine.refresh_addresses();
    engine.update_discovery();
    engine.ensure_placed();
    engine.rebuild_world();
    engine.apply_hotkeys();

    // Network listener: every accepted socket runs its handshake on its own task.
    {
        let identity = engine.identity.clone();
        let tx = engine.internal_tx.clone();
        tokio::spawn(async move {
            loop {
                let (stream, addr) = match listener.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("accept failed: {e}");
                        tokio::time::sleep(Duration::from_millis(200)).await;
                        continue;
                    }
                };
                crate::net::session::configure_keepalive(&stream);
                let id = identity.read().clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    match crate::net::session::handshake(stream, &id, false).await {
                        Ok(h) => {
                            let _ = tx.send(Internal::Handshaked { h: Box::new(h), intent: Intent::Incoming });
                        }
                        Err(e) => tracing::debug!("handshake with {addr} failed: {e}"),
                    }
                });
            }
        });
    }

    // LAN discovery.
    if discovery {
        match discovery::bind_udp(engine.settings.network.port) {
            Ok(sock) => {
                let (heard_tx, mut heard_rx) = mpsc::unbounded_channel();
                tokio::spawn(discovery::run(Arc::new(sock), discovery_rx, engine.discovery_poke.clone(), heard_tx));
                let tx = engine.internal_tx.clone();
                tokio::spawn(async move {
                    while let Some((b, src)) = heard_rx.recv().await {
                        if tx.send(Internal::Beacon(Box::new(b), src)).is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => {
                tracing::warn!("discovery unavailable: {e}");
                engine.port_warning.get_or_insert_with(|| format!("Automatic discovery is unavailable ({e}). Add devices by address instead."));
            }
        }
    }

    // Clipboard change notifications.
    {
        let tx = engine.internal_tx.clone();
        engine.clip.watch(Box::new(move || {
            let _ = tx.send(Internal::ClipboardChanged);
        }));
    }

    if let Err(e) = engine.platform.start_capture(engine.capture.clone()) {
        engine.capture_status = CaptureStatus::Failed;
        engine.capture_detail = Some(e.to_string());
    }

    let handle = EngineHandle { tx: cmd_tx, port: listen_port };
    tokio::spawn(engine.run(cmd_rx, input_rx, internal_rx, session_rx, transfer_rx));
    Ok(handle)
}

impl Engine {
    async fn run(
        mut self,
        mut cmd_rx: mpsc::UnboundedReceiver<Command>,
        mut input_rx: mpsc::UnboundedReceiver<InputEvent>,
        mut internal_rx: mpsc::UnboundedReceiver<Internal>,
        mut session_rx: mpsc::UnboundedReceiver<SessionEvent>,
        mut transfer_rx: mpsc::UnboundedReceiver<TransferEvent>,
    ) {
        let mut tick = tokio::time::interval(Duration::from_millis(50));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                Some(ev) = input_rx.recv() => {
                    self.on_input(ev);
                    // Drain bursts of mouse events in one go.
                    while let Ok(ev) = input_rx.try_recv() {
                        self.on_input(ev);
                    }
                }
                Some(ev) = session_rx.recv() => self.on_session_event(ev),
                Some(cmd) = cmd_rx.recv() => {
                    if let Command::Shutdown(reply) = cmd {
                        self.shutdown();
                        let _ = reply.send(());
                        return;
                    }
                    self.on_command(cmd);
                }
                Some(ev) = internal_rx.recv() => self.on_internal(ev),
                Some(ev) = transfer_rx.recv() => self.on_transfer_event(ev),
                _ = tick.tick() => self.on_tick(),
                else => return,
            }
            if self.dirty && self.last_emit.elapsed() >= Duration::from_millis(80) {
                self.emit_state();
            }
        }
    }

    fn on_command(&mut self, cmd: Command) {
        match cmd {
            Command::GetState(reply) => {
                let _ = reply.send(self.snapshot());
            }
            Command::GetSettings(reply) => {
                let _ = reply.send(self.settings.clone());
            }
            Command::SetSettings(s, reply) => {
                self.apply_settings(*s);
                let _ = reply.send(self.settings.clone());
            }
            Command::GetInfo(reply) => {
                let _ = reply.send(EngineInfo { save_dir: self.save_dir(), log_dir: self.paths.logs(), port: self.listen_port });
            }
            Command::PairStart(peer) => self.pair_start(&peer),
            Command::PairSubmit(code) => self.pair_submit(&code),
            Command::PairCancel => self.pair_cancel(),
            Command::Unpair(peer) => self.unpair(&peer, true),
            Command::SetPeerEnabled(peer, enabled) => self.set_peer_enabled(&peer, enabled),
            Command::SetLayout(positions) => self.set_layout(positions),
            Command::SendFiles(peer, paths) => self.send_files(&peer, paths, crate::protocol::Purpose::Files),
            Command::CancelTransfer(id) => self.cancel_transfer(&id),
            Command::AnswerTransfer(id, accept) => self.answer_transfer(&id, accept),
            Command::ClearTransfers => {
                self.transfers.retain(|_, t| !t.view.state.is_finished());
                self.dirty = true;
            }
            Command::ToggleLock => self.on_hotkey(HotkeyAction::LockToScreen),
            Command::TogglePause => self.on_hotkey(HotkeyAction::PauseSharing),
            Command::RequestPermissions => {
                self.platform.request_accessibility();
                let _ = self.platform.start_capture(self.capture.clone());
                self.dirty = true;
            }
            Command::OpenPermissionSettings => self.platform.open_accessibility_settings(),
            Command::ConnectAddress(address) => self.connect_address(address),
            Command::RegenerateIdentity => self.regenerate_identity(),
            Command::Shutdown(_) => unreachable!(),
        }
    }

    fn on_internal(&mut self, ev: Internal) {
        match ev {
            Internal::Handshaked { h, intent } => self.on_handshaked(*h, intent),
            Internal::ConnectFailed { intent, error } => self.on_connect_failed(intent, error),
            Internal::Resolved { address, addr } => self.on_resolved(address, addr),
            Internal::Beacon(b, src) => self.on_beacon(*b, src),
            Internal::ClipboardChanged => self.on_clipboard_changed(),
            Internal::ClipboardRead(c) => self.on_clipboard_read(c),
            Internal::ClipboardWritten { key } => self.on_clipboard_written(key),
            Internal::ManifestReady { tid, result } => self.on_manifest_ready(tid, result),
        }
    }

    fn on_tick(&mut self) {
        self.ticks += 1;
        self.input_tick();
        self.clipboard_tick();
        if self.ticks.is_multiple_of(20) {
            self.each_second();
        }
        if self.ticks.is_multiple_of(40) {
            let screens = self.platform.screens();
            if !screens.is_empty() && screens != self.screens {
                self.screens = screens;
                self.broadcast(crate::protocol::Msg::Screens(self.screens.clone()));
                self.ensure_placed();
                self.rebuild_world();
                self.dirty = true;
            }
        }
    }

    fn each_second(&mut self) {
        self.stats.events_per_sec = std::mem::take(&mut self.events_this_sec);
        self.stats.bytes_sent = self.traffic.sent.load(Ordering::Relaxed);
        self.stats.bytes_received = self.traffic.received.load(Ordering::Relaxed);
        self.stats.uptime_secs = self.started.elapsed().as_secs();
        if self.settings.advanced.show_stats {
            self.dirty = true;
        }
        self.peers_tick();
        self.transfers_tick();
        if self.ticks.is_multiple_of(100) {
            self.refresh_addresses();
        }
    }

    fn refresh_addresses(&mut self) {
        let addresses: Vec<String> = discovery::local_addresses().into_iter().map(|ip| format!("{ip}:{}", self.listen_port)).collect();
        if addresses != self.addresses {
            self.addresses = addresses;
            self.dirty = true;
        }
    }

    pub(crate) fn emit_state(&mut self) {
        self.dirty = false;
        self.last_emit = Instant::now();
        let _ = self.ui.send(UiEvent::State(Box::new(self.snapshot())));
    }

    pub(crate) fn toast(&self, kind: &'static str, title: impl Into<String>, body: impl Into<String>) {
        let _ = self.ui.send(UiEvent::Toast(Toast { kind, title: title.into(), body: body.into() }));
    }

    pub(crate) fn notify(&self, title: impl Into<String>, body: impl Into<String>) {
        if self.settings.general.notifications {
            let _ = self.ui.send(UiEvent::Notify { title: title.into(), body: body.into() });
        }
    }

    pub(crate) fn my_name(&self) -> String {
        if self.settings.general.device_name.is_empty() {
            self.default_name.clone()
        } else {
            self.settings.general.device_name.clone()
        }
    }

    pub(crate) fn peer_name(&self, id: &str) -> String {
        self.trust
            .peers
            .get(id)
            .map(|p| p.name.clone())
            .or_else(|| self.nearby.get(id).map(|n| n.name.clone()))
            .unwrap_or_else(|| "Unknown device".into())
    }

    pub(crate) fn save_dir(&self) -> PathBuf {
        if self.settings.files.save_dir.trim().is_empty() {
            config::default_save_dir()
        } else {
            PathBuf::from(self.settings.files.save_dir.trim())
        }
    }

    fn apply_settings(&mut self, mut s: Settings) {
        s.sanitize();
        let old = std::mem::replace(&mut self.settings, s);
        if let Err(e) = save_json(&self.paths.settings(), &self.settings) {
            self.toast("error", "Couldn't save settings", e.to_string());
        }
        if old.hotkeys != self.settings.hotkeys {
            self.apply_hotkeys();
        }
        if old.general.device_name != self.settings.general.device_name {
            self.broadcast(crate::protocol::Msg::Rename(self.my_name()));
        }
        if old.network.port != self.settings.network.port {
            self.port_warning = Some("Restart Crispy to start using the new network port.".into());
        }
        if !self.settings.switching.enabled || self.paused {
            if let Focus::Remote { .. } = self.focus {
                self.return_home();
            }
        }
        self.update_discovery();
        self.dirty = true;
    }

    fn apply_hotkeys(&self) {
        let h = &self.settings.hotkeys;
        self.capture.set_hotkeys(vec![
            (HotkeyAction::LockToScreen, h.lock_to_screen.clone()),
            (HotkeyAction::BringHome, h.bring_home.clone()),
            (HotkeyAction::NextDevice, h.next_device.clone()),
            (HotkeyAction::PauseSharing, h.pause_sharing.clone()),
        ]);
    }

    pub(crate) fn update_discovery(&mut self) {
        let id = self.identity.read().clone();
        let name = self.my_name();
        let manual = self.settings.network.manual_peers.clone();
        let discovery_on = self.settings.network.discovery;
        let port = self.listen_port;
        let my_id = self.my_id.clone();
        self.discovery_cfg.send_modify(|c| {
            c.beacon.id = my_id;
            c.beacon.name = name;
            c.beacon.port = port;
            c.beacon.key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &id.public);
            c.manual = manual;
            c.broadcast = discovery_on;
        });
        self.discovery_poke.notify_one();
    }

    fn regenerate_identity(&mut self) {
        let fresh = Identity::generate();
        if let Err(e) = fresh.save(&self.paths.identity()) {
            self.toast("error", "Couldn't create a new identity", e.to_string());
            return;
        }
        self.return_home();
        self.release_remote_input();
        self.controlled_by = None;
        self.conns.clear();
        self.peers.clear();
        self.pairing = None;
        self.trust.peers.clear();
        let _ = self.trust.save(&self.paths.peers());
        let old_id = std::mem::replace(&mut self.my_id, fresh.id());
        *self.identity.write() = Arc::new(fresh);
        if let Some(pos) = self.layout.positions.remove(&old_id) {
            self.layout.positions.insert(self.my_id.clone(), pos);
        }
        self.layout.rev += 1;
        self.layout.author = self.my_id.clone();
        let _ = save_json(&self.paths.layout(), &self.layout);
        self.rebuild_world();
        self.update_discovery();
        self.toast("success", "New identity created", "Pair your other computers again to keep using them.");
        self.dirty = true;
    }

    fn shutdown(&mut self) {
        self.return_home();
        self.release_remote_input();
        self.capture.set_grab(false);
        let (x, y) = self.platform.cursor_pos();
        self.platform.grab_end(x, y);
        self.conns.clear();
    }

    pub(crate) fn snapshot(&self) -> AppState {
        let identity = self.identity.read().clone();
        let me = DeviceInfo {
            id: self.my_id.clone(),
            name: self.my_name(),
            os: self.my_os,
            version: self.version.clone(),
            fingerprint: identity.fingerprint(),
            screens: self.screens.clone(),
            addresses: self.addresses.clone(),
        };

        let mut peers: Vec<PeerView> = Vec::new();
        for p in self.trust.peers.values() {
            let conn = self.peers.get(&p.id).and_then(|c| self.conns.get(c));
            let status = if conn.is_some() {
                PeerStatus::Connected
            } else if self.connecting.get(&p.id).is_some_and(|t| t.elapsed() < Duration::from_secs(5)) && p.enabled {
                PeerStatus::Connecting
            } else {
                PeerStatus::Offline
            };
            peers.push(PeerView {
                id: p.id.clone(),
                name: p.name.clone(),
                os: p.os,
                paired: true,
                status,
                address: conn.map(|c| c.session.addr.to_string()).or_else(|| p.last_address.clone()),
                rtt_ms: conn.and_then(|c| c.rtt_ms),
                screens: p.screens.clone(),
                fingerprint: crate::identity::fingerprint(&p.public_bytes()),
                last_seen: if conn.is_some() { Some(now_ms()) } else { self.nearby.get(&p.id).map(|n| n.last_seen_ms) },
                enabled: p.enabled,
                version: p.version.clone(),
                controlling: matches!(&self.focus, Focus::Remote { peer, .. } if *peer == p.id),
                controlling_me: self.controlled_by.as_deref() == Some(p.id.as_str()),
            });
        }
        let mut nearby: Vec<&Nearby> = self.nearby.values().filter(|n| !self.trust.peers.contains_key(&n.id)).collect();
        nearby.sort_by(|a, b| a.name.cmp(&b.name));
        for n in nearby {
            peers.push(PeerView {
                id: n.id.clone(),
                name: n.name.clone(),
                os: n.os,
                paired: false,
                status: PeerStatus::Nearby,
                address: Some(n.addr.to_string()),
                rtt_ms: None,
                screens: vec![],
                fingerprint: n.fingerprint.clone(),
                last_seen: Some(n.last_seen_ms),
                enabled: true,
                version: None,
                controlling: false,
                controlling_me: false,
            });
        }

        let focus = match &self.focus {
            Focus::Local => FocusView::Local,
            Focus::Remote { peer, .. } => FocusView::Remote { peer_id: peer.clone() },
        };

        let mut warnings = Vec::new();
        if let Some(w) = &self.port_warning {
            warnings.push(Warning { id: "port".into(), level: "warn", title: "Network".into(), body: w.clone() });
        }
        if self.capture_status == CaptureStatus::Failed && self.platform.accessibility() != crate::platform::PermissionState::Denied {
            warnings.push(Warning {
                id: "capture".into(),
                level: "error",
                title: "Keyboard and mouse sharing is off".into(),
                body: self.capture_detail.clone().unwrap_or_else(|| "Crispy couldn't watch the keyboard and mouse.".into()),
            });
        }
        if matches!(self.focus, Focus::Remote { .. }) && self.platform.secure_input_active() {
            warnings.push(Warning {
                id: "secure-input".into(),
                level: "warn",
                title: "Secure keyboard entry is on".into(),
                body: "An app on this Mac (often a password field or a terminal) is protecting the keyboard, so typing stays on this Mac until it's turned off.".into(),
            });
        }

        let mut transfers: Vec<TransferView> = self.transfers.values().rev().map(|t| t.view.clone()).collect();
        transfers.truncate(100);

        AppState {
            me,
            peers,
            layout: self.layout.clone(),
            focus,
            locked: self.locked,
            paused: self.paused,
            transfers,
            permissions: Permissions {
                accessibility: self.platform.accessibility(),
                capture: self.capture_status,
                detail: self.capture_detail.clone(),
            },
            pairing: self.pairing.as_ref().map(|p| p.view()),
            warnings,
            stats: self.stats.clone(),
        }
    }

    /// Every device whose displays should be on the shared canvas, with the size of its desktop.
    fn known_devices(&self) -> Vec<(DeviceId, crate::types::Rect)> {
        let mut out = vec![(self.my_id.clone(), crate::types::bounds_of(&self.screens))];
        for p in self.trust.peers.values() {
            if !p.screens.is_empty() {
                out.push((p.id.clone(), crate::types::bounds_of(&p.screens)));
            }
        }
        out
    }

    /// Make sure every known device has a spot in the arrangement; share the result if it changed.
    pub(crate) fn ensure_placed(&mut self) {
        let devices = self.known_devices();
        let mut layout = self.layout.clone();
        if layout.place_missing(&devices) {
            layout.rev += 1;
            layout.author = self.my_id.clone();
            self.layout = layout;
            let _ = save_json(&self.paths.layout(), &self.layout);
            self.broadcast(crate::protocol::Msg::Layout(self.layout.clone()));
            self.dirty = true;
        }
    }

    pub(crate) fn rebuild_world(&mut self) {
        let mut devices = vec![(self.my_id.clone(), self.screens.clone())];
        for (id, conn) in &self.peers {
            if self.trust.peers.get(id).is_some_and(|p| p.enabled) {
                if let Some(c) = self.conns.get(conn) {
                    devices.push((id.clone(), c.screens.clone()));
                }
            }
        }
        self.world = World::build(&self.layout, &devices);
    }

    fn set_layout(&mut self, positions: BTreeMap<DeviceId, Pos>) {
        let mut layout = self.layout.clone();
        for (id, pos) in positions {
            if pos.x.is_finite() && pos.y.is_finite() && (id == self.my_id || self.trust.peers.contains_key(&id)) {
                layout.positions.insert(id, Pos { x: pos.x.round(), y: pos.y.round() });
            }
        }
        layout.rev = self.layout.rev + 1;
        layout.author = self.my_id.clone();
        self.layout = layout;
        let _ = save_json(&self.paths.layout(), &self.layout);
        self.broadcast(crate::protocol::Msg::Layout(self.layout.clone()));
        self.rebuild_world();
        self.dirty = true;
    }

    pub(crate) fn on_layout(&mut self, from: &str, layout: Layout) {
        if !layout.supersedes(&self.layout) {
            return;
        }
        self.layout = layout;
        let _ = save_json(&self.paths.layout(), &self.layout);
        // Pass it on to peers the sender may not be paired with.
        let msg = crate::protocol::Msg::Layout(self.layout.clone());
        for (id, c) in &self.peers {
            if id != from {
                if let Some(conn) = self.conns.get(c) {
                    conn.session.send(msg.clone());
                }
            }
        }
        self.ensure_placed();
        self.rebuild_world();
        self.dirty = true;
    }
}

/// The set of devices currently reachable (connected, trusted and enabled), sorted by name.
pub(crate) fn sorted_ids<'a>(ids: impl Iterator<Item = (&'a DeviceId, &'a String)>) -> Vec<DeviceId> {
    let mut v: Vec<(&DeviceId, &String)> = ids.collect();
    v.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()).then(a.0.cmp(b.0)));
    v.into_iter().map(|(id, _)| id.clone()).collect()
}

