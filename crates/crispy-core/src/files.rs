//! File transfer: building manifests, streaming chunks, and writing received files safely.
//!
//! Received paths are never trusted: every component is sanitised so a peer can only ever write
//! inside the chosen destination folder. Files are written as `.crispypart` and renamed when
//! complete, so a cancelled or broken transfer never leaves a half-written file under its real name.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::config::ConflictPolicy;
use crate::protocol::{FileEntry, Msg};

/// Payload bytes per chunk: comfortably inside one encrypted frame.
pub const CHUNK: usize = 60 * 1024;
const PART_SUFFIX: &str = ".crispypart";

/// Progress reported from transfer tasks back to the engine.
#[derive(Debug)]
pub enum TransferEvent {
    Progress { id: u64, done: u64, current: Option<String> },
    Finished { id: u64, saved: Vec<PathBuf> },
    Failed { id: u64, error: String },
    Cancelled { id: u64 },
}

/// Walk the given paths into a manifest. Directories keep their structure; symlinks are skipped.
pub fn build_manifest(paths: &[PathBuf]) -> anyhow::Result<(Vec<FileEntry>, Vec<PathBuf>)> {
    let mut entries = Vec::new();
    let mut sources = Vec::new();
    let mut used_roots = std::collections::HashSet::new();
    for root in paths {
        let meta = std::fs::symlink_metadata(root).with_context(|| format!("can't read {}", root.display()))?;
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into());
        let mut top = name.clone();
        let mut n = 2;
        while !used_roots.insert(top.clone()) {
            top = format!("{name} ({n})");
            n += 1;
        }
        if meta.is_file() {
            entries.push(FileEntry { path: top, size: meta.len(), mtime: mtime_of(&meta), dir: false });
            sources.push(root.clone());
        } else if meta.is_dir() {
            for item in walkdir::WalkDir::new(root).follow_links(false).sort_by_file_name() {
                let item = item?;
                let ft = item.file_type();
                if ft.is_symlink() {
                    continue;
                }
                let rel = item.path().strip_prefix(root).unwrap_or(item.path());
                let mut path = top.clone();
                for c in rel.components() {
                    path.push('/');
                    path.push_str(&c.as_os_str().to_string_lossy());
                }
                let meta = item.metadata()?;
                entries.push(FileEntry { path, size: if ft.is_file() { meta.len() } else { 0 }, mtime: mtime_of(&meta), dir: ft.is_dir() });
                sources.push(item.path().to_path_buf());
            }
        }
    }
    if entries.is_empty() {
        bail!("nothing to send");
    }
    Ok((entries, sources))
}

fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Make one received path component safe on every OS.
pub fn sanitize_component(c: &str) -> Option<String> {
    let mut s: String = c
        .chars()
        .map(|ch| if ch.is_control() || "<>:\"/\\|?*".contains(ch) { '_' } else { ch })
        .collect();
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    let s = s.trim_start().to_string();
    if s.is_empty() || s == "." || s == ".." {
        return None;
    }
    let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return Some(format!("_{s}"));
    }
    Some(s.chars().take(200).collect())
}

/// Turn a `/`-separated relative path from a peer into a safe relative path.
pub fn sanitize_relative(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        out.push(sanitize_component(part)?);
    }
    // Belt and braces: nothing absolute or parent-relative may survive.
    if out.as_os_str().is_empty() || out.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(out)
}

/// `photo.jpg` → `photo (2).jpg`, until the name is free.
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or(Path::new(""));
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..).map(|n| parent.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap()
}

/// Validate an offer and compute where each entry will be written.
pub fn plan_destinations(entries: &[FileEntry], dest: &Path, policy: ConflictPolicy) -> anyhow::Result<Vec<Option<PathBuf>>> {
    let mut top_map: std::collections::HashMap<String, Option<PathBuf>> = std::collections::HashMap::new();
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let rel = sanitize_relative(&e.path).ok_or_else(|| anyhow!("unsafe path in transfer: {}", e.path))?;
        let mut comps = rel.components();
        let top = comps.next().unwrap().as_os_str().to_string_lossy().into_owned();
        let rest: PathBuf = comps.collect();
        let top_dest = top_map
            .entry(top.clone())
            .or_insert_with(|| {
                let p = dest.join(&top);
                match policy {
                    ConflictPolicy::Rename => Some(unique_path(&p)),
                    ConflictPolicy::Overwrite => Some(p),
                    ConflictPolicy::Skip => (!p.exists()).then_some(p),
                }
            })
            .clone();
        out.push(top_dest.map(|t| if rest.as_os_str().is_empty() { t } else { t.join(rest) }));
    }
    Ok(out)
}

