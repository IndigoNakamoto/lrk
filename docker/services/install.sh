#!/usr/bin/env bash
# Install litview LaunchAgents (litecoin, native brk, cloudflared, watchdog).
#
# LITVIEW_ROLE=primary (default): litecoin + brk + cloudflared + watchdog
# LITVIEW_ROLE=standby:          litecoin + brk + watchdog (cloudflared plist
#                                installed but not loaded — use promote-standby.sh)
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
AGENTS="$HOME/Library/LaunchAgents"
LOGS="$HOME/Library/Logs/litview"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"
ROLE="${LITVIEW_ROLE:-primary}"

# Allow role from docker/.env when not set in the environment.
ENV_FILE="${LITVIEW_ENV:-$DIR/../.env}"
if [[ -z "${LITVIEW_ROLE:-}" && -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a
  source "$ENV_FILE"
  set +a
  ROLE="${LITVIEW_ROLE:-primary}"
fi

case "$ROLE" in
  primary|standby) ;;
  *)
    echo "install: LITVIEW_ROLE must be primary or standby (got: $ROLE)" >&2
    exit 1
    ;;
esac

mkdir -p "$AGENTS" "$LOGS"
chmod +x "$DIR"/*.sh

# Always write these plists; cloudflared is only *loaded* on primary.
core_labels=(
  com.litview.litecoin
  com.litview.brk
  com.litview.watchdog
)
tunnel_label=com.litview.cloudflared

echo "Role: $ROLE"
echo "Stopping Docker brk (native brk will own :7070 + ~/.brk)..."
docker update --restart=no brk 2>/dev/null || true
docker compose -f "$DIR/../docker-compose.yml" stop brk 2>/dev/null || true

echo "Stopping manual cloudflared (if any)..."
pkill -f 'cloudflared tunnel --config' 2>/dev/null || true
sleep 1

# Plists may hardcode another checkout path (e.g. Projects/lrk); rewrite to this repo.
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

install_plist() {
  local label="$1"
  local src="$DIR/${label}.plist"
  local dst="$AGENTS/${label}.plist"
  # BSD sed: plain substitutions (no -E) so both checkout paths rewrite cleanly.
  sed -e "s|/Users/indigo/Projects/lrk|${REPO_ROOT}|g" \
      -e "s|/Users/indigo/Dev/lrk|${REPO_ROOT}|g" \
      "$src" >"$dst"
}

load_label() {
  local label="$1"
  launchctl bootout "$DOMAIN/$label" 2>/dev/null || true
  launchctl bootstrap "$DOMAIN" "$AGENTS/${label}.plist"
  launchctl enable "$DOMAIN/$label" 2>/dev/null || true
  # Do not kickstart -k here — KeepAlive jobs block kickstart until exit.
  launchctl start "$label" 2>/dev/null || true
  echo "  loaded $label"
}

echo "Installing plists (repo root: $REPO_ROOT)..."
for label in "${core_labels[@]}" "$tunnel_label"; do
  install_plist "$label"
done

for label in "${core_labels[@]}"; do
  load_label "$label"
done

if [[ "$ROLE" == "primary" ]]; then
  load_label "$tunnel_label"
else
  # Standby: ensure tunnel is not running; plist stays on disk for promote-standby.sh.
  launchctl bootout "$DOMAIN/$tunnel_label" 2>/dev/null || true
  pkill -f 'cloudflared tunnel --config' 2>/dev/null || true
  echo "  installed $tunnel_label (not loaded — standby)"
fi

echo
echo "Done. Role=$ROLE  Logs: $LOGS"
echo "  launchctl print $DOMAIN/com.litview.brk | head"
echo "  tail -f $LOGS/brk.out.log"
if [[ "$ROLE" == "standby" ]]; then
  echo "  Standby failover: ./docker/services/promote-standby.sh"
  echo "  Failback:         ./docker/services/demote-standby.sh"
fi
echo
echo "Note: leave Docker Desktop brk stopped. Index resumes from ~/.brk."
