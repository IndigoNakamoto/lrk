#!/usr/bin/env bash
# Stop litview LaunchAgents (keeps plists installed; use start.sh to resume).
set -euo pipefail

UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

# Watchdog first so it does not kickstart anything we are stopping.
labels=(
  com.litview.watchdog
  com.litview.cloudflared
  com.litview.brk
  com.litview.litecoin
)

for label in "${labels[@]}"; do
  if launchctl print "$DOMAIN/$label" &>/dev/null; then
    launchctl bootout "$DOMAIN/$label" 2>/dev/null || true
    echo "stopped $label"
  else
    echo "already stopped $label"
  fi
done

echo
echo "Services stopped. Plists remain in ~/Library/LaunchAgents."
echo "Resume with: ./docker/services/start.sh"