/// Rate limiter for the optional bandwidth cap.
struct Throttle {
    bytes_per_sec: f64,
    started: Instant,
    sent: f64,
}

impl Throttle {
    async fn wait(&mut self, n: usize) {
        if self.bytes_per_sec <= 0.0 {
            return;
        }
        self.sent += n as f64;
        let due = Duration::from_secs_f64(self.sent / self.bytes_per_sec);
        let elapsed = self.started.elapsed();
        if due > elapsed {
            tokio::time::sleep(due - elapsed).await;
        }
    }
}

/// Stream every file of a transfer to the peer. Chunks wait for room on the connection.
pub async fn send_files(
    id: u64,
    entries: Vec<FileEntry>,
    sources: Vec<PathBuf>,
    out: mpsc::Sender<Msg>,
    cancel: Arc<AtomicBool>,
    limit_mbps: u32,
    events: mpsc::UnboundedSender<TransferEvent>,
) {
    let result: anyhow::Result<()> = async {
        let mut done = 0u64;
        let mut last_report = Instant::now();
        let mut throttle = Throttle { bytes_per_sec: limit_mbps as f64 * 1_000_000.0, started: Instant::now(), sent: 0.0 };
        let mut buf = vec![0u8; CHUNK];
        for (i, (entry, src)) in entries.iter().zip(&sources).enumerate() {
            if entry.dir {
                continue;
            }
            let mut f = tokio::fs::File::open(src).await.with_context(|| format!("can't open {}", src.display()))?;
            let mut remaining = entry.size;
            let _ = events.send(TransferEvent::Progress { id, done, current: Some(entry.path.clone()) });
            while remaining > 0 {
                if cancel.load(Ordering::Relaxed) {
                    return Err(anyhow!("cancelled"));
                }
                let want = (remaining as usize).min(CHUNK);
                let n = f.read(&mut buf[..want]).await?;
                if n == 0 {
                    bail!("{} changed while sending", entry.path);
                }
                remaining -= n as u64;
                done += n as u64;
                throttle.wait(n).await;
                out.send(Msg::FileChunk { id, file: i as u32, data: buf[..n].to_vec() })
                    .await
                    .map_err(|_| anyhow!("connection lost"))?;
                if last_report.elapsed() > Duration::from_millis(100) {
                    last_report = Instant::now();
                    let _ = events.send(TransferEvent::Progress { id, done, current: Some(entry.path.clone()) });
                }
            }
        }
        out.send(Msg::FileDone { id }).await.map_err(|_| anyhow!("connection lost"))?;
        let _ = events.send(TransferEvent::Progress { id, done, current: None });
        Ok(())
    }
    .await;
    let _ = events.send(match result {
        Ok(()) => TransferEvent::Finished { id, saved: vec![] },
        Err(e) if cancel.load(Ordering::Relaxed) => {
            let _ = e;
            TransferEvent::Cancelled { id }
        }
        Err(e) => TransferEvent::Failed { id, error: e.to_string() },
    });
}

/// What the engine feeds into a receiving task.
#[derive(Debug)]
pub enum Incoming {
    Chunk { file: u32, data: Vec<u8> },
    Done,
    Cancel,
}

pub struct ReceivePlan {
    pub id: u64,
    pub entries: Vec<FileEntry>,
    pub destinations: Vec<Option<PathBuf>>,
    pub preserve_times: bool,
}

