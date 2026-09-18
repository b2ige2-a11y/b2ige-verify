#!/usr/bin/env python3
"""Validate every distributable, extracted inventory, and external manifest hashes."""
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import sys
import subprocess
import tarfile
import tempfile
import tomllib
from hygiene import ROOT, scan

spec = importlib.util.spec_from_file_location('install_release', ROOT / 'scripts/install-release.py')
installer = importlib.util.module_from_spec(spec); spec.loader.exec_module(installer)
TARGETS = installer.TARGETS


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
    packages = {}
    for member in workspace['members']:
        package = tomllib.loads((root / member / 'Cargo.toml').read_text())['package']
        assert package['version'] == {'workspace': True}
        assert package['publish'] is False or package['publish'] == {'workspace': True}, 'crate publication enabled'
        packages[package['name']] = version
    locked = tomllib.loads((root / 'Cargo.lock').read_text())['package']
    local = [p for p in locked if 'source' not in p]
    assert len(local) == len(packages) and {p['name']: p['version'] for p in local} == packages, 'workspace lock inventory mismatch'
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
    targets = set(TARGETS)
    assert data['version'] == version and data['schema_version'] == '3'
    assert set(data['supported_platforms']) == targets
    assert data['built_target'] in targets
    assert set(data['actually_verified_platforms']) <= {data['built_target']}
    assert set(data['platform_validation']) == targets
    assert {t for t, status in data['platform_validation'].items() if status == 'VERIFIED_NATIVE'} == set(data['actually_verified_platforms'])
    assert data['signing']['publisher_signed'] is False
    assert data['signing']['apple_notarized'] is False
    assert data['publication_ready'] is False and data['owner_publication_authorized'] is False


def target_artifacts(data, version):
    """Schema 3 target indexes bind public target-owned artifacts, not candidates."""
    candidate_manifest(data, version)
    assert data['manifest_role'] == 'release-index'
    name = f'b2ige-{version}-{data["built_target"]}.tar.gz'
    assert set(data['artifact_sha256']) == {name}, 'Expected only the target-owned native archive binding'
    assert re.fullmatch('[a-f0-9]{64}', data['artifact_sha256'][name])
    return name


def checksums(folder):
    listed = installer.checksum_document((folder / 'SHA256SUMS').read_bytes())
    paths = list(folder.iterdir())
    assert all(p.is_file() and not p.is_symlink() for p in paths), 'Unexpected directory/link in release set'
    assert set(listed) == {p.name for p in paths} - {'SHA256SUMS'}, 'Checksum inventory mismatch'
    for name, expected in listed.items():
        assert digest(folder / name) == expected, name
    return listed


def native_sbom_names(version):
    locked = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
    names = {f'b2ige-{version}-native-{p["name"]}.cdx.json' for p in locked if 'source' not in p}
    assert len(names) == 6
    return names


def sbom_hashes(data, folder, version):
    assert data['sbom']['status'] == 'GENERATED'
    hashes = data['sbom']['files']
    assert set(hashes) == native_sbom_names(version), 'Missing/unexpected native SBOM inventory'
    assert {p.name for p in folder.glob('*-native-*.cdx.json')} == set(hashes)
    for name, expected in hashes.items():
        assert digest(folder / name) == expected, 'Native SBOM hash mismatch: ' + name


def validate_sboms(folder):
    subprocess.run([os.environ.get('B2IGE_SBOM_PYTHON', sys.executable),
                    str(ROOT / 'scripts/validate-sbom.py'), str(folder)], check=True)


