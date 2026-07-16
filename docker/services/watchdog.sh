#!/usr/bin/env bash
# Lightweight health checks + launchd kickstarts. Logs to ~/Library/Logs/litview/.
#
# LITVIEW_ROLE=primary (default): keep litecoin, brk, and cloudflared alive.
# LITVIEW_ROLE=standby: keep litecoin + brk; do not restart cloudflared.
#   If public /health fails N times and local /health is OK → promote-standby.sh.
set -euo pipefail

UID_NUM="$(id -u)"
DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
ENV_FILE="${LITVIEW_ENV:-$ROOT/docker/.env}"
LOG_DIR="${LITVIEW_LOG_DIR:-$HOME/Library/Logs/litview}"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/watchdog.log"
DISK_WARN_GB="${DISK_WARN_GB:-50}"
STATE_FILE="$LOG_DIR/standby-public-failures"
ROLE="${LITVIEW_ROLE:-primary}"
PUBLIC_HEALTH_URL="${PUBLIC_HEALTH_URL:-https://litview.space/health}"
FAILOVER_FAILURES="${FAILOVER_FAILURES:-3}"
BRK_PORT="${BRK_PORT:-7070}"

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
  ROLE="${LITVIEW_ROLE:-$ROLE}"
  PUBLIC_HEALTH_URL="${PUBLIC_HEALTH_URL:-https://litview.space/health}"
  FAILOVER_FAILURES="${FAILOVER_FAILURES:-3}"
  BRK_PORT="${BRK_PORT:-7070}"
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

# Litecoin RPC: cookie auth (default) or RPC_USER/RPC_PASSWORD from docker/.env
rpc_auth_args=()
cookie_file="${CHAIN_DATA_DIR:-$HOME/Library/Application Support/Litecoin}/.cookie"
if [[ -n "${RPC_USER:-}" && -n "${RPC_PASSWORD:-}" ]]; then
  rpc_auth_args=(--user "${RPC_USER}:${RPC_PASSWORD}")
elif [[ -f "$cookie_file" ]]; then
  rpc_auth_args=(--user "$(cat "$cookie_file")")
else
  rpc_auth_args=(--user "litecoin:litecoin")
fi

if ! curl -sf --max-time 3 "${rpc_auth_args[@]}" \
  --data-binary '{"jsonrpc":"1.0","id":"wd","method":"getblockcount","params":[]}' \
  -H 'content-type: text/plain;' \
  "http://127.0.0.1:${RPC_PORT:-9332}/" >/dev/null 2>&1; then
  log "WARN litecoin RPC down — kickstarting com.litview.litecoin"
  kick com.litview.litecoin
fi

local_brk_ok=0
if curl -sf --max-time 3 "http://127.0.0.1:${BRK_PORT}/health" >/dev/null 2>&1; then
  local_brk_ok=1
else
  # During deep sync BRK may reset connections; only kick if binary not running.
  if ! pgrep -qx brk; then
    log "WARN brk not running — kickstarting com.litview.brk"
    kick com.litview.brk
  else
    log "INFO brk process up but /health not ready (likely indexing)"
  fi
fi

if [[ "$ROLE" == "standby" ]]; then
  # Standby: never kickstart the tunnel. Auto-promote when public origin is down.
  if [[ "$local_brk_ok" -eq 1 ]]; then
    if curl -sf --max-time 5 "$PUBLIC_HEALTH_URL" >/dev/null 2>&1; then
      printf '0\n' >"$STATE_FILE"
    else
      fails=0
      if [[ -f "$STATE_FILE" ]]; then
        fails="$(cat "$STATE_FILE" 2>/dev/null || echo 0)"
      fi
      # Non-numeric guard
      case "$fails" in
        ''|*[!0-9]*) fails=0 ;;
      esac
      fails=$((fails + 1))
      printf '%s\n' "$fails" >"$STATE_FILE"
      log "WARN public health failed ($fails/${FAILOVER_FAILURES}): $PUBLIC_HEALTH_URL"
      if [[ "$fails" -ge "$FAILOVER_FAILURES" ]]; then
        if pgrep -qf 'cloudflared tunnel'; then
          log "INFO public down but cloudflared already promoted"
          printf '0\n' >"$STATE_FILE"
        else
          log "WARN promoting standby after ${fails} public health failures"
          if "$DIR/promote-standby.sh" >>"$LOG_DIR/failover.log" 2>&1; then
            printf '0\n' >"$STATE_FILE"
          else
            log "ERROR promote-standby.sh failed"
          fi
        fi
      fi
    fi
  fi
else
  # Primary: keep the tunnel alive.
  if ! pgrep -qf 'cloudflared tunnel'; then
    log "WARN cloudflared down — kickstarting com.litview.cloudflared"
    kick com.litview.cloudflared
  fi
fi
