#!/usr/bin/env python3
"""Run the registered contracts affected by a Git diff, conservatively.

This is an invocation layer. Product loaders and Agent Protocol responses retain
all verdict authority; an incomplete selection is an infrastructure error.
"""
import argparse
import fnmatch
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

from ci_summary import load_payload, render_many


TOOL_ROOT = pathlib.Path(__file__).resolve().parents[1]
ROOT = TOOL_ROOT
PRODUCTS = {"behavior", "sideeffect", "blindtest"}
VERDICT_CODES = {"PASS": 0, "FAIL": 1, "INCONCLUSIVE": 2, "ERROR": 3}
IDENTITY = re.compile(r"^[A-Za-z0-9_.-]{1,128}$")
RESPONSE_FIELDS = {
    "protocol_version", "product", "operation", "verdict", "kind", "summary",
    "expected", "observed", "reproduction", "evidence_refs", "source", "scope",
    "limitations", "next_action", "other_failure_count",
}


def user_path(value):
    path = pathlib.Path(value)
    return path if path.is_absolute() else ROOT / path


def read_json(path):
    path.lstat()
    if path.is_symlink() or not path.is_file():
        raise ValueError("configuration must be a regular file")
    data = path.read_bytes()
    if len(data) > 4 * 1024 * 1024:
        raise ValueError("configuration is too large")
    value = json.loads(data)
    if not isinstance(value, dict):
        raise ValueError("configuration must be an object")
    return value


def registry_entries(path):
    return validate_registry(read_json(path))


def validate_registry(value):
    if not isinstance(value, dict):
        raise ValueError("project registry must be an object")
    if value.get("schema_version") != "1" or not isinstance(value.get("entries"), dict):
        raise ValueError("unsupported project registry")
    entries = value["entries"]
    if not entries:
        raise ValueError("project registry has no contracts")
    for identity, entry in entries.items():
        if not isinstance(identity, str) or not identity or not isinstance(entry, dict):
            raise ValueError("invalid project registry entry")
        if set(entry) != {"product", "config", "store", "authorization"}:
            raise ValueError("project registry entry has unknown or missing fields")
        if entry["product"] not in PRODUCTS or not isinstance(entry["config"], str):
            raise ValueError("invalid project registry product/config")
        if not isinstance(entry["store"], str) or not entry["store"]:
            raise ValueError("invalid project registry store")
        if entry["product"] == "behavior":
            if not isinstance(entry["authorization"], str) or not entry["authorization"]:
                raise ValueError("Behavior contract authorization is required")
        elif entry["authorization"] is not None:
            raise ValueError("non-Behavior authorization must be null")
    return entries


