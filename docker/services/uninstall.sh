#!/usr/bin/env bash
set -euo pipefail

AGENTS="$HOME/Library/LaunchAgents"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

labels=(
  com.litview.watchdog
  com.litview.cloudflared
  com.litview.brk
  com.litview.litecoin
)

for label in "${labels[@]}"; do
  launchctl bootout "$DOMAIN/$label" 2>/dev/null || true
  rm -f "$AGENTS/${label}.plist"
  echo "unloaded $label"
done
