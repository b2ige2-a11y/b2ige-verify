#!/usr/bin/env python3
"""Distribution-only checks; no Rust builds, product tests, or configured targets."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import unittest
import zipfile

from build_mcpb import BUNDLE, DIST, ROOT, verified_asset

ARTIFACT = ROOT / "release/artifacts/agent-distribution" / BUNDLE


class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="b2ige-distribution-test-")
        self.addCleanup(self.tmp.cleanup)
        self.work = Path(self.tmp.name)
        self.bundle = self.work / "bundle with spaces"
        self.bundle.mkdir()
        shutil.copyfile(DIST / "mcpb/launch.sh", self.bundle / "launch.sh")
        self.registry = self.work / "trusted registry;literal.json"
        self.registry.write_text('{"schema_version":"1","entries":{}}\n')
        self.env = dict(os.environ)
        self.env.pop("B2IGE_BLINDTEST_SEALED_ROOT", None)

    def run_launcher(self, *args):
        return subprocess.run(["/bin/sh", str(self.bundle / "launch.sh"), *args],
                              env=self.env, capture_output=True, text=True, timeout=10)

    def mock_platform(self, os_name, arch):
        commands = self.work / "commands"
        commands.mkdir(exist_ok=True)
        uname = commands / "uname"
        uname.write_text(f'#!/bin/sh\ncase "$1" in -s) echo {os_name};; -m) echo {arch};; *) exit 1;; esac\n')
        uname.chmod(0o755)
        self.env["PATH"] = f"{commands}:/usr/bin:/bin"

    def test_supported_routes_and_literal_arguments_and_exit(self):
        for system, arch, target in [
            ("Darwin", "arm64", "aarch64-apple-darwin"),
            ("Darwin", "x86_64", "x86_64-apple-darwin"),
            ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ]:
            with self.subTest(target=target):
                self.mock_platform(system, arch)
                binary = self.bundle / "bin" / target / "b2ige-mcp"
                binary.parent.mkdir(parents=True)
                binary.write_text('#!/bin/sh\nprintf "%s\\n" "$0" "$#" "$1" "$2" "${B2IGE_BLINDTEST_SEALED_ROOT-unset}"\nexit 37\n')
                binary.chmod(0o755)
                self.env["B2IGE_BLINDTEST_SEALED_ROOT"] = ""
                result = self.run_launcher("--registry", str(self.registry))
                self.assertEqual(result.returncode, 37)
                self.assertEqual(result.stdout.splitlines(), [str(binary.resolve()), "2", "--registry", str(self.registry), "unset"])
                self.assertEqual(result.stderr, "")

    def test_unsupported_platforms_fail_without_stdout(self):
        for system, arch in [("Linux", "aarch64"), ("Windows_NT", "x86_64"),
                             ("FreeBSD", "x86_64"), ("Darwin", "i386")]:
            with self.subTest(system=system, arch=arch):
                self.mock_platform(system, arch)
                result = self.run_launcher("--registry", str(self.registry))
                self.assertEqual(result.returncode, 64)
                self.assertEqual(result.stdout, "")
                self.assertIn("unsupported platform", result.stderr)

    def test_missing_setup_never_starts(self):
        self.mock_platform("Darwin", "arm64")
        for args, message in [
            ([], "expected --registry"),
            (["--registry", "relative.json"], "absolute path"),
            (["--registry", str(self.work / "missing")], "not a readable file"),
            (["--registry", str(self.registry)], "missing or not executable"),
        ]:
            with self.subTest(args=args):
                result = self.run_launcher(*args)
                self.assertEqual(result.returncode, 64)
                self.assertEqual(result.stdout, "")
                self.assertIn(message, result.stderr)
        self.env["B2IGE_BLINDTEST_SEALED_ROOT"] = "relative"
        result = self.run_launcher("--registry", str(self.registry))
        self.assertEqual(result.returncode, 64)
        self.assertIn("sealed root must be an absolute path", result.stderr)

    def test_corrupt_cached_release_fails_closed(self):
        (self.work / "bad.tar.gz").write_bytes(b"corrupted release")
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            verified_asset(self.work, "https://invalid.example", "bad.tar.gz", "0" * 64)

    def test_final_bundle_integrity_and_native_stdio(self):
        lock = json.loads((DIST / "release-inputs.json").read_text())
        digest = hashlib.sha256(ARTIFACT.read_bytes()).hexdigest()
        self.assertEqual(digest, (DIST / "bundle.sha256").read_text().split()[0])
        server = json.loads((ROOT / "server.json").read_text())
        self.assertEqual(digest, server["packages"][0]["fileSha256"])
        with zipfile.ZipFile(ARTIFACT) as bundle:
            self.assertIsNone(bundle.testzip())
            expected = {"manifest.json", "launch.sh", "README.md", "release-inputs.json", "LICENSE",
                        "SECURITY.md", "TRADEMARKS.md", "THIRD-PARTY-NOTICES.txt"}
            for entry in lock["inputs"]:
                name = f"bin/{entry['target']}/b2ige-mcp"
                expected.add(name)
                self.assertEqual(hashlib.sha256(bundle.read(name)).hexdigest(), entry["binary_sha256"])
                self.assertEqual(bundle.getinfo(name).external_attr >> 16 & 0o777, 0o755)
            self.assertEqual(set(bundle.namelist()), expected)
            self.assertEqual(bundle.read("manifest.json"), (DIST / "mcpb/manifest.json").read_bytes())
            self.assertEqual(bundle.read("launch.sh"), (DIST / "mcpb/launch.sh").read_bytes())
            for name in expected:
                path = self.bundle / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(bundle.read(name))
                path.chmod(bundle.getinfo(name).external_attr >> 16 & 0o777)
        if (platform.system(), platform.machine()) not in {
            ("Darwin", "arm64"), ("Darwin", "x86_64"), ("Linux", "x86_64")
        }:
            self.skipTest("native smoke requires a supported host; bundle integrity checked")
        request = {"protocol_version": "1", "product": "behavior", "operation": "verify",
                   "identity": "unregistered", "output": "agent", "execution_budget": None}
        messages = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "distribution-smoke", "version": "1"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
            {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "b2ige_behavior_verify", "arguments": request}},
        ]
        result = subprocess.run(["/bin/sh", str(self.bundle / "launch.sh"), "--registry", str(self.registry)],
                                input="".join(json.dumps(m) + "\n" for m in messages), env=self.env,
                                text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        responses = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(len(responses), 3)
        self.assertEqual(responses[0]["result"]["serverInfo"]["version"], "0.1.0")
        self.assertEqual({t["name"] for t in responses[1]["result"]["tools"]}, {
            "b2ige_doctor", "b2ige_behavior_verify", "b2ige_sideeffect_verify", "b2ige_blindtest_verify", "b2ige_report"})
        failure = responses[2]["result"]
        self.assertTrue(failure["isError"])
        self.assertEqual(failure["structuredContent"]["verdict"], "ERROR")
        # A malformed registry must fail initialization without any protocol success.
        self.registry.write_text("not json")
        result = self.run_launcher("--registry", str(self.registry))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
