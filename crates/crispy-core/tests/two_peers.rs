//! End to end: two complete engines on one machine, talking over real encrypted TCP sessions,
//! with simulated keyboards, mice, screens and clipboards.

use std::sync::Arc;
use std::time::Duration;

use crispy_core::clipboard::{ClipContent, MemoryClipboard};
use crispy_core::config::Paths;
use crispy_core::engine::{self, Command, EngineHandle, EngineOptions};
use crispy_core::keymap::kb;
use crispy_core::layout::Pos;
use crispy_core::platform::mock::{screen, MockPlatform};
use crispy_core::platform::Inject;
use crispy_core::state::{AppState, FocusView, PairingStage, PeerStatus, TransferState};
use tokio::sync::mpsc;

struct Node {
    handle: EngineHandle,
    platform: Arc<MockPlatform>,
    clip: Arc<MemoryClipboard>,
    _dir: tempfile::TempDir,
    _ui: mpsc::UnboundedReceiver<crispy_core::state::UiEvent>,
}

async fn node(w: f64, h: f64) -> Node {
    let dir = tempfile::tempdir().unwrap();
    let platform = MockPlatform::new(vec![screen(0.0, 0.0, w, h)]);
    let clip = Arc::new(MemoryClipboard::default());
    let (ui_tx, ui_rx) = mpsc::unbounded_channel();
    let handle = engine::start(
        EngineOptions {
            paths: Paths { root: dir.path().to_path_buf() },
            platform: platform.clone(),
            clipboard: clip.clone(),
            version: "test".into(),
            port_override: Some(0),
            discovery: false,
        },
        ui_tx,
    )
    .await
    .unwrap();
    Node { handle, platform, clip, _dir: dir, _ui: ui_rx }
}

