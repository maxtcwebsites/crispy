//! Connections to other devices: dialling, accepting, pairing, hello/heartbeat and teardown.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use base64::Engine as _;

use super::{Engine, Internal};
use crate::identity::{device_id, fingerprint, TrustedPeer};
use crate::net::pairing::{self, SpakeState, ROLE_INITIATOR, ROLE_RESPONDER};
use crate::net::session::{Handshaked, Session, SessionEvent};
use crate::protocol::{Hello, Msg, PairMsg, PROTOCOL_VERSION};
use crate::state::{PairingRole, PairingStage, PairingView, UiEvent};
use crate::types::{now_ms, DeviceId, Os, Screen};

/// Why a connection was opened.
#[derive(Clone, Debug)]
pub(crate) enum Intent {
    /// Someone connected to us.
    Incoming,
    /// Reconnecting to a paired device.
    Trusted(DeviceId),
    /// The user asked to pair (with a known nearby device, or by address).
    Pair(Option<DeviceId>),
}

pub(crate) struct Conn {
    pub session: Session,
    pub remote_id: DeviceId,
    pub trusted: bool,
    /// Set once Hello arrived: the connection is live.
    pub hello: bool,
    pub screens: Vec<Screen>,
    pub rtt_ms: Option<u32>,
    pub opened: Instant,
}

pub(crate) struct Nearby {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub addr: SocketAddr,
    pub fingerprint: String,
    pub last_seen: Instant,
    pub last_seen_ms: u64,
}

pub(crate) struct Pairing {
    pub conn: Option<u64>,
    pub peer_id: Option<DeviceId>,
    pub peer_name: String,
    pub peer_os: Os,
    pub role: PairingRole,
    pub stage: PairingStage,
    pub code: Option<String>,
    pub error: Option<String>,
    pub spake: Option<SpakeState>,
    pub their_spake: Option<Vec<u8>>,
    pub key: Option<Vec<u8>>,
    pub hash: Vec<u8>,
    pub finished: Option<Instant>,
}

impl Pairing {
    fn new(role: PairingRole, stage: PairingStage) -> Pairing {
        Pairing {
            conn: None,
            peer_id: None,
            peer_name: String::new(),
            peer_os: Os::Windows,
            role,
            stage,
            code: None,
            error: None,
            spake: None,
            their_spake: None,
            key: None,
            hash: vec![],
            finished: None,
        }
    }

    pub fn view(&self) -> PairingView {
        PairingView {
            peer_id: self.peer_id.clone().unwrap_or_default(),
            peer_name: if self.peer_name.is_empty() { "New device".into() } else { self.peer_name.clone() },
            peer_os: self.peer_os,
            role: self.role,
            stage: self.stage,
            code: self.code.as_ref().map(|c| format!("{} {}", &c[..3], &c[3..])),
            error: self.error.clone(),
        }
    }
}

const RECONNECT_EVERY: Duration = Duration::from_secs(4);
const NEARBY_TTL: Duration = Duration::from_secs(12);

impl Engine {
    pub(crate) fn send_to(&self, peer: &str, msg: Msg) {
        if let Some(conn) = self.peers.get(peer).and_then(|c| self.conns.get(c)) {
            conn.session.send(msg);
        }
    }

    pub(crate) fn broadcast(&self, msg: Msg) {
        for c in self.peers.values() {
            if let Some(conn) = self.conns.get(c) {
                conn.session.send(msg.clone());
            }
        }
    }

    pub(crate) fn bulk_sender(&self, peer: &str) -> Option<tokio::sync::mpsc::Sender<Msg>> {
        self.peers.get(peer).and_then(|c| self.conns.get(c)).map(|c| c.session.bulk_sender())
    }

    fn spawn_connect(&mut self, addr: SocketAddr, intent: Intent) {
        if let Intent::Trusted(id) = &intent {
            self.connecting.insert(id.clone(), Instant::now());
        }
        let identity = self.identity.read().clone();
        let tx = self.internal_tx.clone();
        tokio::spawn(async move {
            let msg = match crate::net::session::connect(addr, &identity).await {
                Ok(h) => Internal::Handshaked { h: Box::new(h), intent },
                Err(e) => Internal::ConnectFailed { intent, error: e.to_string() },
            };
            let _ = tx.send(msg);
        });
    }

