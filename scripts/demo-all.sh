#!/bin/sh
set -eu
exec python3 "$(dirname "$0")/demo.py" all
