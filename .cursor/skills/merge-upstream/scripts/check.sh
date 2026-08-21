#!/usr/bin/env bash
# Non-destructive Litecoin-merge gate. Run from the repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$ROOT"

fail=0
say() { printf '%s\n' "$*"; }
bad() { say "FAIL: $*"; fail=1; }
ok() { say "OK: $*"; }

CLIENT="modules/brk-client/index.js"

if [[ ! -f "$CLIENT" ]]; then
  bad "missing $CLIENT"
else
  grep -q 'mweb:' "$CLIENT" && grep -q "mweb_balance" "$CLIENT" \
    && ok "client has outputs.mweb" \
    || bad "client missing outputs.mweb — run bindgen with --features bindgen,litecoin"
  grep -q 'hogex:' "$CLIENT" \
    && ok "client has transactions.hogex" \
    || bad "client missing transactions.hogex"
fi

grep -q 'catch_unwind' crates/brk_cli/src/main.rs \
  && ok "cli catch_unwind present" \
  || bad "cli lost catch_unwind around computer.compute"

grep -q 'panic = "unwind"' Cargo.toml \
  && ok 'release panic = "unwind"' \
  || bad 'Cargo.toml release profile is not panic = "unwind"'

if grep -n 'panic!.*underflow\|panic!("Sats underflow' \
  crates/brk_types/src/sats.rs \
  crates/brk_types/src/supply_state.rs \
  crates/brk_types/src/funded_addr_data.rs \
  crates/brk_computer/src/distribution/state/cost_basis/data.rs \
  >/tmp/lrk-merge-panics.txt 2>/dev/null; then
  bad "underflow panics returned:"
  cat /tmp/lrk-merge-panics.txt
else
  ok "no underflow panic! on the known surface"
fi

grep -q 'safeSection' website/scripts/options/partial.js \
  && ok "website safeSection present" \
  || bad "website/scripts/options/partial.js lost safeSection"

grep -q 'MWEBPegPool' crates/brk_indexer/src/processor/txout/mod.rs \
  && ok "indexer handles MWEB output types" \
  || bad "indexer txout no longer matches MWEBPegPool / MWEBPegIn"

say ""
say "Running cargo test --features litecoin (types, rpc, computer)..."
if CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" cargo test --features litecoin --lib \
  -p brk_types -p brk_rpc -p brk_computer; then
  ok "cargo test --features litecoin"
else
  bad "cargo test --features litecoin"
fi

say ""
if [[ "$fail" -ne 0 ]]; then
  say "merge-upstream check FAILED"
  exit 1
fi
say "merge-upstream check passed"
exit 0