/// Write a transfer to disk. Returns the top-level paths that were created.
pub async fn receive_files(plan: ReceivePlan, mut rx: mpsc::UnboundedReceiver<Incoming>, events: mpsc::UnboundedSender<TransferEvent>) {
    let id = plan.id;
    let mut parts: Vec<PathBuf> = Vec::new();
    let result: anyhow::Result<Vec<PathBuf>> = async {
        let total: u64 = plan.entries.iter().map(|e| e.size).sum();
        for (e, d) in plan.entries.iter().zip(&plan.destinations) {
            if let (true, Some(d)) = (e.dir, d) {
                tokio::fs::create_dir_all(d).await?;
            }
        }
        let mut current: Option<(u32, tokio::fs::File, u64)> = None;
        let mut done = 0u64;
        let mut last_report = Instant::now();
        let mut completed: Vec<u32> = Vec::new();

        async fn finish(plan: &ReceivePlan, idx: u32, file: tokio::fs::File, written: u64) -> anyhow::Result<()> {
            let entry = &plan.entries[idx as usize];
            file.sync_all().await.ok();
            drop(file);
            if written != entry.size {
                bail!("{} arrived incomplete", entry.path);
            }
            if let Some(dest) = &plan.destinations[idx as usize] {
                let part = part_path(dest);
                let _ = tokio::fs::remove_file(dest).await;
                tokio::fs::rename(&part, dest).await?;
                if plan.preserve_times && entry.mtime > 0 {
                    let t = filetime::FileTime::from_unix_time(entry.mtime, 0);
                    let _ = filetime::set_file_mtime(dest, t);
                }
            }
            Ok(())
        }

        loop {
            match rx.recv().await {
                Some(Incoming::Chunk { file, data }) => {
                    let idx = file as usize;
                    let entry = plan.entries.get(idx).ok_or_else(|| anyhow!("bad file index"))?;
                    if current.as_ref().map(|c| c.0) != Some(file) {
                        if let Some((pidx, pf, written)) = current.take() {
                            finish(&plan, pidx, pf, written).await?;
                            completed.push(pidx);
                        }
                        let f = match &plan.destinations[idx] {
                            Some(dest) => {
                                if let Some(parent) = dest.parent() {
                                    tokio::fs::create_dir_all(parent).await?;
                                }
                                let part = part_path(dest);
                                parts.push(part.clone());
                                tokio::fs::File::create(&part).await?
                            }
                            // Skipped by the conflict policy: swallow the data.
                            None => tokio::fs::File::create(null_sink()).await?,
                        };
                        current = Some((file, f, 0));
                    }
                    let (_, f, written) = current.as_mut().unwrap();
                    *written += data.len() as u64;
                    if *written > entry.size {
                        bail!("peer sent more data than announced");
                    }
                    if plan.destinations[idx].is_some() {
                        f.write_all(&data).await?;
                    }
                    done += data.len() as u64;
                    if done > total {
                        bail!("peer sent more data than announced");
                    }
                    if last_report.elapsed() > Duration::from_millis(100) {
                        last_report = Instant::now();
                        let _ = events.send(TransferEvent::Progress { id, done, current: Some(entry.path.clone()) });
                    }
                }
                Some(Incoming::Done) => {
                    if let Some((pidx, pf, written)) = current.take() {
                        finish(&plan, pidx, pf, written).await?;
                        completed.push(pidx);
                    }
                    // Zero-byte files never get a chunk: create them now.
                    for (i, e) in plan.entries.iter().enumerate() {
                        if !e.dir && e.size == 0 && !completed.contains(&(i as u32)) {
                            if let Some(dest) = &plan.destinations[i] {
                                if let Some(parent) = dest.parent() {
                                    tokio::fs::create_dir_all(parent).await?;
                                }
                                tokio::fs::File::create(dest).await?;
                            }
                        }
                    }
                    let _ = events.send(TransferEvent::Progress { id, done, current: None });
                    let mut tops: Vec<PathBuf> = Vec::new();
                    for (e, d) in plan.entries.iter().zip(&plan.destinations) {
                        if let Some(d) = d {
                            let depth = e.path.split('/').filter(|s| !s.is_empty()).count();
                            let top = d.ancestors().nth(depth.saturating_sub(1)).unwrap_or(d).to_path_buf();
                            if !tops.contains(&top) {
                                tops.push(top);
                            }
                        }
                    }
                    return Ok(tops);
                }
                Some(Incoming::Cancel) | None => return Err(anyhow!("cancelled")),
            }
        }
    }
    .await;
    match result {
        Ok(saved) => {
            let _ = events.send(TransferEvent::Finished { id, saved });
        }
        Err(e) => {
            for p in parts {
                let _ = tokio::fs::remove_file(p).await;
            }
            let msg = e.to_string();
            let _ = events.send(if msg == "cancelled" { TransferEvent::Cancelled { id } } else { TransferEvent::Failed { id, error: msg } });
        }
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_os_string();
    s.push(PART_SUFFIX);
    PathBuf::from(s)
}

fn null_sink() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

/// Remove clipboard file caches older than a day.
pub fn clean_cache(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().unwrap_or_default() > Duration::from_secs(24 * 3600))
            .unwrap_or(false);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_paths_are_neutralised() {
        assert_eq!(sanitize_relative("../../etc/passwd"), None);
        assert_eq!(sanitize_relative("a/../../b"), None);
        assert_eq!(sanitize_relative("/abs/file"), Some(PathBuf::from("abs").join("file")));
        assert_eq!(sanitize_relative("C:\\Windows\\x"), Some(PathBuf::from("C_").join("Windows").join("x")));
        assert_eq!(sanitize_relative("con.txt"), Some(PathBuf::from("_con.txt")));
        assert_eq!(sanitize_relative("ok/na:me?.txt"), Some(PathBuf::from("ok").join("na_me_.txt")));
        assert_eq!(sanitize_relative("trailing. "), Some(PathBuf::from("trailing")));
        assert_eq!(sanitize_relative(""), None);
    }

    #[test]
    fn unique_names() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("photo.jpg");
        assert_eq!(unique_path(&p), p);
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(unique_path(&p), dir.path().join("photo (2).jpg"));
    }

    #[tokio::test]
    async fn send_and_receive_a_folder() {
        let src = tempfile::tempdir().unwrap();
        let root = src.path().join("Album");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("empty")).unwrap();
        std::fs::write(root.join("a.txt"), vec![b'a'; CHUNK * 2 + 17]).unwrap();
        std::fs::write(root.join("sub/b.bin"), b"hello").unwrap();
        std::fs::write(root.join("zero"), b"").unwrap();
        let single = src.path().join("note.md");
        std::fs::write(&single, b"# hi").unwrap();

        let (entries, sources) = build_manifest(&[root.clone(), single.clone()]).unwrap();
        assert!(entries.iter().any(|e| e.path == "Album/sub/b.bin" && e.size == 5));
        assert!(entries.iter().any(|e| e.path == "Album/empty" && e.dir));

        let dest = tempfile::tempdir().unwrap();
        std::fs::write(dest.path().join("note.md"), b"existing").unwrap();
        let destinations = plan_destinations(&entries, dest.path(), ConflictPolicy::Rename).unwrap();

        let (out_tx, mut out_rx) = mpsc::channel(4);
        let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
        let (in_tx, in_rx) = mpsc::unbounded_channel();
        tokio::spawn(send_files(1, entries.clone(), sources, out_tx, Arc::new(AtomicBool::new(false)), 0, ev_tx.clone()));
        let plan = ReceivePlan { id: 2, entries, destinations, preserve_times: true };
        tokio::spawn(receive_files(plan, in_rx, ev_tx));
        while let Some(m) = out_rx.recv().await {
            match m {
                Msg::FileChunk { file, data, .. } => in_tx.send(Incoming::Chunk { file, data }).unwrap(),
                Msg::FileDone { .. } => in_tx.send(Incoming::Done).unwrap(),
                _ => {}
            }
        }
        let mut finished = 0;
        while finished < 2 {
            match ev_rx.recv().await.unwrap() {
                TransferEvent::Finished { id: 2, saved } => {
                    assert_eq!(saved, vec![dest.path().join("Album"), dest.path().join("note (2).md")]);
                    finished += 1;
                }
                TransferEvent::Finished { .. } => finished += 1,
                TransferEvent::Failed { error, .. } => panic!("{error}"),
                _ => {}
            }
        }
        assert_eq!(std::fs::read(dest.path().join("Album/a.txt")).unwrap().len(), CHUNK * 2 + 17);
        assert_eq!(std::fs::read(dest.path().join("Album/sub/b.bin")).unwrap(), b"hello");
        assert!(dest.path().join("Album/zero").exists());
        assert!(dest.path().join("Album/empty").is_dir());
        assert_eq!(std::fs::read(dest.path().join("note (2).md")).unwrap(), b"# hi");
        assert_eq!(std::fs::read(dest.path().join("note.md")).unwrap(), b"existing");
    }
}
