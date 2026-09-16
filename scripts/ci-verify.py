#!/usr/bin/env python3
"""Run a local verifier, validate output/exit agreement, retain sanitized output only."""
import argparse
import json
import pathlib
import subprocess
import tempfile

from ci_summary import render_payload

parser = argparse.ArgumentParser()
parser.add_argument("product", choices=["behavior", "sideeffect", "blindtest"])
parser.add_argument("config")
parser.add_argument("--b2ige", default="target/release/b2ige")
parser.add_argument("--store")
parser.add_argument("--authorization")
parser.add_argument("--report", default="b2ige-agent-report.json")
parser.add_argument("--summary", help="write a sanitized GitHub Actions step summary")
a = parser.parse_args()
command = [a.b2ige, a.product, "verify", a.config, "--output", "agent", "--protocol", "1"]
for key in ("store", "authorization"):
    if getattr(a, key):
        command.extend(["--" + key, getattr(a, key)])
# Raw child stderr and malformed output are never published as CI artifacts.
try:
    child = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    with tempfile.TemporaryDirectory(prefix="b2ige-ci-") as directory:
        raw = pathlib.Path(directory) / "response.json"
        raw.write_bytes(child.stdout)
        checked = subprocess.run([a.b2ige, "ci-check", str(raw), str(child.returncode)],
                                 stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    if checked.returncode not in (0, 1, 2, 3) or not checked.stdout:
        raise RuntimeError("machine output unavailable")
    pathlib.Path(a.report).write_bytes(checked.stdout)
    if a.summary:
        pathlib.Path(a.summary).write_text(
            render_payload(json.loads(checked.stdout)), encoding="utf-8"
        )
    raise SystemExit(checked.returncode)
except (OSError, RuntimeError, ValueError, TypeError):
    # No usable verifier means no artifact and never green. Remove stale output.
    try:
        pathlib.Path(a.report).unlink(missing_ok=True)
        if a.summary:
            pathlib.Path(a.summary).unlink(missing_ok=True)
    except OSError:
        pass
    raise SystemExit(3)
