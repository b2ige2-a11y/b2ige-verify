#!/usr/bin/env python3
"""Collect reviewed locked dependency notices, without claiming to be an SBOM."""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]


def generate():
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=ROOT))
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    locked = {(p['name'], p['version']): p for p in lock['package'] if 'source' in p}
    packages = [p for p in metadata['packages'] if p['source']]
    assert set(locked) == {(p['name'], p['version']) for p in packages}
    sources = json.loads((ROOT / 'release/license-supplements/sources.json').read_text())
    notices = ['B2IGE Verify 0.2.0 — third-party notices\n\n'
               'Covers the complete Cargo.lock graph, including build/dev/foreign-target dependencies.\n'
               'Inclusion does not assert that every component is linked into every binary.\n'
               'Selected alternatives are recorded in release/dependency-licenses.json.\n']
    inventory = []
    for p in sorted(packages, key=lambda p: (p['name'], p['version'])):
        root = pathlib.Path(p['manifest_path']).parent
        expression = p['license'].replace('/', ' OR ')
        if expression == 'MIT-0': selected = 'MIT-0'
        elif re.search(r'\bMIT\b', expression):
            selected = 'MIT AND Unicode-3.0' if 'AND Unicode-3.0' in expression else 'MIT'
        elif expression == 'Apache-2.0 OR BSL-1.0': selected = 'Apache-2.0'
        elif expression in {'Unicode-3.0', 'Zlib'}: selected = expression
        else: raise SystemExit(f'Unreviewed expression: {p["name"]}: {expression}')
        files = {str(f.relative_to(root)): f.read_bytes() for f in sorted(root.rglob('*'))
                 if f.is_file() and any(w in f.name.upper() for w in ['LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT'])}
        if p['name'] == 'r-efi': files['AUTHORS'] = (root / 'AUTHORS').read_bytes()
        supplement = ROOT / 'release/license-supplements' / (p['name'] + '.txt')
        if supplement.exists(): files['upstream-LICENSE'] = supplement.read_bytes()
        if p['name'] == 'libsqlite3-sys':
            sqlite = (root / 'sqlite3/sqlite3.c').read_text()
            start = sqlite.index('/*\n** 2001 September 15')
            end = sqlite.index('*/', start) + 2
            files['sqlite3-public-domain-header'] = sqlite[start:end].encode()
        if not files: raise SystemExit(f'Missing notice text: {p["name"]}')
        record = {'name': p['name'], 'version': p['version'], 'declared_license': p['license'],
                  'selected_license': selected, 'registry_checksum': locked[p['name'], p['version']]['checksum'],
                  'notice_sha256': {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}}
        if p['name'] in sources: record['supplement_source'] = sources[p['name']]
        inventory.append(record)
        notices.append(f'\n{"=" * 72}\n{p["name"]} {p["version"]}\nDeclared: {p["license"]}\nSelected: {selected}\n')
        for name, data in files.items(): notices.append(f'\n--- {name} ---\n' + data.decode() + '\n')
    return ''.join(notices), json.dumps({'schema_version': '1', 'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
                                      'scope': 'complete locked graph; not an SBOM or vulnerability scan', 'packages': inventory}, indent=2) + '\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--check', action='store_true'); args = parser.parse_args()
    notices, inventory = generate()
    for name, content in [('THIRD-PARTY-NOTICES.txt', notices), ('release/dependency-licenses.json', inventory)]:
        file = ROOT / name
        if args.check:
            assert file.read_bytes() == content.encode(), f'Stale license output: {name}'
        else: file.write_text(content)
    print('Locked dependency license texts and metadata: PASS (137 registry records)')
