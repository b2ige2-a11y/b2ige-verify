#!/usr/bin/env python3
"""Validate every distributable, extracted inventory, and external manifest hashes."""
import hashlib
import json
import pathlib
import re
import sys
import tarfile
import tempfile
import tomllib
from hygiene import ROOT, scan


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def members_safe(members):
    seen = set()
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or '..' in path.parts or member.name in seen:
            raise ValueError('Unsafe or duplicate archive member')
        seen.add(member.name)
        if member.uid or member.gid or member.uname or member.gname:
            raise ValueError('Archive embeds local account metadata')
        if not (member.isfile() or member.isdir()): raise ValueError('Links/devices are not allowed in RC archives')
        if any(p in {'.git', '.b2ige', '.DS_Store', 'target', 'node_modules', '__pycache__', '.cache'} or p == '.env' or p.startswith('.env.') for p in path.parts):
            raise ValueError('Private/cache archive member')


V110_DOCS = ['docs/ADOPTION.md', 'docs/V110-ADOPTION-BENCH.md',
             'docs/V110-DISTRIBUTION.md', 'docs/V110-EXTERNAL-PILOT.md',
             'docs/RELEASE-0.3.0.md', 'external-pilot/v1/protocol.json',
             'benchmarks/adoption-v1/scenarios.json', 'scripts/install-release.py']
SOURCE_TOOLS = ['scripts/adoption-bench.py', 'scripts/adoption_bench.py',
                'scripts/external-pilot.py', 'scripts/external_pilot_ci.py',
                'crates/verify-cli/examples/adoption_fixture.rs',
                'crates/verify-cli/src/ci-workflow.yml']


def product_metadata(root):
    workspace = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']
    version = workspace['package']['version']
    assert version == '0.3.0', 'release product version'
    assert workspace['package']['publish'] is False, 'Cargo publication enabled'
    for member in workspace['members']:
        package = tomllib.loads((root / member / 'Cargo.toml').read_text())['package']
        assert package['version'] == {'workspace': True}
        assert package['publish'] is False or package['publish'] == {'workspace': True}, 'crate publication enabled'
    locked = tomllib.loads((root / 'Cargo.lock').read_text())['package']
    assert all(p['version'] == version for p in locked if 'source' not in p)
    npm = json.loads((root / 'npm/b2ige/package.json').read_text())
    assert npm['version'] == version and npm['private'] is True
    assert "throw Error('Publication blocked:" in npm['scripts']['prepublishOnly']
    native = json.loads((root / 'npm/b2ige/native-manifest.json').read_text())
    assert native['version'] == version and native['schema_version'] == '2'
    schema = json.loads((root / 'release/release-manifest.schema.json').read_text())
    assert schema['properties']['version']['const'] == version
    assert schema['properties']['schema_version']['const'] == '3'
    return version


def candidate_manifest(data, version):
    targets = {'aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu'}
    assert data['version'] == version and data['schema_version'] == '3'
    assert set(data['supported_platforms']) == targets
    assert data['built_target'] in targets
    assert set(data['actually_verified_platforms']) <= {data['built_target']}
    assert set(data['platform_validation']) == targets
    assert {t for t, status in data['platform_validation'].items() if status == 'VERIFIED_NATIVE'} == set(data['actually_verified_platforms'])
    assert data['signing']['publisher_signed'] is False
    assert data['signing']['apple_notarized'] is False
    assert data['publication_ready'] is False and data['owner_publication_authorized'] is False


def validate(folder):
    version = product_metadata(ROOT)
    listed = {}
    for line in (folder / 'SHA256SUMS').read_text().splitlines():
        expected, name = line.split('  ', 1)
        assert pathlib.Path(name).name == name and name not in listed
        assert re.fullmatch('[a-f0-9]{64}', expected)
        assert digest(folder / name) == expected, name
        listed[name] = expected
    archives = sorted([*folder.glob('*.tar.gz'), *folder.glob('*.tgz')])
    assert archives and all(p.name in listed for p in archives), 'Unchecksummed archive'
    for archive in archives:
        with tempfile.TemporaryDirectory(prefix='b2ige-archive-check-') as tmp:
            root = pathlib.Path(tmp)
            with tarfile.open(archive) as t:
                members_safe(t.getmembers()); t.extractall(root, filter='data')
            scan([p for p in root.rglob('*') if p.is_file()])
            expected_names = {f'b2ige-{version}-source.tar.gz', f'b2ige-verify-{version}.tgz',
                              *[f'b2ige-{version}-{t}.tar.gz' for t in ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']]}
            assert archive.name in expected_names, 'stale/unexpected archive name'
            if archive.name.endswith('-source.tar.gz'):
                assert (root / f'b2ige-{version}-source/release/release-manifest.json').is_file()
            elif archive.name.endswith('.tar.gz'):
                assert (root / archive.name[:-7] / 'release-manifest.json').is_file()
            for manifest in root.glob('*/release-manifest.json'):
                data = json.loads(manifest.read_text()); package = manifest.parent
                candidate_manifest(data, version)
                assert archive.name == f'b2ige-{version}-{data["built_target"]}.tar.gz'
                assert package.name == archive.name[:-7]
                for binary, expected in data['binary_sha256'].items():
                    assert pathlib.Path(binary).name == binary
                    assert digest(package / 'bin' / binary) == expected, binary
                assert digest(package / 'THIRD-PARTY-NOTICES.txt') == data['license_notices_sha256']
                assert digest(package / 'RUST-RUNTIME-NOTICES.html') == data['rust_runtime_notices_sha256']
                for required in ['LICENSE', 'TRADEMARKS.md', 'SECURITY.md']: assert (package / required).is_file(), required
                for required in ['docs/VERIFICATION-PROTOCOL.md', 'docs/V100-RELEASE.md',
                                 'conformance/V100-PROTOCOL.md', *V110_DOCS]:
                    assert (package / required).is_file(), required
            for m in root.glob('*/release/release-manifest.json'):
                source = m.parent.parent
                assert source.name == f'b2ige-{version}-source'
                assert product_metadata(source) == version
                candidate_manifest(json.loads(m.read_text()), version)
                for required in [*V110_DOCS, *SOURCE_TOOLS]:
                    assert (source / required).is_file(), required
                paths = sorted(p for p in source.rglob('*') if p.is_file() and p != m)
                actual = hashlib.sha256(b''.join(str(p.relative_to(source)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest() for p in paths)).hexdigest()
                assert actual == json.loads(m.read_text())['source_inventory_sha256'], 'source inventory mismatch'
            if archive.suffix == '.tgz':
                package = root / 'package'; meta = json.loads((package / 'package.json').read_text())
                assert meta['version'] == version
                assert meta['private'] is True and meta['license'] == 'Apache-2.0'
                assert 'prepublishOnly' in meta['scripts']
                for required in ['LICENSE', 'TRADEMARKS.md', 'native-manifest.json', 'bin/launch.cjs']: assert (package / required).is_file()
                assert not (package / 'native').exists(), 'Unreviewed npm native payload'
    for m in folder.glob('*.manifest.json'):
        assert m.name in listed, 'Unchecksummed manifest'
        data = json.loads(m.read_text()); assert data['manifest_role'] == 'release-index'
        candidate_manifest(data, version)
        assert data['artifact_sha256'], 'Missing external artifact hashes'
        for name, expected in data['artifact_sha256'].items(): assert listed.get(name) == expected, name
        assert not data['publication_ready'] and not data['owner_publication_authorized']
    print('Native/source/npm extraction, membership, notices, checksums, binary and source hashes: PASS')


if __name__ == '__main__':
    validate(pathlib.Path(sys.argv[1]))
