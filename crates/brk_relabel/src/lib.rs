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
    ("oklch%2864.34%25 0.191 44.41", "oklch%2864.34%25 0.14 255", any),
    ("oklch(52% 0.16 44.41", "oklch(52% 0.14 255", any),
    ("#f7931a", "#4d8fe0", any),
    ("#a95600", "#2769b7", any),
    ("#c24e00", "#345d9d", any),
    ("#a94400", "#2b4e85", any),
    ("rgb(230 91 0", "rgb(52 93 157", any),
    // Pages' accents as light and dark pairs: the same colors elsewhere are data's, not the brand's.
    ("light-dark(#d4570a, #f2873a)", "light-dark(#2769b7, #69a1e8)", any),
    ("light-dark(#9c3d00, #ffb27a)", "light-dark(#1f5596, #a1c7f8)", any),
    ("light-dark(#fbe9dc, #33221a)", "light-dark(#e4edf9, #19243a)", any),
    ("light-dark(#f6d7bf, #47291a)", "light-dark(#cddcf2, #1d2e4d)", any),
    ("light-dark(#ad470b, #ff8a3d)", "light-dark(#2769b7, #69a1e8)", any),
    ("light-dark(#fff8f2, #170b02)", "light-dark(#f4f8ff, #04101f)", any),
    (
        "light-dark(rgb(184 68 10), rgb(255 128 52))",
        "light-dark(rgb(39 105 183), rgb(105 161 232))",
        any,
    ),
    // Oracle's heat ramp, low to high, and its launch color.
    (
        "light-dark(rgb(250 230 212), rgb(54 16 4))",
        "light-dark(rgb(218 230 248), rgb(6 18 46))",
        any,
    ),
    (
        "light-dark(rgb(245 172 112), rgb(196 66 14))",
        "light-dark(rgb(124 168 232), rgb(30 86 172))",
        any,
    ),
    (
        "light-dark(rgb(232 108 30), rgb(255 152 66))",
        "light-dark(rgb(39 105 183), rgb(112 172 255))",
        any,
    ),
    ("data-color=\"#e86c1e\"", "data-color=\"#345d9d\"", any),
    ("oklch(67.64% 0.191 44.41", "oklch(67.64% 0.14 255", any),
    ("https://bitview.dev/", "https://litview.space/dev/", any),
    ("https://bitview.dev", "https://litview.space/dev/", any),
    ("https://bitcoinresearchkit.org/", "https://litview.space/brk/", any),
    ("https://bitcoinresearchkit.org", "https://litview.space/brk/", any),
    ("bitview.space", "litview.space", any),
    ("sats: \"sats\"", "sats: \"litoshis\"", any),
    ("Bitview", "Litview", word),
    ("Bitcoin", "Litecoin", prose_word),
    ("bitcoin", "litecoin", lower_prose_word),
    ("bitcoind", "litecoind", lower_prose_word),
    ("bitcoin-cli", "litecoin-cli", lower_prose_word),
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

/// Bitcoin's chain facts in prose, and Litecoin's, applied before the rules. Any whitespace in
/// `from` matches any run of it, as doc comments wrap where they like.
const FACTS: &[(&str, &str)] = &[
    // Litecoin enforced BIP30 from genesis: it has no duplicate coinbases.
    (
        ", and the coinbases of blocks 91812 and 91722 once duplicates at 91842 and 91880 overwrite them;",
        ";",
    ),
    (
        ", plus the coinbase outputs of blocks 91812 and 91722, which duplicates at 91842 and 91880 overwrote.",
        ".",
    ),
    (
        "unclaimed block rewards and the two coinbase outputs that BIP30 duplicates overwrote (blocks 91812 and 91722, at 91842 and 91880)",
        "and unclaimed block rewards",
    ),
    (
        "the first priced interval (2010-08-16)",
        "the first priced interval (2012-01-31)",
    ),
    (
        "https://github.com/bitcoinresearchkit/mono",
        "https://github.com/IndigoNakamoto/lrk",
    ),
    // Litecoin's prices: exchange daily closes, the earliest carried back to genesis, then the
    // oracle from 2,200,000.
    (
        "Heights before 340,000 use baked historical exchange prices;",
        "Heights before 2,200,000 (January 2022) use exchange daily closes;",
    ),
    (
        "Before the first exchange price (block 74,641, August 2010), this price and USD amounts valued with it are zero.",
        "Blocks before the first exchange close carry the earliest one.",
    ),
    (
        "Zero before the first exchange price (block 74,641, August 2010).",
        "Blocks before the first exchange close carry the earliest one.",
    ),
    (
        "Blocks before the first price (block 74,641) are left out: the period holding it starts there, and earlier periods are flat zero candles.",
        "Blocks before the first exchange close carry the earliest one.",
    ),
];

/// `text` with every fact and rule applied.
pub fn relabel(text: &str) -> String {
    let text = FACTS.iter().fold(text.to_owned(), |text, &(from, to)| {
        replace_loose(&text, from, to)
    });
    RULES.iter().fold(text, |text, &(from, to, guard)| {
        replace_guarded(&text, from, to, guard)
    })
}