    pub(crate) fn on_connect_failed(&mut self, intent: Intent, error: String) {
        tracing::debug!("connect failed ({intent:?}): {error}");
        if let Intent::Pair(_) = intent {
            if let Some(p) = &mut self.pairing {
                if p.conn.is_none() {
                    p.stage = PairingStage::Failed;
                    p.error = Some("Couldn't reach that computer. Make sure Crispy is open on it and both are on the same network.".into());
                    p.finished = Some(Instant::now());
                    self.dirty = true;
                }
            }
        }
    }

    pub(crate) fn on_handshaked(&mut self, h: Handshaked, intent: Intent) {
        let remote_id = device_id(&h.remote_static);
        if remote_id == self.my_id {
            return; // dialled ourselves
        }
        let trusted = self.trust.is_trusted(&h.remote_static);
        if let Intent::Trusted(id) = &intent {
            if *id != remote_id || !trusted {
                tracing::warn!("device at {} is not the paired device we expected", h.addr);
                return;
            }
        }
        let conn_id = self.next_conn;
        self.next_conn += 1;
        let hash = h.handshake_hash.clone();
        let idle = Duration::from_secs(self.settings.network.heartbeat_timeout_secs as u64);
        let session = Session::start(h, conn_id, idle, self.session_tx.clone(), self.traffic.clone());
        let conn = Conn { session, remote_id: remote_id.clone(), trusted, hello: false, screens: vec![], rtt_ms: None, opened: Instant::now() };
        self.conns.insert(conn_id, conn);

        if trusted {
            self.send_hello(conn_id);
            if matches!(intent, Intent::Pair(_)) {
                // Already paired with this device: nothing to do but connect.
                if let Some(p) = &mut self.pairing {
                    p.stage = PairingStage::Success;
                    p.peer_id = Some(remote_id.clone());
                    p.peer_name = self.trust.peers.get(&remote_id).map(|t| t.name.clone()).unwrap_or_default();
                    p.finished = Some(Instant::now());
                }
            }
        } else if let Intent::Pair(expected) = intent {
            let Some(p) = &mut self.pairing else {
                self.conns.remove(&conn_id);
                return;
            };
            if expected.is_some_and(|e| e != remote_id) {
                p.stage = PairingStage::Failed;
                p.error = Some("A different computer answered at that address.".into());
                p.finished = Some(Instant::now());
                self.conns.remove(&conn_id);
            } else {
                p.conn = Some(conn_id);
                p.peer_id = Some(remote_id);
                p.hash = hash;
                let request = Msg::Pair(PairMsg::Request { name: self.my_name(), os: self.my_os });
                self.conns[&conn_id].session.send(request);
            }
        }
        // Untrusted incoming connections wait for a pairing request (see peers_tick timeout).
        self.dirty = true;
    }

    fn send_hello(&self, conn_id: u64) {
        if let Some(c) = self.conns.get(&conn_id) {
            c.session.send(Msg::Hello(Hello {
                id: self.my_id.clone(),
                name: self.my_name(),
                os: self.my_os,
                version: self.version.clone(),
                protocol: PROTOCOL_VERSION,
                port: self.listen_port,
                screens: self.screens.clone(),
                layout: self.layout.clone(),
            }));
        }
    }

