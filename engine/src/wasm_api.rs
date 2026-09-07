//! The wasm-bindgen surface: what the browser glue may touch. The key
//! stays inside `Vault`; JavaScript sees words in, ciphertext and ids out,
//! never key bytes.

use wasm_bindgen::prelude::*;

use crate::{keys::KeyMaterial, seed, store, sync, vault};

#[wasm_bindgen]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn schema_version() -> u32 {
    store::SCHEMA_VERSION
}

#[wasm_bindgen]
pub fn generate_seed() -> String {
    seed::generate()
}

#[wasm_bindgen]
pub fn normalize_seed(words: &str) -> Result<String, JsError> {
    seed::normalize(words).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn migrations_from(from: u32) -> Vec<String> {
    store::migrations_from(from)
}

/// Statement templates for the glue's prepared statements.
#[wasm_bindgen]
pub fn stmt(name: &str) -> Result<String, JsError> {
    Ok(match name {
        "read_version" => store::stmt::READ_VERSION,
        "upsert_shop" => store::stmt::UPSERT_SHOP,
        "insert_quote" => store::stmt::INSERT_QUOTE,
        "list_quotes" => store::stmt::LIST_QUOTES,
        "insert_co" => store::stmt::INSERT_CO,
        "list_cos" => store::stmt::LIST_COS,
        other => return Err(JsError::new(&format!("unknown stmt {other}"))),
    }
    .to_string())
}

#[wasm_bindgen]
pub fn sha256_hex(bytes: &[u8]) -> String {
    sync::sha256_hex(bytes)
}

#[wasm_bindgen]
pub fn may_push(local_version: u32, remote_version: Option<u32>) -> bool {
    sync::may_push(local_version as u64, remote_version.map(|v| v as u64))
}

/// The unlocked vault: derive once (Argon2id, about a second), then
/// encrypt/decrypt envelopes for the session. Drop zeroizes the key.
#[wasm_bindgen]
pub struct Vault {
    keys: KeyMaterial,
}

#[wasm_bindgen]
impl Vault {
    #[wasm_bindgen(constructor)]
    pub fn new(words: &str) -> Result<Vault, JsError> {
        let canonical = seed::normalize(words).map_err(|e| JsError::new(&e.to_string()))?;
        let seed64 = seed::to_seed_bytes(&canonical).map_err(|e| JsError::new(&e.to_string()))?;
        let keys = KeyMaterial::derive(&seed64).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Vault { keys })
    }

    pub fn stash_id(&self) -> String {
        self.keys.stash_id().to_string()
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, JsError> {
        vault::encrypt(&self.keys, plaintext).map_err(|e| JsError::new(&e.to_string()))
    }

    pub fn decrypt(&self, envelope: &[u8]) -> Result<Vec<u8>, JsError> {
        vault::decrypt(&self.keys, envelope).map_err(|e| JsError::new(&e.to_string()))
    }
}