async fn wait_for<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    for _ in 0..200 {
        if let Some(v) = f() {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
}

async fn wait_state(n: &Node, what: &str, mut f: impl FnMut(&AppState) -> bool) -> AppState {
    for _ in 0..200 {
        let s = n.handle.state().await.unwrap();
        if f(&s) {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
}

fn injected_until(p: &MockPlatform, seen: &mut Vec<Inject>, want: &Inject) -> Option<()> {
    seen.extend(p.take_injected());
    seen.contains(want).then_some(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_computers_share_everything() {
    let a = node(1440.0, 900.0).await;
    let b = node(1920.0, 1080.0).await;

    // --- Pairing with a six digit code -------------------------------------------------------
    a.handle.send(Command::ConnectAddress(format!("127.0.0.1:{}", b.handle.port)));
    let bs = wait_state(&b, "code on B", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::ShowCode)).await;
    let code = bs.pairing.unwrap().code.unwrap();
    wait_state(&a, "code entry on A", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::EnterCode)).await;

    // A wrong code would fail; the right one pairs both sides.
    a.handle.send(Command::PairSubmit(code));
    let a_state = wait_state(&a, "A connected", |s| s.peers.iter().any(|p| p.paired && p.status == PeerStatus::Connected)).await;
    let b_state = wait_state(&b, "B connected", |s| s.peers.iter().any(|p| p.paired && p.status == PeerStatus::Connected)).await;
    let (a_id, b_id) = (a_state.me.id.clone(), b_state.me.id.clone());
    assert_eq!(a_state.peers[0].id, b_id);
    assert_eq!(b_state.peers[0].id, a_id);
    assert_eq!(a_state.peers[0].screens.len(), 1);

    // --- Arrangement: B to the right of A, shared with B ---------------------------------------
    let mut positions = std::collections::BTreeMap::new();
    positions.insert(a_id.clone(), Pos { x: 0.0, y: 0.0 });
    positions.insert(b_id.clone(), Pos { x: 1440.0, y: 0.0 });
    a.handle.send(Command::SetLayout(positions));
    wait_state(&b, "layout on B", |s| s.layout.positions.get(&b_id) == Some(&Pos { x: 1440.0, y: 0.0 })).await;

    // --- Moving off the right edge of A hands keyboard and mouse to B --------------------------
    a.platform.user_move(2000.0, 0.0);
    wait_state(&a, "A driving B", |s| s.focus == FocusView::Remote { peer_id: b_id.clone() }).await;
    assert!(a.platform.cursor_hidden());
    let mut seen = vec![];
    wait_for("cursor entering B", || injected_until(&b.platform, &mut seen, &Inject::MoveTo { x: 2.0, y: 450.0 })).await;
    wait_state(&b, "B controlled by A", |s| s.peers[0].controlling_me).await;

    a.platform.user_move(100.0, 50.0);
    wait_for("mouse move on B", || injected_until(&b.platform, &mut seen, &Inject::MoveTo { x: 102.0, y: 500.0 })).await;

    assert_eq!(a.platform.user_key(kb(0x04), true), crispy_core::capture::Verdict::Swallow);
    a.platform.user_key(kb(0x04), false);
    wait_for("key on B", || injected_until(&b.platform, &mut seen, &Inject::Key { hid: kb(0x04), down: false, repeat: false })).await;
    assert!(seen.contains(&Inject::Key { hid: kb(0x04), down: true, repeat: false }));

    // --- Coming back across the left edge of B -------------------------------------------------
    a.platform.user_move(-500.0, 0.0);
    wait_state(&a, "A local again", |s| s.focus == FocusView::Local).await;
    assert!(!a.platform.cursor_hidden());
    let (x, y) = crispy_core::platform::Platform::cursor_pos(&*a.platform);
    assert_eq!((x, y), (1440.0 - 1.0 - 2.0, 500.0));
    wait_state(&b, "B released", |s| !s.peers[0].controlling_me).await;

    // --- Clipboard follows instantly -----------------------------------------------------------
    a.clip.user_copy(ClipContent { text: Some("hello from A".into()), ..Default::default() });
    wait_for("clipboard on B", || (b.clip.current().text.as_deref() == Some("hello from A")).then_some(())).await;
    // ...and doesn't bounce back as a "new" copy.
    b.clip.user_copy(ClipContent { text: Some("and back from B".into()), ..Default::default() });
    wait_for("clipboard on A", || (a.clip.current().text.as_deref() == Some("and back from B")).then_some(())).await;

    // --- Sending a file -----------------------------------------------------------------------
    let save = tempfile::tempdir().unwrap();
    let mut settings = b.handle.settings().await.unwrap();
    settings.files.save_dir = save.path().to_string_lossy().into_owned();
    b.handle.set_settings(settings).await.unwrap();
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("potato.txt");
    std::fs::write(&file, vec![b'c'; 200_000]).unwrap();
    a.handle.send(Command::SendFiles(b_id.clone(), vec![file]));
    wait_state(&b, "file received", |s| s.transfers.iter().any(|t| t.state == TransferState::Done)).await;
    wait_state(&a, "file sent", |s| s.transfers.iter().any(|t| t.state == TransferState::Done)).await;
    assert_eq!(std::fs::read(save.path().join("potato.txt")).unwrap().len(), 200_000);

    // --- Unpairing removes the trust on both sides --------------------------------------------
    a.handle.send(Command::Unpair(b_id.clone()));
    wait_state(&b, "B forgot A", |s| s.peers.iter().all(|p| !p.paired)).await;
    wait_state(&a, "A forgot B", |s| s.peers.iter().all(|p| !p.paired)).await;

    a.handle.shutdown().await;
    b.handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wrong_pairing_code_is_rejected() {
    let a = node(1440.0, 900.0).await;
    let b = node(1920.0, 1080.0).await;
    a.handle.send(Command::ConnectAddress(format!("127.0.0.1:{}", b.handle.port)));
    let bs = wait_state(&b, "code on B", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::ShowCode)).await;
    let code = bs.pairing.unwrap().code.unwrap().replace(' ', "");
    let wrong = format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000);
    wait_state(&a, "code entry on A", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::EnterCode)).await;
    a.handle.send(Command::PairSubmit(wrong));
    wait_state(&a, "A failed", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::Failed)).await;
    wait_state(&b, "B failed", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::Failed)).await;
    let a_state = a.handle.state().await.unwrap();
    assert!(a_state.peers.iter().all(|p| !p.paired));
    assert!(b.handle.state().await.unwrap().peers.iter().all(|p| !p.paired));

    // "Try again" works for a device that was added by address (not found by discovery).
    let b_id = b.handle.state().await.unwrap().me.id;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    a.handle.send(Command::PairStart(b_id.clone()));
    let bs = wait_state(&b, "new code on B", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::ShowCode)).await;
    let code = bs.pairing.unwrap().code.unwrap();
    wait_state(&a, "code entry again", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::EnterCode)).await;
    a.handle.send(Command::PairSubmit(code));
    wait_state(&a, "A success", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::Success)).await;
    wait_state(&b, "B success", |s| s.pairing.as_ref().is_some_and(|p| p.stage == PairingStage::Success)).await;

    // Dismissing the success dialog on one side must not turn the other side's into a failure.
    a.handle.send(Command::PairCancel);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let b_pairing = b.handle.state().await.unwrap().pairing;
    assert!(b_pairing.as_ref().is_none_or(|p| p.stage == PairingStage::Success), "{b_pairing:?}");
    wait_state(&b, "still connected", |s| s.peers.iter().any(|p| p.paired && p.status == PeerStatus::Connected)).await;
}
