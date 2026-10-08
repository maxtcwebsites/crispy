//! Encrypted, authenticated sessions between two devices.
//!
//! Each TCP connection runs a Noise XX handshake (X25519, ChaCha20-Poly1305, BLAKE2s). Both sides
//! learn the other's static key; the engine then decides whether that key is trusted (paired) or
//! whether this connection is a pairing attempt.
//!
//! After the handshake a reader and a writer task move messages. The writer always prefers
//! latency-sensitive input events over bulk data, so a large file transfer never makes the mouse
//! lag: bulk messages are cut into fragments and input frames are interleaved between them.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use snow::{HandshakeState, StatelessTransportState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::identity::{Identity, NOISE_PARAMS};
use crate::protocol::{decode, encode, Msg};

const NOISE_MAX: usize = 65535;
const TAG: usize = 16;
/// Largest plaintext per frame: Noise limit minus the AEAD tag minus our 1-byte frame kind.
const PLAIN_MAX: usize = NOISE_MAX - TAG - 1;
const PROLOGUE: &[u8] = b"crispy/1";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

const KIND_BATCH: u8 = 1;
const KIND_FRAGMENT: u8 = 2;
const KIND_FRAGMENT_END: u8 = 3;

/// Traffic counters shared by every session, for the stats panel.
#[derive(Default, Debug)]
pub struct Traffic {
    pub sent: AtomicU64,
    pub received: AtomicU64,
}

/// A connection that finished its handshake but is not running yet.
pub struct Handshaked {
    stream: TcpStream,
    transport: StatelessTransportState,
    pub remote_static: Vec<u8>,
    pub handshake_hash: Vec<u8>,
    pub initiator: bool,
    pub addr: SocketAddr,
}

impl std::fmt::Debug for Handshaked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handshaked").field("addr", &self.addr).field("initiator", &self.initiator).finish()
    }
}

async fn write_raw(w: &mut (impl AsyncWriteExt + Unpin), bytes: &[u8]) -> std::io::Result<()> {
    let mut frame = Vec::with_capacity(bytes.len() + 2);
    frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    frame.extend_from_slice(bytes);
    w.write_all(&frame).await
}

async fn read_raw(r: &mut (impl AsyncReadExt + Unpin), buf: &mut Vec<u8>) -> std::io::Result<usize> {
    let mut len = [0u8; 2];
    r.read_exact(&mut len).await?;
    let len = u16::from_be_bytes(len) as usize;
    buf.resize(len, 0);
    r.read_exact(&mut buf[..len]).await?;
    Ok(len)
}

pub async fn handshake(mut stream: TcpStream, identity: &Identity, initiator: bool) -> anyhow::Result<Handshaked> {
    let addr = stream.peer_addr()?;
    stream.set_nodelay(true)?;
    let builder = snow::Builder::new(NOISE_PARAMS.parse()?).local_private_key(&identity.private).prologue(PROLOGUE);
    let mut hs: HandshakeState = if initiator { builder.build_initiator()? } else { builder.build_responder()? };

    let run = async {
        let mut out = vec![0u8; NOISE_MAX];
        let mut inbuf = Vec::with_capacity(NOISE_MAX);
        let mut payload = vec![0u8; NOISE_MAX];
        // XX: -> e ; <- e, ee, s, es ; -> s, se
        for step in 0..3 {
            let our_turn = (step % 2 == 0) == initiator;
            if our_turn {
                let n = hs.write_message(&[], &mut out)?;
                write_raw(&mut stream, &out[..n]).await?;
            } else {
                let n = read_raw(&mut stream, &mut inbuf).await?;
                hs.read_message(&inbuf[..n], &mut payload)?;
            }
        }
        anyhow::Ok(())
    };
    tokio::time::timeout(HANDSHAKE_TIMEOUT, run).await.map_err(|_| anyhow!("handshake timed out"))??;
    if !hs.is_handshake_finished() {
        bail!("handshake incomplete");
    }
    let remote_static = hs.get_remote_static().context("peer sent no static key")?.to_vec();
    let handshake_hash = hs.get_handshake_hash().to_vec();
    let transport = hs.into_stateless_transport_mode()?;
    Ok(Handshaked { stream, transport, remote_static, handshake_hash, initiator, addr })
}

