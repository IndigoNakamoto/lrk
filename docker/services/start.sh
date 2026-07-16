#!/usr/bin/env bash
# Start litview LaunchAgents (plists must already be installed via install.sh).
set -euo pipefail

AGENTS="$HOME/Library/LaunchAgents"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

labels=(
  com.litview.litecoin
  com.litview.brk
  com.litview.cloudflared
  com.litview.watchdog
)

for label in "${labels[@]}"; do
  plist="$AGENTS/${label}.plist"
  if [[ ! -f "$plist" ]]; then
    echo "missing $plist — run ./docker/services/install.sh first" >&2
    exit 1
  fi

  if ! launchctl print "$DOMAIN/$label" &>/dev/null; then
    launchctl bootstrap "$DOMAIN" "$plist"
  fi
  launchctl enable "$DOMAIN/$label" 2>/dev/null || true
  # Do not kickstart -k — KeepAlive jobs block kickstart until exit.
  launchctl start "$label" 2>/dev/null || true
  echo "started $label"
done

echo
echo "Done. Logs: ~/Library/Logs/litview/"
echo "  curl -sf http://127.0.0.1:7070/health && echo OK"
