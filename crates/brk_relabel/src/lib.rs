//! Litview's display text over Bitview's: names, units and the brand color, applied under the
//! `litecoin` feature to the showcases at build time, the API's descriptions and the clients'
//! display names. Identifiers (series names, unit keys, module paths, pool names) stay as they
//! are, so each rule guards what surrounds its match.

/// The Bitview orange, without its closing parenthesis so alpha variants (`/ 0.1)`) match too.
const BRAND_ORANGE: &str = "oklch(64.34% 0.191 44.41";
/// Litecoin's blue at the orange's lightness, so contrast against the page holds.
const BRAND_BLUE: &str = "oklch(64.34% 0.14 255";

type Guard = fn(&str, &str) -> bool;

/// `(from, to, guard)`, in order; the guard sees the text before and after the match.
const RULES: &[(&str, &str, Guard)] = &[
    (BRAND_ORANGE, BRAND_BLUE, any),
    ("#e65b00", "#345d9d", any),
    ("bitview.space", "litview.space", any),
    ("sats: \"sats\"", "sats: \"litoshis\"", any),
    ("Bitview", "Litview", word),
    ("Bitcoin", "Litecoin", prose_word),
    ("bitcoin", "litecoin", lower_prose_word),
    ("BTC", "LTC", prose_word),
    ("sats/vB", "lit/vB", word_start),
    ("sat/vB", "lit/vB", word_start),
    ("sat/PH", "lit/PH", word_start),
    ("₿", "Ł", any),
    ("Satoshis", "Litoshis", word),
    ("satoshis", "litoshis", word),
    ("Sats per dollar", "Litoshis per dollar", word),
    ("sats", "litoshis", amount_unit),
    ("Sats", "Litoshis", amount_unit),
    ("\" sat\"", "\" lit\"", any),
    ("} sat`", "} lit`", any),
];

/// `text` with every rule applied.
pub fn relabel(text: &str) -> String {
    RULES
        .iter()
        .fold(text.to_owned(), |text, &(from, to, guard)| {
            replace_guarded(&text, from, to, guard)
        })
}

fn replace_guarded(text: &str, from: &str, to: &str, guard: Guard) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = 0;
    for (at, _) in text.match_indices(from) {
        if at < rest {
            continue;
        }
        let end = at + from.len();
        if guard(&text[..at], &text[end..]) {
            out.push_str(&text[rest..at]);
            out.push_str(to);
            rest = end;
        }
    }
    out.push_str(&text[rest..]);
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn ends_word(before: &str) -> bool {
    before.chars().next_back().is_some_and(is_word)
}

fn starts_word(after: &str) -> bool {
    after.chars().next().is_some_and(is_word)
}

fn any(_: &str, _: &str) -> bool {
    true
}

fn word_start(before: &str, _: &str) -> bool {
    !ends_word(before)
}

fn word(before: &str, after: &str) -> bool {
    !ends_word(before) && !starts_word(after)
}

/// Not an object key (`Bitcoin:`, a unit's name in the API).
fn not_key(after: &str) -> bool {
    !after.starts_with(':')
}

/// Not a pool named after the chain (`Bitcoin.com`).
fn not_pool(after: &str) -> bool {
    ![".com", ".COM", ".TOP", ".top", " Guild", " Pool", " pool"]
        .iter()
        .any(|pool| after.starts_with(pool))
}

fn prose_word(before: &str, after: &str) -> bool {
    word(before, after) && not_key(after) && not_pool(after)
}

/// Not part of a path, address, file or identifier (`bitcoin/bitcoin`, `bitcoin-core`).
fn lower_prose_word(before: &str, after: &str) -> bool {
    let joined = |c: Option<char>| matches!(c, Some('/' | '.' | '-' | '@' | ':'));
    prose_word(before, after)
        && !joined(before.chars().next_back())
        && !joined(after.chars().next())
}

/// `sats` as the unit of an amount shown (`50,000 sats`, `${n} sats`, `<small>sats</small>`, "in
/// sats"), never as a name in code (`(sats) =>`, `"sats"`, `price_sats`).
fn amount_unit(before: &str, after: &str) -> bool {
    let line = before.rsplit('\n').next().unwrap_or(before);
    if starts_word(after) || ends_word(before) || line.contains("@param") {
        return false;
    }
    if before.ends_with("<small>") || before.ends_with(" in ") {
        return true;
    }
    let Some(spaced) = before.strip_suffix(' ') else {
        return false;
    };
    spaced
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, '}' | ')' | 'K' | 'M' | 'k'))
        && !after.trim_start().starts_with(['=', '('])
        && !after.strip_prefix('.').is_some_and(starts_word)
}

#[cfg(test)]
mod tests {
    use super::relabel;

    #[test]
    fn relabels_display_text_only() {
        for (from, to) in [
            ("Bitview Studio", "Litview Studio"),
            ("https://bitview.space/api", "https://litview.space/api"),
            (
                "<title>Explorer · Bitview</title>",
                "<title>Explorer · Litview</title>",
            ),
            ("import { BitviewClient }", "import { BitviewClient }"),
            (
                "/modules/bitview-client/index.js",
                "/modules/bitview-client/index.js",
            ),
            ("Bitcoin Core's selection", "Litecoin Core's selection"),
            ("Bitcoin: [\"\", \" BTC\"]", "Bitcoin: [\"\", \" LTC\"]"),
            (
                "content=\"Bitcoin's price\"",
                "content=\"Litecoin's price\"",
            ),
            ("series(v, \"In bitcoin\")", "series(v, \"In litecoin\")"),
            (
                "usd: \"USD\", btc: \"BTC\", sats: \"sats\"",
                "usd: \"USD\", btc: \"LTC\", sats: \"litoshis\"",
            ),
            ("${d.bitcoin}", "${d.bitcoin}"),
            ("sats to bitcoin", "sats to litecoin"),
            ("github.com/bitcoin/bitcoin", "github.com/bitcoin/bitcoin"),
            ("The BTC/USD price", "The LTC/USD price"),
            ("BTC.com and BTC Guild", "BTC.com and BTC Guild"),
            ("<title>Bitcoin.com</title>", "<title>Bitcoin.com</title>"),
            ("/** @param {number} sats */", "/** @param {number} sats */"),
            (
                "[\"btc\", \"Bitcoin\"], [\"sats\", \"Satoshis\"]",
                "[\"btc\", \"Litecoin\"], [\"sats\", \"Litoshis\"]",
            ),
            ("SATS_PER_BTC", "SATS_PER_BTC"),
            ("0.01 BTC, 50,000 sats", "0.01 LTC, 50,000 litoshis"),
            ("`${grouped(v)} sats`", "`${grouped(v)} litoshis`"),
            ("<small>sats</small>", "<small>litoshis</small>"),
            ("1-10 Sats", "1-10 Litoshis"),
            ("100K+ BTC", "100K+ LTC"),
            ("1 to 10 sats.", "1 to 10 litoshis."),
            (
                "const btc = (sats) => sats / 1e8",
                "const btc = (sats) => sats / 1e8",
            ),
            ("[\"sats\", \"btc\"]", "[\"sats\", \"btc\"]"),
            ("price_sats", "price_sats"),
            ("5 sat/vB and 2 sats/vB", "5 lit/vB and 2 lit/vB"),
            ("₿412K", "Ł412K"),
            ("The Satoshi Trail", "The Satoshi Trail"),
            (
                "fill=\"oklch(64.34% 0.191 44.41)\"",
                "fill=\"oklch(64.34% 0.14 255)\"",
            ),
        ] {
            assert_eq!(relabel(from), to, "{from}");
        }
    }
}
