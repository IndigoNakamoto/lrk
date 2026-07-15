#!/usr/bin/env bash
# Run native BRK (litecoin-featured) on :7070 for the Cloudflare tunnel.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BRK_BIN="${BRK_BIN:-$ROOT/target/release/brk}"
DATADIR="${LITECOIN_DATADIR:-$HOME/Library/Application Support/Litecoin}"
BRKDIR="${BRK_DATA_DIR:-$HOME/.brk}"
PORT="${BRK_PORT:-7070}"

if [[ ! -x "$BRK_BIN" ]]; then
  echo "brk: missing binary at $BRK_BIN (build with: cargo build --release -p brk_cli --features litecoin)" >&2
  exit 1
fi

# Prefer cookie when present; otherwise ~/.brk/config.toml rpcuser/rpcpassword.
exec "$BRK_BIN" \
  --chain litecoin \
  --brkport "$PORT" \
  --bitcoindir "$DATADIR" \
  --blocksdir "$DATADIR/blocks" \
  --brkdir "$BRKDIR"
