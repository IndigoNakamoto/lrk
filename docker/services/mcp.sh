#!/usr/bin/env bash
# Run the Litview MCP server on :3111 for the Cloudflare tunnel (mcp.litview.space).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MCP_BIN="${MCP_BIN:-$ROOT/target/release-ltc/bitview_mcp}"

if [[ ! -x "$MCP_BIN" ]]; then
  echo "bitview_mcp: missing binary at $MCP_BIN (build with: cargo build --profile release-ltc -p bitview_mcp --features litecoin)" >&2
  exit 1
fi

# The public API origin, not :7070, so tool calls reuse Cloudflare's cache.
exec "$MCP_BIN" \
  --api "${MCP_API:-https://litview.space}" \
  --url "${MCP_URL:-https://mcp.litview.space/}" \
  --name "${MCP_NAME:-Litview}"
