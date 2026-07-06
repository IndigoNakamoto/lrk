#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="$DIR/config.yml"
COMPOSE="$DIR/../docker-compose.yml"

if [[ ! -f "$CONFIG" ]]; then
  echo "Missing $CONFIG"
  echo "Run ./setup.sh first, then fill in config.yml"
  exit 1
fi

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared is not installed. Run: brew install cloudflared"
  exit 1
fi

if ! curl -sf --max-time 2 http://localhost:7070/health >/dev/null 2>&1; then
  echo "Warning: LRK is not responding at http://localhost:7070/health"
  echo "Start it with: docker compose -f $COMPOSE up -d"
  echo
fi

echo "Starting Cloudflare Tunnel (Ctrl+C to stop)..."
exec cloudflared tunnel --config "$CONFIG" run