def required_entries(revision, path):
    """Read the controller-retained inventory, never a working-tree replacement.

    The caller must supply this pin independently of the candidate. The workflow
    supplies the event's base SHA and runs the adapter from that trusted checkout.
    """
    if not re.fullmatch(r"[0-9a-f]{40}", revision or ""):
        raise ValueError("an independently retained full trusted revision is required")
    if (not path or path.startswith("/") or "\\" in path
            or any(part in ("", ".", "..") for part in path.split("/"))):
        raise ValueError("required registry must be a repository-relative path")
    pin = trusted_revision(revision, "controller")
    result = subprocess.run(
        ["git", "show", f"{pin}:{path}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False,
    )
    if result.returncode or len(result.stdout) > 4 * 1024 * 1024:
        raise ValueError("trusted required registry is unavailable")
    return validate_registry(json.loads(result.stdout))


def checked_entries(path, revision, required_path):
    required = required_entries(revision, required_path)
    entries = registry_entries(path)
    if entries != required:
        raise ValueError("registry differs from the independently retained required contracts")
    return entries


def file_identity(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("approved input must be a regular file")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def check_candidate(approval, head, entries):
    """Admission only: the controller approves provenance; product loaders verify runtime.

    Never generate this approval from the candidate or refresh it automatically.
    It must be supplied through the same trusted boundary as the verifier itself.
    """
    if (set(approval) != {"schema_version", "head", "contracts"}
            or approval["schema_version"] != "1" or approval["head"] != head
            or not isinstance(approval["contracts"], dict)
            or set(approval["contracts"]) != set(entries)):
        raise ValueError("current candidate approval is missing or mismatched")
    for identity, entry in entries.items():
        pin = approval["contracts"][identity]
        if (not isinstance(pin, dict)
                or set(pin) != {"config_sha256", "authorization_sha256", "target_identity"}):
            raise ValueError("incomplete candidate contract approval")
        config_path = user_path(entry["config"])
        if file_identity(config_path) != pin["config_sha256"]:
            raise ValueError("candidate configuration differs from approval")
        config = read_json(config_path)
        authorization = entry["authorization"]
        actual_authorization = file_identity(user_path(authorization)) if authorization else None
        if actual_authorization != pin["authorization_sha256"]:
            raise ValueError("candidate authorization differs from approval")
        try:
            if entry["product"] == "blindtest":
                target_identity = config["target"]["image"]
            else:
                target = config["after"] if entry["product"] == "behavior" else config["trigger"]
                target_identity = target["identity" if entry["product"] == "behavior" else "executable_hash"]
                executable = pathlib.Path(target["executable"])
                if not executable.is_absolute() or file_identity(executable) != target_identity:
                    raise ValueError("actual candidate executable differs from approval")
            if (not isinstance(target_identity, str)
                    or not re.fullmatch(r"sha256:[0-9a-f]{64}", target_identity)
                    or target_identity != pin["target_identity"]):
                raise ValueError("candidate target identity differs from approval")
        except (KeyError, TypeError) as error:
            raise ValueError("candidate target identity is unavailable") from error


def check_controller_isolation(entries):
    """Admission for privileged CI only; native local verifiers are not sandboxes."""
    if any(entry["product"] != "blindtest" for entry in entries.values()):
        raise ValueError("trusted CI blocks native Behavior/SideEffect execution; a reviewed isolated runtime is required")
    for entry in entries.values():
        if read_json(user_path(entry["config"])).get("required_isolation") != "DOCKER_ISOLATION":
            raise ValueError("trusted CI requires existing BlindTest DOCKER_ISOLATION")


def publish_reports(directory, reports):
    """Serialize only already checked Agent payloads into a fresh upload directory."""
    for parent in (directory, *directory.parents):
        if parent.is_symlink():
            raise ValueError("artifact directory symlinks are refused")
    # No reuse, including an empty preexisting directory or dangling symlink.
    directory.mkdir()
    for identity, payload in reports:
        path = directory / f"{output_name(identity)}.json"
        with path.open("x", encoding="utf-8") as stream:
            json.dump(payload, stream, sort_keys=True)
            stream.write("\n")
    # The workflow cannot upload stale/partial output after any earlier failure.
    if os.environ.get("GITHUB_OUTPUT"):
        with pathlib.Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as stream:
            stream.write("artifacts_ready=true\n")


def changed_files(base, head):
    if not base:
        raise ValueError("a trusted diff base revision is required")
    # Treat renames as deletion + addition so both paths' contracts are checked.
    result = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", "-z", "--diff-filter=ACDMRT", base, head, "--"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode:
        raise ValueError("diff base or head revision is unavailable")
    return [item for item in result.stdout.decode("utf-8", "strict").split("\0") if item]


def trusted_revision(value, label):
    if (not isinstance(value, str) or not value or len(value) > 256
            or value.startswith("-")
            or any(ord(char) < 32 or ord(char) == 127 for char in value)):
        raise ValueError(f"invalid trusted Git {label} revision")
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "--end-of-options", f"{value}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    revision = result.stdout.decode("ascii", "strict").strip()
    if result.returncode or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError(f"trusted Git {label} revision is unavailable")
    return revision


def clean_pattern(value):
    if not isinstance(value, str) or not value or "\0" in value or "\\" in value:
        raise ValueError("diff map patterns must be non-empty POSIX paths")
    return value[2:] if value.startswith("./") else value


def diff_map(path, identities):
    value = read_json(path)
    if value.get("schema_version") != "1":
        raise ValueError("unsupported diff map")
    if set(value) != {"schema_version", "shared", "contracts"}:
        raise ValueError("diff map has unknown or missing fields")
    if not isinstance(value["shared"], list) or not isinstance(value["contracts"], dict):
        raise ValueError("invalid diff map shape")
    if set(value["contracts"]) != set(identities):
        raise ValueError("diff map must name every registered contract exactly once")
    shared = [clean_pattern(pattern) for pattern in value["shared"]]
    contracts = {}
    for identity in identities:
        patterns = value["contracts"][identity]
        if not isinstance(patterns, list) or not patterns:
            raise ValueError("every contract needs at least one diff pattern")
        contracts[identity] = [clean_pattern(pattern) for pattern in patterns]
    return shared, contracts


def select(identities, files, mapping):
    if not files:
        return list(identities), "no changed paths; all registered contracts"
    if mapping is None:
        return list(identities), "no diff map; conservative all-contract verification"
    shared, contracts = mapping
    selected = set()
    uncovered = []
    for path in files:
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in shared):
            selected.update(identities)
            continue
        matches = [identity for identity, patterns in contracts.items()
                   if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)]
        if not matches:
            uncovered.append(path)
        selected.update(matches)
    if uncovered:
        raise ValueError("diff contains paths outside the reviewed contract map")
    return [identity for identity in identities if identity in selected], "reviewed diff map"