fn replace_loose(text: &str, from: &str, to: &str) -> String {
    let mut words = from.split_whitespace();
    let first = words.next().unwrap();
    let words: Vec<&str> = words.collect();
    let mut out = String::new();
    let (mut rest, mut search) = (0, 0);
    while let Some(found) = text[search..].find(first) {
        let at = search + found;
        search = at + first.len();
        if let Some(len) = loose_match(&text[search..], &words) {
            out.push_str(&text[rest..at]);
            out.push_str(to);
            search += len;
            rest = search;
        }
    }
    out.push_str(&text[rest..]);
    out
}

/// The length of `words` at the start of `after`, each after a run of whitespace.
fn loose_match(after: &str, words: &[&str]) -> Option<usize> {
    let mut at = 0;
    for word in words {
        let gap = after[at..].len() - after[at..].trim_start().len();
        if gap == 0 || !after[at + gap..].starts_with(word) {
            return None;
        }
        at += gap + word.len();
    }
    Some(at)
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
            (
                "--brand-text: light-dark(oklch(52% 0.16 44.41), var(--brand));",
                "--brand-text: light-dark(oklch(52% 0.14 255), var(--brand));",
            ),
            (
                "fill=%22oklch%2864.34%25 0.191 44.41%29%22",
                "fill=%22oklch%2864.34%25 0.14 255%29%22",
            ),
            ("--now: #f7931a;", "--now: #4d8fe0;"),
            ("--now-text: #a95600;", "--now-text: #2769b7;"),
            (
                "--bitview-version-orange: #c24e00;",
                "--bitview-version-orange: #345d9d;",
            ),
            ("rgb(230 91 0 / 0.16)", "rgb(52 93 157 / 0.16)"),
            (
                "--accent: light-dark(#d4570a, #f2873a);",
                "--accent: light-dark(#2769b7, #69a1e8);",
            ),
            (
                "--accent: light-dark(#ad470b, #ff8a3d);",
                "--accent: light-dark(#2769b7, #69a1e8);",
            ),
            ("colors: [\"#ff8a3d\"]", "colors: [\"#ff8a3d\"]"),
            (
                "--answer: light-dark(rgb(184 68 10), rgb(255 128 52));",
                "--answer: light-dark(rgb(39 105 183), rgb(105 161 232));",
            ),
            (
                "--heat-hi: light-dark(rgb(232 108 30), rgb(255 152 66));",
                "--heat-hi: light-dark(rgb(39 105 183), rgb(112 172 255));",
            ),
            (
                "data-app=\"Oracle\" data-color=\"#e86c1e\"",
                "data-app=\"Oracle\" data-color=\"#345d9d\"",
            ),
            (
                "--orange: oklch(67.64% 0.191 44.41);",
                "--orange: oklch(67.64% 0.14 255);",
            ),
            (
                "<link rel=\"canonical\" href=\"https://bitview.dev/\" />",
                "<link rel=\"canonical\" href=\"https://litview.space/dev/\" />",
            ),
            (
                "<a href=\"https://bitview.dev\">Bitview</a>",
                "<a href=\"https://litview.space/dev/\">Litview</a>",
            ),
            ("https://perf.bitview.dev", "https://perf.bitview.dev"),
            (
                "<a href=\"https://bitcoinresearchkit.org\">Bitcoin Research Kit</a>",
                "<a href=\"https://litview.space/brk/\">Litecoin Research Kit</a>",
            ),
            (
                "content=\"https://bitcoinresearchkit.org/\"",
                "content=\"https://litview.space/brk/\"",
            ),
            (
                "without a bitcoind round-trip",
                "without a litecoind round-trip",
            ),
            (
                "as mempool.space and bitcoin-cli report it",
                "as mempool.space and litecoin-cli report it",
            ),
            ("~/.bitcoin/bitcoin.conf", "~/.bitcoin/bitcoin.conf"),
        ] {
            assert_eq!(relabel(from), to, "{from}");
        }
    }

    #[test]
    fn states_litecoin_facts() {
        for (from, to) in [
            (
                "genesis coinbase, which are unspendable, and the\ncoinbases of blocks 91812 and 91722 once duplicates at 91842 and 91880 overwrite them; it\nincludes",
                "genesis coinbase, which are unspendable; it\nincludes",
            ),
            (
                "order: spent outputs, plus the\ncoinbase outputs of blocks 91812 and 91722, which duplicates at 91842 and 91880 overwrote.",
                "order: spent outputs.",
            ),
            (
                "output values, unclaimed block rewards and the two coinbase outputs that BIP30 duplicates overwrote (blocks 91812 and 91722, at 91842 and 91880).",
                "output values, and unclaimed block rewards.",
            ),
            (
                "[GitHub](https://github.com/bitcoinresearchkit/mono/issues)",
                "[GitHub](https://github.com/IndigoNakamoto/lrk/issues)",
            ),
            (
                "the coinbases of blocks 91812",
                "the coinbases of blocks 91812",
            ),
            (
                "estimate. Heights before\n340,000 use baked historical exchange prices; later heights",
                "estimate. Heights before 2,200,000 (January 2022) use exchange daily closes; later heights",
            ),
            (
                "ticker. Before\nthe first exchange price (block 74,641, August 2010), this price and USD\namounts valued with it are zero. At",
                "ticker. Blocks before the first exchange close carry the earliest one. At",
            ),
        ] {
            assert_eq!(relabel(from), to, "{from}");
        }
    }
}