pub async fn connect(addr: SocketAddr, identity: &Identity) -> anyhow::Result<Handshaked> {
    let stream = tokio::time::timeout(Duration::from_secs(4), TcpStream::connect(addr))
        .await
        .map_err(|_| anyhow!("timed out connecting to {addr}"))??;
    configure_keepalive(&stream);
    handshake(stream, identity, true).await
}

pub fn configure_keepalive(stream: &TcpStream) {
    let sock = socket2::SockRef::from(stream);
    let ka = socket2::TcpKeepalive::new().with_time(Duration::from_secs(10)).with_interval(Duration::from_secs(5));
    let _ = sock.set_tcp_keepalive(&ka);
}

/// What a running session reports back to the engine.
#[derive(Debug)]
pub enum SessionEvent {
    Message { conn: u64, msg: Msg },
    Closed { conn: u64, reason: String },
}

/// The engine's handle to a running session. Dropping it closes the connection.
pub struct Session {
    pub conn: u64,
    pub remote_static: Vec<u8>,
    pub handshake_hash: Vec<u8>,
    pub initiator: bool,
    pub addr: SocketAddr,
    hi: mpsc::UnboundedSender<Msg>,
    lo: mpsc::Sender<Msg>,
    tasks: [JoinHandle<()>; 2],
}

impl Drop for Session {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}

impl Session {
    pub fn start(
        h: Handshaked,
        conn: u64,
        idle_timeout: Duration,
        events: mpsc::UnboundedSender<SessionEvent>,
        traffic: Arc<Traffic>,
    ) -> Session {
        let Handshaked { stream, transport, remote_static, handshake_hash, initiator, addr } = h;
        let transport = Arc::new(transport);
        let (r, w) = stream.into_split();
        let (hi_tx, hi_rx) = mpsc::unbounded_channel();
        let (lo_tx, lo_rx) = mpsc::channel(32);

        let reader = {
            let transport = transport.clone();
            let events = events.clone();
            let traffic = traffic.clone();
            tokio::spawn(async move {
                let reason = match read_loop(r, &transport, conn, idle_timeout, &events, &traffic).await {
                    Ok(()) => "connection closed".to_string(),
                    Err(e) => e.to_string(),
                };
                let _ = events.send(SessionEvent::Closed { conn, reason });
            })
        };
        let writer = tokio::spawn(async move {
            if let Err(e) = write_loop(w, &transport, hi_rx, lo_rx, &traffic).await {
                let _ = events.send(SessionEvent::Closed { conn, reason: e.to_string() });
            }
        });
        Session { conn, remote_static, handshake_hash, initiator, addr, hi: hi_tx, lo: lo_tx, tasks: [reader, writer] }
    }

    /// Queue a message. Bulk messages may be dropped if the peer is hopelessly behind; use
    /// [`Session::send_bulk`] when you need back-pressure instead.
    pub fn send(&self, msg: Msg) {
        if msg.is_bulk() {
            if let Err(e) = self.lo.try_send(msg) {
                tracing::debug!("bulk queue full, dropping message: {e}");
            }
        } else {
            let _ = self.hi.send(msg);
        }
    }

    /// A sender for bulk data that waits while the connection is busy (back-pressure).
    pub fn bulk_sender(&self) -> mpsc::Sender<Msg> {
        self.lo.clone()
    }
}

