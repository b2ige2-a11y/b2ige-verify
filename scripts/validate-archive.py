#!/usr/bin/env python3
"""Validate every distributable, extracted inventory, and external manifest hashes."""
import hashlib
import json
import pathlib
import re
import sys
import tarfile
import tempfile
from hygiene import scan


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


def validate(folder):
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
            for manifest in root.glob('*/release-manifest.json'):
                data = json.loads(manifest.read_text()); package = manifest.parent
                for binary, expected in data['binary_sha256'].items():
                    assert pathlib.Path(binary).name == binary
                    assert digest(package / 'bin' / binary) == expected, binary
                assert digest(package / 'THIRD-PARTY-NOTICES.txt') == data['license_notices_sha256']
                assert digest(package / 'RUST-RUNTIME-NOTICES.html') == data['rust_runtime_notices_sha256']
                for required in ['LICENSE', 'TRADEMARKS.md', 'SECURITY.md']: assert (package / required).is_file(), required
            for m in root.glob('*/release/release-manifest.json'):
                source = m.parent.parent
                paths = sorted(p for p in source.rglob('*') if p.is_file() and p != m)
                actual = hashlib.sha256(b''.join(str(p.relative_to(source)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest() for p in paths)).hexdigest()
                assert actual == json.loads(m.read_text())['source_inventory_sha256'], 'source inventory mismatch'
            if archive.suffix == '.tgz':
                package = root / 'package'; meta = json.loads((package / 'package.json').read_text())
                assert meta['private'] is True and meta['license'] == 'Apache-2.0'
                assert 'prepublishOnly' in meta['scripts']
                for required in ['LICENSE', 'TRADEMARKS.md', 'native-manifest.json', 'bin/launch.cjs']: assert (package / required).is_file()
                assert not (package / 'native').exists(), 'Unreviewed npm native payload'
    for m in folder.glob('*.manifest.json'):
        assert m.name in listed, 'Unchecksummed manifest'
        data = json.loads(m.read_text()); assert data['manifest_role'] == 'release-index'
        assert data['artifact_sha256'], 'Missing external artifact hashes'
        for name, expected in data['artifact_sha256'].items(): assert listed.get(name) == expected, name
        assert not data['publication_ready'] and not data['owner_publication_authorized']
    print('Native/source/npm extraction, membership, notices, checksums, binary and source hashes: PASS')


if __name__ == '__main__':
    validate(pathlib.Path(sys.argv[1]))
