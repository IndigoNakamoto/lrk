//! Under `litecoin`, the showcases to embed are Litview's: a copy of `showcases/` with its display
//! text relabeled, then `litecoin/` laid over it for what text rules can't change (images).

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[path = "src/relabel.rs"]
#[allow(dead_code)]
mod relabel;

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

fn main() -> io::Result<()> {
    println!("cargo::rerun-if-changed=src/relabel.rs");
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
            let text = fs::read_to_string(&path)?;
            fs::write(&target, relabel::relabel(&text))?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
