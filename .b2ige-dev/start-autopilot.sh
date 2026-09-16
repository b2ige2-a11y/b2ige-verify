#!/bin/bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
mkdir -p .b2ige-dev/runs
LOG="$ROOT/.b2ige-dev/runs/launcher.log"
PIDFILE="$ROOT/.b2ige-dev/runs/autopilot.pid"

if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
  echo "Autopilot is already running as PID $(cat "$PIDFILE")."
  exit 0
fi

GOAL="${*:-}"
if [ -n "$GOAL" ]; then
  CMD=(python3 "$ROOT/.b2ige-dev/autopilot.py" --goal "$GOAL")
else
  CMD=(python3 "$ROOT/.b2ige-dev/autopilot.py")
fi

nohup caffeinate -dimsu "${CMD[@]}" >>"$LOG" 2>&1 &
PID=$!
echo "$PID" > "$PIDFILE"
echo "B2IGE Autopilot started: PID $PID"
echo "Log: $LOG"
echo "Status: ./.b2ige-dev/status-autopilot.sh"
