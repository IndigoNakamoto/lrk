#!/usr/bin/env bash
# Install litview LaunchAgents (native brk, watchdog; optional litecoin/cloudflared).
#
# Scripts + brk binary are copied to ~/Library/Application Support/litview so
# launchd can exec them. Chain/index data may live on an external volume via
# LITECOIN_DATADIR / BRK_DATA_DIR (see docker/.env).
#
# macOS TCC: LaunchAgents cannot write to removable volumes unless the user
# grants Full Disk Access / Removable Volumes. When LITECOIN_DATADIR is under
# /Volumes, litecoin is NOT installed as a LaunchAgent — start it from Terminal
# with start-litecoin.sh instead.
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"
AGENTS="$HOME/Library/LaunchAgents"
LOGS="$HOME/Library/Logs/litview"
SUPPORT="$HOME/Library/Application Support/litview"
UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

ENV_FILE="${LITVIEW_ENV:-$DIR/../.env}"
if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

LITECOIN_DATADIR="${LITECOIN_DATADIR:-${CHAIN_DATA_DIR:-$HOME/Library/Application Support/Litecoin}}"
# Strip quotes if present from .env
LITECOIN_DATADIR="${LITECOIN_DATADIR%\"}"
LITECOIN_DATADIR="${LITECOIN_DATADIR#\"}"
BRK_DATA_DIR="${BRK_DATA_DIR:-$HOME/.brk}"