def validate(folder, *, public=False, expected_commit=None):
    version = product_metadata(ROOT)
    listed = checksums(folder)
    external = {}
    for m in folder.glob('*.manifest.json'):
        data = json.loads(m.read_text(), object_pairs_hook=installer.unique_object)
        name = target_artifacts(data, version)
        assert m.name == name[:-7] + '.manifest.json', 'Target manifest filename mismatch'
        assert listed.get(name) == data['artifact_sha256'][name], 'Target archive hash mismatch'
        external[data['built_target']] = data
    assert external, 'Missing external target manifest'
    expected = {f'b2ige-{version}-source.tar.gz'}
    for target in external:
        expected.update({f'b2ige-{version}-{target}.tar.gz', f'b2ige-{version}-{target}.manifest.json'})
    if public:
        assert expected_commit and re.fullmatch('[a-f0-9]{40}', expected_commit), 'An exact qualified commit is required'
        assert set(external) == set(TARGETS), 'Expected all three public targets'
        for data in external.values():
            assert data['git_commit'] == expected_commit and data['working_tree_dirty'] is False, 'Public commit/clean identity mismatch'
            assert data['ci_provenance']['GITHUB_SHA'] == expected_commit, 'Public CI commit mismatch'
            assert data['actually_verified_platforms'] == [data['built_target']], 'Missing native runtime gate'
    else:
        assert len(external) == 1, 'Expected one target per candidate bundle'
        expected.update({f'b2ige-verify-{version}.tgz', f'b2ige-{version}-npm.cdx.json', *native_sbom_names(version)})
        data = next(iter(external.values()))
        sbom_hashes(data, folder, version)
        npm_sbom = data['sbom']['npm']
        assert npm_sbom['status'] == 'GENERATED' and npm_sbom['file'] == f'b2ige-{version}-npm.cdx.json'
        assert listed.get(npm_sbom['file']) == npm_sbom['sha256'], 'npm SBOM hash mismatch'
        validate_sboms(folder)
    assert set(listed) == expected, 'Unexpected/missing public or candidate assets'
    embedded = {}
    source_data = None
    archives = sorted([*folder.glob('*.tar.gz'), *folder.glob('*.tgz')])
    assert archives and all(p.name in listed for p in archives), 'Unchecksummed archive'
    for archive in archives:
        with tempfile.TemporaryDirectory(prefix='b2ige-archive-check-') as tmp:
            root = pathlib.Path(tmp)
            if archive.name.endswith('.tar.gz') and not archive.name.endswith('-source.tar.gz'):
                installer.validated_container(archive.read_bytes())
            with tarfile.open(archive) as t:
                members_safe(t.getmembers())
                if archive.name.endswith('.tar.gz') and not archive.name.endswith('-source.tar.gz'):
                    target = archive.name.removeprefix(f'b2ige-{version}-').removesuffix('.tar.gz')
                    installer.validated_members(t, archive.name[:-7], version, target)
                t.extractall(root, filter='data')
            scan([p for p in root.rglob('*') if p.is_file()])
            expected_names = {f'b2ige-{version}-source.tar.gz', f'b2ige-verify-{version}.tgz',
                              *[f'b2ige-{version}-{t}.tar.gz' for t in ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']]}
            assert archive.name in expected_names, 'stale/unexpected archive name'
            if archive.name.endswith('-source.tar.gz'):
                assert (root / f'b2ige-{version}-source/release/release-manifest.json').is_file()
            elif archive.name.endswith('.tar.gz'):
                assert (root / archive.name[:-7] / 'release-manifest.json').is_file()
            for manifest in root.glob('*/release-manifest.json'):
                data = json.loads(manifest.read_text(), object_pairs_hook=installer.unique_object); package = manifest.parent
                candidate_manifest(data, version)
                assert data['manifest_role'] == 'embedded' and data['artifact_sha256'] == {}
                embedded[data['built_target']] = data
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
                sbom_hashes(data, package, version)
                validate_sboms(package)
            for m in root.glob('*/release/release-manifest.json'):
                source = m.parent.parent
                assert source.name == f'b2ige-{version}-source'
                assert product_metadata(source) == version
                source_data = json.loads(m.read_text(), object_pairs_hook=installer.unique_object)
                candidate_manifest(source_data, version)
                for required in [*V110_DOCS, *SOURCE_TOOLS]:
                    assert (source / required).is_file(), required
                paths = sorted(p for p in source.rglob('*') if p.is_file() and p != m)
                actual = hashlib.sha256(b''.join(str(p.relative_to(source)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest() for p in paths)).hexdigest()
                assert actual == source_data['source_inventory_sha256'], 'source inventory mismatch'
            if archive.suffix == '.tgz':
                package = root / 'package'; meta = json.loads((package / 'package.json').read_text())
                assert meta['version'] == version
                assert meta['private'] is True and meta['license'] == 'Apache-2.0'
                assert 'prepublishOnly' in meta['scripts']
                for required in ['LICENSE', 'TRADEMARKS.md', 'native-manifest.json', 'bin/launch.cjs']: assert (package / required).is_file()
                assert not (package / 'native').exists(), 'Unreviewed npm native payload'
    assert set(embedded) == set(external), 'Missing native archive or external manifest'
    # Only post-packaging runtime evidence and the index's role/binding may change.
    mutable = {'manifest_role', 'artifact_sha256', 'actually_verified_platforms',
               'platform_validation', 'platform_validation_scope',
               'local_release_candidate_ready', 'known_limitations'}
    for target, data in external.items():
        assert {k: v for k, v in data.items() if k not in mutable} == {
            k: v for k, v in embedded[target].items() if k not in mutable}, 'External/embedded manifest mismatch'
        assert source_data and data['git_commit'] == source_data['git_commit'], 'Source commit mismatch'
        assert data['source_inventory_sha256'] == source_data['source_inventory_sha256'], 'Cross-target normal source mismatch'
    assert source_data == embedded[source_data['built_target']], 'Source/native provenance mismatch'
    print(('Public 8-file set' if public else 'Candidate bundle') +
          ' extraction, safety, checksums, manifests, binaries, embedded SBOMs and source: PASS')


if __name__ == '__main__':
    validate(pathlib.Path(sys.argv[1]))
