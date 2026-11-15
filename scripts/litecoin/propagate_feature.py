#!/usr/bin/env python3
"""Keep the `litecoin` Cargo feature forwarded through the whole workspace.

Cargo unifies features per build, so once `brk_chain/litecoin` is on anywhere every crate sees
Litecoin primitives. A crate with `cfg(feature = "litecoin")` code that is not itself switched on
then fails to compile (or silently runs its Bitcoin branch). This script gives every workspace
crate that depends on a crate with a `litecoin` feature its own `litecoin` feature forwarding to
all such dependencies, iterating to a fixpoint. Re-run after adding cfg code to a new crate:

    python3 scripts/litecoin/propagate_feature.py
"""

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "litecoin"


def manifests():
    for path in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        yield path
    yield ROOT / "examples/custom_plugin/Cargo.toml"


def package_name(data):
    return data["package"]["name"]


def deps(data):
    """(dependency key, package name, optional) for normal dependencies."""
    out = []
    for key, spec in data.get("dependencies", {}).items():
        if isinstance(spec, dict):
            package = spec.get("package", key)
            out.append((key, package, bool(spec.get("optional"))))
        else:
            out.append((key, key, False))
    return out


def workspace_packages():
    root = tomllib.loads((ROOT / "Cargo.toml").read_text())
    renames = {}
    for key, spec in root["workspace"]["dependencies"].items():
        if isinstance(spec, dict) and "path" in spec:
            renames[key] = spec.get("package", key)
    return renames


def set_feature(path, entries):
    text = path.read_text()
    line = f"{FEATURE} = [{', '.join(f'\"{e}\"' for e in entries)}]"
    if re.search(rf"^{FEATURE} = \[.*\]$", text, re.M):
        new = re.sub(rf"^{FEATURE} = \[.*\]$", line, text, count=1, flags=re.M)
    elif re.search(r"^\[features\]$", text, re.M):
        new = re.sub(r"^\[features\]\n", f"[features]\n{line}\n", text, count=1, flags=re.M)
    else:
        new = re.sub(r"^\[dependencies\]$", f"[features]\n{line}\n\n[dependencies]", text, count=1, flags=re.M)
    if new != text:
        path.write_text(new)
        return True
    return False


def main():
    renames = workspace_packages()
    paths = {}
    data = {}
    for path in manifests():
        d = tomllib.loads(path.read_text())
        name = package_name(d)
        paths[name] = path
        data[name] = d

    def has_feature(name):
        return FEATURE in tomllib.loads(paths[name].read_text()).get("features", {})

    changed = True
    rounds = 0
    while changed:
        changed = False
        rounds += 1
        for name, d in data.items():
            entries = set()
            current = tomllib.loads(paths[name].read_text()).get("features", {}).get(FEATURE, [])
            entries.update(e for e in current if "/" not in e or e.split("/")[0].rstrip("?") not in {k for k, _, _ in deps(d)})
            for key, package, optional in deps(d):
                package = renames.get(key, package) if key in renames and package == key else package
                if package in paths and has_feature(package):
                    entries.add(f"{key}{'?' if optional else ''}/{FEATURE}")
            if entries and set(entries) != set(current):
                if set_feature(paths[name], sorted(entries)):
                    changed = True
    print(f"litecoin feature propagated in {rounds} rounds")


if __name__ == "__main__":
    main()
