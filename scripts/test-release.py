#!/usr/bin/env python3
"""Focused negative tests for publication packaging safeguards."""
import copy
import json
import shutil
import subprocess
import importlib.util
import pathlib
import os
import tarfile
import tempfile
import unittest
from hygiene import ROOT, scan
from installed_cli_smoke import check_cli

spec = importlib.util.spec_from_file_location('validate_archive', pathlib.Path(__file__).with_name('validate-archive.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)


class ReleaseSafety(unittest.TestCase):
    def test_product_versions_and_publication_guards(self):
        self.assertEqual(module.product_metadata(ROOT), '0.3.0')
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            names = ['Cargo.toml', 'Cargo.lock', 'npm/b2ige/package.json',
                     'npm/b2ige/native-manifest.json', 'release/release-manifest.schema.json',
                     *[str(p.relative_to(ROOT)) for p in (ROOT / 'crates').glob('*/Cargo.toml')]]
            for name in names:
                p = root / name
                p.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / name, p)
            for name, old, new in [
                ('Cargo.toml', 'publish = false', 'publish = true'),
                ('crates/verify-cli/Cargo.toml', 'publish = false', 'publish = true'),
                ('Cargo.lock', '0.3.0', '9.9.9'),
                ('Cargo.lock', 'name = "verify-cli"', 'name = "missing-workspace-package"'),
                ('npm/b2ige/package.json', '0.3.0', '9.9.9'),
                ('npm/b2ige/native-manifest.json', '0.3.0', '9.9.9'),
                ('release/release-manifest.schema.json', '0.3.0', '9.9.9'),
                ('npm/b2ige/package.json', '"private": true', '"private": false'),
                ('npm/b2ige/package.json', "throw Error('Publication blocked:", "console.log('Publication allowed:"),
            ]:
                with self.subTest(name=name, mutation=old):
                    p = root / name
                    original = p.read_text()
                    self.assertIn(old, original)
                    p.write_text(original.replace(old, new))
                    with self.assertRaises(AssertionError):
                        module.product_metadata(root)
                    p.write_text(original)
        guard = json.loads((ROOT / 'npm/b2ige/package.json').read_text())['scripts']['prepublishOnly']
        self.assertNotEqual(subprocess.run(guard, shell=True, capture_output=True).returncode, 0)

    def test_installed_smoke_rejects_stale_or_missing_commands_and_fallback(self):
        binary = ROOT / 'target/release/b2ige'
        self.assertTrue(binary.is_file(), 'build release workspace first')
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            wrapper = root / 'b2ige'
            # Delegate untouched commands to the actual CLI. Help remains current
            # while each mutation independently breaks an installed surface.
            for index, mutation in enumerate([
                'if [ "$1" = "--version" ]; then echo "verify-cli 0.2.0"; exit 0; fi',
                'if [ "$1" = "inspect" ]; then exit 64; fi',
                'if [ "$1" = "trust" ] && [ "$3" != "--help" ]; then exit 0; fi',
            ]):
                work = root / str(index)
                work.mkdir()
                wrapper.write_text('#!/bin/sh\n' + mutation + '\nexec "' + str(binary) + '" "$@"\n')
                wrapper.chmod(0o755)
                with self.assertRaises(AssertionError):
                    check_cli(wrapper, '0.3.0', work)
            marker = root / 'fallback-used'
            wrapper.write_text('#!/bin/sh\n: > "' + str(marker) + '"\nexit 0\n')
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'])
            with self.assertRaisesRegex(AssertionError, 'fallback forbidden'):
                check_cli(root / 'absent/b2ige', '0.3.0', root, env)
            self.assertFalse(marker.exists())

    def test_platform_signing_and_candidate_attestation_boundaries(self):
        targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']
        data = dict(version='0.3.0', schema_version='3', supported_platforms=targets,
                    built_target=targets[0], actually_verified_platforms=[],
                    platform_validation={t: 'NOT_RUN' for t in targets},
                    signing=dict(publisher_signed=False, apple_notarized=False),
                    publication_ready=False, owner_publication_authorized=False)
        module.candidate_manifest(data, '0.3.0')
        mutations = [
            lambda d: d.update(version='9.9.9'),
            lambda d: d['supported_platforms'].append('x86_64-pc-windows-msvc'),
            lambda d: d['supported_platforms'].append('aarch64-unknown-linux-gnu'),
            lambda d: d['actually_verified_platforms'].append(targets[1]),
            lambda d: d['signing'].update(publisher_signed=True),
            lambda d: d['signing'].update(apple_notarized=True),
            lambda d: d.update(publication_ready=True),
            lambda d: d.update(owner_publication_authorized=True),
        ]
        for mutation in mutations:
            changed = copy.deepcopy(data)
            mutation(changed)
            with self.assertRaises(AssertionError):
                module.candidate_manifest(changed, '0.3.0')

    def test_v110_packaging_and_deferred_external_evidence(self):
        for required in [*module.V110_DOCS, *module.SOURCE_TOOLS]:
            self.assertTrue((ROOT / required).is_file(), required)
        state = (ROOT / 'v110/STATE.md').read_text().split('## V110-D')[1]
        self.assertIn('Status: FRAMEWORK COMPLETE — EXTERNAL EVIDENCE DEFERRED', state)
        self.assertNotIn('Status: COMPLETE', state)
        notes = (ROOT / 'docs/RELEASE-0.3.0.md').read_text()
        self.assertIn('Actual genuine external-user\nevidence is DEFERRED', notes)
        self.assertIn('No AI/internal run is represented as external evidence', notes)
        self.assertIn('post-release validation objective, not a correctness gate', notes)

    def test_rejects_archive_escape_links_duplicate_and_private_members(self):
        for name in ['../escape', '/absolute', 'package/.env', 'package/.git/config', 'package/.b2ige/store.json', 'package/target/binary']:
            with self.subTest(name=name), self.assertRaises(ValueError): module.members_safe([tarfile.TarInfo(name)])
        member = tarfile.TarInfo('package/link'); member.type = tarfile.SYMTYPE; member.linkname = '../outside'
        with self.assertRaises(ValueError): module.members_safe([member])
        member = tarfile.TarInfo('package/file')
        with self.assertRaises(ValueError): module.members_safe([member, member])

    def test_public_synthetic_schema_and_safe_members(self):
        module.members_safe([tarfile.TarInfo('package/LICENSE'), tarfile.TarInfo('package/schemas/blindtest-sealed-suite.schema.json')])
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / 'public.json'; p.write_text('{"private_canary":{"type":"string"}}')
            scan([p])

    def test_refuses_provider_keys_and_private_runtime_values(self):
        # Construct fake values at runtime; do not ship secret-like constants.
        cases = [b'gh' + b'p_' + b'A' * 36, b'sk-' + b'proj-' + b'Z' * 40,
                 b'sk-' + b'ant-' + b'Z' * 40, b'sk_' + b'live_' + b'Z' * 32,
                 b'Authorization: ' + b'Bearer ' + b'Z' * 32,
                 b'BLINDTEST_PRIVATE_' + b'CANARY_' + b'a' * 32,
                 b'/Us' + b'ers/private-owner/build.rs', b'-----BEGIN ' + b'PRIVATE KEY-----']
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / 'payload'
            for payload in cases:
                p.write_bytes(payload)
                with self.assertRaises(SystemExit): scan([p])


if __name__ == '__main__':
    unittest.main()
