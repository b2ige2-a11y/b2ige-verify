#!/usr/bin/env python3
"""Bootstrap a local B2IGE registry without creating or approving contracts."""
import argparse
import json
import os
import pathlib
import subprocess
import sys


MAX_BYTES = 4 * 1024 * 1024


def path_from(root, value):
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def registry_state(path):
    try:
        parent = path.parent
        parent_info = parent.lstat()
    except FileNotFoundError:
        parent_info = None
    except OSError:
        return "unsafe", None
    if parent_info is not None and (parent.is_symlink() or not parent.is_dir()):
        return "unsafe", None
    try:
        path.lstat()
    except FileNotFoundError:
        return "missing", None
    except OSError:
        return "unsafe", None
    if path.is_symlink() or not path.is_file():
        return "unsafe", None
    try:
        data = path.read_bytes()
        if len(data) > MAX_BYTES:
            return "invalid", None
        value = json.loads(data)
    except (OSError, ValueError, TypeError):
        return "invalid", None
    if (not isinstance(value, dict) or value.get("schema_version") != "1"
            or not isinstance(value.get("entries"), dict)):
        return "invalid", None
    return "valid", value


def binary_candidates(root, explicit):
    if explicit:
        return [path_from(root, explicit)]
    values = []
    if os.environ.get("B2IGE_BINARY"):
        values.append(path_from(root, os.environ["B2IGE_BINARY"]))
    for name in ("b2ige.exe", "b2ige"):
        values.extend([root / "target" / "release" / name, root / "bin" / name])
    return values


def real_binary(root, explicit):
    for path in binary_candidates(root, explicit):
        try:
            if path.is_file() and not path.is_symlink():
                return path.resolve()
        except OSError:
            continue
    return None


def output(status, registry, *, ready=None, doctor=None, build=False, preserved=False):
    value = {
        "schema_version": "1",
        "kind": "setup",
        "status": status,
        "registry_preserved": preserved,
        "build_performed": build,
        "verification_performed": False,
        "next_action": "Register reviewed typed product configs; setup never approves baselines or hidden suites.",
    }
    if ready is not None:
        value["ready"] = ready
    if doctor is not None:
        value["doctor"] = doctor
    # Keep the path relative when possible; setup output is not an evidence artifact.
    try:
        value["registry"] = str(registry.relative_to(pathlib.Path.cwd()))
    except ValueError:
        value["registry"] = str(registry)
    print(json.dumps(value, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", default=".", help="project directory (default: current directory)")
    parser.add_argument("--binary", help="trusted b2ige path; skips source build when supplied")
    parser.add_argument("--skip-build", action="store_true", help="do not build a missing source binary")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    root = pathlib.Path(args.project_root).resolve()
    registry = root / ".b2ige" / "project.json"
    state, _ = registry_state(registry)
    if state == "unsafe":
        print("B2IGE setup refused: registry path is a symlink or not a regular file", file=sys.stderr)
        return 3
    if state == "invalid":
        output("recovery_required", registry, preserved=True)
        print("B2IGE setup refused to overwrite a malformed existing registry", file=sys.stderr)
        return 3
    if args.dry_run:
        output("would_preserve" if state == "valid" else "would_initialize", registry,
               preserved=state == "valid")
        return 0

    binary = real_binary(root, args.binary)
    built = False
    if binary is None:
        if args.skip_build:
            print("B2IGE setup needs a trusted b2ige binary; source build was disabled", file=sys.stderr)
            return 3
        if not (root / "Cargo.toml").is_file():
            print("B2IGE setup needs a source workspace or --binary", file=sys.stderr)
            return 3
        try:
            result = subprocess.run(["cargo", "build", "--workspace", "--release", "--locked"],
                                    cwd=root, check=False, timeout=20 * 60)
        except (OSError, subprocess.TimeoutExpired):
            print("B2IGE setup source build failed; no registry was changed", file=sys.stderr)
            return 3
        if result.returncode != 0:
            print("B2IGE setup source build failed; no registry was changed", file=sys.stderr)
            return 3
        binary = real_binary(root, args.binary)
        built = True
    if binary is None:
        print("B2IGE setup built no trusted b2ige executable; no registry was changed", file=sys.stderr)
        return 3

    if state == "missing":
        try:
            result = subprocess.run([str(binary), "init"], cwd=root, stdout=subprocess.PIPE,
                                    stderr=subprocess.DEVNULL, text=True, check=False, timeout=60)
            created = json.loads(result.stdout)
        except (OSError, subprocess.TimeoutExpired, ValueError, TypeError):
            print("B2IGE setup initialization failed; existing configuration was not overwritten", file=sys.stderr)
            return 3
        if result.returncode != 0 or not isinstance(created, dict):
            print("B2IGE setup initialization failed; existing configuration was not overwritten", file=sys.stderr)
            return 3
        if registry_state(registry)[0] != "valid":
            print("B2IGE setup did not create a valid registry; no verification was performed", file=sys.stderr)
            return 3
        output("initialized", registry, build=built, preserved=False)
        return 0

    try:
        result = subprocess.run([str(binary), "doctor"], cwd=root, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, text=True, check=False, timeout=60)
        doctor = json.loads(result.stdout)
    except (OSError, subprocess.TimeoutExpired, ValueError, TypeError):
        print("B2IGE setup could not read the preserved doctor result; registry was not changed", file=sys.stderr)
        return 3
    if not isinstance(doctor, dict) or doctor.get("verification_performed") is not False:
        print("B2IGE setup received an invalid doctor result; registry was not changed", file=sys.stderr)
        return 3
    output("preserved_existing", registry, ready=doctor.get("ready") is True,
           doctor=doctor, build=built, preserved=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
