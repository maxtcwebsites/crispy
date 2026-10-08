//! File transfers between devices: offers, accept/decline, progress and completion.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use super::{Engine, Internal};
use crate::files::{self, Incoming, ReceivePlan, TransferEvent};
use crate::protocol::{FileEntry, FileOffer, Msg, Purpose};
use crate::state::{Direction, FileView, TransferState, TransferView, UiEvent};
use crate::types::{now_ms, DeviceId};

/// Hard ceiling for one transfer, whatever the settings say.
const MAX_TRANSFER_BYTES: u64 = 1 << 40;
const MAX_ENTRIES: usize = 200_000;

pub(crate) struct Transfer {
    pub view: TransferView,
    pub peer: DeviceId,
    /// The id both sides use on the wire (chosen by the sender).
    pub wire_id: u64,
    pub purpose: Purpose,
    pub entries: Vec<FileEntry>,
    pub sources: Vec<PathBuf>,
    pub cancel: Option<Arc<AtomicBool>>,
    pub sink: Option<mpsc::UnboundedSender<Incoming>>,
    speed_mark: Option<(Instant, u64)>,
}

impl Transfer {
    fn finish(&mut self, state: TransferState, error: Option<String>) {
        if self.view.state.is_finished() {
            return;
        }
        self.view.state = state;
        self.view.error = error;
        self.view.finished_at = Some(now_ms());
        self.view.bytes_per_sec = 0.0;
        self.view.current_file = None;
        if let Some(c) = &self.cancel {
            c.store(true, Ordering::Relaxed);
        }
        if let Some(s) = self.sink.take() {
            let _ = s.send(Incoming::Cancel);
        }
    }
}

fn file_views(entries: &[FileEntry]) -> (Vec<FileView>, usize) {
    // Show top-level items: a folder counts once, with its total size.
    let mut tops: Vec<FileView> = Vec::new();
    for e in entries {
        let top = e.path.split('/').next().unwrap_or(&e.path).to_string();
        match tops.iter_mut().find(|t| t.name == top) {
            Some(t) => t.size += e.size,
            None => tops.push(FileView { name: top, size: e.size }),
        }
    }
    let count = entries.iter().filter(|e| !e.dir).count();
    tops.truncate(8);
    (tops, count)
}

impl Engine {
    fn new_tid(&mut self) -> u64 {
        self.next_tid += 1;
        self.next_tid
    }

    fn find_transfer(&mut self, peer: &str, wire_id: u64, dir: Direction) -> Option<&mut Transfer> {
        self.transfers.values_mut().find(|t| t.peer == peer && t.wire_id == wire_id && t.view.direction == dir)
    }

