//! hc-engine: the House Calls local backend, one headless Rust crate
//! (docs plan: bradley.io docs/housecalls/local-harness-plan.md).
//!
//! The engine owns four things and deliberately nothing else:
//!   - the SCHEMA and its one-way migrations (store),
//!   - the CRYPTO: seed phrase -> keys, envelope encrypt/decrypt (seed, keys, vault),
//!   - the STASH protocol: ids, version rules, envelope format (sync),
//!   - safe SQL rendering for executors without prepared statements (sqlrender).
//!
//! It does NOT own transport or a database runtime: DuckDB-WASM executes in
//! the browser, the DuckDB CLI executes in native tests, and both are handed
//! the same SQL. That injection seam is what makes the engine universal.

pub mod keys;
pub mod seed;
pub mod sqlrender;
pub mod store;
pub mod sync;
pub mod vault;

#[cfg(target_arch = "wasm32")]
mod wasm_api;

#[derive(thiserror::Error, Debug)]
pub enum EngineError {
    #[error("bad seed phrase: {0}")]
    BadSeed(String),
    #[error("crypto failure: {0}")]
    Crypto(String),
    #[error("bad envelope: {0}")]
    Envelope(String),
    #[error("sql rendering: {0}")]
    Sql(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
