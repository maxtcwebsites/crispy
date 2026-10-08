//! Clipboard access. The engine decides *what* to sync; this module only reads and writes the
//! system clipboard and reports changes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::protocol::{ClipItem, ClipboardData};

/// What is on a clipboard, independent of the platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipContent {
    pub text: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub png: Option<Vec<u8>>,
    /// Hash of the decoded pixels, so an image re-encoded by the OS is still recognised.
    pub image_key: Option<u64>,
    pub files: Vec<PathBuf>,
}

impl ClipContent {
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.html.is_none() && self.rtf.is_none() && self.png.is_none() && self.files.is_empty()
    }

    /// Identity used to recognise our own writes coming back as "changes".
    pub fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        if !self.files.is_empty() {
            "files".hash(&mut h);
            self.files.hash(&mut h);
        } else if let Some(t) = &self.text {
            "text".hash(&mut h);
            t.hash(&mut h);
        } else if let Some(k) = self.image_key {
            "image".hash(&mut h);
            k.hash(&mut h);
        } else if let Some(html) = &self.html {
            "html".hash(&mut h);
            html.hash(&mut h);
        } else {
            "empty".hash(&mut h);
        }
        h.finish()
    }

    pub fn to_wire(&self) -> ClipboardData {
        let mut items = Vec::new();
        if let Some(t) = &self.text {
            items.push(ClipItem::Text(t.clone()));
        }
        if let Some(t) = &self.html {
            items.push(ClipItem::Html(t.clone()));
        }
        if let Some(t) = &self.rtf {
            items.push(ClipItem::Rtf(t.clone()));
        }
        if let Some(p) = &self.png {
            items.push(ClipItem::Png(p.clone()));
        }
        ClipboardData { items }
    }

    pub fn from_wire(data: ClipboardData) -> ClipContent {
        let mut c = ClipContent::default();
        for item in data.items {
            match item {
                ClipItem::Text(t) => c.text = Some(t),
                ClipItem::Html(t) => c.html = Some(t),
                ClipItem::Rtf(t) => c.rtf = Some(t),
                ClipItem::Png(p) => c.png = Some(p),
            }
        }
        c
    }
}

pub fn pixel_key(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    (width, height).hash(&mut h);
    rgba.hash(&mut h);
    h.finish()
}

pub trait ClipboardBackend: Send + Sync + 'static {
    fn read(&self, images: bool) -> Option<ClipContent>;
    /// Write content; returns the content's `image_key` as computed locally (for images).
    fn write(&self, content: &ClipContent) -> anyhow::Result<Option<u64>>;
    fn write_files(&self, files: &[PathBuf]) -> anyhow::Result<()>;
    /// Start watching; `on_change` is called from a background thread whenever the clipboard changes.
    fn watch(&self, on_change: Box<dyn Fn() + Send + 'static>);
}

/// The clipboard for this OS.
pub fn native() -> Arc<dyn ClipboardBackend> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    return Arc::new(system::SystemClipboard);
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return Arc::new(MemoryClipboard::default());
}

/// In-memory clipboard for tests and unsupported platforms.
#[derive(Default)]
pub struct MemoryClipboard {
    content: Mutex<ClipContent>,
    watcher: Mutex<Option<Box<dyn Fn() + Send>>>,
}

impl MemoryClipboard {
    /// Simulate the user copying something.
    pub fn user_copy(&self, content: ClipContent) {
        *self.content.lock() = content;
        if let Some(w) = self.watcher.lock().as_ref() {
            w();
        }
    }
    pub fn current(&self) -> ClipContent {
        self.content.lock().clone()
    }
}

