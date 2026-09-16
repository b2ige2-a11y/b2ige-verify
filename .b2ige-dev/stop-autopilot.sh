#!/bin/bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
PIDFILE="$ROOT/.b2ige-dev/runs/autopilot.pid"
if [ ! -f "$PIDFILE" ]; then
  echo "No pid file."
  exit 0
fi
PID="$(cat "$PIDFILE")"
if kill -0 "$PID" 2>/dev/null; then
  kill "$PID"
  echo "Stopped PID $PID"
else
  echo "PID $PID is not running."
fi
rm -f "$PIDFILE"