async fn read_loop(
    mut r: OwnedReadHalf,
    transport: &StatelessTransportState,
    conn: u64,
    idle_timeout: Duration,
    events: &mpsc::UnboundedSender<SessionEvent>,
    traffic: &Traffic,
) -> anyhow::Result<()> {
    let mut nonce = 0u64;
    let mut cipher = Vec::with_capacity(NOISE_MAX);
    let mut plain = vec![0u8; NOISE_MAX];
    let mut fragments: Vec<u8> = Vec::new();
    loop {
        let n = match tokio::time::timeout(idle_timeout, read_raw(&mut r, &mut cipher)).await {
            Err(_) => bail!("peer stopped responding"),
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Ok(r) => r?,
        };
        traffic.received.fetch_add(n as u64 + 2, Ordering::Relaxed);
        let len = transport.read_message(nonce, &cipher[..n], &mut plain).map_err(|_| anyhow!("decryption failed"))?;
        nonce += 1;
        let Some((&kind, body)) = plain[..len].split_first() else { bail!("empty frame") };
        match kind {
            KIND_BATCH => {
                let mut rest = body;
                while rest.len() >= 2 {
                    let l = u16::from_be_bytes([rest[0], rest[1]]) as usize;
                    if rest.len() < 2 + l {
                        bail!("truncated batch");
                    }
                    let msg = decode(&rest[2..2 + l])?;
                    rest = &rest[2 + l..];
                    if events.send(SessionEvent::Message { conn, msg }).is_err() {
                        return Ok(());
                    }
                }
            }
            KIND_FRAGMENT | KIND_FRAGMENT_END => {
                if fragments.len() + body.len() > crate::protocol::MAX_MESSAGE as usize {
                    bail!("message too large");
                }
                fragments.extend_from_slice(body);
                if kind == KIND_FRAGMENT_END {
                    let msg = decode(&fragments)?;
                    fragments = Vec::new();
                    if events.send(SessionEvent::Message { conn, msg }).is_err() {
                        return Ok(());
                    }
                }
            }
            other => bail!("unknown frame kind {other}"),
        }
    }
}

struct FrameWriter<'a> {
    w: OwnedWriteHalf,
    transport: &'a StatelessTransportState,
    nonce: u64,
    cipher: Vec<u8>,
    traffic: &'a Traffic,
}

impl FrameWriter<'_> {
    async fn send(&mut self, plain: &[u8]) -> anyhow::Result<()> {
        let n = self.transport.write_message(self.nonce, plain, &mut self.cipher)?;
        self.nonce += 1;
        write_raw(&mut self.w, &self.cipher[..n]).await?;
        self.traffic.sent.fetch_add(n as u64 + 2, Ordering::Relaxed);
        Ok(())
    }
}

/// Drop absolute mouse moves that are immediately superseded by another one.
fn coalesce(batch: &mut Vec<Msg>) {
    let mut i = 0;
    while i + 1 < batch.len() {
        if matches!(batch[i], Msg::MouseMove { .. }) && matches!(batch[i + 1], Msg::MouseMove { .. }) {
            batch.remove(i);
        } else {
            i += 1;
        }
    }
}

async fn write_loop(
    w: OwnedWriteHalf,
    transport: &StatelessTransportState,
    mut hi: mpsc::UnboundedReceiver<Msg>,
    mut lo: mpsc::Receiver<Msg>,
    traffic: &Traffic,
) -> anyhow::Result<()> {
    let mut fw = FrameWriter { w, transport, nonce: 0, cipher: vec![0u8; NOISE_MAX], traffic };
    let mut pending: VecDeque<Vec<u8>> = VecDeque::new();
    let mut batch: Vec<Msg> = Vec::new();
    loop {
        while let Ok(m) = hi.try_recv() {
            batch.push(m);
            if batch.len() >= 512 {
                break;
            }
        }
        if !batch.is_empty() {
            coalesce(&mut batch);
            let mut frame = vec![KIND_BATCH];
            for m in batch.drain(..) {
                let bytes = encode(&m);
                if bytes.len() + 3 > PLAIN_MAX {
                    // Unusually large message on the fast path: send it fragmented, right now.
                    for f in fragment(&bytes) {
                        fw.send(&f).await?;
                    }
                    continue;
                }
                if frame.len() + 2 + bytes.len() > PLAIN_MAX {
                    fw.send(&frame).await?;
                    frame.truncate(1);
                }
                frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
                frame.extend_from_slice(&bytes);
            }
            if frame.len() > 1 {
                fw.send(&frame).await?;
            }
            continue;
        }
        if let Some(f) = pending.pop_front() {
            fw.send(&f).await?;
            continue;
        }
        tokio::select! {
            biased;
            m = hi.recv() => match m {
                Some(m) => batch.push(m),
                None => return Ok(()),
            },
            m = lo.recv() => match m {
                Some(m) => {
                    let bytes = encode(&m);
                    if bytes.len() + 3 <= PLAIN_MAX {
                        let mut frame = Vec::with_capacity(bytes.len() + 3);
                        frame.push(KIND_BATCH);
                        frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
                        frame.extend_from_slice(&bytes);
                        pending.push_back(frame);
                    } else {
                        pending.extend(fragment(&bytes));
                    }
                }
                None => return Ok(()),
            },
        }
    }
}