impl ClipboardBackend for MemoryClipboard {
    fn read(&self, _images: bool) -> Option<ClipContent> {
        let c = self.content.lock().clone();
        (!c.is_empty()).then_some(c)
    }
    fn write(&self, content: &ClipContent) -> anyhow::Result<Option<u64>> {
        *self.content.lock() = content.clone();
        if let Some(w) = self.watcher.lock().as_ref() {
            w();
        }
        Ok(content.image_key)
    }
    fn write_files(&self, files: &[PathBuf]) -> anyhow::Result<()> {
        *self.content.lock() = ClipContent { files: files.to_vec(), ..Default::default() };
        if let Some(w) = self.watcher.lock().as_ref() {
            w();
        }
        Ok(())
    }
    fn watch(&self, on_change: Box<dyn Fn() + Send + 'static>) {
        *self.watcher.lock() = Some(on_change);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod system {
    use super::*;
    use clipboard_rs::common::RustImage;
    use clipboard_rs::{
        Clipboard, ClipboardContent, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext,
        ContentFormat, RustImageData,
    };
    use std::time::Duration;

    pub struct SystemClipboard;

    /// Another app may hold the clipboard open for a moment (Windows): retry briefly.
    fn context() -> Option<ClipboardContext> {
        for _ in 0..5 {
            if let Ok(c) = ClipboardContext::new() {
                return Some(c);
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        None
    }

    fn image_key(img: &RustImageData) -> Option<u64> {
        let rgba = img.to_rgba8().ok()?;
        Some(pixel_key(rgba.width(), rgba.height(), rgba.as_raw()))
    }

    struct Handler(Box<dyn Fn() + Send>);
    impl ClipboardHandler for Handler {
        fn on_clipboard_change(&mut self) {
            (self.0)();
        }
    }

    impl ClipboardBackend for SystemClipboard {
        fn read(&self, images: bool) -> Option<ClipContent> {
            let ctx = context()?;
            let mut c = ClipContent::default();
            if ctx.has(ContentFormat::Files) {
                if let Ok(files) = ctx.get_files() {
                    c.files = files
                        .into_iter()
                        .map(|f| {
                            let f = f.strip_prefix("file://").map(|s| s.to_string()).unwrap_or(f);
                            PathBuf::from(percent_decode(&f))
                        })
                        .filter(|p| p.exists())
                        .collect();
                }
            }
            if ctx.has(ContentFormat::Text) {
                c.text = ctx.get_text().ok().filter(|t| !t.is_empty());
            }
            if ctx.has(ContentFormat::Html) {
                c.html = ctx.get_html().ok().filter(|t| !t.is_empty());
            }
            if ctx.has(ContentFormat::Rtf) {
                c.rtf = ctx.get_rich_text().ok().filter(|t| !t.is_empty());
            }
            if images && c.files.is_empty() && ctx.has(ContentFormat::Image) {
                if let Ok(img) = ctx.get_image() {
                    c.image_key = image_key(&img);
                    c.png = img.to_png().ok().map(|b| b.get_bytes().to_vec());
                }
            }
            (!c.is_empty()).then_some(c)
        }

        fn write(&self, content: &ClipContent) -> anyhow::Result<Option<u64>> {
            let ctx = context().ok_or_else(|| anyhow::anyhow!("clipboard unavailable"))?;
            let mut items = Vec::new();
            let mut key = None;
            if let Some(t) = &content.text {
                items.push(ClipboardContent::Text(t.clone()));
            }
            if let Some(t) = &content.html {
                items.push(ClipboardContent::Html(t.clone()));
            }
            if let Some(t) = &content.rtf {
                items.push(ClipboardContent::Rtf(t.clone()));
            }
            if let Some(png) = &content.png {
                let img = RustImageData::from_bytes(png).map_err(|e| anyhow::anyhow!("{e}"))?;
                key = image_key(&img);
                items.push(ClipboardContent::Image(img));
            }
            if items.is_empty() {
                return Ok(None);
            }
            ctx.set(items).map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(key)
        }

        fn write_files(&self, files: &[PathBuf]) -> anyhow::Result<()> {
            let ctx = context().ok_or_else(|| anyhow::anyhow!("clipboard unavailable"))?;
            let list = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            ctx.set_files(list).map_err(|e| anyhow::anyhow!("{e}"))
        }

        fn watch(&self, on_change: Box<dyn Fn() + Send + 'static>) {
            let _ = std::thread::Builder::new().name("crispy-clipboard".into()).spawn(move || {
                let Ok(mut watcher) = ClipboardWatcherContext::<Handler>::new() else {
                    tracing::warn!("clipboard watcher unavailable");
                    return;
                };
                watcher.add_handler(Handler(on_change));
                // Keep the shutdown channel alive: dropping it stops the watcher.
                let _shutdown = watcher.get_shutdown_channel();
                watcher.start_watch();
            });
        }
    }

    fn percent_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_identify_content_not_formatting() {
        let a = ClipContent { text: Some("hi".into()), html: Some("<b>hi</b>".into()), ..Default::default() };
        let b = ClipContent { text: Some("hi".into()), rtf: Some("{\\rtf hi}".into()), ..Default::default() };
        assert_eq!(a.key(), b.key());
        let c = ClipContent { text: Some("bye".into()), ..Default::default() };
        assert_ne!(a.key(), c.key());
    }

    #[test]
    fn wire_round_trip() {
        let a = ClipContent { text: Some("hi".into()), png: Some(vec![1, 2, 3]), ..Default::default() };
        let back = ClipContent::from_wire(a.to_wire());
        assert_eq!(back.text, a.text);
        assert_eq!(back.png, a.png);
    }
}
