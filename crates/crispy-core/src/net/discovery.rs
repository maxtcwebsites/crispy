//! Zero-configuration discovery on the local network.
//!
//! Every device periodically broadcasts a small beacon on each network interface and answers the
//! beacons it hears with a direct reply, so discovery also works where broadcasts only flow one
//! way. Devices added by address are beaconed directly, which covers networks that block
//! broadcasts altogether. Beacons are only hints: trust always comes from the encrypted handshake.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, watch, Notify};

use crate::types::{DeviceId, Os};

pub const MAGIC: &str = "crispy";
const INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Beacon {
    pub magic: String,
    pub v: u32,
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// TCP port the device accepts sessions on.
    pub port: u16,
    /// Base64 public key; the id must be derived from it, so beacons can't claim another's id.
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub reply: bool,
}

#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    pub beacon: Beacon,
    pub broadcast: bool,
    /// host or host:port entries from settings.
    pub manual: Vec<String>,
    pub port: u16,
}

pub fn bind_udp(port: u16) -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    sock.set_reuse_address(true)?;
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
    let _ = sock.set_reuse_port(true);
    sock.set_broadcast(true)?;
    sock.set_nonblocking(true)?;
    sock.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).into())?;
    UdpSocket::from_std(sock.into())
}

/// Directed broadcast address of every IPv4 interface, plus the limited broadcast address.
fn broadcast_targets() -> Vec<Ipv4Addr> {
    let mut out = vec![Ipv4Addr::BROADCAST];
    if let Ok(ifs) = if_addrs::get_if_addrs() {
        for i in ifs {
            if let if_addrs::IfAddr::V4(v4) = i.addr {
                if v4.ip.is_loopback() {
                    continue;
                }
                let b = v4.broadcast.unwrap_or_else(|| {
                    Ipv4Addr::from(u32::from(v4.ip) | !u32::from(v4.netmask))
                });
                if !out.contains(&b) {
                    out.push(b);
                }
            }
        }
    }
    out
}

/// Local IPv4 addresses, shown in the UI so the user can add this device by address elsewhere.
pub fn local_addresses() -> Vec<Ipv4Addr> {
    let mut out: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| match i.addr {
            if_addrs::IfAddr::V4(v4) if !v4.ip.is_loopback() && !v4.ip.is_link_local() => Some(v4.ip),
            _ => None,
        })
        .collect();
    // Private LAN addresses first: they are the ones that matter for Crispy.
    out.sort_by_key(|ip| (!ip.is_private(), u32::from(*ip)));
    out.dedup();
    out
}

pub async fn resolve(entry: &str, default_port: u16) -> Option<SocketAddr> {
    let entry = entry.trim();
    if entry.is_empty() {
        return None;
    }
    if let Ok(sa) = entry.parse::<SocketAddr>() {
        return Some(sa);
    }
    if let Ok(ip) = entry.parse::<IpAddr>() {
        return Some(SocketAddr::new(ip, default_port));
    }
    let with_port = if entry.contains(':') { entry.to_string() } else { format!("{entry}:{default_port}") };
    tokio::net::lookup_host(with_port).await.ok()?.find(|a| a.is_ipv4())
}

/// Run discovery until the socket fails. Heard beacons are sent to `heard` with their source.
pub async fn run(
    socket: Arc<UdpSocket>,
    config: watch::Receiver<DiscoveryConfig>,
    poke: Arc<Notify>,
    heard: mpsc::UnboundedSender<(Beacon, SocketAddr)>,
) {
    let sender = {
        let socket = socket.clone();
        let mut config = config.clone();
        async move {
            loop {
                let cfg = config.borrow_and_update().clone();
                let payload = serde_json::to_vec(&cfg.beacon).unwrap_or_default();
                if cfg.broadcast {
                    for ip in broadcast_targets() {
                        let _ = socket.send_to(&payload, SocketAddr::new(IpAddr::V4(ip), cfg.port)).await;
                    }
                }
                for entry in &cfg.manual {
                    if let Some(addr) = resolve(entry, cfg.port).await {
                        let _ = socket.send_to(&payload, addr).await;
                    }
                }
                tokio::select! {
                    _ = tokio::time::sleep(INTERVAL) => {}
                    _ = poke.notified() => {}
                    r = config.changed() => if r.is_err() { return; },
                }
            }
        }
    };

    let receiver = async move {
        let mut buf = vec![0u8; 2048];
        let mut replied: HashMap<SocketAddr, Instant> = HashMap::new();
        loop {
            let (n, src) = match socket.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    // Windows reports ICMP "port unreachable" from earlier sends as a recv error.
                    tracing::trace!("discovery recv: {e}");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            };
            let Ok(beacon) = serde_json::from_slice::<Beacon>(&buf[..n]) else { continue };
            let cfg = config.borrow().clone();
            if beacon.magic != MAGIC || beacon.id == cfg.beacon.id {
                continue;
            }
            if !beacon.reply {
                let due = replied.get(&src).is_none_or(|t| t.elapsed() > Duration::from_secs(3));
                if due {
                    replied.insert(src, Instant::now());
                    let mut me = cfg.beacon.clone();
                    me.reply = true;
                    let _ = socket.send_to(&serde_json::to_vec(&me).unwrap_or_default(), src).await;
                }
                if replied.len() > 256 {
                    replied.retain(|_, t| t.elapsed() < Duration::from_secs(30));
                }
            }
            if heard.send((beacon, src)).is_err() {
                return;
            }
        }
    };

    tokio::join!(sender, receiver);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beacon(id: &str, port: u16) -> Beacon {
        Beacon { magic: MAGIC.into(), v: 1, id: id.into(), name: id.into(), os: Os::Linux, port, key: String::new(), reply: false }
    }

    #[tokio::test]
    async fn two_devices_find_each_other_by_address() {
        let sa = Arc::new(bind_udp(0).unwrap());
        let sb = Arc::new(bind_udp(0).unwrap());
        let pa = sa.local_addr().unwrap().port();
        let pb = sb.local_addr().unwrap().port();
        let (_ta, ra) = watch::channel(DiscoveryConfig {
            beacon: beacon("a", 1111),
            broadcast: false,
            manual: vec![format!("127.0.0.1:{pb}")],
            port: pa,
        });
        let (_tb, rb) = watch::channel(DiscoveryConfig { beacon: beacon("b", 2222), broadcast: false, manual: vec![], port: pb });
        let (ha, mut heard_a) = mpsc::unbounded_channel();
        let (hb, mut heard_b) = mpsc::unbounded_channel();
        tokio::spawn(run(sa, ra, Arc::new(Notify::new()), ha));
        tokio::spawn(run(sb, rb, Arc::new(Notify::new()), hb));
        let (b_heard, _) = tokio::time::timeout(Duration::from_secs(5), heard_b.recv()).await.unwrap().unwrap();
        assert_eq!((b_heard.id.as_str(), b_heard.port), ("a", 1111));
        // b answers directly, so a learns about b even though b never beacons a.
        let (a_heard, _) = tokio::time::timeout(Duration::from_secs(5), heard_a.recv()).await.unwrap().unwrap();
        assert_eq!((a_heard.id.as_str(), a_heard.reply), ("b", true));
    }

    #[tokio::test]
    async fn resolves_entries() {
        assert_eq!(resolve("10.0.0.5", 24727).await, Some("10.0.0.5:24727".parse().unwrap()));
        assert_eq!(resolve("10.0.0.5:99", 24727).await, Some("10.0.0.5:99".parse().unwrap()));
        assert_eq!(resolve("  ", 1).await, None);
    }
}
