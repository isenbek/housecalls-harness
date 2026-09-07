//! The schema and its one-way migrations. The engine owns HOW the store
//! opens and evolves (the apex dbrunner lesson); each tool owns WHAT lives
//! in its tables. Executors run these statements verbatim: duckdb-wasm in
//! the browser, the DuckDB CLI in native tests, same SQL.

use serde_json::Value;

use crate::sqlrender;
use crate::Result;

/// Bump ONLY by appending a new entry to MIGRATIONS.
pub const SCHEMA_VERSION: u32 = 1;

/// Each migration is a list of statements applied in order, in one
/// executor transaction if the executor has them.
const MIGRATIONS: &[&[&str]] = &[
    // v1: the shelf as it exists (quote pad + change-order pad), plus the
    // engine's own bookkeeping.
    &[
        "CREATE TABLE IF NOT EXISTS hc_meta (key TEXT PRIMARY KEY, value TEXT)",
        "CREATE TABLE IF NOT EXISTS shop (
            id INTEGER PRIMARY KEY DEFAULT 1,
            name TEXT, phone TEXT, email TEXT, city TEXT,
            license TEXT, insured TEXT, deposit_pct TEXT, valid_days TEXT, terms TEXT
        )",
        "CREATE TABLE IF NOT EXISTS quotes (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            customer TEXT, address TEXT, scope TEXT,
            exclusions TEXT, notes TEXT,
            deposit_pct TEXT, valid_until TEXT,
            items_json TEXT NOT NULL,
            total_cents BIGINT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS change_orders (
            id TEXT PRIMARY KEY,
            co_number TEXT NOT NULL,
            created_at TEXT NOT NULL,
            customer TEXT, address TEXT, job_ref TEXT,
            description TEXT, reason TEXT,
            amount_cents BIGINT NOT NULL,
            days TEXT,
            signed_name TEXT, signed_date TEXT,
            signature_png TEXT,
            photos_json TEXT
        )",
    ],
];

/// Statements to bring a store at `from` (0 = fresh) up to SCHEMA_VERSION.
/// The version stamp ships as the LAST statement so a half-applied
/// migration re-runs (all DDL is IF NOT EXISTS for exactly that reason).
pub fn migrations_from(from: u32) -> Vec<String> {
    let mut out = Vec::new();
    for (i, steps) in MIGRATIONS.iter().enumerate() {
        let target = (i + 1) as u32;
        if target <= from {
            continue;
        }
        out.extend(steps.iter().map(|s| s.to_string()));
        out.push(format!(
            "INSERT OR REPLACE INTO hc_meta (key, value) VALUES ('schema_version', '{target}')"
        ));
    }
    out
}

/// Engine-owned statement templates the glue binds params into (real
/// prepared statements in the browser; sqlrender in CLI tests).
pub mod stmt {
    pub const READ_VERSION: &str =
        "SELECT value FROM hc_meta WHERE key = 'schema_version'";
    pub const UPSERT_SHOP: &str = "INSERT OR REPLACE INTO shop \
        (id, name, phone, email, city, license, insured, deposit_pct, valid_days, terms) \
        VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    pub const INSERT_QUOTE: &str = "INSERT OR REPLACE INTO quotes \
        (id, created_at, customer, address, scope, exclusions, notes, deposit_pct, valid_until, items_json, total_cents) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    pub const LIST_QUOTES: &str = "SELECT id, created_at, customer, total_cents \
        FROM quotes ORDER BY created_at DESC LIMIT 100";
    pub const INSERT_CO: &str = "INSERT OR REPLACE INTO change_orders \
        (id, co_number, created_at, customer, address, job_ref, description, reason, amount_cents, days, signed_name, signed_date, signature_png, photos_json) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    pub const LIST_COS: &str = "SELECT id, co_number, created_at, customer, amount_cents \
        FROM change_orders ORDER BY created_at DESC LIMIT 100";
}

/// Render one template + params to a literal statement (CLI executors).
pub fn render_stmt(template: &str, params: &[Value]) -> Result<String> {
    sqlrender::render(template, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_store_gets_everything_once() {
        let all = migrations_from(0);
        assert!(all.iter().any(|s| s.contains("CREATE TABLE IF NOT EXISTS quotes")));
        assert!(all.last().unwrap().contains("schema_version"));
        assert!(migrations_from(SCHEMA_VERSION).is_empty());
    }
}
