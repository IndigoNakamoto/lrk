#!/usr/bin/env bash
# Run native bitviewd (litecoin-featured) on :7070 for the Cloudflare tunnel.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BRK_BIN="${BRK_BIN:-$ROOT/target/release-ltc/bitviewd}"
DATADIR="${LITECOIN_DATADIR:-$HOME/Library/Application Support/Litecoin}"
BRKDIR="${BITVIEW_DATA_DIR:-${BRK_DATA_DIR:-$HOME/.bitview}}"
PORT="${BRK_PORT:-7070}"

if [[ ! -x "$BRK_BIN" ]]; then
  echo "bitviewd: missing binary at $BRK_BIN (build with: cargo build --profile release-ltc -p bitviewd --features litecoin)" >&2
  exit 1
fi

# Prefer cookie when present; otherwise config.toml rpcuser/rpcpassword.
# Force IPv4 — Litecoin listens on *:9332 (v4); localhost can resolve to ::1.
# Litecoin builds embed Litview showcases (relabeled + litecoin/ overlay). Use
# BRK_WEBSITE=true (default) for that; a path serves a filesystem tree instead.
WEBSITE="${BRK_WEBSITE:-true}"
exec "$BRK_BIN" \
  --serverport "$PORT" \
  --rpcconnect 127.0.0.1 \
  --bitcoindir "$DATADIR" \
  --blocksdir "$DATADIR/blocks" \
  --bitviewdir "$BRKDIR" \
  --website "$WEBSITE"
