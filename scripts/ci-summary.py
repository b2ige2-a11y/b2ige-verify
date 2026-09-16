#!/usr/bin/env python3
"""Render a sanitized Agent Protocol response for a CI step summary."""
import argparse
import pathlib
import os

from ci_summary import load_payload, render_error, render_payload


parser = argparse.ArgumentParser()
parser.add_argument("input", nargs="?", type=pathlib.Path)
parser.add_argument("--output", type=pathlib.Path, default=None)
args = parser.parse_args()

try:
    payload = load_payload(args.input) if args.input and args.input.is_file() else None
    rendered = render_payload(payload) if payload is not None else render_error()
except (OSError, ValueError, TypeError):
    rendered = render_error()

output = args.output or (pathlib.Path(os.environ["GITHUB_STEP_SUMMARY"])
                         if os.environ.get("GITHUB_STEP_SUMMARY") else None)
if output:
    output.write_text(rendered + "\n", encoding="utf-8")
else:
    print(rendered)
