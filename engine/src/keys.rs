//! Key material from the seed. Two independent derivations:
//!   - the ENCRYPTION key (Argon2id over the BIP-39 seed, fixed context
//!     salt): never leaves the process, zeroized on drop;
//!   - the STASH ID (SHA-256 over a domain-separated tag + seed): PUBLIC,
//!     it is how the same twelve words find the same backup on the server
//!     with no registration. Knowing the id must reveal nothing about the
//!     key, hence two unrelated derivations.
//!
//! Boring primitives only, by plan: BIP-39, Argon2id, SHA-256.

use argon2::{Algorithm, Argon2, Params, Version};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::{EngineError, Result};

/// Domain separation: bump the suffix only with a migration story.
const KDF_SALT: &[u8] = b"housecalls-harness/enc/v1";
const ID_TAG: &[u8] = b"housecalls-harness/stash-id/v1";

/// OWASP-shaped Argon2id: 19 MiB, 2 passes. About a second on a phone,
/// which is the right price for a key that guards someone's business.
fn argon() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).expect("static params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub struct KeyMaterial {
    pub(crate) enc_key: [u8; 32],
    stash_id: String,
}

impl KeyMaterial {
    pub fn derive(seed64: &[u8; 64]) -> Result<Self> {
        let mut enc_key = [0u8; 32];
        argon()
            .hash_password_into(seed64, KDF_SALT, &mut enc_key)
            .map_err(|e| EngineError::Crypto(e.to_string()))?;

        let mut h = Sha256::new();
        h.update(ID_TAG);
        h.update(seed64);
        let stash_id = hex::encode(h.finalize());

        Ok(Self { enc_key, stash_id })
    }

    /// Public, URL-safe, unguessable without the words.
    pub fn stash_id(&self) -> &str {
        &self.stash_id
    }
}

impl Drop for KeyMaterial {
    fn drop(&mut self) {
        self.enc_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;

    #[test]
    fn deterministic_and_distinct() {
        let w = seed::generate();
        let s = seed::to_seed_bytes(&w).unwrap();
        let a = KeyMaterial::derive(&s).unwrap();
        let b = KeyMaterial::derive(&s).unwrap();
        assert_eq!(a.enc_key, b.enc_key);
        assert_eq!(a.stash_id(), b.stash_id());
        // id must not be derivable from the key or vice versa; at minimum
        // they must not share bytes trivially.
        assert_ne!(hex::encode(a.enc_key), *a.stash_id());

        let w2 = seed::generate();
        let s2 = seed::to_seed_bytes(&w2).unwrap();
        let c = KeyMaterial::derive(&s2).unwrap();
        assert_ne!(a.stash_id(), c.stash_id());
        assert_ne!(a.enc_key, c.enc_key);
    }

    #[test]
    fn stash_id_is_hex64() {
        let s = seed::to_seed_bytes(&seed::generate()).unwrap();
        let k = KeyMaterial::derive(&s).unwrap();
        assert_eq!(k.stash_id().len(), 64);
        assert!(k.stash_id().chars().all(|c| c.is_ascii_hexdigit()));
    }
}
