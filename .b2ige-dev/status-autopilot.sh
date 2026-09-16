#!/bin/bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
PIDFILE="$ROOT/.b2ige-dev/runs/autopilot.pid"
if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
  echo "RUNNING pid=$(cat "$PIDFILE")"
else
  echo "NOT RUNNING"
fi
echo
if [ -f "$ROOT/.b2ige-dev/STATE.json" ]; then
  cat "$ROOT/.b2ige-dev/STATE.json"
fi
echo
echo "=== recent log ==="
tail -n 60 "$ROOT/.b2ige-dev/runs/launcher.log" 2>/dev/null || true