    pub(crate) fn on_session_event(&mut self, ev: SessionEvent) {
        match ev {
            SessionEvent::Closed { conn, reason } => self.on_closed(conn, reason),
            SessionEvent::Message { conn, msg } => {
                let Some(c) = self.conns.get(&conn) else { return };
                if !c.trusted {
                    match msg {
                        Msg::Pair(p) => self.on_pair_msg(conn, p),
                        _ => {
                            self.conns.remove(&conn);
                        }
                    }
                    return;
                }
                let peer = c.remote_id.clone();
                match msg {
                    Msg::Hello(h) => self.on_hello(conn, h),
                    Msg::Pair(p) => {
                        if self.pairing.as_ref().is_some_and(|x| x.conn == Some(conn)) {
                            self.on_pair_msg(conn, p);
                        }
                    }
                    _ if !c.hello || self.peers.get(&peer) != Some(&conn) => {}
                    Msg::Ping(t) => self.conns[&conn].session.send(Msg::Pong(t)),
                    Msg::Pong(t) => {
                        if let Some(c) = self.conns.get_mut(&conn) {
                            c.rtt_ms = Some(now_ms().saturating_sub(t).min(9_999) as u32);
                        }
                    }
                    Msg::Screens(screens) => {
                        if let Some(c) = self.conns.get_mut(&conn) {
                            c.screens = screens.clone();
                        }
                        if let Some(p) = self.trust.peers.get_mut(&peer) {
                            p.screens = screens;
                            let _ = self.trust.save(&self.paths.peers());
                        }
                        self.ensure_placed();
                        self.rebuild_world();
                        self.dirty = true;
                    }
                    Msg::Layout(l) => self.on_layout(&peer, l),
                    Msg::Rename(name) => {
                        if let Some(p) = self.trust.peers.get_mut(&peer) {
                            p.name = name.chars().take(64).collect();
                            let _ = self.trust.save(&self.paths.peers());
                            self.dirty = true;
                        }
                    }
                    Msg::Unpair => {
                        let name = self.peer_name(&peer);
                        self.unpair(&peer, false);
                        self.toast("info", format!("{name} removed this computer"), "Pair again to share with it.");
                    }
                    m @ (Msg::Enter { .. }
                    | Msg::Leave
                    | Msg::TakeOver
                    | Msg::MouseMove { .. }
                    | Msg::MouseRel { .. }
                    | Msg::Button { .. }
                    | Msg::Wheel { .. }
                    | Msg::Key { .. }) => self.on_control_msg(&peer, m),
                    Msg::Clipboard(data) => self.on_remote_clipboard(&peer, data),
                    Msg::FileOffer(o) => self.on_file_offer(&peer, o),
                    Msg::FileAnswer { id, accept } => self.on_file_answer(&peer, id, accept),
                    Msg::FileChunk { id, file, data } => self.on_file_chunk(&peer, id, file, data),
                    Msg::FileDone { id } => self.on_file_done(&peer, id),
                    Msg::FileCancel { id, reason } => self.on_file_cancel(&peer, id, reason),
                }
            }
        }
    }

    fn on_hello(&mut self, conn_id: u64, h: Hello) {
        let Some(c) = self.conns.get(&conn_id) else { return };
        let peer = c.remote_id.clone();
        if h.id != peer {
            self.conns.remove(&conn_id);
            return;
        }
        if h.protocol != PROTOCOL_VERSION {
            let name = h.name.clone();
            self.conns.remove(&conn_id);
            self.toast("warn", format!("Update Crispy on {name}"), format!("{name} runs Crispy {}, which can't talk to this version.", h.version));
            return;
        }
        if !self.trust.peers.get(&peer).is_some_and(|p| p.enabled) {
            self.conns.remove(&conn_id);
            return;
        }

        // Two connections to the same device (both dialled at once): keep the one dialled by the
        // device with the smaller id. Both sides apply the same rule, so they agree.
        if let Some(&existing) = self.peers.get(&peer) {
            if existing != conn_id {
                let preferred_initiator = self.my_id.clone().min(peer.clone());
                let initiator_of = |c: &Conn| if c.session.initiator { self.my_id.clone() } else { peer.clone() };
                let new_ok = initiator_of(&self.conns[&conn_id]) == preferred_initiator;
                let old_ok = self.conns.get(&existing).is_some_and(|c| initiator_of(c) == preferred_initiator);
                if old_ok && !new_ok {
                    self.conns.remove(&conn_id);
                    return;
                }
                self.conns.remove(&existing);
            }
        }

        let addr = self.conns[&conn_id].session.addr;
        let c = self.conns.get_mut(&conn_id).unwrap();
        c.hello = true;
        c.screens = h.screens.clone();
        self.peers.insert(peer.clone(), conn_id);
        self.connecting.remove(&peer);

        let first_time = {
            let p = self.trust.peers.get_mut(&peer).unwrap();
            let first = p.version.is_none();
            p.name = h.name.chars().take(64).collect();
            p.os = h.os;
            p.version = Some(h.version.clone());
            p.screens = h.screens.clone();
            p.last_address = Some(SocketAddr::new(addr.ip(), h.port).to_string());
            first
        };
        let _ = self.trust.save(&self.paths.peers());

        if h.layout.supersedes(&self.layout) {
            self.layout = h.layout;
            let _ = crate::config::save_json(&self.paths.layout(), &self.layout);
        }
        self.ensure_placed();
        // Our arrangement may be newer than theirs: make sure they have it.
        self.conns[&conn_id].session.send(Msg::Layout(self.layout.clone()));
        self.rebuild_world();
        if first_time {
            self.toast("success", format!("{} is connected", h.name), "Move your cursor across the shared edge to use it.");
        }
        self.dirty = true;
    }

