#!/usr/bin/env python3
"""Validate a staged 0.3.0 public set without executing binaries or publishing."""
import argparse
import importlib.util
import pathlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=pathlib.Path)
    parser.add_argument('--commit', required=True, help='Independently qualified exact-main commit')
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location('validate_archive', pathlib.Path(__file__).with_name('validate-archive.py'))
    validator = importlib.util.module_from_spec(spec); spec.loader.exec_module(validator)
    validator.validate(args.directory, public=True, expected_commit=args.commit)


if __name__ == '__main__':
    main()
