#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

if [[ -f .env ]]; then
  # shellcheck disable=SC1091
  source .env
fi

TUNNEL_NAME="${TUNNEL_NAME:-litview-demo}"
HOSTNAME="${HOSTNAME:-litview.space}"

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared is not installed. Run: brew install cloudflared"
  exit 1
fi

echo "Routing DNS for tunnel '$TUNNEL_NAME':"
echo "  $HOSTNAME"
echo "  www.$HOSTNAME"
echo

cloudflared tunnel route dns "$TUNNEL_NAME" "$HOSTNAME"
cloudflared tunnel route dns "$TUNNEL_NAME" "www.$HOSTNAME"

echo
echo "Done. DNS may take a minute to propagate."
