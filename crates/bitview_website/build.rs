//! Under `litecoin`, the showcases to embed are Litview's: a copy of `showcases/` with its display
//! text relabeled, then `litecoin/` laid over it for what text rules can't change (images).

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// The extensions of the files relabeled; any other is copied as is.
const TEXT: &[&str] = &[
    "html",
    "js",
    "mjs",
    "json",
    "txt",
    "xml",
    "md",
    "svg",
    "webmanifest",
    "css",
];
/// Paths (from the showcases' root, by prefix) copied as is: the generated clients are code, and
/// the release notes are upstream's history, true of Bitcoin.
const VERBATIM: &[&str] = &["modules", "changes/CHANGELOG.md", "changes/v"];
/// The favicon's link, under a URL of its own: litview.space's 0.11 server sent its favicon as
/// immutable for a year, so browsers (and the CDN) holding it never ask for this one otherwise.
const FAVICON: (&str, &str) = ("favicon/favicon.svg\"", "favicon/favicon.svg?v=litview\"");
/// Bitcoin's chain facts and words written into the pages, and Litecoin's: (path, upstream's text, Litecoin's),
/// every occurrence replaced before relabeling. Each must still be found, so an upstream rewrite
/// fails the build rather than ship Bitcoin's numbers.
const FACTS: &[(&str, &str, &str)] = &[
    (
        "explorer/index.html",
        r#"      const COIN = "BTC";
      const ATOMS = "sats";
      const RATE_UNIT = "sat/vB";
      const BLOCK_SECONDS = 600;
      const HALVING_BLOCKS = 210_000;
      // The blocks "within" a short and a longer wait: 30 minutes and an hour at 600 seconds a block.
      const SOON = [3, 6];"#,
        r#"      const COIN = "LTC";
      const ATOMS = "litoshis";
      const RATE_UNIT = "lit/vB";
      const BLOCK_SECONDS = 150;
      const HALVING_BLOCKS = 840_000;
      // The blocks "within" a short and a longer wait: 10 and 20 minutes at 150 seconds a block.
      const SOON = [4, 8];"#,
    ),
    (
        "_chain/index.html",
        "const HALVING_EPOCH_BLOCKS = 210_000;",
        "const HALVING_EPOCH_BLOCKS = 840_000;",
    ),
    (
        "_chain/index.html",
        "const TARGET_BLOCK_SECONDS = 600;",
        "const TARGET_BLOCK_SECONDS = 150;",
    ),
    (
        "_chain/index.html",
        r#"TARGET_BLOCK_SECONDS, "~10mn")"#,
        r#"TARGET_BLOCK_SECONDS, "~2.5mn")"#,
    ),
    (
        "_chain/index.html",
        "initial-value: 600;",
        "initial-value: 150;",
    ),
    (
        "_chain/index.html",
        "var(--block-interval, 600)",
        "var(--block-interval, 150)",
    ),
    ("calendar/index.html", "210_000", "840_000"),
    (
        "globe/index.html",
        r#"        ["2012-11-01", "1st halving"],
        ["2016-07-01", "2nd halving"],
        ["2020-05-01", "3rd halving"],
        ["2021-05-01", "China bans mining"],
        ["2024-04-01", "4th halving"],"#,
        r#"        ["2015-08-01", "1st halving"],
        ["2019-08-01", "2nd halving"],
        ["2021-05-01", "China bans mining"],
        ["2023-08-01", "3rd halving"],"#,
    ),
    (
        "studio/index.html",
        "KNOWN_HALVINGS = [1354116278, 1468082773, 1589225023, 1713571767], HALVING_BLOCKS = 210_000;",
        "KNOWN_HALVINGS = [1440543530, 1565000170, 1690988793], HALVING_BLOCKS = 840_000;",
    ),
    (
        "studio/index.html",
        "* HALVING_BLOCKS) * 600)",
        "* HALVING_BLOCKS) * 150)",
    ),
    (
        "studio/index.html",
        "- liveHeight) * 600 }",
        "- liveHeight) * 150 }",
    ),
    (
        "studio/index.html",
        "Math.floor(facts.height / 210_000)",
        "Math.floor(facts.height / 840_000)",
    ),
    (
        "studio/presets.js",
        "* 210_000 > facts.height",
        "* 840_000 > facts.height",
    ),
    (
        "studio/presets.js",
        "(facts?.height ?? 840_000) / 210_000",
        "(facts?.height ?? 3_360_000) / 840_000",
    ),
    (
        "studio/presets.js",
        r#"levels: [{ value: 600, color: "gray", dash: 3 }]"#,
        r#"levels: [{ value: 150, color: "gray", dash: 3 }]"#,
    ),
    (
        "series/index.html",
        r#"avg: "average", opreturn: "OP_RETURN" });"#,
        r#"avg: "average", opreturn: "OP_RETURN", btc: "LTC", sats: "litoshis", sat: "litoshi" });"#,
    ),
    (
        "series/index.html",
        r#"["Value type", info.type]"#,
        r#"["Value type", ({ Bitcoin: "Litecoin", Sats: "Litoshis" })[info.type] ?? info.type]"#,
    ),
    (
        "studio/index.html",
        r#"txs: "transactions", utxos: "UTXOs" };"#,
        r#"txs: "transactions", utxos: "UTXOs", btc: "LTC", sats: "litoshis", sat: "litoshi" };"#,
    ),
    // The oracle reads Litecoin from 2,200,000; its accuracy from Litview's own run of it
    // (`oracle_tune`), against each block's exchange range (`oracle_truth`).
    (
        "oracle/index.html",
        "Math.ceil(508_000 / DAY) * DAY",
        "Math.ceil(2_200_000 / DAY) * DAY",
    ),
    (
        "oracle/index.html",
        "<strong>0.15%</strong><span>median distance from the exchanges' price range, block by block, over 596,251 blocks since January 2015.</span>",
        "<strong>0.18%</strong><span>median distance from the exchanges' price range, block by block, over 1,013,333 blocks since January 2022.</span>",
    ),
    (
        "oracle/index.html",
        r#"            <div><dt>95% of blocks</dt><dd>within 1.2%</dd></div>
            <div><dt>99% of blocks</dt><dd>within 3.4%</dd></div>
            <div><dt>Since 2022</dt><dd>none off by 10%</dd></div>"#,
        r#"            <div><dt>95% of blocks</dt><dd>within 2.2%</dd></div>
            <div><dt>99% of blocks</dt><dd>within 12.9%</dd></div>
            <div><dt>Since 2024</dt><dd>31 blocks off by 10%</dd></div>"#,
    ),
    (
        "oracle/index.html",
        r#"These numbers aren't measured live; they come from brk_oracle's own test, in <a href="https://github.com/bitcoinresearchkit/mono/blob/main/crates/brk_oracle/README.md#accuracy">its README</a>. It ran the oracle over every block from 340,000 (January 2015) through 952,314 and measured the 596,251 that exchange data covers against exchanges' high and low at that block; a block inside that range counts as 0%. So 2026 is partial (its fainter bar). It was weakest in 2015 and 2017, when fewer payments made a fainter fingerprint."#,
        r#"These numbers aren't measured live; they come from Litview's own run of the oracle over every block from 2,200,000 (January 2022) through 3,213,332, each measured against exchanges' LTC/USD high and low over that block's minutes: Bitfinex's minute candles from the previous block's time to its own (its last trade within the hour when none fell there, else Coinbase's hourly candle). A block inside that range counts as 0%. Before 2,200,000, prices are exchange daily closes. So 2026 is partial (its fainter bar). Its misses cluster in 2022 and 2023: twice, in March 2022 and August 2023, it settled on twice the price until its relock, which checks every 144 blocks whether a multiple of its price fits the last two days' payments better, moved it back. Since then its worst moments were crashes faster than it follows, as on October 10, 2025."#,
    ),
    (
        "oracle/index.html",
        "/** Median distance from the exchanges' range, by year (brk_oracle's README, through block 952,314). */\n      const YEARS = [[2015, 0.26], [2016, 0.33], [2017, 0.45], [2018, 0.18], [2019, 0.16], [2020, 0.1], [2021, 0.07], [2022, 0.07], [2023, 0.1], [2024, 0.1], [2025, 0.11], [2026, 0.1]];",
        "/** Median distance from the exchanges' range, by year (Litview's run, through block 3,213,332). */\n      const YEARS = [[2022, 0.24], [2023, 0.2], [2024, 0.21], [2025, 0.18], [2026, 0.12]];",
    ),
    (
        "oracle/index.html",
        "((median / 0.45) * 100)",
        "((median / 0.24) * 100)",
    ),
    // A week is 4,032 blocks at 150 seconds, not 1,008.
    (
        "oracle/index.html",
        "const WEEK = 7 * 86400, FETCH = 1300;",
        "const WEEK = 7 * 86400, FETCH = 5200;",
    ),
    ("oracle/index.html", "searchAt.size > 1008", "searchAt.size > 4032"),
    // Its search reaches 8 bins either way (brk_oracle's litecoin `Config::default`).
    (
        "oracle/index.html",
        "12 bins below (a higher price), 11 above. */\n      const BELOW = 12, ABOVE = 11;",
        "8 bins below (a higher price), 8 above. */\n      const BELOW = 8, ABOVE = 8;",
    ),
    (
        "oracle/index.html",
        "never more than about <b>16% higher</b> or <b>13% lower</b>",
        "never more than about <b>10% higher</b> or <b>9% lower</b>",
    ),
    // The Trail and Logs stay served, off the home page.
    (
        "index.html",
        "      ]);\n      // (The developers' named",
        "      ].filter((t) => ![\"trail\", \"logs\"].includes(t.id)));\n      // (The developers' named",
    ),
    (
        "index.html",
        r#"["studio", "oracle", "globe", "trail", "stack", "calendar"]"#,
        r#"["studio", "oracle", "globe", "stack", "calendar"]"#,
    ),
    // Litecoin's dates: genesis on 7 October 2011, halvings in 2015, 2019 and 2023; its price
    // moves from May 2013 (earlier days carry the first exchange close).
    (
        "index.html",
        "Every day of every year since 2009,",
        "Every day of every year since 2011,",
    ),
    ("index.html", "what any plan since 2010", "what any plan since 2013"),
    (
        "calendar/index.html",
        "Every day of every year since 2009,",
        "Every day of every year since 2011,",
    ),
    (
        "calendar/index.html",
        r#"GENESIS = "2009-01-03";"#,
        r#"GENESIS = "2011-10-07";"#,
    ),
    ("calendar/index.html", "y <= 2009;", "y <= 2011;"),
    ("calendar/index.html", "&& y > 2009,", "&& y > 2011,"),
    (
        "calendar/index.html",
        "for (let i = 0; i < 144; i++)",
        "for (let i = 0; i < 576; i++)",
    ),
    ("stack/index.html", "what any plan since 2010", "what any plan since 2013"),
    (
        "compass/index.html",
        "/** The first day read: the price begins in July 2010, nothing before it counts. */\n      const START = (Date.UTC(2010, 6, 1) - EPOCH) / DAY;",
        "/** The first day read: the price begins in May 2013, its days before the first close's carried. */\n      const START = (Date.UTC(2013, 5, 1) - EPOCH) / DAY;",
    ),
    (
        "studio/index.html",
        "/** Bitcoin's first block (3 January 2009): all of history's start, for a range over it. */\n      const GENESIS = 1230940800;",
        "/** Litecoin's first block (7 October 2011): all of history's start, for a range over it. */\n      const GENESIS = 1317945600;",
    ),
    (
        "studio/index.html",
        r#"year: [2009, "y and at are years, 2009 (the genesis block's) and on"]"#,
        r#"year: [2011, "y and at are years, 2011 (the genesis block's) and on"]"#,
    ),
    (
        "studio/index.html",
        "1 from 2012, 2 from 2016, 3 from 2020, 4 from 2024",
        "1 from 2015, 2 from 2019, 3 from 2023",
    ),
    (
        "studio/index.html",
        "(2009 the first, the genesis block's)",
        "(2011 the first, the genesis block's)",
    ),
    (
        "studio/index.html",
        r#""Every 210,000 blocks""#,
        r#""Every 840,000 blocks""#,
    ),
    (
        "studio/index.html",
        "halving (210,000 blocks)",
        "halving (840,000 blocks)",
    ),
    (
        "studio/index.html",
        "a block each ten minutes",
        "a block each two and a half minutes",
    ),
    (
        "studio/index.html",
        "a halving its 210,000 blocks at ten minutes each",
        "a halving its 840,000 blocks at two and a half minutes each",
    ),
    (
        "studio/index.html",
        "halving: (210_000 * 600) / 86400",
        "halving: (840_000 * 150) / 86400",
    ),
    ("studio/index.html", "2020-05-11", "2019-08-05"),
    ("studio/index.html", "2024-04-20", "2023-08-02"),
    (
        "studio/index.html",
        "the price's start in 2010",
        "the price's start in 2013",
    ),
    (
        "studio/index.html",
        "(price before 2010 starts at zero)",
        "(price before 2013 is its first close, carried)",
    ),
    (
        "studio/llms.txt",
        "a block each ten minutes",
        "a block each two and a half minutes",
    ),
    (
        "studio/llms.txt",
        "halving (210,000 blocks)",
        "halving (840,000 blocks)",
    ),
    ("studio/llms.txt", "2020-05-11", "2019-08-05"),
    ("studio/llms.txt", "2024-04-20", "2023-08-02"),
    (
        "studio/presets.js",
        r#"label: `${n ? 2008 + 4 * n : 2009}–${n + 1 < cycles ? 2012 + 4 * n : "now"}`"#,
        r#"label: `${2011 + 4 * n}–${n + 1 < cycles ? 2015 + 4 * n : "now"}`"#,
    ),
    (
        "studio/presets.js",
        "(date - 2009-01-03)",
        "(date - 2011-10-07)",
    ),
    // perf.bitview.dev benchmarks Bitcoin: the dev page states Litview's own run (2026-11-14, a
    // 64 GB Mac, from block 0 to 3,213,170: indexed in 3,823 s, computed in 545 s).
    (
        "index.html",
        r##"<a href="https://perf.bitview.dev">Benchmarks<svg class="glyph" aria-hidden="true"><use href="#i-out" /></svg></a>"##,
        "",
    ),
    (
        "dev/index.html",
        "        <a href=\"https://perf.bitview.dev\">Benchmarks</a>\n",
        "",
    ),
    (
        "dev/index.html",
        r#"          <div><dt>First index</dt><dd>~1½ h<small>from block 0, 16 GB Mac mini</small></dd></div>
          <div><dt>Peak memory</dt><dd>8.4 GiB<small>16 GB recommended</small></dd></div>
          <div><dt>On disk</dt><dd>290 GiB<small>beside Bitcoin Core's</small></dd></div>
          <div><dt>Series</dt><dd>21,154<small>across their indexes</small></dd></div>
          <div><dt>API operations</dt><dd>98<small>series, blocks, mempool…</small></dd></div>
        </dl>
        <p class="specs-foot">Measured on an Apple M6 Mac mini, 16 GB, external NVMe SSD; every run at <a href="https://perf.bitview.dev">perf.bitview.dev</a>.</p>"#,
        r#"          <div><dt>First index</dt><dd>~1¼ h<small>from block 0, 64 GB Mac</small></dd></div>
          <div><dt>Memory</dt><dd>4.4 GiB<small>serving at the tip</small></dd></div>
          <div><dt>On disk</dt><dd>94 GiB<small>beside Bitcoin Core's</small></dd></div>
          <div><dt>Series</dt><dd>17,030<small>across their indexes</small></dd></div>
          <div><dt>API operations</dt><dd>98<small>series, blocks, mempool…</small></dd></div>
        </dl>
        <p class="specs-foot">Measured on an Apple silicon Mac, 64 GB, internal SSD, indexing to block 3,213,170 in November 2026.</p>"#,
    ),
    (
        "dev/index.html",
        r#"<span class="label">Install, or update</span><button type="button" data-copy='rustup update &amp;&amp; RUSTFLAGS="-C target-cpu=native" cargo install --locked bitviewd'>Copy</button></div>
          <pre class="terminal"><span class="line"><span class="prompt">$</span> rustup update &amp;&amp; RUSTFLAGS="-C <span class="w">target-cpu=native</span>" cargo install <span class="w">--locked</span> bitviewd</span>"#,
        r#"<span class="label">Install</span><button type="button" data-copy='git clone https://github.com/LitecoinResearchKit/lrk &amp;&amp; cd lrk &amp;&amp; RUSTFLAGS="-C target-cpu=native" cargo install --locked --path crates/bitviewd --features litecoin --profile release-ltc'>Copy</button></div>
          <pre class="terminal"><span class="line"><span class="prompt">$</span> git clone https://github.com/LitecoinResearchKit/lrk &amp;&amp; cd lrk</span><span class="line"><span class="prompt">$</span> RUSTFLAGS="-C <span class="w">target-cpu=native</span>" cargo install <span class="w">--locked</span> --path crates/bitviewd <span class="w">--features litecoin --profile release-ltc</span></span>"#,
    ),
    (
        "dev/index.html",
        "16 GB of memory recommended, and about 290 GiB of disk on top of Bitcoin Core's.",
        "16 GB of memory or more, and about 95 GiB of disk on top of Bitcoin Core's.",
    ),
    (
        "dev/index.html",
        "About an hour and a half on a 16 GB Mac mini with a fast SSD;",
        "About an hour and a quarter on a 64 GB Mac with a fast SSD;",
    ),
    (
        "index.html",
        r#"copy: "cargo install --locked bitviewd" }"#,
        r#"copy: "git clone https://github.com/LitecoinResearchKit/lrk && cd lrk && cargo install --locked --path crates/bitviewd --features litecoin --profile release-ltc" }"#,
    ),
    (
        "index.html",
        r#"" cargo install", el("br"), "--locked bitviewd")"#,
        r#"" cargo install", el("br"), "--features litecoin")"#,
    ),
    // OpenSats supports upstream's project, not this fork.
    (
        "dev/index.html",
        "            <a class=\"button\" href=\"https://opensats.org/projects/bitcoinresearchkit\">Supported by OpenSats ↗</a>\n",
        "",
    ),
    (
        "brk/index.html",
        "            <a class=\"button\" href=\"https://opensats.org/projects/bitcoinresearchkit\">Supported by OpenSats ↗</a>\n",
        "",
    ),
];

