//! The envelope: AES-256-GCM over an opaque byte payload (in practice the
//! serialized DuckDB file). What leaves the phone is this envelope and
//! nothing else; the server that stores it holds other people's ciphertext.
//!
//! Layout: b"HCV1" | format u8 | nonce [12] | ciphertext+tag. The format
//! byte exists so a future layout can coexist with old backups.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};

use crate::keys::KeyMaterial;
use crate::{EngineError, Result};

const MAGIC: &[u8; 4] = b"HCV1";
const FORMAT_V1: u8 = 1;
const HEADER: usize = 4 + 1 + 12;

pub fn encrypt(keys: &KeyMaterial, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&keys.enc_key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| EngineError::Crypto(e.to_string()))?;
    let mut out = Vec::with_capacity(HEADER + ct.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_V1);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt(keys: &KeyMaterial, envelope: &[u8]) -> Result<Vec<u8>> {
    if envelope.len() < HEADER {
        return Err(EngineError::Envelope("too short".into()));
    }
    if &envelope[..4] != MAGIC {
        return Err(EngineError::Envelope("not an HCV1 envelope".into()));
    }
    if envelope[4] != FORMAT_V1 {
        return Err(EngineError::Envelope(format!("unknown format {}", envelope[4])));
    }
    let nonce = Nonce::from_slice(&envelope[5..17]);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&keys.enc_key));
    cipher
        .decrypt(nonce, &envelope[17..])
        .map_err(|_| EngineError::Envelope("wrong words or corrupted backup".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{keys::KeyMaterial, seed};

    fn km() -> KeyMaterial {
        KeyMaterial::derive(&seed::to_seed_bytes(&seed::generate()).unwrap()).unwrap()
    }

    #[test]
    fn roundtrip_byte_identical() {
        let k = km();
        let payload: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
        let env = encrypt(&k, &payload).unwrap();
        assert_ne!(&env[HEADER..], &payload[..]);
        assert_eq!(decrypt(&k, &env).unwrap(), payload);
    }

    #[test]
    fn wrong_words_fail_closed() {
        let a = km();
        let b = km();
        let env = encrypt(&a, b"the shop's whole winter").unwrap();
        assert!(decrypt(&b, &env).is_err());
    }

    #[test]
    fn tamper_fails_closed() {
        let k = km();
        let mut env = encrypt(&k, b"quote history").unwrap();
        let last = env.len() - 1;
        env[last] ^= 1;
        assert!(decrypt(&k, &env).is_err());
    }

    #[test]
    fn nonces_never_repeat_across_envelopes() {
        let k = km();
        let a = encrypt(&k, b"x").unwrap();
        let b = encrypt(&k, b"x").unwrap();
        assert_ne!(a[5..17], b[5..17]);
        assert_ne!(a, b);
    }
}
