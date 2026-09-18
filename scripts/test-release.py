#!/usr/bin/env python3
"""Focused negative tests for publication packaging safeguards."""
import copy
import gzip
import hashlib
import io
import json
import shutil
import subprocess
import importlib.util
import pathlib
import os
import tarfile
import tempfile
import unittest
import tomllib
from unittest import mock
from hygiene import ROOT, scan
from installed_cli_smoke import check_cli

spec = importlib.util.spec_from_file_location('validate_archive', pathlib.Path(__file__).with_name('validate-archive.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)


def encoded(data):
    return (json.dumps(data, sort_keys=True, indent=2) + '\n').encode()


def archive_files(path, contents):
    """Deterministic synthetic USTAR fixture, never platform qualification evidence."""
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, data in sorted(contents.items()):
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o755 if '/bin/' in name else 0o644
            archive.addfile(info, io.BytesIO(data))
    path.write_bytes(gzip.compress(stream.getvalue(), mtime=0))


def write_checksums(folder):
    (folder / 'SHA256SUMS').write_text(''.join(
        module.digest(p) + '  ' + p.name + '\n'
        for p in sorted(folder.iterdir()) if p.name != 'SHA256SUMS'))


def public_fixture(folder, source_target=module.TARGETS[0]):
    """Three synthetic targets with deliberately different SBOM/provenance bytes."""
    version = '0.3.0'
    source_names = {'Cargo.toml', 'Cargo.lock', 'npm/b2ige/package.json',
                    'npm/b2ige/native-manifest.json', 'release/release-manifest.schema.json',
                    *module.V110_DOCS, *module.SOURCE_TOOLS,
                    *[str(p.relative_to(ROOT)) for p in (ROOT / 'crates').glob('*/Cargo.toml')]}
    source = {name: (ROOT / name).read_bytes() for name in source_names}
    inventory = hashlib.sha256(b''.join(name.encode() + b'\0' + hashlib.sha256(source[name]).digest()
                                        for name in sorted(source))).hexdigest()
    locked = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
    components = [dict(type='library', name=p['name'], version=p['version'],
                       **{'bom-ref': p['name'] + '@' + p['version']},
                       hashes=[{'alg': 'SHA-256', 'content': p['checksum']}] if 'checksum' in p else [])
                  for p in locked]
    for index, target in enumerate(module.TARGETS):
        name = f'b2ige-{version}-{target}'
        contents = {f'bin/{binary}': b'synthetic executable bytes: ' + target.encode()
                    for binary in module.installer.BINARIES}
        contents.update({required: b'synthetic notice/document\n' for required in
                         ['LICENSE', 'TRADEMARKS.md', 'SECURITY.md', 'THIRD-PARTY-NOTICES.txt',
                          'RUST-RUNTIME-NOTICES.html', 'docs/VERIFICATION-PROTOCOL.md',
                          'docs/V100-RELEASE.md', 'conformance/V100-PROTOCOL.md', *module.V110_DOCS]})
        sboms = {}
        for component in components:
            if not component['name'].startswith('verify-'):
                continue
            bom = dict(bomFormat='CycloneDX', specVersion='1.5', version=1,
                       metadata=dict(component=component, timestamp=f'2026-01-01T00:00:0{index}Z'),
                       components=[c for c in components if c != component],
                       dependencies=[{'ref': c['bom-ref'], 'dependsOn': []} for c in components])
            filename = f'b2ige-{version}-native-{component["name"]}.cdx.json'
            contents[filename] = encoded(bom)
            sboms[filename] = hashlib.sha256(contents[filename]).hexdigest()
        assert len(sboms) == 6
        npm_bom = encoded(dict(bomFormat='CycloneDX', specVersion='1.5', version=1,
                              metadata=dict(component=dict(type='application', name='@b2ige/verify', version=version))))
        data = json.loads((ROOT / 'release/release-manifest.json').read_text())
        data.update(version=version, manifest_role='embedded', artifact_sha256={},
                    git_commit='a' * 40, working_tree_dirty=False, built_target=target,
                    rust_toolchain='synthetic fixture: ' + target,
                    actually_verified_platforms=[], platform_validation={t: 'NOT_RUN' for t in module.TARGETS},
                    platform_validation_scope={t: 'synthetic fixture' for t in module.TARGETS},
                    local_release_candidate_ready=False, source_inventory_sha256=inventory,
                    binary_sha256={b: hashlib.sha256(contents['bin/' + b]).hexdigest() for b in module.installer.BINARIES},
                    license_notices_sha256=hashlib.sha256(contents['THIRD-PARTY-NOTICES.txt']).hexdigest(),
                    rust_runtime_notices_sha256=hashlib.sha256(contents['RUST-RUNTIME-NOTICES.html']).hexdigest(),
                    build_timestamp_nonsemantic=f'2026-01-01T00:00:0{index}Z',
                    ci_provenance={'GITHUB_SHA': 'a' * 40, 'GITHUB_RUN_ID': 'synthetic-fixture'},
                    sbom=dict(status='GENERATED', files=sboms,
                              npm=dict(status='GENERATED', file=f'b2ige-{version}-npm.cdx.json',
                                       sha256=hashlib.sha256(npm_bom).hexdigest())))
        contents['release-manifest.json'] = encoded(data)
        archive_files(folder / (name + '.tar.gz'), {name + '/' + k: v for k, v in contents.items()})
        if target == source_target:
            source['release/release-manifest.json'] = encoded(data)
            archive_files(folder / f'b2ige-{version}-source.tar.gz',
                          {f'b2ige-{version}-source/' + k: v for k, v in source.items()})
        data.update(manifest_role='release-index', actually_verified_platforms=[target],
                    artifact_sha256={name + '.tar.gz': module.digest(folder / (name + '.tar.gz'))})
        data['platform_validation'][target] = 'VERIFIED_NATIVE'
        (folder / (name + '.manifest.json')).write_bytes(encoded(data))
    write_checksums(folder)
    return npm_bom


class PublicationAssembly(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.original = pathlib.Path(cls.temp.name) / 'original'
        cls.original.mkdir()
        cls.npm_bom = public_fixture(cls.original, module.installer.host_target())

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def setUp(self):
        self.work = tempfile.TemporaryDirectory()
        self.addCleanup(self.work.cleanup)
        self.folder = pathlib.Path(self.work.name) / 'assets'
        shutil.copytree(self.original, self.folder)
        self.target = module.installer.host_target()
        self.other_target = next(t for t in module.TARGETS if t != self.target)
        self.name = f'b2ige-0.3.0-{self.target}'
        self.manifest = self.folder / (self.name + '.manifest.json')
        # CI installs CycloneDX/jsonschema tooling AFTER these unit tests. Mock
        # only that subprocess boundary; extraction, hashes, inventory, manifests
        # and security checks run normally. The fresh package gate runs the real
        # schema/dependency checker on both sidecars and each embedded SBOM set.
        patch = mock.patch.object(module, 'validate_sboms')
        self.schema_check = patch.start()
        self.addCleanup(patch.stop)

    def validate(self):
        module.validate(self.folder, public=True, expected_commit='a' * 40)

    def mutate_manifest(self, mutation):
        data = json.loads(self.manifest.read_text())
        mutation(data)
        self.manifest.write_bytes(encoded(data))
        write_checksums(self.folder)

    def mutate_archive(self, filename, mutation):
        archive = self.folder / filename
        with tarfile.open(archive) as t:
            contents = {m.name: t.extractfile(m).read() for m in t.getmembers() if m.isfile()}
        mutation(contents)
        archive_files(archive, contents)
        if filename == self.name + '.tar.gz':
            self.mutate_manifest(lambda m: m['artifact_sha256'].update({filename: module.digest(archive)}))
        write_checksums(self.folder)

    def candidate(self):
        for target in (t for t in module.TARGETS if t != self.target):
            for suffix in ['.tar.gz', '.manifest.json']:
                (self.folder / f'b2ige-0.3.0-{target}{suffix}').unlink()
        with tarfile.open(self.folder / (self.name + '.tar.gz')) as t:
            for m in t.getmembers():
                if m.name.endswith('.cdx.json'):
                    (self.folder / pathlib.Path(m.name).name).write_bytes(t.extractfile(m).read())
        (self.folder / 'b2ige-0.3.0-npm.cdx.json').write_bytes(self.npm_bom)
        names = ['package.json', 'native-manifest.json',
                 *[str(p.relative_to(ROOT / 'npm/b2ige')) for p in (ROOT / 'npm/b2ige/bin').glob('*.cjs')]]
        package = {f'package/{n}': (ROOT / 'npm/b2ige' / n).read_bytes() for n in names}
        package.update({f'package/{n}': (ROOT / n).read_bytes() for n in ['LICENSE', 'TRADEMARKS.md']})
        archive_files(self.folder / 'b2ige-verify-0.3.0.tgz', package)
        write_checksums(self.folder)

    def test_public_eight_assets_with_different_runner_sboms(self):
        self.assertEqual(len(list(self.folder.iterdir())), 8)
        manifests = [json.loads(p.read_text()) for p in self.folder.glob('*.manifest.json')]
        self.assertEqual(len({tuple(sorted(m['sbom']['files'].values())) for m in manifests}), 3)
        self.validate()
        self.assertEqual(self.schema_check.call_count, 3)

    def test_exact_target_binding_rejects_missing_wrong_and_extra_assets(self):
        original = self.manifest.read_bytes()
        for extra in ['b2ige-0.3.0-source.tar.gz', 'b2ige-verify-0.3.0.tgz',
                      *module.native_sbom_names('0.3.0'), 'b2ige-0.3.0-npm.cdx.json',
                      f'b2ige-0.3.0-{self.other_target}.tar.gz', self.manifest.name, 'unrelated.txt']:
            with self.subTest(extra=extra):
                self.manifest.write_bytes(original)
                self.mutate_manifest(lambda d: d['artifact_sha256'].update({extra: '0' * 64}))
                with self.assertRaisesRegex(AssertionError, 'target-owned'):
                    self.validate()
        for bindings in [{}, {f'b2ige-0.3.0-{self.other_target}.tar.gz': '0' * 64}]:
            self.mutate_manifest(lambda d: d.update(artifact_sha256=bindings))
            with self.assertRaisesRegex(AssertionError, 'target-owned'):
                self.validate()

    def test_public_rejects_candidate_only_and_unsupported_assets(self):
        for name in ['b2ige-verify-0.3.0.tgz', 'b2ige-0.3.0-native-verify-cli.cdx.json',
                     'b2ige-0.3.0-npm.cdx.json', 'b2ige-0.3.0-x86_64-pc-windows-msvc.tar.gz',
                     'b2ige-0.3.0-aarch64-unknown-linux-gnu.tar.gz', 'b2ige.mcpb', 'extra.txt']:
            with self.subTest(name=name):
                (self.folder / name).write_bytes(b'unexpected')
                write_checksums(self.folder)
                with self.assertRaisesRegex(AssertionError, 'Unexpected/missing'):
                    self.validate()
                (self.folder / name).unlink()

    def test_public_checksums_require_all_seven_and_no_self_or_duplicates(self):
        path = self.folder / 'SHA256SUMS'
        lines = path.read_text().splitlines(keepends=True)
        for index in range(7):
            path.write_text(''.join(lines[:index] + lines[index + 1:]))
            with self.assertRaisesRegex(AssertionError, 'Checksum inventory'):
                self.validate()
        for extra in [lines[0], '0' * 64 + '  SHA256SUMS\n', '0' * 64 + '  ../escape\n']:
            path.write_text(''.join(lines) + extra)
            with self.assertRaises((AssertionError, ValueError)):
                self.validate()

    def test_target_hash_mismatch_even_with_updated_release_checksums(self):
        self.mutate_manifest(lambda d: d['artifact_sha256'].update({self.name + '.tar.gz': '0' * 64}))
        with self.assertRaisesRegex(AssertionError, 'Target archive hash'):
            self.validate()

    def test_target_version_commit_and_embedded_identity_mismatches(self):
        original = self.manifest.read_bytes()
        for mutation in [lambda d: d.update(version='0.2.0'),
                         lambda d: d.update(built_target=self.other_target),
                         lambda d: d.update(git_commit='b' * 40),
                         lambda d: d.update(working_tree_dirty=True),
                         lambda d: d['ci_provenance'].update(GITHUB_SHA='b' * 40),
                         lambda d: d['binary_sha256'].update(b2ige='0' * 64),
                         lambda d: d.update(source_inventory_sha256='0' * 64)]:
            self.manifest.write_bytes(original)
            self.mutate_manifest(mutation)
            with self.assertRaises(AssertionError):
                self.validate()

    def test_candidate_checksums_cover_source_npm_and_all_sboms(self):
        self.candidate()
        module.validate(self.folder)
        listed = module.checksums(self.folder)
        self.assertEqual(len(listed), 11)
        self.assertEqual(self.schema_check.call_count, 2)
        for name in ['b2ige-0.3.0-source.tar.gz', 'b2ige-verify-0.3.0.tgz',
                     'b2ige-0.3.0-npm.cdx.json', *module.native_sbom_names('0.3.0')]:
            path = self.folder / name
            original = path.read_bytes()
            path.write_bytes(original + b'tampered')
            with self.assertRaises(AssertionError):
                module.validate(self.folder)
            path.write_bytes(original)
        missing = self.folder / 'b2ige-verify-0.3.0.tgz'
        missing.unlink()
        write_checksums(self.folder)
        with self.assertRaisesRegex(AssertionError, 'Unexpected/missing'):
            module.validate(self.folder)

    def test_embedded_six_sboms_reject_missing_or_rehashed_tampering(self):
        archive = self.folder / (self.name + '.tar.gz')
        original_archive, original_manifest = archive.read_bytes(), self.manifest.read_bytes()
        sbom = sorted(module.native_sbom_names('0.3.0'))[0]
        for mutation in [lambda files: files.pop(self.name + '/' + sbom),
                         lambda files: files.update({self.name + '/' + sbom: b'{}'})]:
            archive.write_bytes(original_archive)
            self.manifest.write_bytes(original_manifest)
            self.mutate_archive(archive.name, mutation)
            with self.assertRaises(AssertionError):
                self.validate()

    def test_source_version_and_normal_content_tampering_reject(self):
        archive = self.folder / 'b2ige-0.3.0-source.tar.gz'
        original = archive.read_bytes()
        for member in ['Cargo.toml', 'docs/RELEASE-0.3.0.md']:
            archive.write_bytes(original)
            self.mutate_archive(archive.name, lambda files: files.update({
                'b2ige-0.3.0-source/' + member: files['b2ige-0.3.0-source/' + member].replace(b'0.3.0', b'9.9.9')}))
            with self.assertRaises(AssertionError):
                self.validate()

    def test_npm_smoke_uses_candidate_checksum_without_manifest_binding(self):
        self.candidate()
        binary = (ROOT / 'target/release/b2ige').read_bytes()
        self.mutate_archive(self.name + '.tar.gz', lambda files: files.update({self.name + '/bin/b2ige': binary}))
        self.mutate_manifest(lambda d: d['binary_sha256'].update(b2ige=hashlib.sha256(binary).hexdigest()))
        command = ['python3', str(ROOT / 'scripts/npm-archive-smoke.py'), str(self.folder)]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('tamper checks: PASS', result.stdout)
        self.assertEqual(set(json.loads(self.manifest.read_text())['artifact_sha256']), {self.name + '.tar.gz'})
        npm = self.folder / 'b2ige-verify-0.3.0.tgz'
        npm.write_bytes(npm.read_bytes() + b'tampered')
        self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)

    def test_historical_install_versions_remain_supported(self):
        self.assertEqual(module.installer.RELEASES['0.2.0'], module.TARGETS)
        with self.assertRaises(AssertionError):
            historical = subprocess.check_output(['git', 'show', 'HEAD:release/release-manifest.json'], cwd=ROOT)
            module.target_artifacts(json.loads(historical), '0.3.0')


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
