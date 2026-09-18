#!/usr/bin/env python3
"""Deterministic offline installer and CI-bootstrap adversarial controls."""
import copy
import gzip
import hashlib
import importlib.util
import io
import json
import os
import pathlib
import socket
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), ROOT / 'scripts' / (name + '.py'))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


installer = module('install-release')
validator = module('validate-workflows')
ControllerTests = module('test-ci-controller').ControllerTests
BINARY = pathlib.Path(os.environ.get('B2IGE_TEST_BINARY', ROOT / 'target/release/b2ige')).resolve()
PIN = 'a' * 40


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.target = installer.TARGETS[0]
        self.name = installer.filename('0.3.0', self.target)
        self.archive = self.root / self.name
        self.sums = self.root / 'SHA256SUMS'
        self.destination = self.root / 'installed'
        self.package = self.name[:-7]

    def archive_fixture(self, extra=None, mutate=None, missing=None, binary=None, manifest_data=None):
        binary = binary or b'#!/bin/sh\necho \"verify-cli 0.3.0\"\n'
        files = {'bin/' + name: binary for name in installer.BINARIES}
        files.update({name: b'public notice' for name in ['LICENSE', 'TRADEMARKS.md', 'SECURITY.md', 'THIRD-PARTY-NOTICES.txt', 'RUST-RUNTIME-NOTICES.html']})
        manifest = {'schema_version': '3', 'manifest_role': 'embedded', 'version': '0.3.0',
                    'built_target': self.target, 'supported_platforms': list(installer.TARGETS),
                    'binary_sha256': {name: hashlib.sha256(binary).hexdigest() for name in installer.BINARIES},
                    'license_notices_sha256': hashlib.sha256(b'public notice').hexdigest(),
                    'rust_runtime_notices_sha256': hashlib.sha256(b'public notice').hexdigest()}
        if mutate:
            mutate(manifest)
        files['release-manifest.json'] = json.dumps(manifest).encode() if manifest_data is None else manifest_data
        if missing:
            del files[missing]
        with tarfile.open(self.archive, 'w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, data in files.items():
                member = tarfile.TarInfo(self.package + '/' + name)
                member.size = len(data)
                member.mode = 0o755 if name.startswith('bin/') else 0o644
                tar.addfile(member, io.BytesIO(data))
            if extra:
                for member in extra if isinstance(extra, list) else [extra]:
                    tar.addfile(member, io.BytesIO(b'x' * member.size))
        self.checksum()

    def checksum(self):
        self.sums.write_text(hashlib.sha256(self.archive.read_bytes()).hexdigest() + '  ' + self.name + '\n')

    def install(self, **kwargs):
        return installer.install(self.archive, self.sums, self.destination, target=self.target, **kwargs)

    def refused(self, **kwargs):
        with mock.patch.object(installer.subprocess, 'run') as execute, \
                mock.patch.object(installer, 'host_target', return_value=self.target), \
                mock.patch.object(pathlib.Path, 'mkdir', side_effect=AssertionError('write before validation')) as mkdir:
            with self.assertRaises((ValueError, tarfile.TarError, EOFError, OSError)):
                self.install(**kwargs)
            execute.assert_not_called()
            mkdir.assert_not_called()
        self.assertFalse(self.destination.exists())

    def test_safe_offline_extraction_and_verified_smoke(self):
        self.archive_fixture()
        with mock.patch.object(installer.urllib.request, 'build_opener', side_effect=AssertionError('network forbidden')), \
                mock.patch.object(installer.urllib.request, 'urlopen', side_effect=AssertionError('network forbidden')), \
                mock.patch.object(installer, 'download', side_effect=AssertionError('network forbidden')), \
                mock.patch.object(socket, 'socket', side_effect=AssertionError('network forbidden')), \
                mock.patch.object(installer, 'host_target', return_value=self.target):
            self.assertEqual(installer.main(['--offline', '--archive', str(self.archive), '--checksums', str(self.sums),
                                            '--destination', str(self.destination), '--smoke']), 0)
        self.assertTrue((self.destination / self.package / 'bin/b2ige').is_file())
        self.assertFalse((self.destination / '.b2ige').exists())

    def test_wrong_installed_version_is_refused(self):
        self.archive_fixture(binary=b'#!/bin/sh\necho "verify-cli 9.9.9"\n')
        with mock.patch.object(installer, 'host_target', return_value=self.target):
            with self.assertRaisesRegex(ValueError, 'CLI version mismatch'):
                self.install(smoke=True)

    def test_historical_exact_urls_retained(self):
        for target in installer.TARGETS:
            archive, sums = installer.urls('0.2.0', target)
            self.assertEqual(archive, installer.REPOSITORY + '/v0.2.0/' + installer.filename('0.2.0', target))
            self.assertEqual(sums, installer.REPOSITORY + '/v0.2.0/SHA256SUMS')

    def test_checksum_controls_no_execution(self):
        self.archive_fixture()
        original = self.sums.read_text()
        for value in ['0' * 64 + '  ' + self.name + '\n', '', original * 2,
                      original.replace(self.name, 'other.tar.gz'), 'bad checksum', original + 'broken\n']:
            with self.subTest(value=value[:12]):
                self.sums.write_text(value)
                self.refused(smoke=True)

    def test_checksum_syntax_and_exact_filename_matrix(self):
        self.archive_fixture()
        original = self.sums.read_text()
        digest = original.split()[0]
        for data in [original.encode(), original.replace('\n', '\r\n').encode()]:
            self.assertEqual(installer.checksum_document(data)[self.name], digest)
        for value in [original * 2, original.upper(), original.replace(digest, 'g' * 64),
                      original.replace('  ', ' '), original.replace('  ', '\t'),
                      original.replace('  ', '   '), ' ' + original, original.rstrip() + ' \n',
                      original + '\n', original.replace(self.name, '*' + self.name),
                      *[original.replace(self.name, name) for name in
                        ['/' + self.name, '../' + self.name, 'dir/' + self.name,
                         'dir\\' + self.name, './' + self.name, 'b2\u0456ge.tar.gz', '.', '..']],
                      original + digest + '  path/' + self.name + '\n']:
            with self.subTest(value=value[:90]):
                self.sums.write_bytes(value.encode())
                self.refused(smoke=True)
        # Repeated digests for different filenames are unambiguous: lookup uses
        # the complete exact filename, never basename or substring matching.
        entries = installer.checksum_document((original + digest + '  other.tar.gz\n').encode())
        self.assertEqual(entries[self.name], digest)
        self.sums.write_text(original.replace(self.name, self.name + '.extra'))
        self.refused(smoke=True)

    def test_archive_member_controls(self):
        names = ['/absolute', '../escape', self.package + '/../escape', self.package + '/a/../../escape',
                 'unexpected/root', self.package + '/a\\b', self.package + '/C:evil',
                 self.package + '/NUL', self.package + '/com1.txt', self.package + '/trailing.',
                 self.package + '/bin/b2ige', self.package + '/BIN/B2IGE', self.package + '/bin',
                 self.package + '/BIN', self.package + '/' + '/'.join(['a'] * installer.MAX_DEPTH)]
        for name in names:
            with self.subTest(name=name):
                member = tarfile.TarInfo(name)
                self.archive_fixture(extra=member)
                self.refused(smoke=True)
        for kind in [tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.FIFOTYPE, tarfile.CHRTYPE, tarfile.BLKTYPE,
                     tarfile.GNUTYPE_SPARSE, tarfile.CONTTYPE, b's', b'X']:
            with self.subTest(kind=kind):
                member = tarfile.TarInfo(self.package + '/escape')
                member.type = kind
                member.linkname = '../../outside'
                self.archive_fixture(extra=member)
                self.refused()

    def test_unicode_implicit_parent_and_directory_payload_collisions(self):
        for names in [[self.package + '/caf\u00e9', self.package + '/cafe\u0301'],
                      [self.package + '/A/one', self.package + '/a/two'],
                      [self.package + '/caf\u00e9/one', self.package + '/cafe\u0301/two']]:
            self.archive_fixture(extra=[tarfile.TarInfo(name) for name in names])
            self.refused(smoke=True)
        directory = tarfile.TarInfo(self.package + '/payload')
        directory.type = tarfile.DIRTYPE
        directory.size = 1
        self.archive_fixture(extra=directory)
        self.refused(smoke=True)

    def test_complete_archive_validation_precedes_all_writes_and_execution(self):
        marker = self.root / 'executable-ran'
        self.archive_fixture(binary=('#!/bin/sh\ntouch "' + str(marker) + '"\n').encode())
        original = self.archive.read_bytes()
        raw = gzip.decompress(original)
        with tarfile.open(fileobj=io.BytesIO(original), mode='r:gz') as tar:
            last = tar.getmembers()[-1]
            end = last.offset_data + (last.size + 511) // 512 * 512
        extra = tarfile.TarInfo(self.package + '/non-required-file')
        extra.size = 1024
        pax = tarfile.TarInfo(self.package + '/extension')
        pax.type = tarfile.XHDTYPE
        corrupt_crc = bytearray(original)
        corrupt_crc[-8] ^= 1
        cases = [original[:-8], bytes(corrupt_crc), gzip.compress(raw[:end]),
                 gzip.compress(raw[:end] + bytes(512)), gzip.compress(raw[:end] + b'bad header'),
                 gzip.compress(raw[:end] + extra.tobuf() + b'x' * 600),
                 gzip.compress(raw + b'unparsed trailing payload'),
                 gzip.compress(raw[:end] + pax.tobuf() + bytes(1024))]
        for data in cases:
            with self.subTest(length=len(data)):
                self.archive.write_bytes(data)
                self.checksum()
                self.refused(smoke=True)
                self.assertFalse(marker.exists())
        for metadata in [{'GNU.sparse.map': '0,1'}, {'path': '../escape'}, {'size': '999'}]:
            with tarfile.open(self.archive, 'w:gz', format=tarfile.PAX_FORMAT) as tar:
                member = tarfile.TarInfo(self.package + '/pax')
                member.pax_headers = metadata
                tar.addfile(member)
            self.checksum()
            self.refused(smoke=True)

    def test_extraction_failure_cleans_only_new_destination(self):
        self.archive_fixture()
        sentinel = self.root / 'user-file'
        sentinel.write_text('preserved')
        with mock.patch.object(installer.shutil, 'copyfileobj', side_effect=OSError('synthetic write failure')), \
                mock.patch.object(installer, 'host_target', return_value=self.target), \
                mock.patch.object(installer.subprocess, 'run') as execute, self.assertRaises(OSError):
            self.install(smoke=True)
        execute.assert_not_called()
        self.assertFalse(self.destination.exists())
        self.assertEqual(sentinel.read_text(), 'preserved')

    def test_manifest_and_binary_controls(self):
        for mutation in [lambda m: m.update(version='9.9.9'), lambda m: m.update(built_target='wrong'),
                         lambda m: m.update(supported_platforms=[]), lambda m: m.update(supported_platforms=self.target),
                         lambda m: m.update(schema_version='1'), lambda m: m.update(schema_version=3),
                         lambda m: m.update(manifest_role='release-index'),
                         lambda m: m['binary_sha256'].pop('b2ige'),
                         lambda m: m['binary_sha256'].update(extra='0' * 64),
                         lambda m: m['binary_sha256'].update(b2ige='0' * 64),
                         lambda m: m.update(license_notices_sha256='0' * 64),
                         lambda m: m.update(rust_runtime_notices_sha256='0' * 64)]:
            self.archive_fixture(mutate=mutation)
            self.refused(smoke=True)
        for name in [*['bin/' + name for name in installer.BINARIES], 'release-manifest.json',
                     'LICENSE', 'TRADEMARKS.md', 'SECURITY.md', 'THIRD-PARTY-NOTICES.txt', 'RUST-RUNTIME-NOTICES.html']:
            self.archive_fixture(missing=name)
            self.refused(smoke=True)
        for data in [b'not JSON', b'[]', b'null', b'{"schema_version":"3","schema_version":"3"}']:
            self.archive_fixture(manifest_data=data)
            self.refused(smoke=True)
        member = tarfile.TarInfo(self.package + '/bin/b2ige')
        member.mode = 0o644
        self.archive_fixture(extra=member, missing='bin/b2ige',
                             mutate=lambda m: m['binary_sha256'].update(b2ige=hashlib.sha256(b'').hexdigest()))
        self.refused(smoke=True)
        self.archive_fixture()
        self.archive.write_bytes(self.archive.read_bytes() + b'corrupt after packaging')
        self.refused(smoke=True)

    def test_filename_version_target_and_malformed_archive(self):
        self.archive_fixture()
        self.refused(version='1.0.0')
        original = self.archive
        self.archive = self.root / 'bad.tar.gz'
        self.archive.write_bytes(original.read_bytes())
        self.refused()
        self.archive = original
        with self.assertRaises(ValueError):
            installer.install(self.archive, self.sums, self.destination, target=installer.TARGETS[1])
        self.archive.write_bytes(b'not a tar file')
        self.checksum()
        self.refused()

    def test_archive_resource_limits_precede_writes(self):
        self.archive_fixture()
        for constant, value in [('MAX_ARCHIVE', len(self.archive.read_bytes()) - 1),
                                ('MAX_CONTENT', 1), ('MAX_MEMBERS', 1), ('MAX_PATH', 20), ('MAX_CHECKSUMS', 10)]:
            with self.subTest(constant=constant), mock.patch.object(installer, constant, value):
                self.refused(smoke=True)

    def test_existing_and_symlink_destinations_preserved(self):
        self.archive_fixture()
        self.destination.mkdir()
        sentinel = self.destination / 'keep'
        sentinel.write_text('unchanged')
        with self.assertRaises(ValueError):
            self.install()
        self.assertEqual(sentinel.read_text(), 'unchanged')
        sentinel.unlink()
        self.destination.rmdir()
        self.destination.symlink_to(self.root / 'absent', target_is_directory=True)
        with self.assertRaises(ValueError):
            self.install()
        self.assertTrue(self.destination.is_symlink())
        parent_link = self.root / 'parent-link'
        parent_link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ValueError):
            installer.install(self.archive, self.sums, parent_link / 'new', target=self.target)
        self.assertFalse((self.root / 'new').exists())

    def test_exact_online_inventory_and_platform(self):
        for target in installer.TARGETS:
            archive, sums = installer.urls('0.3.0', target)
            self.assertEqual(archive, installer.REPOSITORY + '/v0.3.0/' + installer.filename('0.3.0', target))
            self.assertEqual(sums, installer.REPOSITORY + '/v0.3.0/SHA256SUMS')
            self.assertNotIn('latest', archive)
        for version in ['latest', 'main', 'v0.3.0', '9.9.9', '../0.3.0']:
            with self.assertRaises(ValueError):
                installer.urls(version, installer.TARGETS[0])
        for system, machine in [('Linux', 'aarch64'), ('Windows', 'AMD64'), ('Unknown', 'x86_64')]:
            with mock.patch.object(installer.platform, 'system', return_value=system), \
                    mock.patch.object(installer.platform, 'machine', return_value=machine), self.assertRaises(ValueError):
                installer.host_target()
        self.assertEqual(installer.RELEASES, {'0.2.0': installer.TARGETS, '0.3.0': installer.TARGETS})

    def test_online_driver_downloads_only_exact_assets(self):
        self.archive_fixture()
        archive_data, checksum_data = self.archive.read_bytes(), self.sums.read_bytes()
        expected = installer.urls('0.3.0', self.target)
        def download(url, path, limit):
            self.assertIn(url, expected)
            path.write_bytes(checksum_data if url.endswith('/SHA256SUMS') else archive_data)
        with mock.patch.object(installer, 'download', side_effect=download) as network:
            self.assertEqual(installer.main(['--version', '0.3.0', '--target', self.target,
                                            '--destination', str(self.destination)]), 0)
            self.assertEqual([c.args[0] for c in network.call_args_list], [expected[1], expected[0]])

    def test_online_redirects_preserve_release_and_cdn_boundary(self):
        selected = installer.urls('0.3.0', self.target)[0]
        request = installer.urllib.request.Request(selected)
        handler = installer.HTTPSOnly(selected)
        for destination in [selected,
                            'https://release-assets.githubusercontent.com/github-production-release-asset/1/asset?sig=bounded-test',
                            'https://objects.githubusercontent.com/github-production-release-asset/1/asset?sig=bounded-test']:
            result = handler.redirect_request(request, None, 302, 'Found', {}, destination)
            self.assertEqual(result.full_url, destination)
        for destination in [selected.replace('/v0.3.0/', '/v9.9.9/'),
                            selected.replace('/download/v0.3.0/', '/latest/download/'),
                            selected.replace('b2ige2-a11y/', 'other/'),
                            selected.replace(self.name, 'SHA256SUMS'),
                            selected.replace('https://', 'http://'),
                            'https://example.invalid/' + self.name,
                            'https://release-assets.githubusercontent.com.example.invalid/github-production-release-asset/1/x',
                            'https://user@release-assets.githubusercontent.com/github-production-release-asset/1/x',
                            'https://release-assets.githubusercontent.com/unrelated',
                            'file:///tmp/archive']:
            with self.subTest(destination=destination), self.assertRaises(ValueError):
                handler.redirect_request(request, None, 302, 'Found', {}, destination)
        with mock.patch.object(installer.urllib.request, 'build_opener', side_effect=AssertionError('network forbidden')):
            with self.assertRaises(ValueError):
                installer.download('https://example.invalid/' + self.name, self.archive, 100)
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = b'x' * 11
        opener = mock.Mock()
        opener.open.return_value = response
        with mock.patch.object(installer.urllib.request, 'build_opener', return_value=opener):
            with self.assertRaises(ValueError):
                installer.download(selected, self.archive, 10)
        self.assertFalse(self.archive.exists())

    def test_offline_bad_arguments_never_network(self):
        with mock.patch.object(installer, 'download', side_effect=AssertionError('network forbidden')):
            self.assertEqual(installer.main(['--offline', '--destination', str(self.destination)]), 3)


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.registry = self.root / 'registry.json'
        self.entry = {'product': 'sideeffect', 'config': 'reviewed.json', 'store': 'runs', 'authorization': None}
        self.registry.write_text(json.dumps({'schema_version': '1', 'entries': {'login': self.entry}}))
        self.options = ['--identity', 'login', '--provider', 'github-actions', '--registry', str(self.registry),
                        '--verifier-repo', 'b2ige2-a11y/b2ige-verify', '--verifier-ref', PIN]

    def call(self, extra=(), options=None, cwd=None):
        return subprocess.run([str(BINARY), 'ci', 'init', *(self.options if options is None else options), *extra],
                              cwd=cwd or self.root, capture_output=True, text=True, timeout=30)

    def test_preview_no_write_determinism_and_write(self):
        before = sorted(self.root.rglob('*'))
        a, b = self.call(), self.call()
        self.assertEqual(a.returncode, 0, a.stderr)
        self.assertEqual(a.stdout, b.stdout)
        self.assertEqual(before, sorted(self.root.rglob('*')))
        self.assertIn('verification_performed: false', a.stdout)
        self.assertIn('separate isolated build provisioning REQUIRED', a.stdout)
        yaml = a.stdout.split('--- workflow ---\n')[1].rstrip() + '\n'
        doc = validator.parse(yaml)
        validator.validate_bootstrap(doc)
        workflow = self.root / '.github/workflows/b2ige-verify.yml'
        self.assertEqual(self.call(['--write']).returncode, 0)
        self.assertEqual(workflow.read_text(), yaml)
        self.assertNotEqual(self.call(['--write']).returncode, 0)
        self.assertEqual(workflow.read_text(), yaml)

    def test_invalid_fields_and_injection(self):
        for key, values in {
            '--identity': ['unknown', '../login', 'x;touch injected', 'x\nrun: evil', '$(whoami)', '${{ github.token }}', ''],
            '--provider': ['gitlab', 'github'],
            '--verifier-ref': ['main', 'master', 'latest', 'v0.3.0', 'abcd', 'a' * 39, 'g' * 40,
                               'A' * 40, 'a' * 40 + '\n', '${{ github.sha }}', '$(whoami)'],
            '--verifier-repo': ['owner/repo;evil', 'owner/../repo', 'x\nrun: evil', '$(whoami)/repo'],
        }.items():
            for value in values:
                with self.subTest(key=key, value=value):
                    options = self.options.copy()
                    options[options.index(key) + 1] = value
                    self.assertNotEqual(self.call(['--write'], options).returncode, 0)
        options = self.options[:-2]
        self.assertNotEqual(self.call(options=options).returncode, 0)
        self.assertFalse((self.root / '.github').exists())
        self.assertFalse((self.root / 'injected').exists())

    def test_registry_and_root_are_literal_paths_not_workflow_source(self):
        hostile = self.root / "literal ' $(touch injected) ${{ github.token }}\npath"
        hostile.mkdir()
        registry = hostile / "registry ' $(touch injected).json"
        registry.write_bytes(self.registry.read_bytes())
        options = self.options.copy()
        options[options.index('--registry') + 1] = str(registry)
        ordinary = self.call().stdout.split('--- workflow ---\n')[1]
        preview = self.call(['--root', str(hostile)], options)
        self.assertEqual(preview.returncode, 0, preview.stderr)
        self.assertEqual(preview.stdout.split('--- workflow ---\n')[1], ordinary)
        self.assertFalse((hostile / '.github').exists())
        created = self.call(['--root', str(hostile), '--write'], options)
        self.assertEqual(created.returncode, 0, created.stderr)
        self.assertEqual((hostile / '.github/workflows/b2ige-verify.yml').read_text(), ordinary.rstrip() + '\n')
        self.assertFalse((self.root / 'injected').exists())

    def test_workflows_directory_symlink_is_never_followed(self):
        outside = self.root / 'outside'
        outside.mkdir()
        (self.root / '.github').mkdir()
        (self.root / '.github/workflows').symlink_to(outside, target_is_directory=True)
        result = self.call(['--write'])
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(outside.iterdir()), [])

    def test_template_tokens_are_literal_and_numeric_pin_is_string(self):
        self.registry.write_text(json.dumps({'schema_version': '1', 'entries': {'__REPO__': self.entry}}))
        options = self.options.copy()
        options[1] = '__REPO__'
        options[options.index('--verifier-repo') + 1] = '__PIN__/__IDENTITY__'
        options[-1] = '1' * 40
        result = self.call(options=options)
        self.assertEqual(result.returncode, 0, result.stderr)
        doc = validator.parse(result.stdout.split('--- workflow ---\n')[1])
        validator.validate_bootstrap(doc)
        self.assertEqual(doc['jobs']['verify']['steps'][1]['with']['ref'], '1' * 40)
        self.assertEqual(doc['jobs']['verify']['steps'][1]['with']['repository'], '__PIN__/__IDENTITY__')

    def test_bad_registry_and_duplicate_identity(self):
        entry = json.dumps(self.entry)
        for data in ['{malformed', '{}', '{"schema_version":"1","entries":{"login":' + entry + ',"login":' + entry + '}}',
                     '{"schema_version":"1","schema_version":"1","entries":{"login":' + entry + '}}',
                     json.dumps({'schema_version': '2', 'entries': {'login': self.entry}}),
                     json.dumps({'schema_version': '1', 'entries': {'login': dict(self.entry, config='')}})]:
            self.registry.write_text(data)
            self.assertNotEqual(self.call(['--write']).returncode, 0)
            self.assertEqual(self.registry.read_text(), data)
            self.assertFalse((self.root / '.github').exists())

    def test_destination_controls_and_changed_cwd(self):
        for path in ['../other.yml', '/tmp/other.yml', '.github/workflows/../other.yml',
                     '.github/workflows/a/b.yml', '.github/workflows/a;evil.yml', 'elsewhere.yml']:
            self.assertNotEqual(self.call(['--workflow', path, '--write']).returncode, 0)
        other = self.root / 'other'
        other.mkdir()
        self.assertEqual(self.call(cwd=other).returncode, 0)
        self.assertFalse((other / '.github').exists())
        self.assertEqual(self.call(['--root', str(other), '--write']).returncode, 0)
        workflow = other / '.github/workflows/b2ige-verify.yml'
        workflow.unlink()
        workflow.symlink_to(self.root / 'absent')
        self.assertNotEqual(self.call(['--root', str(other), '--write']).returncode, 0)
        self.assertTrue(workflow.is_symlink())
        (self.root / '.github').symlink_to(other / '.github', target_is_directory=True)
        self.assertNotEqual(self.call(['--write']).returncode, 0)

    def test_controller_mutations_rejected(self):
        doc = validator.parse(self.call().stdout.split('--- workflow ---\n')[1])
        edits = [
            lambda d: d['permissions'].update(contents='write'),
            lambda d: d['jobs']['verify']['steps'][0]['with'].update(ref='${{ github.event.pull_request.head.sha }}'),
            lambda d: d['jobs']['verify']['steps'][1]['with'].update(ref='main'),
            lambda d: d['jobs']['verify']['steps'][2].update(**{'working-directory': 'project'}),
            lambda d: d['jobs']['verify']['steps'][3].update(run='echo automatic approval'),
            lambda d: d['jobs']['verify']['steps'][4].update(run='git checkout "$B2IGE_CANDIDATE_REVISION"'),
            lambda d: d['jobs']['verify']['steps'][5].update(run='b2ige doctor'),
            lambda d: d['jobs']['verify']['steps'][5].update(run='candidate/scripts/diff-verify.py || true'),
            lambda d: d['jobs']['verify']['steps'][6]['with'].update(path='project/**'),
            lambda d: d['jobs']['verify']['steps'][6]['with'].update(path='.b2ige/**'),
            lambda d: d['jobs']['verify']['steps'][6]['with'].update(path='private/oracle'),
            lambda d: d['jobs']['verify']['steps'][5].update(**{'continue-on-error': True}),
        ]
        for edit in edits:
            changed = copy.deepcopy(doc)
            edit(changed)
            with self.assertRaises((AssertionError, TypeError)):
                validator.validate_bootstrap(changed)

    def test_external_project_base_inputs_and_missing_approval_fail_closed(self):
        project = self.root / 'project'
        project.mkdir()
        def git(*args):
            return subprocess.check_output(['git', '-C', str(project), *args], text=True).strip()
        git('init', '-q')
        git('config', 'user.name', 'Synthetic Test')
        git('config', 'user.email', 'synthetic@example.invalid')
        (project / '.b2ige').mkdir()
        registry = project / '.b2ige/project.json'
        registry.write_bytes(self.registry.read_bytes())
        git('add', '.')
        git('commit', '-qm', 'trusted fixture')
        base = git('rev-parse', 'HEAD')
        (project / 'scripts').mkdir()
        (project / 'scripts/ci-verify.py').write_text('raise RuntimeError("candidate adapter must not execute")')
        registry.write_text(json.dumps({'schema_version': '1', 'entries': {'substituted': self.entry}}))
        git('add', '.')
        git('commit', '-qm', 'candidate fixture')
        head = git('rev-parse', 'HEAD')
        command = [sys.executable, str(ROOT / 'scripts/diff-verify.py'), '--project-root', str(project),
                   '--base', base, '--head', head, '--trusted-revision', base, '--b2ige', str(BINARY)]
        result = subprocess.run(command, cwd=self.root, capture_output=True, text=True)
        self.assertEqual(result.returncode, 3)
        self.assertFalse(json.loads(result.stdout)['gate_pass'])
        git('checkout', '-q', base)
        # Trusted registry restored; independently pinned tooling still refuses
        # absent candidate approval instead of running candidate adapter/code.
        result = subprocess.run(command, cwd=self.root, capture_output=True, text=True)
        self.assertEqual(result.returncode, 3)
        self.assertFalse((project / 'b2ige-agent-reports').exists())
        diff = module('diff-verify')
        self.assertEqual(diff.TOOL_ROOT, ROOT)
        diff.ROOT = project
        self.assertEqual(diff.required_entries(base, '.b2ige/project.json')['login'], self.entry)

    def test_readiness_malformed_and_nonpass_cannot_turn_green(self):
        raw = self.root / 'response.json'
        for value in [b'not JSON', b'{}', b'{"kind":"readiness","ready":true}',
                      *[json.dumps({'protocol_version': '1', 'verdict': v, 'product': 'behavior',
                                    'operation': 'verify'}).encode() for v in ['FAIL', 'INCONCLUSIVE', 'ERROR']]]:
            raw.write_bytes(value)
            result = subprocess.run([str(BINARY), 'ci-check', str(raw), '0'], capture_output=True)
            self.assertEqual(result.returncode, 3)
        # Complete transport fixtures: first prove each payload is accepted with
        # its real code, then show coercing that same payload to exit 0 is ERROR.
        payload = {'protocol_version': '1', 'product': 'behavior', 'operation': 'verify',
                   'kind': 'synthetic_transport', 'summary': 'transport test only',
                   'expected': None, 'observed': None, 'reproduction': None, 'evidence_refs': [],
                   'source': {'artifact_id': 'synthetic', 'integrity_hash': 'sha256:' + 'a' * 64},
                   'scope': {'description': 'transport only', 'coverage': {}, 'budget': {}},
                   'limitations': ['not authoritative evidence'], 'next_action': 'review', 'other_failure_count': 0}
        for verdict, code in [('FAIL', 1), ('INCONCLUSIVE', 2), ('ERROR', 3)]:
            payload['verdict'] = verdict
            raw.write_text(json.dumps(payload))
            matching = subprocess.run([str(BINARY), 'ci-check', str(raw), str(code)], capture_output=True)
            self.assertEqual(matching.returncode, code)
            coerced = subprocess.run([str(BINARY), 'ci-check', str(raw), '0'], capture_output=True)
            self.assertEqual(coerced.returncode, 3)
            self.assertEqual(json.loads(coerced.stdout)['verdict'], 'ERROR')


if __name__ == '__main__':
    unittest.main()
