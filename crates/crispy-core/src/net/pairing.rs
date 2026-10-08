//! Pairing with a 6-digit code, using SPAKE2 (a password-authenticated key exchange).
//!
//! The code is shown on one computer and typed on the other. SPAKE2 turns that low-entropy code
//! into a strong shared key without ever revealing it: an attacker in the middle gets exactly one
//! guess per attempt (1 in a million) and cannot brute-force the code offline. The exchange is
//! bound to the Noise handshake hash, so a successful pairing also proves there is no
//! man-in-the-middle on the encrypted channel, and both static keys can be trusted.

use rand::Rng;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

pub const CODE_LEN: usize = 6;
pub const ROLE_INITIATOR: u8 = b'I';
pub const ROLE_RESPONDER: u8 = b'R';

pub type SpakeState = Spake2<Ed25519Group>;

pub fn generate_code() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{n:06}")
}

/// Keep only digits, so "482 913" and "482-913" both work.
pub fn normalize_code(code: &str) -> Option<String> {
    let digits: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    (digits.len() == CODE_LEN).then_some(digits)
}

pub fn start(code: &str, handshake_hash: &[u8]) -> (SpakeState, Vec<u8>) {
    let id = [b"crispy-pair-v1:".as_slice(), handshake_hash].concat();
    Spake2::<Ed25519Group>::start_symmetric(&Password::new(code.as_bytes()), &Identity::new(&id))
}

pub fn finish(state: SpakeState, their_msg: &[u8]) -> anyhow::Result<Vec<u8>> {
    state.finish(their_msg).map_err(|e| anyhow::anyhow!("pairing exchange failed: {e:?}"))
}

/// Key-confirmation tag: proves knowledge of the SPAKE2 key for one role, bound to this channel.
pub fn confirm_tag(key: &[u8], handshake_hash: &[u8], role: u8) -> Vec<u8> {
    Sha256::new()
        .chain_update(b"crispy-confirm-v1")
        .chain_update([role])
        .chain_update(key)
        .chain_update(handshake_hash)
        .finalize()
        .to_vec()
}

pub fn tags_equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(code_a: &str, code_b: &str) -> bool {
        let hash = b"handshake-hash";
        let (sa, ma) = start(code_a, hash);
        let (sb, mb) = start(code_b, hash);
        let ka = finish(sa, &mb).unwrap();
        let kb = finish(sb, &ma).unwrap();
        tags_equal(&confirm_tag(&ka, hash, ROLE_INITIATOR), &confirm_tag(&kb, hash, ROLE_INITIATOR))
    }

    #[test]
    fn same_code_pairs() {
        assert!(run("482913", "482913"));
    }

    #[test]
    fn wrong_code_fails() {
        assert!(!run("482913", "482914"));
    }

    #[test]
    fn different_channels_fail() {
        let (sa, ma) = start("111111", b"channel-a");
        let (sb, mb) = start("111111", b"channel-b");
        let ka = finish(sa, &mb).unwrap();
        let kb = finish(sb, &ma).unwrap();
        assert_ne!(ka, kb);
    }

    #[test]
    fn codes() {
        let c = generate_code();
        assert_eq!(c.len(), 6);
        assert_eq!(normalize_code(" 482-913 "), Some("482913".into()));
        assert_eq!(normalize_code("12345"), None);
    }
}
