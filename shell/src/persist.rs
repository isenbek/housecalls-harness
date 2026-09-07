//! The shell's wallet loop as functions, so it is tested code rather than
//! click-handler logic: draft in, twelve words, HCV1 envelope on disk, and
//! back. Same envelope the web stash holds; a future shell⇄stash sync is a
//! transport errand, not a format one.

use hc_engine::{keys::KeyMaterial, seed, vault};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub fn save_encrypted<T: Serialize>(value: &T, words: &str, path: &Path) -> Result<usize, String> {
    let canonical = seed::normalize(words).map_err(|e| e.to_string())?;
    let s64 = seed::to_seed_bytes(&canonical).map_err(|e| e.to_string())?;
    let keys = KeyMaterial::derive(&s64).map_err(|e| e.to_string())?;
    let plain = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let env = vault::encrypt(&keys, &plain).map_err(|e| e.to_string())?;
    std::fs::write(path, &env).map_err(|e| e.to_string())?;
    Ok(env.len())
}

pub fn load_encrypted<T: DeserializeOwned>(words: &str, path: &Path) -> Result<T, String> {
    let canonical = seed::normalize(words).map_err(|e| e.to_string())?;
    let s64 = seed::to_seed_bytes(&canonical).map_err(|e| e.to_string())?;
    let keys = KeyMaterial::derive(&s64).map_err(|e| e.to_string())?;
    let env = std::fs::read(path).map_err(|_| "no saved draft on this device".to_string())?;
    let plain = vault::decrypt(&keys, &env).map_err(|e| e.to_string())?;
    serde_json::from_slice(&plain).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_engine::seed;

    #[test]
    fn wallet_loop_on_disk() {
        let dir = std::env::temp_dir().join(format!("hc-shell-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("draft.hcv1");
        let words = seed::generate();
        let draft = vec![("200A panel".to_string(), "1".to_string(), "800".to_string())];

        let n = save_encrypted(&draft, &words, &path).unwrap();
        assert!(n > 17, "envelope has header + ciphertext");
        let raw = std::fs::read(&path).unwrap();
        assert_eq!(&raw[..4], b"HCV1", "what is on disk is the envelope, not the draft");
        assert!(!String::from_utf8_lossy(&raw).contains("200A"), "plaintext never touches disk");

        let back: Vec<(String, String, String)> = load_encrypted(&words, &path).unwrap()
;
        assert_eq!(back, draft);

        let wrong = seed::generate();
        assert!(load_encrypted::<Vec<(String, String, String)>>(&wrong, &path).is_err(), "wrong words fail closed");
        std::fs::remove_dir_all(&dir).ok();
    }
}
