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
/// Bitcoin's chain facts written into the pages, and Litecoin's: (path, upstream's text, Litecoin's),
/// every occurrence replaced before relabeling. Each must still be found, so an upstream rewrite
/// fails the build rather than ship Bitcoin's numbers.
const FACTS: &[(&str, &str, &str)] = &[
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
