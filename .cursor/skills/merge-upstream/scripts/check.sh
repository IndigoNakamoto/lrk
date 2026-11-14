#!/usr/bin/env bash
# Non-destructive Litecoin-merge gate for the Bitview layout. Run from the repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

fail=0
say() { printf '%s\n' "$*"; }
bad() { say "FAIL: $*"; fail=1; }
ok() { say "OK: $*"; }
# check <file> <pattern> <ok message> <fail message>
check() {
  if grep -q -- "$2" "$1" 2>/dev/null; then ok "$3"; else bad "$4"; fi
}

CLIENT="modules/bitview-client/index.js"
check "$CLIENT" "mwebSupply" "client has mweb.mwebSupply" \
  "client missing mweb series: run cargo bindgen --features litecoin"
check "$CLIENT" "totalSupply" "client has supply.totalSupply" \
  "client missing supply.totalSupply: run cargo bindgen --features litecoin"
check "$CLIENT" "mwebPegPool" "client has the MWEB output types" \
  "client missing mwebPegPool / mwebPegIn output types"

check Cargo.toml '\[profile.release-ltc\]' "release-ltc profile present" \
  "Cargo.toml lost [profile.release-ltc] (panic = \"unwind\")"
check crates/bitview/src/update.rs 'catch_unwind' "update catch_unwind present" \
  "crates/bitview/src/update.rs lost the litecoin catch_unwind around compute"
check crates/brk_types/src/sats.rs 'feature = "litecoin"' "Sats saturating SubAssign present" \
  "crates/brk_types/src/sats.rs lost the litecoin saturating SubAssign"
check crates/bitview_primitives/src/funded_addr_data.rs 'feature = "litecoin"' \
  "address balance clamp present" \
  "crates/bitview_primitives/src/funded_addr_data.rs lost the litecoin balance clamp"

check crates/brk_types/src/output_type.rs 'MWEBPegPool' "MWEB output types present" \
  "brk_types OutputType lost MWEBPegPool / MWEBPegIn"
check crates/bitview_plugin_outputs/src/value/compute.rs 'is_mweb()' \
  "creations journal skips MWEB outputs" \
  "outputs plugin creations no longer skip MWEB (distribution would count the HogAddr)"
check crates/bitview_plugin_inputs/src/spends.rs 'is_mweb()' "spends journal skips MWEB" \
  "inputs plugin spends no longer skip MWEB outputs"
check crates/bitview_plugin_utxos/src/state/received.rs 'is_mweb()' \
  "UTXO cohorts skip MWEB outputs" \
  "utxos plugin would panic on MWEB outputs (SpendableType::get_mut)"
check crates/bitview_plugin_transactions/src/fees/compute/block.rs 'hogex' \
  "fee pass excludes HogEx from volume and distributions" \
  "transactions fee pass lost the HogEx exclusion"
check crates/bitview_default/src/lib.rs 'bitview_plugin_mweb' "MWEB plugin registered" \
  "bitview_default no longer composes bitview_plugin_mweb"
check showcases/studio/presets.js 't.mweb' "Studio MWEB presets present (guarded)" \
  "showcases/studio/presets.js lost the guarded MWEB section"

say ""
say "Checking both builds..."
cargo check -q -p bitviewd --features litecoin && ok "litecoin build" || bad "litecoin build"
cargo check -q --workspace && ok "bitcoin build" || bad "bitcoin build"

say ""
say "Running litecoin tests..."
if cargo test -q --features litecoin \
  -p brk_types -p brk_rpc -p brk_reader -p bitview_primitives \
  -p bitview_plugin_mweb -p bitview_plugin_supply -p bitview_plugin_transactions; then
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
