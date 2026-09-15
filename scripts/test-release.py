#!/usr/bin/env python3
"""Focused negative tests for publication packaging safeguards."""
import importlib.util
import pathlib
import tarfile
import tempfile
import unittest
from hygiene import scan

spec = importlib.util.spec_from_file_location('validate_archive', pathlib.Path(__file__).with_name('validate-archive.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)


class ReleaseSafety(unittest.TestCase):
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