fn fragment(bytes: &[u8]) -> Vec<Vec<u8>> {
    let chunks: Vec<&[u8]> = bytes.chunks(PLAIN_MAX).collect();
    let last = chunks.len() - 1;
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let mut f = Vec::with_capacity(c.len() + 1);
            f.push(if i == last { KIND_FRAGMENT_END } else { KIND_FRAGMENT });
            f.extend_from_slice(c);
            f
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClipItem, ClipboardData};
    use tokio::net::TcpListener;

    async fn pair_of_sessions() -> (Session, Session, mpsc::UnboundedReceiver<SessionEvent>, mpsc::UnboundedReceiver<SessionEvent>, Identity, Identity) {
        let a = Identity::generate();
        let b = Identity::generate();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let b2 = b.clone();
        let accept = tokio::spawn(async move {
            let (s, _) = listener.accept().await.unwrap();
            handshake(s, &b2, false).await.unwrap()
        });
        let ha = connect(addr, &a).await.unwrap();
        let hb = accept.await.unwrap();
        assert_eq!(ha.remote_static, b.public);
        assert_eq!(hb.remote_static, a.public);
        assert_eq!(ha.handshake_hash, hb.handshake_hash);
        let traffic = Arc::new(Traffic::default());
        let (ta, ra) = mpsc::unbounded_channel();
        let (tb, rb) = mpsc::unbounded_channel();
        let sa = Session::start(ha, 1, Duration::from_secs(5), ta, traffic.clone());
        let sb = Session::start(hb, 2, Duration::from_secs(5), tb, traffic);
        (sa, sb, ra, rb, a, b)
    }

    #[tokio::test]
    async fn messages_flow_both_ways_including_huge_ones() {
        let (sa, sb, mut ra, mut rb, _, _) = pair_of_sessions().await;
        let big = ClipboardData { items: vec![ClipItem::Png(vec![7u8; 300_000])] };
        sa.send(Msg::Clipboard(big.clone()));
        sa.send(Msg::Key { hid: 0x07_0004, down: true, repeat: false });
        sb.send(Msg::Ping(42));

        let mut got = vec![];
        while got.len() < 2 {
            match rb.recv().await.unwrap() {
                SessionEvent::Message { msg, .. } => got.push(msg),
                SessionEvent::Closed { reason, .. } => panic!("closed: {reason}"),
            }
        }
        // The key press overtakes the large clipboard payload.
        assert_eq!(got[0], Msg::Key { hid: 0x07_0004, down: true, repeat: false });
        assert_eq!(got[1], Msg::Clipboard(big));
        match ra.recv().await.unwrap() {
            SessionEvent::Message { msg, .. } => assert_eq!(msg, Msg::Ping(42)),
            e => panic!("{e:?}"),
        }
    }

    #[tokio::test]
    async fn closing_one_side_is_noticed() {
        let (sa, _sb, _ra, mut rb, _, _) = pair_of_sessions().await;
        drop(sa);
        loop {
            if let SessionEvent::Closed { conn, .. } = rb.recv().await.unwrap() {
                assert_eq!(conn, 2);
                break;
            }
        }
    }

    #[test]
    fn coalescing_keeps_order_of_clicks() {
        let mut b = vec![
            Msg::MouseMove { x: 1.0, y: 1.0 },
            Msg::MouseMove { x: 2.0, y: 2.0 },
            Msg::Button { button: crate::types::MouseButton::Left, down: true },
            Msg::MouseMove { x: 3.0, y: 3.0 },
            Msg::MouseMove { x: 4.0, y: 4.0 },
        ];
        coalesce(&mut b);
        assert_eq!(b.len(), 3);
        assert_eq!(b[0], Msg::MouseMove { x: 2.0, y: 2.0 });
        assert_eq!(b[2], Msg::MouseMove { x: 4.0, y: 4.0 });
    }
}
