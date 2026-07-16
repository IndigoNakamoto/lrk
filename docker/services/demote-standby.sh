#!/usr/bin/env bash
# Demote this host: stop cloudflared so the primary can own the tunnel again.
# Does not stop Litecoin or BRK (warm standby stays indexed).
set -euo pipefail

LOG_DIR="${LITVIEW_LOG_DIR:-$HOME/Library/Logs/litview}"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/failover.log"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"
LABEL=com.litview.cloudflared

log() {
  printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG"
}

log "INFO demoting standby — unloading $LABEL"
launchctl bootout "$DOMAIN/$LABEL" 2>/dev/null || true
pkill -f 'cloudflared tunnel --config' 2>/dev/null || true
sleep 1

if pgrep -qf 'cloudflared tunnel'; then
  log "WARN cloudflared still present after demote"
  exit 1
fi

log "OK cloudflared stopped (warm standby)"