def output_name(identity):
    if IDENTITY.fullmatch(identity):
        return identity
    digest = hashlib.sha256(identity.encode()).hexdigest()[:12]
    return f"contract-{digest}"


def validate_report(path, product, exit_code):
    if path.is_symlink() or not path.is_file():
        return None
    try:
        payload = load_payload(path)
    except (OSError, ValueError, TypeError):
        return None
    if not isinstance(payload, dict):
        return None
    if (set(payload) != RESPONSE_FIELDS
            or payload.get("protocol_version") != "1"
            or not isinstance(payload.get("kind"), str)
            or not payload["kind"]
            or payload.get("kind") == "readiness"
            or not isinstance(payload.get("summary"), str)
            or not isinstance(payload.get("evidence_refs"), list)
            or not isinstance(payload.get("limitations"), list)
            or not isinstance(payload.get("next_action"), str)
            or not isinstance(payload.get("other_failure_count"), int)
            or isinstance(payload["other_failure_count"], bool)
            or payload["other_failure_count"] < 0
            or payload.get("product") not in PRODUCTS
            or payload.get("product") != product
            or payload.get("operation") != "verify"
            or payload.get("verdict") not in VERDICT_CODES
            or VERDICT_CODES[payload["verdict"]] != exit_code
            or (payload["verdict"] != "ERROR"
                and (not isinstance(payload.get("source"), dict)
                     or not isinstance(payload.get("scope"), dict)))):
        return None
    return payload


def priority(codes):
    if any(code == 3 for code in codes):
        return 3
    if any(code == 2 for code in codes):
        return 2
    if any(code == 1 for code in codes):
        return 1
    return 0


def plan_payload(base, head, files, selection, reason):
    return {
        "schema_version": "1",
        "kind": "diff_aware_ci",
        "base": base,
        "head": head,
        "changed_file_count": len(files),
        "selected_contracts": selection,
        "selection": reason,
    }


