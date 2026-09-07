//! The native executor: the DuckDB CLI on a temp database, running the
//! engine's real migrations and statements. This is the "same SQL both
//! sides" guarantee under test: what duckdb-wasm will run in the browser,
//! v1.3 of the real engine runs here first. Skips (loudly) if no `duckdb`
//! binary is present.

use std::process::Command;

use hc_engine::store::{self, stmt};
use serde_json::json;

struct Cli {
    db: std::path::PathBuf,
}

impl Cli {
    fn new(dir: &std::path::Path) -> Self {
        Self { db: dir.join("hc.duckdb") }
    }
    fn exec(&self, sql: &str) -> Result<String, String> {
        let out = Command::new("duckdb")
            .arg(&self.db)
            .arg("-json")
            .arg("-c")
            .arg(sql)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

fn have_duckdb() -> bool {
    Command::new("duckdb").arg("--version").output().is_ok()
}

#[test]
fn migrations_and_statements_run_on_real_duckdb() {
    if !have_duckdb() {
        eprintln!("SKIP: duckdb CLI not on PATH; browser executor still covers this SQL");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let cli = Cli::new(dir.path());

    for sql in store::migrations_from(0) {
        cli.exec(&sql).unwrap_or_else(|e| panic!("migration failed: {e}\nSQL: {sql}"));
    }
    let v = cli.exec(stmt::READ_VERSION).unwrap();
    assert!(v.contains(&format!("\"{}\"", store::SCHEMA_VERSION)), "version row: {v}");

    // Re-running from 0 must be harmless (IF NOT EXISTS discipline).
    for sql in store::migrations_from(0) {
        cli.exec(&sql).unwrap();
    }

    let q = store::render_stmt(
        stmt::INSERT_QUOTE,
        &[
            json!("q_0001"),
            json!("2026-09-06T20:00:00Z"),
            json!("O'Brien & Sons"),
            json!("123 Main St"),
            json!("Panel swap"),
            json!(""),
            json!(""),
            json!("25"),
            json!("2026-09-20"),
            json!("[{\"desc\":\"200A panel\",\"qty\":\"1\",\"unit\":\"800\"}]"),
            json!(155000),
        ],
    )
    .unwrap();
    cli.exec(&q).unwrap();

    let listed = cli.exec(stmt::LIST_QUOTES).unwrap();
    assert!(listed.contains("O'Brien & Sons"), "list: {listed}");
    assert!(listed.contains("155000"));

    let co = store::render_stmt(
        stmt::INSERT_CO,
        &[
            json!("co_0001"),
            json!("CO-20260906-01"),
            json!("2026-09-06T20:05:00Z"),
            json!("O'Brien & Sons"),
            json!("123 Main St"),
            json!("q_0001"),
            json!("Knob-and-tube found; replace run"),
            json!("Opened the wall"),
            json!(45000),
            json!("1"),
            json!("Pat O'Brien"),
            json!("2026-09-06"),
            json!(null),
            json!(null),
        ],
    )
    .unwrap();
    cli.exec(&co).unwrap();
    let cos = cli.exec(stmt::LIST_COS).unwrap();
    assert!(cos.contains("CO-20260906-01"));
}
