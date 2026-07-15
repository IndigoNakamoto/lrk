#!/usr/bin/env bash
# Lightweight health checks + launchd kickstarts. Logs to ~/Library/Logs/litview/.
set -euo pipefail

UID_NUM="$(id -u)"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ENV_FILE="${LITVIEW_ENV:-$ROOT/docker/.env}"
LOG_DIR="${LITVIEW_LOG_DIR:-$HOME/Library/Logs/litview}"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/watchdog.log"
DISK_WARN_GB="${DISK_WARN_GB:-50}"

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

log() {
  printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >>"$LOG"
}

kick() {
  local label="$1"
  launchctl kickstart -k "gui/${UID_NUM}/${label}" 2>>"$LOG" || \
    launchctl kickstart "gui/${UID_NUM}/${label}" 2>>"$LOG" || true
}

# Disk headroom (Data volume). Soft warn only — no free space action.
avail_gb="$(df -g /System/Volumes/Data 2>/dev/null | awk 'NR==2 {print $4}')"
if [[ -n "${avail_gb:-}" ]] && [[ "$avail_gb" -lt "$DISK_WARN_GB" ]]; then
  log "WARN disk ${avail_gb}GB free (< ${DISK_WARN_GB}GB)"
fi

# Litecoin RPC (creds from docker/.env when present)
if ! curl -sf --max-time 3 --user "${RPC_USER:-litecoin}:${RPC_PASSWORD:-litecoin}" \
  --data-binary '{"jsonrpc":"1.0","id":"wd","method":"getblockcount","params":[]}' \
  -H 'content-type: text/plain;' \
  "http://127.0.0.1:${RPC_PORT:-9332}/" >/dev/null 2>&1; then
  log "WARN litecoin RPC down — kickstarting com.litview.litecoin"
  kick com.litview.litecoin
fi

# BRK HTTP (only alert/kick if process should be managed)
if ! curl -sf --max-time 3 "http://127.0.0.1:${BRK_PORT:-7070}/health" >/dev/null 2>&1; then
  # During deep sync BRK may reset connections; only kick if binary not running.
  if ! pgrep -qx brk; then
    log "WARN brk not running — kickstarting com.litview.brk"
    kick com.litview.brk
  else
    log "INFO brk process up but /health not ready (likely indexing)"
  fi
fi

# Tunnel
if ! pgrep -qf 'cloudflared tunnel'; then
  log "WARN cloudflared down — kickstarting com.litview.cloudflared"
  kick com.litview.cloudflared
fi