mkdir -p "$AGENTS" "$LOGS" "$SUPPORT/services" "$SUPPORT/bin"
chmod +x "$DIR"/*.sh

echo "Deploying scripts + brk binary to $SUPPORT ..."
cp -f "$DIR/litecoin.sh" "$DIR/brk.sh" "$DIR/watchdog.sh" "$SUPPORT/services/"
# Prefer repo release binary; fall back to existing deploy.
if [[ -x "$REPO_ROOT/target/release/brk" ]]; then
  cp -f "$REPO_ROOT/target/release/brk" "$SUPPORT/bin/brk"
elif [[ ! -x "$SUPPORT/bin/brk" ]]; then
  echo "error: missing $REPO_ROOT/target/release/brk — build with:" >&2
  echo "  CARGO_TARGET_DIR=$REPO_ROOT/target cargo build --release -p brk_cli --features litecoin" >&2
  exit 1
fi
chmod +x "$SUPPORT/services/"*.sh "$SUPPORT/bin/brk"

# Local copies tuned for Application Support layout
sed -i '' \
  -e 's|^ROOT=.*|ROOT="${LITVIEW_ROOT:-$HOME/Library/Application Support/litview}"|' \
  -e 's|BRK_BIN="${BRK_BIN:-$ROOT/target/release/brk}"|BRK_BIN="${BRK_BIN:-$ROOT/bin/brk}"|' \
  "$SUPPORT/services/brk.sh"
sed -i '' \
  -e 's|^ROOT=.*|ROOT="${LITVIEW_ROOT:-$HOME/Library/Application Support/litview}"|' \
  "$SUPPORT/services/watchdog.sh"

cp -f "$ENV_FILE" "$SUPPORT/.env" 2>/dev/null || true

# Helper to start litecoind from an interactive session (has Removable Volume access)
cat > "$SUPPORT/start-litecoin.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
QT="\${LITECOIN_QT:-/Applications/Litecoin-Qt.app/Contents/MacOS/Litecoin-Qt}"
DATADIR="\${LITECOIN_DATADIR:-$LITECOIN_DATADIR}"
PIDFILE="\${LITECOIN_PIDFILE:-\$DATADIR/litecoind.pid}"
mkdir -p "\$DATADIR"
if [[ -f "\$PIDFILE" ]] && kill -0 "\$(cat "\$PIDFILE" 2>/dev/null)" 2>/dev/null; then
  echo "litecoind already running pid \$(cat "\$PIDFILE")"
  exit 0
fi
# Keep Mac awake while syncing (24h); re-run as needed
if ! pgrep -qf 'caffeinate -dims sleep'; then
  nohup caffeinate -dims sleep 86400 >/tmp/litecoin-caffeinate.log 2>&1 &
fi
exec "\$QT" -daemon -datadir="\$DATADIR" -pid="\$PIDFILE"
EOF
chmod +x "$SUPPORT/start-litecoin.sh"

echo "Stopping Docker brk (native brk will own :7070)..."
docker update --restart=no brk 2>/dev/null || true
docker compose -f "$DIR/../docker-compose.yml" stop brk 2>/dev/null || true

echo "Stopping manual litview cloudflared (if any)..."
# Only the litview config — do not touch other Cloudflare tunnels on this Mac.
pkill -f "cloudflared tunnel --config ${DIR}/../cloudflared/config.yml" 2>/dev/null || true
sleep 1

write_plist() {
  local label="$1"
  local program="$2"
  local logname="$3"
  local keepalive="${4:-false}"
  local interval="${5:-}"
  local dst="$AGENTS/${label}.plist"
  {
    cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${program}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
PLIST
    if [[ "$keepalive" == "true" ]]; then
      cat <<PLIST
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>30</integer>
PLIST
    fi
    if [[ -n "$interval" ]]; then
      cat <<PLIST
  <key>StartInterval</key>
  <integer>${interval}</integer>
PLIST
    fi
    cat <<PLIST
  <key>StandardOutPath</key>
  <string>${LOGS}/${logname}.out.log</string>
  <key>StandardErrorPath</key>
  <string>${LOGS}/${logname}.err.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
    <key>LITECOIN_DATADIR</key>
    <string>${LITECOIN_DATADIR}</string>
    <key>BRK_DATA_DIR</key>
    <string>${BRK_DATA_DIR}</string>
    <key>BRK_BIN</key>
    <string>${SUPPORT}/bin/brk</string>
    <key>LITVIEW_ENV</key>
    <string>${SUPPORT}/.env</string>
    <key>BRK_WEBSITE</key>
    <string>${REPO_ROOT}/website</string>
    <key>LOG</key>
    <string>info</string>
  </dict>
</dict>
</plist>
PLIST
  } >"$dst"
}

load_label() {
  local label="$1"
  local dst="$AGENTS/${label}.plist"
  # KeepAlive jobs can leave launchd in a bad state if we bootstrap immediately
  # after bootout ("Bootstrap failed: 5: Input/output error").
  launchctl bootout "$DOMAIN/$label" 2>/dev/null || true
  sleep 1
  local ok=0
  local i
  for i in 1 2 3 4 5; do
    if launchctl bootstrap "$DOMAIN" "$dst" 2>/dev/null; then
      ok=1
      break
    fi
    launchctl bootout "$DOMAIN/$label" 2>/dev/null || true
    sleep 2
  done
  if [[ "$ok" -ne 1 ]]; then
    echo "error: launchctl bootstrap failed for $label" >&2
    echo "  try: launchctl bootout $DOMAIN/$label; sleep 2; launchctl bootstrap $DOMAIN $dst" >&2
    exit 1
  fi
  launchctl enable "$DOMAIN/$label" 2>/dev/null || true
  # Prefer start over kickstart -k (kickstart waits for KeepAlive exit).
  launchctl start "$label" 2>/dev/null || true
  echo "  loaded $label"
}

echo "Installing agents..."

# Litecoin: LaunchAgent only when datadir is on the internal disk
if [[ "$LITECOIN_DATADIR" == /Volumes/* ]]; then
  echo "  skip com.litview.litecoin (datadir on removable volume — launchd TCC)"
  echo "       start from Terminal: $SUPPORT/start-litecoin.sh"
  launchctl bootout "$DOMAIN/com.litview.litecoin" 2>/dev/null || true
  rm -f "$AGENTS/com.litview.litecoin.plist"
else
  write_plist com.litview.litecoin "$SUPPORT/services/litecoin.sh" litecoin true
  load_label com.litview.litecoin
fi

write_plist com.litview.brk "$SUPPORT/services/brk.sh" brk true
load_label com.litview.brk

write_plist com.litview.watchdog "$SUPPORT/services/watchdog.sh" watchdog false 60
load_label com.litview.watchdog

CF_CONFIG="$DIR/../cloudflared/config.yml"
CF_CREDS="$DIR/../cloudflared/credentials.json"
if [[ -s "$CF_CONFIG" && -s "$CF_CREDS" ]] && grep -q 'tunnel:' "$CF_CONFIG"; then
  src="$DIR/com.litview.cloudflared.plist"
  dst="$AGENTS/com.litview.cloudflared.plist"
  sed -e "s|/Users/indigo/Projects/lrk|${REPO_ROOT}|g" \
      -e "s|/Users/indigo/Dev/lrk|${REPO_ROOT}|g" \
      "$src" >"$dst"
  load_label com.litview.cloudflared
else
  echo "  skip com.litview.cloudflared (need non-empty cloudflared/config.yml + credentials.json)"
  launchctl bootout "$DOMAIN/com.litview.cloudflared" 2>/dev/null || true
  rm -f "$AGENTS/com.litview.cloudflared.plist"
fi

echo
echo "Done. Logs: $LOGS"
echo "  launchctl print $DOMAIN/com.litview.brk | head"
echo "  tail -f $LOGS/brk.out.log"
echo "  data: LITECOIN_DATADIR=$LITECOIN_DATADIR  BRK_DATA_DIR=$BRK_DATA_DIR"
echo
echo "Note: leave Docker Desktop brk stopped. Index resumes from BRK_DATA_DIR."