/// Apps whose pages name no share card: each gets `litecoin/assets/cards/{app}.jpg`.
const CARDS: &[&str] = &["brk", "calendar", "changes", "globe", "logs", "oracle", "series"];

fn main() -> io::Result<()> {
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_LITECOIN");
    if env::var_os("CARGO_FEATURE_LITECOIN").is_none() {
        return Ok(());
    }
    println!("cargo::rerun-if-changed=showcases");
    println!("cargo::rerun-if-changed=litecoin");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("showcases");
    if out.exists() {
        fs::remove_dir_all(&out)?;
    }
    for (file, ..) in FACTS {
        assert!(
            Path::new("showcases").join(file).exists(),
            "no showcases/{file}"
        );
    }
    for app in CARDS {
        assert!(
            Path::new("litecoin/assets/cards").join(format!("{app}.jpg")).exists(),
            "no litecoin/assets/cards/{app}.jpg"
        );
    }
    copy(Path::new("showcases"), &out, "", true)?;
    let overlay = Path::new("litecoin");
    if overlay.exists() {
        copy(overlay, &out, "", false)?;
    }
    Ok(())
}

fn copy(from: &Path, to: &Path, relative: &str, relabel: bool) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let path = entry?.path();
        let name = path.file_name().unwrap();
        let target = to.join(name);
        let relative = format!("{relative}{}", name.to_string_lossy());
        let relabel = relabel && !VERBATIM.iter().any(|v| relative.starts_with(v));
        if path.is_dir() {
            copy(&path, &target, &format!("{relative}/"), relabel)?;
        } else if relabel
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| TEXT.contains(&e))
        {
            let mut text = fs::read_to_string(&path)?;
            for (_, from, to) in FACTS.iter().filter(|(file, ..)| *file == relative) {
                assert!(text.contains(from), "{relative} no longer has {from:?}");
                text = text.replace(from, to);
            }
            if let Some(app) = CARDS.iter().find(|app| relative == format!("{app}/index.html")) {
                assert!(!text.contains("og:image"), "{relative} names its own og:image now");
                assert!(text.contains("  </head>"), "{relative} has no \"  </head>\"");
                let card = format!(
                    r#"    <meta property="og:image" content="https://litview.space/assets/cards/{app}.jpg" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta name="twitter:card" content="summary_large_image" />
  </head>"#
                );
                text = text.replacen("  </head>", &card, 1);
            }
            let text = text.replace(FAVICON.0, FAVICON.1);
            fs::write(&target, brk_relabel::relabel(&text))?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
