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
    // (`oracle_tune`), against each block's day's exchange close.
    (
        "oracle/index.html",
        "Math.ceil(508_000 / DAY) * DAY",
        "Math.ceil(2_200_000 / DAY) * DAY",
    ),
    (
        "oracle/index.html",
        "<strong>0.15%</strong><span>median distance from the exchanges' price range, block by block, over 596,251 blocks since January 2015.</span>",
        "<strong>1.19%</strong><span>median distance from the day's exchange close, block by block, over 1,013,333 blocks since January 2022.</span>",
    ),
    (
        "oracle/index.html",
        r#"            <div><dt>95% of blocks</dt><dd>within 1.2%</dd></div>
            <div><dt>99% of blocks</dt><dd>within 3.4%</dd></div>
            <div><dt>Since 2022</dt><dd>none off by 10%</dd></div>"#,
        r#"            <div><dt>95% of blocks</dt><dd>within 7.9%</dd></div>
            <div><dt>99% of blocks</dt><dd>within 16.2%</dd></div>
            <div><dt>Off by 10%</dt><dd>3.5% of blocks</dd></div>"#,
    ),
    (
        "oracle/index.html",
        r#"These numbers aren't measured live; they come from brk_oracle's own test, in <a href="https://github.com/bitcoinresearchkit/mono/blob/main/crates/brk_oracle/README.md#accuracy">its README</a>. It ran the oracle over every block from 340,000 (January 2015) through 952,314 and measured the 596,251 that exchange data covers against exchanges' high and low at that block; a block inside that range counts as 0%. So 2026 is partial (its fainter bar). It was weakest in 2015 and 2017, when fewer payments made a fainter fingerprint."#,
        r#"These numbers aren't measured live; they come from Litview's own run of the oracle over every block from 2,200,000 (January 2022) through 3,213,332, each measured against its day's LTC/USD exchange close, so the day's own swings count against it. Before 2,200,000, prices are those closes. So 2026 is partial (its fainter bar). It was weakest in 2022. Twice, in March 2022 and August 2023, it settled on twice the price until its relock, which checks every 144 blocks whether a multiple of its price fits the last two days' payments better, moved it back."#,
    ),
    (
        "oracle/index.html",
        "/** Median distance from the exchanges' range, by year (brk_oracle's README, through block 952,314). */\n      const YEARS = [[2015, 0.26], [2016, 0.33], [2017, 0.45], [2018, 0.18], [2019, 0.16], [2020, 0.1], [2021, 0.07], [2022, 0.07], [2023, 0.1], [2024, 0.1], [2025, 0.11], [2026, 0.1]];",
        "/** Median distance from the day's exchange close, by year (Litview's run, through block 3,213,332). */\n      const YEARS = [[2022, 1.96], [2023, 1.04], [2024, 1.09], [2025, 1.28], [2026, 0.87]];",
    ),
    (
        "oracle/index.html",
        "((median / 0.45) * 100)",
        "((median / 1.96) * 100)",
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
];

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
            fs::write(&target, brk_relabel::relabel(&text))?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
