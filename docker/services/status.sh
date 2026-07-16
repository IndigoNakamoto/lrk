#!/usr/bin/env bash
# Show whether litview LaunchAgents are loaded / running.
set -euo pipefail

UID_NUM="$(id -u)"
DOMAIN="gui/${UID_NUM}"

labels=(
  com.litview.litecoin
  com.litview.brk
  com.litview.cloudflared
  com.litview.watchdog
)

for label in "${labels[@]}"; do
  if ! launchctl print "$DOMAIN/$label" &>/dev/null; then
    printf '%-28s  unloaded\n' "$label"
    continue
  fi
  state="$(launchctl print "$DOMAIN/$label" 2>/dev/null | awk '/state = / { print $3; exit }')"
  pid="$(launchctl print "$DOMAIN/$label" 2>/dev/null | awk '/pid = / { print $3; exit }')"
  if [[ -n "${pid:-}" && "$pid" != "0" ]]; then
    printf '%-28s  %-12s  pid %s\n' "$label" "${state:-running}" "$pid"
  else
    printf '%-28s  %s\n' "$label" "${state:-loaded}"
  fi
done

echo
if curl -sf --max-time 2 http://127.0.0.1:7070/health >/dev/null; then
  echo "brk health: OK"
else
  echo "brk health: down"
fi