def main():
    global ROOT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", help="trusted project checkout; tooling remains independently pinned")
    parser.add_argument("--base", required=True, help="trusted Git base revision")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--registry", default=".b2ige/project.json")
    parser.add_argument("--trusted-revision", required=True,
                        help="full controller-approved commit SHA, independent of the candidate")
    parser.add_argument("--required-registry", default=".b2ige/project.json",
                        help="required registry path in the trusted commit (not the working tree)")
    parser.add_argument("--candidate-approval",
                        help="controller-retained head/config/target approval; required for execution")
    parser.add_argument("--map", dest="diff_map")
    parser.add_argument("--b2ige", default="target/release/b2ige")
    parser.add_argument("--report-dir", default="b2ige-agent-reports")
    parser.add_argument("--summary", help="write a sanitized GitHub Actions step summary")
    parser.add_argument("--plan-only", action="store_true")
    parser.add_argument("--trusted-controller", action="store_true",
                        help="privileged CI: require isolated BlindTest; native targets remain blocked")
    parser.add_argument("--artifact-dir",
                        help="new directory for allowlisted checked Agent output; requires --trusted-controller")
    args = parser.parse_args()
    if args.project_root:
        ROOT = pathlib.Path(args.project_root).resolve()

    try:
        entries = checked_entries(user_path(args.registry), args.trusted_revision,
                                  args.required_registry)
        if args.trusted_controller:
            check_controller_isolation(entries)
            if not args.artifact_dir or args.plan_only:
                raise ValueError("trusted CI requires fresh sanitized artifacts and actual verification")
        elif args.artifact_dir:
            raise ValueError("artifact staging requires trusted controller mode")
        identities = list(entries)
        base = trusted_revision(args.base, "base")
        head = trusted_revision(args.head, "head")
        files = changed_files(base, head)
        mapping = diff_map(user_path(args.diff_map), identities) if args.diff_map else None
        selected, reason = select(identities, files, mapping)
        if not selected:
            raise ValueError("diff selected no registered contract")
        plan = plan_payload(base, head, files, selected, reason)
        if args.plan_only:
            plan["plan_only"] = True
            print(json.dumps(plan, sort_keys=True))
            return 0
        if not args.candidate_approval:
            raise ValueError("controller candidate approval is required")
        approval = read_json(user_path(args.candidate_approval))
        check_candidate(approval, head, entries)
        report_dir = user_path(args.report_dir)
        if report_dir.is_symlink() or (report_dir.exists() and not report_dir.is_dir()):
            raise ValueError("report directory must be a real directory")
        if report_dir.exists() and any(report_dir.iterdir()):
            raise ValueError("report directory must be new or empty; stale reports are not reused")
        report_dir.mkdir(parents=True, exist_ok=True)
    except (OSError, ValueError):
        print(json.dumps({"schema_version": "1", "kind": "diff_aware_ci", "gate_pass": False,
                          "error": "diff-aware verification setup failed"}, sort_keys=True))
        if args.summary:
            try:
                pathlib.Path(args.summary).write_text(
                    "## B2IGE Verify · ERROR\n\nDiff-aware verification could not establish complete contract selection and current candidate binding.\n\n"
                    "Next action: restore controller-approved inputs and the retained contract inventory; provide the independently approved current head/config/target binding and check diff/base configuration.\n",
                    encoding="utf-8",
                )
            except OSError:
                pass
        print("Diff-aware verification setup failed; no contract verdict was produced. "
              "Trusted CI requires approved BlindTest DOCKER_ISOLATION; native Behavior/SideEffect "
              "remain blocked pending a reviewed isolated runtime. Build and approval alone do not isolate execution."
              if args.trusted_controller else
              "Diff-aware verification setup failed; no contract verdict was produced.",
              file=sys.stderr)
        return 3

    results = []
    summaries = []
    artifacts = []
    codes = []
    for identity in selected:
        entry = entries[identity]
        report = report_dir / f"{output_name(identity)}.json"
        command = [
            sys.executable,
            str(TOOL_ROOT / "scripts/ci-verify.py"),
            entry["product"],
            entry["config"],
            "--b2ige",
            args.b2ige,
            "--store",
            entry["store"],
            "--report",
            str(report),
        ]
        if entry["authorization"] is not None:
            command.extend(["--authorization", entry["authorization"]])
        try:
            if args.trusted_controller:
                check_controller_isolation(entries)
            check_candidate(approval, head, entries)
            child = subprocess.run(command, cwd=ROOT, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, check=False)
            code = child.returncode if child.returncode in VERDICT_CODES.values() else 3
            check_candidate(approval, head, entries)
        except (OSError, ValueError):
            code = 3
        payload = validate_report(report, entry["product"], code)
        if payload is None:
            code = 3
            try:
                report.unlink(missing_ok=True)
            except OSError:
                pass
            summaries.append((identity, {
                "product": entry["product"],
                "operation": "verify",
                "kind": "infrastructure_error",
                "verdict": "ERROR",
            }))
        else:
            summaries.append((identity, payload))
            artifacts.append((identity, payload))
        codes.append(code)
        results.append({
            "identity": identity,
            "product": entry["product"],
            "exit_code": code,
            "report": str(report.relative_to(ROOT)) if report.is_relative_to(ROOT) else None,
        })

    final_code = priority(codes)
    if args.trusted_controller:
        try:
            publish_reports(user_path(args.artifact_dir), artifacts)
        except (OSError, ValueError):
            final_code = 3
            print("Sanitized artifact staging failed; supply a new directory under a real parent "
                  "and a writable GitHub step-output file. No upload was authorized.", file=sys.stderr)
    plan.update({"results": results, "ci_exit_code": final_code, "gate_pass": final_code == 0})
    if args.summary:
        try:
            pathlib.Path(args.summary).write_text(render_many(summaries), encoding="utf-8")
        except OSError:
            final_code = 3
            plan["ci_exit_code"] = final_code
            plan["gate_pass"] = False
    print(json.dumps(plan, sort_keys=True))
    return final_code


if __name__ == "__main__":
    raise SystemExit(main())