    pub(crate) fn send_files(&mut self, peer: &DeviceId, paths: Vec<PathBuf>, purpose: Purpose) {
        if paths.is_empty() {
            return;
        }
        if !self.peers.contains_key(peer) {
            self.toast("warn", format!("{} isn't connected", self.peer_name(peer)), "Files can be sent once it's back online.");
            return;
        }
        let tid = self.new_tid();
        let names: Vec<FileView> = paths
            .iter()
            .take(8)
            .map(|p| FileView { name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), size: 0 })
            .collect();
        self.transfers.insert(
            tid,
            Transfer {
                view: TransferView {
                    id: tid.to_string(),
                    peer_id: peer.clone(),
                    peer_name: self.peer_name(peer),
                    direction: Direction::Send,
                    purpose: if purpose == Purpose::Files { "files" } else { "clipboard" },
                    files: names,
                    file_count: paths.len(),
                    total_bytes: 0,
                    done_bytes: 0,
                    bytes_per_sec: 0.0,
                    state: TransferState::Pending,
                    error: None,
                    started_at: now_ms(),
                    finished_at: None,
                    save_path: None,
                    current_file: None,
                },
                peer: peer.clone(),
                wire_id: tid,
                purpose,
                entries: vec![],
                sources: vec![],
                cancel: None,
                sink: None,
                speed_mark: None,
            },
        );
        let tx = self.internal_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = files::build_manifest(&paths).map_err(|e| e.to_string());
            let _ = tx.send(Internal::ManifestReady { tid, result });
        });
        self.dirty = true;
    }

    pub(crate) fn on_manifest_ready(&mut self, tid: u64, result: Result<(Vec<FileEntry>, Vec<PathBuf>), String>) {
        let clip_limit = self.settings.clipboard.max_files_mb as u64 * 1024 * 1024;
        let Some(t) = self.transfers.get_mut(&tid) else { return };
        if t.view.state.is_finished() {
            return;
        }
        let (entries, sources) = match result {
            Ok(v) => v,
            Err(e) => {
                t.finish(TransferState::Failed, Some(e));
                self.dirty = true;
                return;
            }
        };
        let total: u64 = entries.iter().map(|e| e.size).sum();
        if t.purpose == Purpose::Clipboard && total > clip_limit {
            // Too big to share through the clipboard: quietly drop it.
            self.transfers.remove(&tid);
            self.toast(
                "info",
                "Copied files are too large to share",
                format!("Use Send files for anything over {} MB, or raise the limit in Settings → Clipboard.", clip_limit / (1024 * 1024)),
            );
            return;
        }
        let (files, count) = file_views(&entries);
        t.view.files = files;
        t.view.file_count = count;
        t.view.total_bytes = total;
        t.view.state = TransferState::Awaiting;
        let offer = FileOffer { id: t.wire_id, purpose: t.purpose, entries: entries.clone(), total_bytes: total };
        t.entries = entries;
        t.sources = sources;
        let peer = t.peer.clone();
        // The offer can be large (many files): send it on the bulk lane.
        if let Some(tx) = self.bulk_sender(&peer) {
            tokio::spawn(async move {
                let _ = tx.send(Msg::FileOffer(offer)).await;
            });
        }
        self.dirty = true;
    }

    pub(crate) fn on_file_offer(&mut self, peer: &DeviceId, offer: FileOffer) {
        let total: u64 = offer.entries.iter().map(|e| e.size).sum();
        if offer.entries.is_empty() || offer.entries.len() > MAX_ENTRIES || total != offer.total_bytes || total > MAX_TRANSFER_BYTES {
            self.send_to(peer, Msg::FileCancel { id: offer.id, reason: "invalid offer".into() });
            return;
        }
        let clipboard = offer.purpose == Purpose::Clipboard;
        if clipboard && (!self.settings.clipboard.enabled || !self.settings.clipboard.files) {
            self.send_to(peer, Msg::FileAnswer { id: offer.id, accept: false });
            return;
        }
        if offer.entries.iter().any(|e| files::sanitize_relative(&e.path).is_none()) {
            self.send_to(peer, Msg::FileCancel { id: offer.id, reason: "unsafe file names".into() });
            return;
        }
        let tid = self.new_tid();
        let (views, count) = file_views(&offer.entries);
        let peer_name = self.peer_name(peer);
        self.transfers.insert(
            tid,
            Transfer {
                view: TransferView {
                    id: tid.to_string(),
                    peer_id: peer.clone(),
                    peer_name: peer_name.clone(),
                    direction: Direction::Receive,
                    purpose: if clipboard { "clipboard" } else { "files" },
                    files: views,
                    file_count: count,
                    total_bytes: total,
                    done_bytes: 0,
                    bytes_per_sec: 0.0,
                    state: TransferState::Awaiting,
                    error: None,
                    started_at: now_ms(),
                    finished_at: None,
                    save_path: None,
                    current_file: None,
                },
                peer: peer.clone(),
                wire_id: offer.id,
                purpose: offer.purpose,
                entries: offer.entries,
                sources: vec![],
                cancel: None,
                sink: None,
                speed_mark: None,
            },
        );
        if clipboard || self.settings.files.auto_accept {
            self.accept_incoming(tid);
        } else {
            let _ = self.ui.send(UiEvent::Attention);
            self.notify(format!("{peer_name} wants to send you files"), format!("{count} item(s) · {}", human_size(total)));
        }
        self.dirty = true;
    }

    fn accept_incoming(&mut self, tid: u64) {
        let (dest, preserve) = match self.transfers.get(&tid).map(|t| t.purpose) {
            Some(Purpose::Clipboard) => (self.paths.clipboard_cache().join(format!("{tid:x}")), true),
            Some(Purpose::Files) => (self.save_dir(), self.settings.files.preserve_timestamps),
            None => return,
        };
        let policy = self.settings.files.conflict;
        let events = self.transfer_tx.clone();
        let Some(t) = self.transfers.get_mut(&tid) else { return };
        let result = std::fs::create_dir_all(&dest)
            .map_err(anyhow::Error::from)
            .and_then(|_| files::plan_destinations(&t.entries, &dest, policy));
        let destinations = match result {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("Couldn't save to {}: {e}", dest.display());
                let (peer, id) = (t.peer.clone(), t.wire_id);
                t.finish(TransferState::Failed, Some(msg.clone()));
                self.send_to(&peer, Msg::FileCancel { id, reason: msg });
                self.dirty = true;
                return;
            }
        };
        let (sink, rx) = mpsc::unbounded_channel();
        t.sink = Some(sink);
        t.view.state = TransferState::Active;
        t.view.save_path = Some(dest.to_string_lossy().into_owned());
        let plan = ReceivePlan { id: tid, entries: t.entries.clone(), destinations, preserve_times: preserve };
        let (peer, wire_id) = (t.peer.clone(), t.wire_id);
        tokio::spawn(files::receive_files(plan, rx, events));
        self.send_to(&peer, Msg::FileAnswer { id: wire_id, accept: true });
        self.dirty = true;
    }

    pub(crate) fn answer_transfer(&mut self, id: &str, accept: bool) {
        let Ok(tid) = id.parse::<u64>() else { return };
        let Some(t) = self.transfers.get_mut(&tid) else { return };
        if t.view.direction != Direction::Receive || t.view.state != TransferState::Awaiting {
            return;
        }
        if accept {
            self.accept_incoming(tid);
        } else {
            let (peer, wire_id) = (t.peer.clone(), t.wire_id);
            t.finish(TransferState::Declined, None);
            self.send_to(&peer, Msg::FileAnswer { id: wire_id, accept: false });
        }
        self.dirty = true;
    }

    pub(crate) fn on_file_answer(&mut self, peer: &DeviceId, id: u64, accept: bool) {
        let limit = self.settings.files.bandwidth_limit_mbps;
        let events = self.transfer_tx.clone();
        let out = self.bulk_sender(peer);
        let Some(t) = self.find_transfer(peer, id, Direction::Send) else { return };
        if t.view.state != TransferState::Awaiting {
            return;
        }
        if !accept {
            t.finish(TransferState::Declined, None);
            self.dirty = true;
            return;
        }
        let Some(out) = out else { return };
        let cancel = Arc::new(AtomicBool::new(false));
        t.cancel = Some(cancel.clone());
        t.view.state = TransferState::Active;
        let tid: u64 = t.view.id.parse().unwrap_or(0);
        let (entries, sources) = (std::mem::take(&mut t.entries), std::mem::take(&mut t.sources));
        tokio::spawn(async move {
            // The wire id is what the receiver knows; progress events carry our local id.
            let (fwd_tx, mut fwd_rx) = mpsc::unbounded_channel();
            let sender = tokio::spawn(files::send_files(id, entries, sources, out, cancel, limit, fwd_tx));
            while let Some(ev) = fwd_rx.recv().await {
                let ev = match ev {
                    TransferEvent::Progress { done, current, .. } => TransferEvent::Progress { id: tid, done, current },
                    TransferEvent::Finished { saved, .. } => TransferEvent::Finished { id: tid, saved },
                    TransferEvent::Failed { error, .. } => TransferEvent::Failed { id: tid, error },
                    TransferEvent::Cancelled { .. } => TransferEvent::Cancelled { id: tid },
                };
                let _ = events.send(ev);
            }
            let _ = sender.await;
        });
        self.dirty = true;
    }

    pub(crate) fn on_file_chunk(&mut self, peer: &DeviceId, id: u64, file: u32, data: Vec<u8>) {
        if let Some(t) = self.find_transfer(peer, id, Direction::Receive) {
            if let Some(s) = &t.sink {
                let _ = s.send(Incoming::Chunk { file, data });
            }
        }
    }

    pub(crate) fn on_file_done(&mut self, peer: &DeviceId, id: u64) {
        if let Some(t) = self.find_transfer(peer, id, Direction::Receive) {
            if let Some(s) = &t.sink {
                let _ = s.send(Incoming::Done);
            }
        }
    }

    pub(crate) fn on_file_cancel(&mut self, peer: &DeviceId, id: u64, reason: String) {
        let reason: String = reason.chars().take(200).collect();
        for dir in [Direction::Send, Direction::Receive] {
            if let Some(t) = self.find_transfer(peer, id, dir) {
                let state = if reason == "cancelled" { TransferState::Cancelled } else { TransferState::Failed };
                t.finish(state, (state == TransferState::Failed).then_some(reason.clone()));
                self.dirty = true;
            }
        }
    }

    pub(crate) fn cancel_transfer(&mut self, id: &str) {
        let Ok(tid) = id.parse::<u64>() else { return };
        let Some(t) = self.transfers.get_mut(&tid) else { return };
        if t.view.state.is_finished() {
            return;
        }
        let (peer, wire_id, dir, state) = (t.peer.clone(), t.wire_id, t.view.direction, t.view.state);
        t.finish(TransferState::Cancelled, None);
        if dir == Direction::Receive && state == TransferState::Awaiting {
            self.send_to(&peer, Msg::FileAnswer { id: wire_id, accept: false });
        } else {
            self.send_to(&peer, Msg::FileCancel { id: wire_id, reason: "cancelled".into() });
        }
        self.dirty = true;
    }

    pub(crate) fn on_transfer_event(&mut self, ev: TransferEvent) {
        match ev {
            TransferEvent::Progress { id, done, current } => {
                if let Some(t) = self.transfers.get_mut(&id) {
                    if t.view.state.is_finished() {
                        return;
                    }
                    let now = Instant::now();
                    match t.speed_mark {
                        Some((at, bytes)) if now.duration_since(at).as_secs_f64() >= 0.5 => {
                            let rate = (done.saturating_sub(bytes)) as f64 / now.duration_since(at).as_secs_f64();
                            t.view.bytes_per_sec = if t.view.bytes_per_sec == 0.0 { rate } else { t.view.bytes_per_sec * 0.6 + rate * 0.4 };
                            t.speed_mark = Some((now, done));
                        }
                        None => t.speed_mark = Some((now, done)),
                        _ => {}
                    }
                    t.view.done_bytes = done;
                    if current.is_some() {
                        t.view.current_file = current;
                    }
                    self.dirty = true;
                }
            }
            TransferEvent::Finished { id, saved } => {
                let Some(t) = self.transfers.get_mut(&id) else { return };
                if t.view.state.is_finished() {
                    return;
                }
                t.view.done_bytes = t.view.total_bytes;
                t.sink = None;
                t.finish(TransferState::Done, None);
                let (dir, purpose, peer, name, count) = (t.view.direction, t.purpose, t.peer.clone(), t.view.peer_name.clone(), t.view.file_count);
                if dir == Direction::Receive {
                    if purpose == Purpose::Clipboard {
                        self.clipboard_files_arrived(&peer, saved);
                    } else {
                        self.notify(format!("Files from {name}"), format!("{count} item(s) saved to {}", self.save_dir().display()));
                        self.toast("success", format!("Received from {name}"), format!("{count} item(s) saved to {}", self.save_dir().display()));
                        if self.settings.files.reveal_when_done {
                            if let Some(first) = saved.into_iter().next() {
                                let _ = self.ui.send(UiEvent::Reveal(first));
                            }
                        }
                    }
                } else if purpose == Purpose::Files {
                    self.toast("success", format!("Sent to {name}"), format!("{count} item(s) delivered."));
                }
                self.dirty = true;
            }
            TransferEvent::Failed { id, error } => {
                if let Some(t) = self.transfers.get_mut(&id) {
                    let (peer, wire_id) = (t.peer.clone(), t.wire_id);
                    t.sink = None;
                    t.finish(TransferState::Failed, Some(error.clone()));
                    self.send_to(&peer, Msg::FileCancel { id: wire_id, reason: error });
                    self.dirty = true;
                }
            }
            TransferEvent::Cancelled { id } => {
                if let Some(t) = self.transfers.get_mut(&id) {
                    t.sink = None;
                    t.finish(TransferState::Cancelled, None);
                    self.dirty = true;
                }
            }
        }
    }

    pub(crate) fn transfers_peer_gone(&mut self, peer: &str) {
        for t in self.transfers.values_mut().filter(|t| t.peer == peer) {
            if !t.view.state.is_finished() {
                t.finish(TransferState::Failed, Some("The connection was lost.".into()));
                self.dirty = true;
            }
        }
    }

    pub(crate) fn transfers_tick(&mut self) {
        // Keep the list bounded: forget the oldest finished transfers beyond 200.
        let finished: Vec<u64> = self.transfers.iter().filter(|(_, t)| t.view.state.is_finished()).map(|(k, _)| *k).collect();
        if finished.len() > 200 {
            for k in &finished[..finished.len() - 200] {
                self.transfers.remove(k);
            }
        }
    }
}

pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}
