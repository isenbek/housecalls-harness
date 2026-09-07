//! Safe literal rendering for executors that cannot bind parameters (the
//! DuckDB CLI in native tests). The browser path uses real prepared
//! statements through duckdb-wasm; this exists so BOTH paths run the same
//! engine-emitted SQL. Strings are single-quote doubled; only JSON scalar
//! types are accepted, so an object can never splice into a statement.

use serde_json::Value;

use crate::{EngineError, Result};

pub fn literal(v: &Value) -> Result<String> {
    Ok(match v {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => return Err(EngineError::Sql(format!("non-scalar param: {other}"))),
    })
}

/// Substitute `?` placeholders left-to-right. `?` inside string literals in
/// the template is not supported by design: engine templates never put one
/// there, and the renderer is not a SQL parser.
pub fn render(template: &str, params: &[Value]) -> Result<String> {
    let mut out = String::with_capacity(template.len() + 32);
    let mut it = params.iter();
    for ch in template.chars() {
        if ch == '?' {
            let p = it
                .next()
                .ok_or_else(|| EngineError::Sql("more ? than params".into()))?;
            out.push_str(&literal(p)?);
        } else {
            out.push(ch);
        }
    }
    if it.next().is_some() {
        return Err(EngineError::Sql("more params than ?".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_scalars() {
        let sql = render(
            "INSERT INTO t VALUES (?, ?, ?, ?)",
            &[json!("O'Brien & Sons"), json!(42.5), json!(true), json!(null)],
        )
        .unwrap();
        assert_eq!(sql, "INSERT INTO t VALUES ('O''Brien & Sons', 42.5, TRUE, NULL)");
    }

    #[test]
    fn quote_doubling_defeats_breakout() {
        let evil = json!("x'); DROP TABLE quotes; --");
        let sql = render("INSERT INTO t VALUES (?)", &[evil]).unwrap();
        assert_eq!(sql, "INSERT INTO t VALUES ('x''); DROP TABLE quotes; --')");
    }

    #[test]
    fn arity_is_enforced_both_ways() {
        assert!(render("VALUES (?, ?)", &[json!(1)]).is_err());
        assert!(render("VALUES (?)", &[json!(1), json!(2)]).is_err());
    }

    #[test]
    fn objects_are_refused() {
        assert!(render("VALUES (?)", &[serde_json::json!({"a": 1})]).is_err());
    }
}
