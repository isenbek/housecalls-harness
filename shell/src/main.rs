//! hc-shell: the P4 Dioxus shell. One codebase, dx-buildable for desktop,
//! web, and (with the SDKs installed) Android/iOS, riding hc-engine as a
//! plain crate: the money math, the twelve words, and the envelope all run
//! native here, no wasm bridge, no JavaScript.
//!
//! v1 surface: the truck quote pad, plus the wallet loop against a local
//! encrypted file (the same HCV1 envelope the web stash holds, so a future
//! shell⇄stash sync is a transport question, not a format one).

mod persist;

use dioxus::prelude::*;
use hc_engine::{money, seed};
use serde::{Deserialize, Serialize};

const STYLE: &str = r#"
  :root { color-scheme: dark; }
  body { margin: 0; background: #1d1d1a; color: #e8e6df; font-family: system-ui, sans-serif; }
  .wrap { max-width: 560px; margin: 0 auto; padding: 16px; }
  h1 { font-size: 20px; margin: 8px 0 2px; }
  .sub { color: #a09d92; font-size: 13px; margin: 0 0 14px; }
  .panel { background: #252521; border: 1px solid #3a3a34; border-radius: 10px; padding: 12px; margin: 0 0 14px; }
  .bar { display: flex; justify-content: space-between; align-items: center; font-weight: 700; margin-bottom: 8px; }
  label { display: grid; gap: 4px; font-size: 12px; color: #a09d92; margin-bottom: 8px; }
  input { min-height: 42px; padding: 8px 10px; background: #1a1a17; color: #e8e6df;
          border: 1px solid #3a3a34; border-radius: 8px; font-size: 15px; width: 100%; box-sizing: border-box; }
  .item { display: grid; grid-template-columns: 1fr 64px 90px 84px 40px; gap: 6px; align-items: center; margin-bottom: 6px; }
  .ext { font-family: ui-monospace, monospace; text-align: right; font-size: 13px; }
  button { min-height: 42px; padding: 8px 14px; border-radius: 8px; border: 1px solid #3a3a34;
           background: #2e2e29; color: #e8e6df; font-size: 14px; cursor: pointer; }
  button.primary { background: #2f5d8a; border-color: #2f5d8a; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 8px; }
  .totals { font-family: ui-monospace, monospace; font-size: 15px; }
  .words { font-family: ui-monospace, monospace; background: #1a1a17; border: 1px dashed #3a3a34;
           border-radius: 8px; padding: 10px; line-height: 1.8; user-select: all; }
  .note { color: #a09d92; font-size: 12.5px; }
  .out { white-space: pre-wrap; font-family: ui-monospace, monospace; font-size: 13px;
         background: #1a1a17; border: 1px solid #3a3a34; border-radius: 8px; padding: 10px; }
"#;

#[derive(Serialize, Deserialize, Clone, Default)]
struct Draft {
    shop_name: String,
    shop_city: String,
    shop_license: String,
    customer: String,
    scope: String,
    deposit_pct: String,
    items: Vec<(String, String, String)>, // desc, qty, unit
}

fn draft_path() -> std::path::PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("housecalls-shell");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("draft.hcv1")
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut draft = use_signal(|| Draft {
        deposit_pct: "25".into(),
        items: vec![(String::new(), "1".into(), String::new())],
        ..Default::default()
    });
    let mut words = use_signal(|| Option::<String>::None);
    let mut restore_words = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut text_out = use_signal(String::new);

    let d = draft.read().clone();
    let pairs: Vec<(String, String)> = d.items.iter().map(|(_, q, u)| (q.clone(), u.clone())).collect();
    let total = money::quote_total(&pairs);
    let deposit = money::deposit_cents(total, &d.deposit_pct);

    let make_text = {
        let d = d.clone();
        move || {
            let mut out = format!("QUOTE from {}\n", if d.shop_name.is_empty() { "(your shop)" } else { &d.shop_name });
            if !d.shop_license.is_empty() {
                out += &format!("License {}\n", d.shop_license);
            }
            if !d.customer.is_empty() {
                out += &format!("For: {}\n", d.customer);
            }
            if !d.scope.is_empty() {
                out += &format!("Scope: {}\n", d.scope);
            }
            out += "\n";
            for (desc, qty, unit) in &d.items {
                if desc.is_empty() && money::line_total(qty, unit) == 0 {
                    continue;
                }
                out += &format!("{}  {} x {} = {}\n", desc, qty, money::money(money::to_cents(unit)), money::money(money::line_total(qty, unit)));
            }
            out += &format!("TOTAL: {}\n", money::money(total));
            if deposit > 0 {
                out += &format!("Deposit to schedule: {} ({}%)\n", money::money(deposit), d.deposit_pct);
            }
            out
        }
    };

    rsx! {
        style { {STYLE} }
        div { class: "wrap",
            h1 { "House Calls · quote pad" }
            p { class: "sub", "native shell (Dioxus) · engine v{hc_engine::store::SCHEMA_VERSION} · free, offline, reports to no one" }

            div { class: "panel",
                div { class: "bar", "Your shop" }
                label { "Business name"
                    input { value: "{d.shop_name}", oninput: move |e| draft.write().shop_name = e.value() }
                }
                label { "City"
                    input { value: "{d.shop_city}", oninput: move |e| draft.write().shop_city = e.value() }
                }
                label { "License #"
                    input { value: "{d.shop_license}", oninput: move |e| draft.write().shop_license = e.value() }
                }
            }

            div { class: "panel",
                div { class: "bar", "This quote" }
                label { "Customer"
                    input { value: "{d.customer}", oninput: move |e| draft.write().customer = e.value() }
                }
                label { "Scope"
                    input { value: "{d.scope}", oninput: move |e| draft.write().scope = e.value() }
                }
                for (i, (desc, qty, unit)) in d.items.iter().cloned().enumerate() {
                    div { class: "item", key: "{i}",
                        input { placeholder: "Item or labor", value: "{desc}",
                            oninput: move |e| draft.write().items[i].0 = e.value() }
                        input { placeholder: "Qty", value: "{qty}",
                            oninput: move |e| draft.write().items[i].1 = e.value() }
                        input { placeholder: "$ each", value: "{unit}",
                            oninput: move |e| draft.write().items[i].2 = e.value() }
                        span { class: "ext", {money::money(money::line_total(&qty, &unit))} }
                        button { onclick: move |_| { let mut w = draft.write(); if w.items.len() > 1 { w.items.remove(i); } }, "×" }
                    }
                }
                div { class: "row",
                    button { onclick: move |_| draft.write().items.push((String::new(), "1".into(), String::new())), "+ Add line" }
                    label { style: "flex:0 0 110px", "Deposit %"
                        input { value: "{d.deposit_pct}", oninput: move |e| draft.write().deposit_pct = e.value() }
                    }
                }
                p { class: "totals",
                    "Total: " {money::money(total)}
                    if deposit > 0 { {format!("   ·   Deposit: {}", money::money(deposit))} }
                }
                div { class: "row",
                    button { class: "primary", onclick: move |_| text_out.set(make_text()), "Quote as text" }
                }
                if !text_out.read().is_empty() {
                    div { class: "out", {text_out.read().clone()} }
                }
            }

            div { class: "panel",
                div { class: "bar", "Backup (twelve words, local file)" }
                p { class: "note",
                    "Same envelope the web stash holds: the engine encrypts your draft with words only you have. This shell writes it to a local file; syncing that file to the stash is a later errand."
                }
                if words.read().is_none() {
                    div { class: "row",
                        button { onclick: move |_| { words.set(Some(seed::generate())); }, "Create backup words" }
                    }
                }
                if let Some(w) = words.read().clone() {
                    p { class: "words", {w.clone()} }
                    div { class: "row",
                        button { class: "primary",
                            onclick: move |_| {
                                let d = draft.read().clone();
                                let res = persist::save_encrypted(&d, &w, &draft_path())
                                    .map(|n| format!("Encrypted draft saved: {} bytes at {}", n, draft_path().display()));
                                status.set(res.unwrap_or_else(|e| format!("save failed: {e}")));
                            },
                            "Save encrypted"
                        }
                    }
                }
                label { "Restore with words"
                    input { value: "{restore_words}", placeholder: "twelve words",
                        oninput: move |e| restore_words.set(e.value()) }
                }
                div { class: "row",
                    button {
                        onclick: move |_| {
                            let phrase = restore_words.read().clone();
                            let res: Result<Draft, String> = persist::load_encrypted(&phrase, &draft_path());
                            match res {
                                Ok(d2) => { draft.set(d2); status.set("Draft restored from the encrypted file.".into()); }
                                Err(e) => status.set(format!("restore failed: {e}")),
                            }
                        },
                        "Restore"
                    }
                }
                if !status.read().is_empty() {
                    p { class: "note", {status.read().clone()} }
                }
            }
        }
    }
}
