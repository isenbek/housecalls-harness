//! The shelf's money math, promoted from TypeScript into the engine so
//! every shell (web glue, Dioxus native, whatever comes) computes a quote
//! the same way. Cents internally, dollars at the edges; a customer never
//! sees a floating-point artifact. Mirrors the web tests in
//! bradley.io/scripts/housecalls-quote-selftest.mjs.

/// Parse a money field ("$1,250.00", "19.99") to cents. Garbage and
/// negatives are zero, never an error: a field mid-edit is not a fault.
pub fn to_cents(raw: &str) -> i64 {
    let cleaned: String = raw.chars().filter(|c| !"$, \t".contains(*c)).collect();
    match cleaned.parse::<f64>() {
        Ok(n) if n.is_finite() && n >= 0.0 => (n * 100.0).round() as i64,
        _ => 0,
    }
}

/// Quantity fields allow decimals ("2.5"); garbage is zero.
pub fn parse_qty(raw: &str) -> f64 {
    let cleaned: String = raw.chars().filter(|c| !", \t".contains(*c)).collect();
    match cleaned.parse::<f64>() {
        Ok(n) if n.is_finite() && n >= 0.0 => n,
        _ => 0.0,
    }
}

pub fn line_total(qty: &str, unit: &str) -> i64 {
    (parse_qty(qty) * to_cents(unit) as f64).round() as i64
}

pub fn quote_total(items: &[(String, String)]) -> i64 {
    items.iter().map(|(q, u)| line_total(q, u)).sum()
}

pub fn deposit_cents(total: i64, pct: &str) -> i64 {
    match pct.parse::<f64>() {
        Ok(p) if p.is_finite() && p > 0.0 => ((total as f64 * p.min(100.0)) / 100.0).round() as i64,
        _ => 0,
    }
}

/// $1,250.00 formatting without a locale crate: the shelf sells in USD.
pub fn money(cents: i64) -> String {
    let neg = cents < 0;
    let cents = cents.abs();
    let dollars = cents / 100;
    let rem = cents % 100;
    let mut s = dollars.to_string();
    let mut grouped = String::new();
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*b as char);
    }
    s = grouped;
    format!("{}${}.{:02}", if neg { "-" } else { "" }, s, rem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_parse_mirrors_the_web_suite() {
        assert_eq!(to_cents("19.99"), 1999);
        assert_eq!(to_cents("0.1"), 10);
        assert_eq!(to_cents("$1,250.00"), 125000);
        assert_eq!(to_cents("abc"), 0);
        assert_eq!(to_cents("-5"), 0);
    }

    #[test]
    fn qty_and_lines() {
        assert_eq!(parse_qty("2.5"), 2.5);
        assert_eq!(parse_qty(""), 0.0);
        assert_eq!(line_total("3", "12.50"), 3750);
        // the classic float trap cannot appear
        assert_eq!(line_total("3", "0.1"), 30);
    }

    #[test]
    fn totals_and_deposit() {
        let items = vec![("1".into(), "100".into()), ("2".into(), "25.25".into())];
        assert_eq!(quote_total(&items), 15050);
        assert_eq!(deposit_cents(15050, "25"), 3763);
        assert_eq!(deposit_cents(1000, "150"), 1000);
        assert_eq!(deposit_cents(1000, "-5"), 0);
        assert_eq!(deposit_cents(1000, "junk"), 0);
    }

    #[test]
    fn money_renders_like_money() {
        assert_eq!(money(125000), "$1,250.00");
        assert_eq!(money(30), "$0.30");
        assert_eq!(money(155000), "$1,550.00");
    }
}
