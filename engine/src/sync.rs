//! The stash protocol brain: version rules and integrity. Transport lives
//! in the glue (fetch in the browser); the server stores {envelope,
//! version, sha256} keyed by stash id and enforces the same version rule.

use sha2::{Digest, Sha256};

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// May a device at `local` overwrite a stash at `remote`? Strictly greater
/// wins; equal is a no-op, and a stale phone must PULL first rather than
/// clobber a newer backup blindly (last-write-wins with a seatbelt).
pub fn may_push(local_version: u64, remote_version: Option<u64>) -> bool {
    match remote_version {
        None => true,
        Some(r) => local_version > r,
    }
}

/// After a successful pull of `remote`, the device's next push version.
pub fn next_version_after_pull(remote_version: u64) -> u64 {
    remote_version + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_seatbelt() {
        assert!(may_push(1, None));
        assert!(may_push(5, Some(4)));
        assert!(!may_push(4, Some(4)));
        assert!(!may_push(3, Some(4)));
        assert_eq!(next_version_after_pull(4), 5);
    }

    #[test]
    fn hash_is_stable_hex64() {
        let h = sha256_hex(b"envelope");
        assert_eq!(h.len(), 64);
        assert_eq!(h, sha256_hex(b"envelope"));
        assert_ne!(h, sha256_hex(b"envelope2"));
    }
}