    fn on_closed(&mut self, conn_id: u64, reason: String) {
        let Some(conn) = self.conns.remove(&conn_id) else { return };
        tracing::info!("connection to {} closed: {reason}", conn.remote_id);
        if let Some(p) = &mut self.pairing {
            if p.conn == Some(conn_id) && p.stage != PairingStage::Success {
                p.stage = PairingStage::Failed;
                p.error.get_or_insert_with(|| "The other computer closed the connection.".into());
                p.finished = Some(Instant::now());
            }
        }
        if self.peers.get(&conn.remote_id) == Some(&conn_id) {
            self.peers.remove(&conn.remote_id);
            self.peer_disconnected(&conn.remote_id);
        }
        self.dirty = true;
    }

    fn peer_disconnected(&mut self, peer: &str) {
        let was_focus = matches!(&self.focus, super::Focus::Remote { peer: p, .. } if p == peer);
        self.input_peer_gone(peer);
        self.transfers_peer_gone(peer);
        self.rebuild_world();
        if was_focus {
            let name = self.peer_name(peer);
            self.toast("warn", format!("Lost connection to {name}"), "Your keyboard and mouse are back on this computer.");
        }
    }

    /// Heartbeats, reconnects and timeouts. Runs once a second.
    pub(crate) fn peers_tick(&mut self) {
        let now_ms = now_ms();
        for c in self.peers.values() {
            if let Some(conn) = self.conns.get(c) {
                conn.session.send(Msg::Ping(now_ms));
            }
        }

        // Forget devices that stopped announcing themselves.
        let before = self.nearby.len();
        self.nearby.retain(|_, n| n.last_seen.elapsed() < NEARBY_TTL);
        if self.nearby.len() != before {
            self.dirty = true;
        }

        // Unpaired connections that aren't pairing are dropped quickly; pairing gets two minutes.
        let pairing_conn = self.pairing.as_ref().and_then(|p| p.conn);
        let stale: Vec<u64> = self
            .conns
            .iter()
            .filter(|(id, c)| {
                !c.trusted
                    && if Some(**id) == pairing_conn { c.opened.elapsed() > Duration::from_secs(120) } else { c.opened.elapsed() > Duration::from_secs(15) }
            })
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            self.on_closed(id, "pairing timed out".into());
        }

        if let Some(p) = &self.pairing {
            if p.finished.is_some_and(|t| t.elapsed() > Duration::from_secs(if p.stage == PairingStage::Success { 3 } else { 30 })) {
                self.pairing = None;
                self.dirty = true;
            }
        }

