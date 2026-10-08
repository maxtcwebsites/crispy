//! Clipboard sharing. Text, rich text and images travel inline; copied files travel as a
//! file transfer when the cursor moves to the other computer, so copying a huge folder never
//! floods the network unless you actually go and paste it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{Engine, Internal};
use crate::clipboard::ClipContent;
use crate::config::ClipboardMode;
use crate::protocol::{ClipboardData, Msg, Purpose};
use crate::types::DeviceId;

#[derive(Default)]
pub(crate) struct ClipState {
    /// What this computer's clipboard holds (as last read or written by Crispy).
    current: Option<ClipContent>,
    version: u64,
    /// Key of the content we last saw or wrote, to ignore our own writes coming back as changes.
    last_key: Option<u64>,
    ignore_until: Option<Instant>,
    read_due: Option<Instant>,
    reading: bool,
    /// Which clipboard version each peer already has.
    sent: HashMap<DeviceId, u64>,
    /// Which version's files each peer already has.
    files_sent: HashMap<DeviceId, u64>,
}

impl Engine {
    fn clipboard_on(&self) -> bool {
        self.settings.clipboard.enabled
    }

    pub(crate) fn on_clipboard_changed(&mut self) {
        if !self.clipboard_on() {
            return;
        }
        // Apps often write several formats in a row: read once things settle.
        self.clip_state.read_due = Some(Instant::now() + Duration::from_millis(150));
    }

    pub(crate) fn clipboard_tick(&mut self) {
        let due = self.clip_state.read_due.is_some_and(|t| Instant::now() >= t);
        if !due || self.clip_state.reading {
            return;
        }
        self.clip_state.read_due = None;
        self.clip_state.reading = true;
        let clip = self.clip.clone();
        let images = self.settings.clipboard.images;
        let tx = self.internal_tx.clone();
        tokio::task::spawn_blocking(move || {
            let _ = tx.send(Internal::ClipboardRead(clip.read(images)));
        });
    }

    pub(crate) fn on_clipboard_read(&mut self, content: Option<ClipContent>) {
        self.clip_state.reading = false;
        let Some(content) = content else { return };
        let key = content.key();
        if self.clip_state.last_key == Some(key) {
            return;
        }
        if self.clip_state.ignore_until.is_some_and(|t| Instant::now() < t) {
            self.clip_state.last_key = Some(key);
            return;
        }
        self.clip_state.last_key = Some(key);
        self.clip_state.version += 1;
        self.clip_state.current = Some(content);

        if self.settings.clipboard.mode == ClipboardMode::Instant {
            let peers: Vec<DeviceId> = self.peers.keys().cloned().collect();
            for p in peers {
                self.push_clipboard_inline(&p);
            }
        }
        // If the cursor is already over there, files go now too.
        if let super::Focus::Remote { peer, .. } = self.focus.clone() {
            self.push_clipboard_files(&peer);
        }
    }

    /// Filter content by what the user chose to share, and by size.
    fn shareable(&self, c: &ClipContent) -> Option<ClipboardData> {
        let s = &self.settings.clipboard;
        let mut out = ClipContent::default();
        if s.text {
            out.text = c.text.clone();
        }
        if s.rich_text {
            out.html = c.html.clone();
            out.rtf = c.rtf.clone();
        }
        if s.images {
            out.png = c.png.clone().filter(|p| p.len() as u64 <= s.max_image_mb as u64 * 1024 * 1024);
        }
        if !c.files.is_empty() {
            // File lists also carry their paths as text; that text is meaningless elsewhere.
            out.text = None;
            out.html = None;
            out.rtf = None;
        }
        (!out.is_empty()).then(|| out.to_wire())
    }

    fn push_clipboard_inline(&mut self, peer: &DeviceId) {
        let version = self.clip_state.version;
        if self.clip_state.sent.get(peer) == Some(&version) {
            return;
        }
        let Some(c) = &self.clip_state.current else { return };
        if let Some(data) = self.shareable(c) {
            self.send_to(peer, Msg::Clipboard(data));
        }
        self.clip_state.sent.insert(peer.clone(), version);
    }

    fn push_clipboard_files(&mut self, peer: &DeviceId) {
        let s = &self.settings.clipboard;
        if !s.enabled || !s.files {
            return;
        }
        let version = self.clip_state.version;
        let Some(c) = &self.clip_state.current else { return };
        if c.files.is_empty() || self.clip_state.files_sent.get(peer) == Some(&version) {
            return;
        }
        let files = c.files.clone();
        self.clip_state.files_sent.insert(peer.clone(), version);
        self.send_files(peer, files, Purpose::Clipboard);
    }

    /// The cursor just moved to `peer`: make sure it has our clipboard.
    pub(crate) fn on_focus_moved_to_peer(&mut self, peer: &DeviceId) {
        if !self.clipboard_on() {
            return;
        }
        self.push_clipboard_inline(peer);
        self.push_clipboard_files(peer);
    }

    /// A peer stopped driving this computer: hand it back what was copied here meanwhile.
    pub(crate) fn on_peer_left_us(&mut self, peer: &DeviceId) {
        if !self.clipboard_on() {
            return;
        }
        self.push_clipboard_inline(peer);
        self.push_clipboard_files(peer);
    }

    pub(crate) fn on_remote_clipboard(&mut self, from: &DeviceId, data: ClipboardData) {
        if !self.clipboard_on() {
            return;
        }
        let incoming = ClipContent::from_wire(data);
        let Some(filtered) = self.shareable(&incoming).map(ClipContent::from_wire) else { return };
        self.clip_state.last_key = Some(filtered.key());
        if filtered.png.is_some() {
            // The OS may re-encode the image; ignore the change notification our write causes.
            self.clip_state.ignore_until = Some(Instant::now() + Duration::from_millis(1200));
        }
        self.clip_state.version += 1;
        self.clip_state.sent.insert(from.clone(), self.clip_state.version);
        self.clip_state.current = Some(filtered.clone());
        let clip = self.clip.clone();
        let tx = self.internal_tx.clone();
        tokio::task::spawn_blocking(move || match clip.write(&filtered) {
            Ok(key) => {
                let _ = tx.send(Internal::ClipboardWritten { key });
            }
            Err(e) => tracing::warn!("clipboard write failed: {e}"),
        });
    }

    pub(crate) fn on_clipboard_written(&mut self, key: Option<u64>) {
        if let (Some(k), Some(c)) = (key, self.clip_state.current.as_mut()) {
            c.image_key = Some(k);
            self.clip_state.last_key = Some(c.key());
        }
    }

    /// Files copied on a peer arrived: put them on this clipboard.
    pub(crate) fn clipboard_files_arrived(&mut self, from: &DeviceId, paths: Vec<std::path::PathBuf>) {
        let content = ClipContent { files: paths.clone(), ..Default::default() };
        self.clip_state.last_key = Some(content.key());
        self.clip_state.ignore_until = Some(Instant::now() + Duration::from_millis(1200));
        self.clip_state.version += 1;
        self.clip_state.sent.insert(from.clone(), self.clip_state.version);
        self.clip_state.files_sent.insert(from.clone(), self.clip_state.version);
        self.clip_state.current = Some(content);
        let clip = self.clip.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = clip.write_files(&paths) {
                tracing::warn!("couldn't put files on the clipboard: {e}");
            }
        });
    }
}
