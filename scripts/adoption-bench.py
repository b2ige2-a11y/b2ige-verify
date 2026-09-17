#!/usr/bin/env python3
"""Reproduce the source-checkout V110 adoption bench; never update P8 baselines."""
import argparse
import json
import subprocess
from pathlib import Path
from adoption_bench import execute, accepted, read, semantic


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', nargs='?', help='new output directory, never overwritten')
    parser.add_argument('--build', action='store_true', help='measure source preparation separately')
    parser.add_argument('--compare', nargs=2, metavar=('FIRST', 'SECOND'))
    args = parser.parse_args()
    try:
        if args.compare:
            equal = semantic(read(args.compare[0])) == semantic(read(args.compare[1]))
            print(json.dumps({'semantic_equal': equal}))
            return 0 if equal else 3
        if not args.output: parser.error('a new output directory is required')
        result = execute(Path(args.output), build=args.build)
        print(json.dumps({'adoption_gate_pass': accepted(result), 'aggregate': result['aggregate']}))
        return 0 if accepted(result) else 3
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        # Never publish raw command output or private physical paths.
        print(json.dumps({'adoption_gate_pass': False, 'error': type(error).__name__}))
        return 3


if __name__ == '__main__':
    raise SystemExit(main())
