#!/usr/bin/env python3
"""Focused, local tests for adoption adapters; no product or Docker execution."""
import importlib.util
import contextlib
import copy
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from ci_summary import render_payload


ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("diff_verify", ROOT / "scripts/diff-verify.py")
diff_verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diff_verify)


class AdoptionTests(unittest.TestCase):
    def test_required_inventory_rejects_pr_removal_and_replacement_before_execution(self):
        required = {
            name: {"product": "behavior", "config": f"{name}.json",
                   "store": f"{name}-store", "authorization": f"{name}-approval.json"}
            for name in ("passing", "failing")
        }
        mutations = []
        removed = copy.deepcopy(required)
        del removed["failing"]
        mutations.append(removed)
        for field in ("config", "store", "authorization"):
            replaced = copy.deepcopy(required)
            replaced["failing"][field] = "candidate-replacement"
            mutations.append(replaced)
        added = copy.deepcopy(required)
        added["extra"] = required["passing"]
        mutations.append(added)
        with tempfile.TemporaryDirectory() as directory:
            registry = pathlib.Path(directory) / "project.json"
            for entries in mutations:
                with self.subTest(entries=entries):
                    registry.write_text(json.dumps({"schema_version": "1", "entries": entries}))
                    output = io.StringIO()
                    with mock.patch.object(diff_verify, "required_entries", return_value=required), \
                            mock.patch.object(diff_verify.subprocess, "run") as run, \
                            mock.patch.object(sys, "argv", ["diff-verify", "--base", "a" * 40,
                                "--trusted-revision", "a" * 40, "--registry", str(registry)]), \
                            contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
                        self.assertEqual(diff_verify.main(), 3)
                    self.assertFalse(json.loads(output.getvalue())["gate_pass"])
                    run.assert_not_called()
            registry.write_text(json.dumps({"schema_version": "1", "entries": required}))
            with mock.patch.object(diff_verify, "required_entries", return_value=required):
                self.assertEqual(diff_verify.checked_entries(registry, "a" * 40, "project.json"), required)

    def test_required_inventory_is_loaded_from_retained_commit_not_worktree(self):
        pin = "a" * 40
        registry = {"schema_version": "1", "entries": {
            "required": {"product": "sideeffect", "config": "config.json",
                         "store": "store", "authorization": None}}}
        with mock.patch.object(diff_verify, "trusted_revision", return_value=pin), \
                mock.patch.object(diff_verify.subprocess, "run", return_value=
                    subprocess.CompletedProcess([], 0, json.dumps(registry).encode())) as run:
            self.assertEqual(diff_verify.required_entries(pin, ".b2ige/project.json"), registry["entries"])
            self.assertEqual(run.call_args.args[0], ["git", "show", f"{pin}:.b2ige/project.json"])
        for invalid in (None, "HEAD", "main", "b" * 39):
            with self.subTest(pin=invalid), self.assertRaises(ValueError):
                diff_verify.required_entries(invalid, ".b2ige/project.json")
        with mock.patch.object(diff_verify, "trusted_revision", return_value=pin), \
                mock.patch.object(diff_verify.subprocess, "run", return_value=
                    subprocess.CompletedProcess([], 128, b"")), self.assertRaises(ValueError):
            diff_verify.required_entries(pin, ".b2ige/project.json")

    def test_candidate_binding_for_all_products(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            executable = root / "candidate"
            executable.write_bytes(b"current build")
            target_hash = diff_verify.file_identity(executable)
            authorization = root / "authorization.json"
            authorization.write_text("{}")
            for product, config in (
                ("behavior", {"after": {"executable": str(executable), "identity": target_hash}}),
                ("sideeffect", {"trigger": {"executable": str(executable), "executable_hash": target_hash}}),
                ("blindtest", {"target": {"image": target_hash}}),
            ):
                with self.subTest(product=product):
                    path = root / "config.json"
                    path.write_text(json.dumps(config))
                    entries = {"contract": {"product": product, "config": str(path),
                        "authorization": str(authorization) if product == "behavior" else None,
                        "store": "store"}}
                    approval = {"schema_version": "1", "head": "b" * 40, "contracts": {
                        "contract": {"config_sha256": diff_verify.file_identity(path),
                            "authorization_sha256": diff_verify.file_identity(authorization)
                                if product == "behavior" else None,
                            "target_identity": target_hash}}}
                    diff_verify.check_candidate(approval, "b" * 40, entries)
                    for field in ("config_sha256", "authorization_sha256", "target_identity"):
                        bad = copy.deepcopy(approval)
                        bad["contracts"]["contract"][field] = "sha256:" + "0" * 64
                        with self.assertRaises(ValueError):
                            diff_verify.check_candidate(bad, "b" * 40, entries)
                    with self.assertRaises(ValueError):
                        diff_verify.check_candidate(approval, "c" * 40, entries)
                    missing = copy.deepcopy(approval)
                    missing["contracts"] = {}
                    with self.assertRaises(ValueError):
                        diff_verify.check_candidate(missing, "b" * 40, entries)
                    if product != "blindtest":
                        executable.write_bytes(b"old passing build")
                        with self.assertRaises(ValueError):
                            diff_verify.check_candidate(approval, "b" * 40, entries)
                        executable.write_bytes(b"current build")
                    else:
                        path.write_text(json.dumps({"target": {"image": "mutable:latest"}}))
                        approval["contracts"]["contract"].update(
                            config_sha256=diff_verify.file_identity(path), target_identity="mutable:latest")
                        with self.assertRaises(ValueError):
                            diff_verify.check_candidate(approval, "b" * 40, entries)

    def test_gate_requires_current_head_binding_even_with_passing_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            config = root / "config.json"
            config.write_text(json.dumps({"target": {"image": "sha256:" + "a" * 64}}))
            entries = {"contract": {"product": "blindtest", "config": str(config),
                "authorization": None, "store": "store"}}
            approval = {"schema_version": "1", "head": "b" * 40, "contracts": {
                "contract": {"config_sha256": diff_verify.file_identity(config),
                    "authorization_sha256": None, "target_identity": "sha256:" + "a" * 64}}}
            approval_path = root / "approval.json"
            for mode in ("missing", "stale", "correct", "changed_during_execution"):
                with self.subTest(mode=mode):
                    approval["head"] = "a" * 40 if mode == "stale" else "b" * 40
                    approval_path.write_text(json.dumps(approval))
                    argv = ["diff-verify", "--base", "a" * 40, "--head", "b" * 40,
                        "--trusted-revision", "a" * 40, "--report-dir", str(root / mode)]
                    if mode != "missing":
                        argv.extend(["--candidate-approval", str(approval_path)])
                    output = io.StringIO()
                    def child(*args, **kwargs):
                        if mode == "changed_during_execution":
                            config.write_text("{}")
                        return subprocess.CompletedProcess([], 0)
                    with mock.patch.object(diff_verify, "checked_entries", return_value=entries), \
                            mock.patch.object(diff_verify, "trusted_revision", side_effect=lambda value, label: value), \
                            mock.patch.object(diff_verify, "changed_files", return_value=["src/change"]), \
                            mock.patch.object(diff_verify.subprocess, "run", side_effect=child) as run, \
                            mock.patch.object(diff_verify, "validate_report", return_value={"verdict": "PASS"}), \
                            mock.patch.object(sys, "argv", argv), \
                            contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
                        self.assertEqual(diff_verify.main(), 0 if mode == "correct" else 3)
                    self.assertEqual(json.loads(output.getvalue())["gate_pass"], mode == "correct")
                    if mode in ("missing", "stale"):
                        run.assert_not_called()
                    else:
                        run.assert_called_once()

    def test_summary_omits_untrusted_text_but_keeps_actionable_observable(self):
        summary = render_payload({
            "protocol_version": "1",
            "product": "behavior",
            "operation": "verify",
            "kind": "behavior_exact",
            "verdict": "FAIL",
            "expected": {"kind": "bytes", "sha256": "sha256:" + "a" * 64, "length": 4},
            "observed": {"kind": "bytes", "sha256": "sha256:" + "b" * 64, "length": 7},
            "source": {"artifact_id": "source", "integrity_hash": "sha256:" + "c" * 64},
            "scope": {"coverage": {"cases": 1}, "budget": {"execution_budget": 2}},
            "summary": "candidate /private/secret",
            "evidence_refs": ["not displayed"],
            "other_failure_count": 1,
        })
        self.assertIn("Expected (sanitized)", summary)
        self.assertIn("length=7", summary)
        self.assertNotIn("private", summary)
        self.assertNotIn("not displayed", summary)

    def test_diff_map_selects_contracts_and_rejects_uncovered_paths(self):
        mapping = (["Cargo.toml"], {"a": ["src/a/**"], "b": ["src/b/**"]})
        self.assertEqual(diff_verify.select(["a", "b"], ["src/a/main.rs"], mapping),
                         (["a"], "reviewed diff map"))
        with self.assertRaisesRegex(ValueError, "outside"):
            diff_verify.select(["a", "b"], ["docs/unknown.md"], mapping)
        self.assertEqual(diff_verify.select(["a", "b"], ["Cargo.toml"], mapping)[0], ["a", "b"])

    def test_rename_selects_both_contracts_and_rejects_unmapped_old_path(self):
        with tempfile.TemporaryDirectory(prefix="b2ige-diff-rename-") as directory:
            root = pathlib.Path(directory)

            def git(*args):
                return subprocess.run(["git", *args], cwd=root, check=True,
                                      capture_output=True).stdout

            git("init")
            git("config", "diff.renames", "true")
            old_path = "src/a/file name.rs"
            new_path = "src/b/file name.rs"
            (root / old_path).parent.mkdir(parents=True)
            (root / old_path).write_text("contract-relevant content\n", encoding="utf-8")
            git("add", "--", old_path)
            base = git("write-tree").decode().strip()
            (root / new_path).parent.mkdir(parents=True)
            (root / old_path).rename(root / new_path)
            git("add", "-A")
            head = git("write-tree").decode().strip()
            self.assertEqual(git("diff", "--name-status", "-z", base, head, "--"),
                             f"R100\0{old_path}\0{new_path}\0".encode())

            with mock.patch.object(diff_verify, "ROOT", root):
                files = diff_verify.changed_files(base, head)
            self.assertCountEqual(files, [old_path, new_path])
            mapping = ([], {"a": ["src/a/**"], "b": ["src/b/**"]})
            self.assertEqual(diff_verify.select(["a", "b"], files, mapping)[0], ["a", "b"])
            with self.assertRaisesRegex(ValueError, "outside"):
                diff_verify.select(["b"], files, ([], {"b": ["src/b/**"]}))

    def test_setup_dry_run_and_recovery_do_not_write_existing_registry(self):
        with tempfile.TemporaryDirectory(prefix="b2ige-adoption-") as directory:
            root = pathlib.Path(directory)
            dry = subprocess.run([sys.executable, str(ROOT / "scripts/setup.py"),
                                  "--project-root", str(root), "--dry-run"],
                                 capture_output=True, text=True, check=False)
            self.assertEqual(dry.returncode, 0)
            self.assertFalse((root / ".b2ige").exists())
            registry = root / ".b2ige/project.json"
            registry.parent.mkdir()
            registry.write_text("{malformed")
            recovery = subprocess.run([sys.executable, str(ROOT / "scripts/setup.py"),
                                       "--project-root", str(root), "--skip-build"],
                                      capture_output=True, text=True, check=False)
            self.assertEqual(recovery.returncode, 3)
            self.assertEqual(registry.read_text(), "{malformed")
            payload = json.loads(recovery.stdout)
            self.assertEqual(payload["status"], "recovery_required")

            real_root = root / "real"
            (real_root / ".b2ige").mkdir(parents=True)
            real_registry = real_root / ".b2ige/project.json"
            real_registry.write_text('{"schema_version":"1","entries":{}}')
            linked_root = root / "linked"
            linked_root.mkdir()
            try:
                os.symlink(real_root / ".b2ige", linked_root / ".b2ige", target_is_directory=True)
            except (NotImplementedError, OSError):
                pass
            else:
                linked = subprocess.run([sys.executable, str(ROOT / "scripts/setup.py"),
                                         "--project-root", str(linked_root), "--dry-run"],
                                        capture_output=True, text=True, check=False)
                self.assertEqual(linked.returncode, 3)
                self.assertEqual(real_registry.read_text(), '{"schema_version":"1","entries":{}}')


if __name__ == "__main__":
    unittest.main()
