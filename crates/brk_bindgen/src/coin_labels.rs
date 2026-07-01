//! Chain-specific display labels for generated client docs and cohort names.
//!
//! Internal type and field names stay Bitcoin-compatible (`Sats`, `price_sats`, …).
//! Only user-facing `short`/`long` cohort labels and schema descriptions are relabeled.

use brk_chain::Chain;
use serde_json::Value;

/// Replace Bitcoin-centric display copy with Litecoin terms.
pub fn relabel_display_text(text: &str, chain: Chain) -> String {
    if chain == Chain::Bitcoin {
        return text.to_string();
    }

    match text {
        "0 sats" => return "0 litoshis".to_string(),
        "0 Sats" => return "0 Litoshis".to_string(),
        _ => {}
    }

    let mut out = text.to_string();
    for (from, to) in LITECOIN_DISPLAY_REPLACEMENTS {
        if out.contains(from) {
            out = out.replace(from, to);
        }
    }
    out
}

/// Relabel cohort constant JSON (`short` / `long` fields only; `id` stays stable).
pub fn relabel_cohort_constants(value: Value, chain: Chain) -> Value {
    if chain == Chain::Bitcoin {
        return value;
    }

    match value {
        Value::Object(mut map) => {
            for (key, val) in map.iter_mut() {
                if matches!(key.as_str(), "short" | "long")
                    && let Value::String(s) = val
                {
                    *s = relabel_display_text(s, chain);
                } else {
                    *val = relabel_cohort_constants(std::mem::take(val), chain);
                }
            }
            Value::Object(map)
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|v| relabel_cohort_constants(v, chain))
                .collect(),
        ),
        other => other,
    }
}

/// Relabel every OpenAPI `description` field for chain-specific client docs.
pub fn relabel_openapi_descriptions(value: &mut Value, chain: Chain) {
    if chain == Chain::Bitcoin {
        return;
    }

    match value {
        Value::Object(map) => {
            if let Some(Value::String(desc)) = map.get_mut("description") {
                *desc = relabel_display_text(desc, chain);
            }
            for val in map.values_mut() {
                relabel_openapi_descriptions(val, chain);
            }
        }
        Value::Array(items) => {
            for item in items {
                relabel_openapi_descriptions(item, chain);
            }
        }
        _ => {}
    }
}

const LITECOIN_DISPLAY_REPLACEMENTS: &[(&str, &str)] = &[
    ("Bitcoin Research Kit", "Litecoin Research Kit"),
    ("Bitcoin Core", "Litecoin Core"),
    ("Bitcoin address", "Litecoin address"),
    ("Bitcoin block", "Litecoin block"),
    ("Bitcoin amount", "Litecoin amount"),
    ("Bitcoin protocol", "Litecoin protocol"),
    ("Bitcoin network", "Litecoin network"),
    ("Bitcoin node", "Litecoin node"),
    ("Bitcoin consensus", "Litecoin consensus"),
    ("Bitcoin data", "Litecoin data"),
    ("Bitcoin primitives", "Litecoin primitives"),
    ("bitcoin latest", "litecoin latest"),
    ("Bitview", "Litview"),
    ("BTC/USD", "LTC/USD"),
    (
        "satoshis (1 BTC = 100,000,000 sats)",
        "litoshis (1 LTC = 100,000,000 lits)",
    ),
    (
        "Fractional satoshis (f64) - for representing USD prices in sats",
        "Fractional litoshis (f64) - for representing USD prices in lits",
    ),
    ("When BTC is $100,000:", "When LTC is $100,000:"),
    ("supply in BTC", "supply in LTC"),
    ("in BTC.", "in LTC."),
    ("satoshis", "litoshis"),
    (" in sats", " in lits"),
    (" (sats)", " (lits)"),
    (" × sats", " × lits"),
    (" Sats", " Lits"),
    (" sats", " lits"),
    (" Sat", " Lit"),
    (" BTC", " LTC"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn relabels_amount_cohort_short_labels() {
        assert_eq!(
            relabel_display_text("1-10 sats", Chain::Litecoin),
            "1-10 lits"
        );
        assert_eq!(
            relabel_display_text("100k+ BTC", Chain::Litecoin),
            "100k+ LTC"
        );
        assert_eq!(
            relabel_display_text("0 sats", Chain::Litecoin),
            "0 litoshis"
        );
    }

    #[test]
    fn relabels_sats_type_description() {
        let desc = "Amount in satoshis (1 BTC = 100,000,000 sats)";
        assert_eq!(
            relabel_display_text(desc, Chain::Litecoin),
            "Amount in litoshis (1 LTC = 100,000,000 lits)"
        );
    }

    #[test]
    fn bitcoin_chain_is_unchanged() {
        assert_eq!(
            relabel_display_text("1-10 sats", Chain::Bitcoin),
            "1-10 sats"
        );
    }

    #[test]
    fn relabels_cohort_json_short_only() {
        let input = json!({
            "_1satTo10sats": {
                "id": "1sat_to_10sats",
                "short": "1-10 sats",
                "long": "1-10 Sats"
            }
        });
        let out = relabel_cohort_constants(input, Chain::Litecoin);
        assert_eq!(out["_1satTo10sats"]["id"], "1sat_to_10sats");
        assert_eq!(out["_1satTo10sats"]["short"], "1-10 lits");
        assert_eq!(out["_1satTo10sats"]["long"], "1-10 Lits");
    }
}
