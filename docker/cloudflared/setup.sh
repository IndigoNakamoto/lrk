#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "=== LRK Cloudflare Tunnel setup ==="
echo

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "Note: cloudflared is not installed yet (brew install cloudflared)"
  echo
fi

if [[ ! -f config.yml ]]; then
  cp config.yml.example config.yml
  echo "Created config.yml from config.yml.example"
else
  echo "config.yml already exists (left unchanged)"
fi

if [[ ! -f .env ]]; then
  cp .env.example .env
  echo "Created .env from .env.example"
fi

echo
echo "Fill in config.yml, then run the steps in README.md:"
echo
echo "  1. cloudflared tunnel login"
echo "  2. cloudflared tunnel create \${TUNNEL_NAME:-litview-demo}"
echo "  3. Edit config.yml:"
echo "       - tunnel: <UUID from step 2>"
echo "       - credentials-file: ./credentials.json"
echo "     Then: cp ~/.cloudflared/<UUID>.json ./credentials.json"
echo "  4. ./route-dns.sh"
echo "  5. docker compose -f ../docker-compose.yml up -d"
echo "  6. ./start.sh"
echo
