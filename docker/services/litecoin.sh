#!/usr/bin/env bash
# Keep Litecoin Core alive for launchd. Qt -daemon forks; we wait on the pid
# so KeepAlive restarts after a crash (e.g. disk write failure).
set -euo pipefail

QT="${LITECOIN_QT:-/Applications/Litecoin-Qt.app/Contents/MacOS/Litecoin-Qt}"
DATADIR="${LITECOIN_DATADIR:-$HOME/Library/Application Support/Litecoin}"
PIDFILE="${LITECOIN_PIDFILE:-$DATADIR/litecoind.pid}"

if [[ ! -x "$QT" ]]; then
  echo "litecoin: missing binary at $QT" >&2
  exit 1
fi

mkdir -p "$DATADIR"

wait_pid() {
  local pid="$1"
  while kill -0 "$pid" 2>/dev/null; do
    sleep 10
  done
}

# Already running (e.g. previous GUI session) — adopt and wait.
if [[ -f "$PIDFILE" ]]; then
  existing="$(cat "$PIDFILE" 2>/dev/null || true)"
  if [[ -n "${existing:-}" ]] && kill -0 "$existing" 2>/dev/null; then
    echo "litecoin: adopting pid $existing"
    wait_pid "$existing"
    echo "litecoin: process $existing exited"
    exit 1
  fi
  rm -f "$PIDFILE"
fi

echo "litecoin: starting daemon"
"$QT" -daemon -datadir="$DATADIR" -pid="$PIDFILE"

for _ in $(seq 1 60); do
  if [[ -f "$PIDFILE" ]]; then
    pid="$(cat "$PIDFILE")"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      echo "litecoin: running pid $pid"
      wait_pid "$pid"
      echo "litecoin: process $pid exited"
      exit 1
    fi
  fi
  sleep 1
done

echo "litecoin: failed to start (no pid file)" >&2
exit 1
