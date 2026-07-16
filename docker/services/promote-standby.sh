#!/usr/bin/env bash
# Promote this host to serve litview.space via cloudflared (standby failover).
# Requires local BRK /health OK and the cloudflared LaunchAgent plist installed.
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
AGENTS="$HOME/Library/LaunchAgents"
LOG_DIR="${LITVIEW_LOG_DIR:-$HOME/Library/Logs/litview}"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/failover.log"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"
LABEL=com.litview.cloudflared
PLIST="$AGENTS/${LABEL}.plist"
BRK_PORT="${BRK_PORT:-7070}"

log() {
  printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG"
}

if [[ ! -f "$PLIST" ]]; then
  log "ERROR missing $PLIST — run: LITVIEW_ROLE=standby ./docker/services/install.sh"
  exit 1
fi

if ! curl -sf --max-time 3 "http://127.0.0.1:${BRK_PORT}/health" >/dev/null; then
  log "ERROR local brk /health not OK on :${BRK_PORT} — refuse promote"
  exit 1
fi

if pgrep -qf 'cloudflared tunnel'; then
  log "INFO cloudflared already running — nothing to do"
  exit 0
fi

log "INFO promoting standby — loading $LABEL"
launchctl bootout "$DOMAIN/$LABEL" 2>/dev/null || true
launchctl bootstrap "$DOMAIN" "$PLIST"
launchctl enable "$DOMAIN/$LABEL" 2>/dev/null || true
launchctl start "$LABEL" 2>/dev/null || \
  launchctl kickstart "$DOMAIN/$LABEL" 2>/dev/null || true

sleep 2
if pgrep -qf 'cloudflared tunnel'; then
  log "OK cloudflared running after promote"
else
  log "ERROR cloudflared did not start — check $LOG_DIR/cloudflared.err.log"
  exit 1
fi
