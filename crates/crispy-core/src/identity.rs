//! Device identity (a long-term X25519 key) and the list of devices the user has paired with.
//!
//! There is no account and no server: a device *is* its key. Pairing stores the other device's
//! public key, and every later connection proves possession of it through the Noise handshake.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{load_json, save_json};
use crate::types::{now_ms, DeviceId, Os, Screen};

pub const NOISE_PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

#[derive(Clone)]
pub struct Identity {
    pub private: Vec<u8>,
    pub public: Vec<u8>,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity").field("id", &self.id()).finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct StoredIdentity {
    private: String,
    public: String,
}

impl Identity {
    pub fn generate() -> Identity {
        let kp = snow::Builder::new(NOISE_PARAMS.parse().expect("valid noise params"))
            .generate_keypair()
            .expect("keypair generation");
        Identity { private: kp.private, public: kp.public }
    }

    pub fn load_or_create(path: &Path) -> anyhow::Result<Identity> {
        if let Ok(bytes) = std::fs::read(path) {
            let stored: StoredIdentity = serde_json::from_slice(&bytes).context("identity file is corrupt")?;
            let private = B64.decode(stored.private)?;
            let public = B64.decode(stored.public)?;
            if private.len() != 32 || public.len() != 32 {
                return Err(anyhow!("identity file has wrong key sizes"));
            }
            return Ok(Identity { private, public });
        }
        let id = Identity::generate();
        id.save(path)?;
        Ok(id)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        save_json(path, &StoredIdentity { private: B64.encode(&self.private), public: B64.encode(&self.public) })?;
        restrict_permissions(path);
        Ok(())
    }

    pub fn id(&self) -> DeviceId {
        device_id(&self.public)
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(&self.public)
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

/// Stable short identifier derived from the public key.
pub fn device_id(public: &[u8]) -> DeviceId {
    let digest = Sha256::new().chain_update(b"crispy-device-id").chain_update(public).finalize();
    hex::encode(&digest[..8])
}

/// Human-comparable fingerprint, e.g. `7F3A-91C2-0B44-E81D`.
pub fn fingerprint(public: &[u8]) -> String {
    let digest = Sha256::digest(public);
    let hex = hex::encode_upper(&digest[..8]);
    hex.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("-")
}

/// A device the user explicitly paired with.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrustedPeer {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub public_key: String,
    pub last_address: Option<String>,
    pub enabled: bool,
    pub paired_at: u64,
    pub version: Option<String>,
    /// Remembered so the arrangement can still be drawn while the device is offline.
    pub screens: Vec<Screen>,
}

impl Default for TrustedPeer {
    fn default() -> Self {
        TrustedPeer {
            id: String::new(),
            name: String::new(),
            os: Os::Windows,
            public_key: String::new(),
            last_address: None,
            enabled: true,
            paired_at: 0,
            version: None,
            screens: vec![],
        }
    }
}

impl TrustedPeer {
    pub fn new(public: &[u8], name: String, os: Os) -> TrustedPeer {
        TrustedPeer {
            id: device_id(public),
            name,
            os,
            public_key: B64.encode(public),
            paired_at: now_ms(),
            ..Default::default()
        }
    }

    pub fn public_bytes(&self) -> Vec<u8> {
        B64.decode(&self.public_key).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TrustStore {
    pub peers: BTreeMap<DeviceId, TrustedPeer>,
}

impl TrustStore {
    pub fn load(path: &Path) -> TrustStore {
        let mut store: TrustStore = load_json(path);
        // Never trust an entry whose id does not match its key.
        store.peers.retain(|id, p| device_id(&p.public_bytes()) == *id && p.id == *id);
        store
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        save_json(path, self)?;
        restrict_permissions(path);
        Ok(())
    }

    /// Is this exact public key trusted?
    pub fn is_trusted(&self, public: &[u8]) -> bool {
        self.peers.get(&device_id(public)).is_some_and(|p| p.public_bytes() == public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_persists_and_derives_stable_ids() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("identity.json");
        let a = Identity::load_or_create(&p).unwrap();
        let b = Identity::load_or_create(&p).unwrap();
        assert_eq!(a.public, b.public);
        assert_eq!(a.id().len(), 16);
        assert_eq!(a.fingerprint().len(), 19);
        assert_ne!(a.id(), Identity::generate().id());
    }

    #[test]
    fn trust_store_rejects_tampered_entries() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("peers.json");
        let other = Identity::generate();
        let mut store = TrustStore::default();
        let peer = TrustedPeer::new(&other.public, "Studio PC".into(), Os::Windows);
        store.peers.insert(peer.id.clone(), peer.clone());
        let mut forged = TrustedPeer::new(&Identity::generate().public, "Evil".into(), Os::Linux);
        forged.id = "0000000000000000".into();
        store.peers.insert(forged.id.clone(), forged);
        store.save(&p).unwrap();
        let loaded = TrustStore::load(&p);
        assert_eq!(loaded.peers.len(), 1);
        assert!(loaded.is_trusted(&other.public));
        assert!(!loaded.is_trusted(&Identity::generate().public));
    }
}