        // Reconnect to paired devices.
        let candidates: Vec<(DeviceId, SocketAddr)> = self
            .trust
            .peers
            .values()
            .filter(|p| p.enabled && !self.peers.contains_key(&p.id))
            .filter(|p| self.connecting.get(&p.id).is_none_or(|t| t.elapsed() > RECONNECT_EVERY))
            .filter_map(|p| {
                let addr = self
                    .nearby
                    .get(&p.id)
                    .map(|n| n.addr)
                    .or_else(|| p.last_address.as_ref().and_then(|a| a.parse().ok()))?;
                Some((p.id.clone(), addr))
            })
            .collect();
        for (id, addr) in candidates {
            self.spawn_connect(addr, Intent::Trusted(id));
        }
    }

    pub(crate) fn on_beacon(&mut self, b: crate::net::discovery::Beacon, src: SocketAddr) {
        if b.magic != crate::net::discovery::MAGIC || b.id == self.my_id {
            return;
        }
        let Ok(key) = base64::engine::general_purpose::STANDARD.decode(&b.key) else { return };
        if device_id(&key) != b.id {
            return;
        }
        let addr = SocketAddr::new(src.ip(), b.port);
        let is_new = !self.nearby.contains_key(&b.id);
        let name: String = b.name.chars().take(64).collect();
        let changed = self.nearby.get(&b.id).is_none_or(|n| n.name != name || n.addr != addr);
        self.nearby.insert(
            b.id.clone(),
            Nearby { id: b.id.clone(), name, os: b.os, addr, fingerprint: fingerprint(&key), last_seen: Instant::now(), last_seen_ms: now_ms() },
        );
        if is_new || changed {
            self.dirty = true;
        }
        let wanted = self.trust.peers.get(&b.id).is_some_and(|p| p.enabled && p.public_bytes() == key);
        if wanted && !self.peers.contains_key(&b.id) && self.connecting.get(&b.id).is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
            self.spawn_connect(addr, Intent::Trusted(b.id));
        }
    }

    pub(crate) fn connect_address(&mut self, address: String) {
        let address = address.trim().to_string();
        if address.is_empty() {
            return;
        }
        let mut p = Pairing::new(PairingRole::Initiator, PairingStage::Connecting);
        p.peer_name = address.clone();
        self.pairing = Some(p);
        self.dirty = true;
        let port = self.settings.network.port;
        let tx = self.internal_tx.clone();
        tokio::spawn(async move {
            let addr = crate::net::discovery::resolve(&address, port).await;
            let _ = tx.send(Internal::Resolved { address, addr });
        });
    }

    pub(crate) fn on_resolved(&mut self, address: String, addr: Option<SocketAddr>) {
        let Some(addr) = addr else {
            if let Some(p) = &mut self.pairing {
                p.stage = PairingStage::Failed;
                p.error = Some(format!("Couldn't find “{address}” on the network."));
                p.finished = Some(Instant::now());
                self.dirty = true;
            }
            return;
        };
        // Remember the address so discovery beacons reach it directly from now on.
        if !self.settings.network.manual_peers.iter().any(|m| m.trim() == address) {
            self.settings.network.manual_peers.push(address);
            let _ = crate::config::save_json(&self.paths.settings(), &self.settings);
            self.update_discovery();
        }
        self.spawn_connect(addr, Intent::Pair(None));
    }

    pub(crate) fn pair_start(&mut self, peer: &str) {
        // Found by discovery, or reached by address before (so "Try again" works for both).
        let found = match self.nearby.get(peer) {
            Some(n) => Some((n.addr, n.name.clone(), n.os)),
            None => self.pair_addresses.get(peer).map(|(a, name, os)| (*a, name.clone(), *os)),
        };
        let Some((addr, name, os)) = found else {
            self.toast("warn", "That device went away", "Make sure Crispy is open on it.");
            return;
        };
        if let Some(old) = self.pairing.take() {
            if let Some(c) = old.conn {
                if self.conns.get(&c).is_some_and(|c| !c.trusted) {
                    self.conns.remove(&c);
                }
            }
        }
        let mut p = Pairing::new(PairingRole::Initiator, PairingStage::Connecting);
        p.peer_id = Some(peer.to_string());
        p.peer_name = name;
        p.peer_os = os;
        self.pairing = Some(p);
        self.spawn_connect(addr, Intent::Pair(Some(peer.to_string())));
        self.dirty = true;
    }

    pub(crate) fn pair_submit(&mut self, code: &str) {
        let Some(p) = &mut self.pairing else { return };
        if p.role != PairingRole::Initiator || p.stage != PairingStage::EnterCode {
            return;
        }
        let Some(code) = pairing::normalize_code(code) else {
            p.error = Some("Enter all six digits.".into());
            self.dirty = true;
            return;
        };
        let Some(their) = p.their_spake.clone() else { return };
        let (state, msg) = pairing::start(&code, &p.hash);
        match pairing::finish(state, &their) {
            Ok(key) => {
                let confirm = pairing::confirm_tag(&key, &p.hash, ROLE_INITIATOR);
                p.key = Some(key);
                p.stage = PairingStage::Verifying;
                p.error = None;
                if let Some(c) = p.conn.and_then(|c| self.conns.get(&c)) {
                    c.session.send(Msg::Pair(PairMsg::Response { spake: msg, confirm }));
                }
            }
            Err(e) => self.pair_failed(e.to_string()),
        }
        self.dirty = true;
    }

    /// Close the pairing dialog. Before it finished, this also tells the other computer to stop.
    pub(crate) fn pair_cancel(&mut self) {
        if let Some(p) = self.pairing.take() {
            let finished = matches!(p.stage, PairingStage::Success | PairingStage::Failed);
            if let (false, Some(conn)) = (finished, p.conn) {
                if let Some(c) = self.conns.get(&conn) {
                    c.session.send(Msg::Pair(PairMsg::Reject { reason: "Pairing was cancelled on the other computer.".into() }));
                }
            }
        }
        self.dirty = true;
    }

    fn pair_failed(&mut self, error: String) {
        if let Some(p) = &mut self.pairing {
            p.stage = PairingStage::Failed;
            p.error = Some(error);
            p.finished = Some(Instant::now());
            p.spake = None;
            p.key = None;
            if let Some(c) = p.conn.take() {
                if self.conns.get(&c).is_some_and(|c| !c.trusted) {
                    self.conns.remove(&c);
                }
            }
        }
        self.dirty = true;
    }

    fn on_pair_msg(&mut self, conn_id: u64, msg: PairMsg) {
        match msg {
            PairMsg::Request { name, os } => {
                let busy = self.pairing.as_ref().is_some_and(|p| p.conn != Some(conn_id) && p.finished.is_none());
                let too_fast = self.last_pair_request.is_some_and(|t| t.elapsed() < Duration::from_secs(1));
                let Some(c) = self.conns.get(&conn_id) else { return };
                if busy || too_fast {
                    c.session.send(Msg::Pair(PairMsg::Reject { reason: "The other computer is busy. Try again in a moment.".into() }));
                    return;
                }
                self.last_pair_request = Some(Instant::now());
                let code = pairing::generate_code();
                let hash = c.session.handshake_hash.clone();
                let (state, spake) = pairing::start(&code, &hash);
                let mut p = Pairing::new(PairingRole::Responder, PairingStage::ShowCode);
                p.conn = Some(conn_id);
                p.peer_id = Some(c.remote_id.clone());
                p.peer_name = name.chars().take(64).collect();
                p.peer_os = os;
                p.code = Some(code.clone());
                p.spake = Some(state);
                p.hash = hash;
                c.session.send(Msg::Pair(PairMsg::Challenge { name: self.my_name(), os: self.my_os, spake }));
                let peer_name = p.peer_name.clone();
                self.pairing = Some(p);
                let _ = self.ui.send(UiEvent::Attention);
                self.notify(format!("{peer_name} wants to pair"), format!("Enter {} {} on {peer_name}.", &code[..3], &code[3..]));
            }
            PairMsg::Challenge { name, os, spake } => {
                let Some(p) = &mut self.pairing else { return };
                if p.conn != Some(conn_id) || p.role != PairingRole::Initiator {
                    return;
                }
                p.peer_name = name.chars().take(64).collect();
                p.peer_os = os;
                p.their_spake = Some(spake);
                p.stage = PairingStage::EnterCode;
                if let (Some(id), Some(c)) = (p.peer_id.clone(), self.conns.get(&conn_id)) {
                    self.pair_addresses.insert(id, (c.session.addr, p.peer_name.clone(), p.peer_os));
                }
                let _ = self.ui.send(UiEvent::Attention);
            }
            PairMsg::Response { spake, confirm } => {
                let Some(p) = &mut self.pairing else { return };
                if p.conn != Some(conn_id) || p.role != PairingRole::Responder {
                    return;
                }
                let Some(state) = p.spake.take() else { return };
                let hash = p.hash.clone();
                let ok = pairing::finish(state, &spake)
                    .ok()
                    .filter(|key| pairing::tags_equal(&confirm, &pairing::confirm_tag(key, &hash, ROLE_INITIATOR)));
                match ok {
                    Some(key) => {
                        let tag = pairing::confirm_tag(&key, &hash, ROLE_RESPONDER);
                        if let Some(c) = self.conns.get(&conn_id) {
                            c.session.send(Msg::Pair(PairMsg::Confirm { confirm: tag }));
                        }
                        self.complete_pairing(conn_id);
                    }
                    None => {
                        if let Some(c) = self.conns.get(&conn_id) {
                            c.session.send(Msg::Pair(PairMsg::Reject { reason: "The code didn't match. Try again with the new code.".into() }));
                        }
                        self.pair_failed("The code entered on the other computer didn't match.".into());
                    }
                }
            }
            PairMsg::Confirm { confirm } => {
                let Some(p) = &self.pairing else { return };
                if p.conn != Some(conn_id) || p.role != PairingRole::Initiator {
                    return;
                }
                let ok = p.key.as_ref().is_some_and(|k| pairing::tags_equal(&confirm, &pairing::confirm_tag(k, &p.hash, ROLE_RESPONDER)));
                if ok {
                    self.complete_pairing(conn_id);
                } else {
                    self.pair_failed("The other computer couldn't be verified.".into());
                }
            }
            PairMsg::Reject { reason } => {
                if self.pairing.as_ref().is_some_and(|p| p.conn == Some(conn_id)) {
                    self.pair_failed(reason.chars().take(200).collect());
                }
            }
        }
        self.dirty = true;
    }

    fn complete_pairing(&mut self, conn_id: u64) {
        let Some(p) = &mut self.pairing else { return };
        let Some(c) = self.conns.get_mut(&conn_id) else { return };
        let peer = TrustedPeer::new(&c.session.remote_static, p.peer_name.clone(), p.peer_os);
        c.trusted = true;
        p.stage = PairingStage::Success;
        p.finished = Some(Instant::now());
        p.spake = None;
        p.key = None;
        p.code = None;
        let name = peer.name.clone();
        self.trust.peers.insert(peer.id.clone(), peer);
        if let Err(e) = self.trust.save(&self.paths.peers()) {
            self.toast("error", "Couldn't save the pairing", e.to_string());
        }
        self.send_hello(conn_id);
        self.toast("success", format!("Paired with {name}"), "Arrange your screens in Arrangement, then just move the cursor across.");
        self.dirty = true;
    }

    pub(crate) fn unpair(&mut self, peer: &str, tell_them: bool) {
        if tell_them {
            self.send_to(peer, Msg::Unpair);
        }
        let conn = self.peers.remove(peer);
        self.peer_disconnected(peer);
        if let Some(c) = conn {
            // Let the Unpair message flush before the connection drops.
            if let Some(conn) = self.conns.remove(&c) {
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    drop(conn);
                });
            }
        }
        self.trust.peers.remove(peer);
        let _ = self.trust.save(&self.paths.peers());
        if self.layout.positions.remove(peer).is_some() {
            self.layout.rev += 1;
            self.layout.author = self.my_id.clone();
            let _ = crate::config::save_json(&self.paths.layout(), &self.layout);
            self.broadcast(Msg::Layout(self.layout.clone()));
        }
        self.rebuild_world();
        self.dirty = true;
    }

    pub(crate) fn set_peer_enabled(&mut self, peer: &str, enabled: bool) {
        let Some(p) = self.trust.peers.get_mut(peer) else { return };
        p.enabled = enabled;
        let _ = self.trust.save(&self.paths.peers());
        if !enabled {
            if let Some(c) = self.peers.remove(peer) {
                self.conns.remove(&c);
            }
            self.peer_disconnected(peer);
        } else {
            self.connecting.remove(peer);
        }
        self.dirty = true;
    }
}
