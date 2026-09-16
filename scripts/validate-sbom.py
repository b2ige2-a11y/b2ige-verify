#!/usr/bin/env python3
"""Offline standard schema, dependency references, inventory and registry hash checks."""
import hashlib
import json
import pathlib
import sys
import tomllib
from jsonschema import Draft7Validator, FormatChecker
from referencing import Registry, Resource
from hygiene import ROOT, scan

folder = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'release/artifacts'
schemas = ROOT / 'release/sbom-schema'
registry = Registry()
for record in json.loads((schemas / 'sources.json').read_text()):
    data = (schemas / record['file']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == record['sha256'], 'Schema hash mismatch'
    schema = json.loads(data)
    registry = registry.with_resource(schema['$id'], Resource.from_contents(schema))
schema = json.loads((schemas / 'bom-1.5.schema.json').read_text())
Draft7Validator.check_schema(schema)
validator = Draft7Validator(schema, registry=registry, format_checker=FormatChecker())
locked = {(p['name'], p['version']):p for p in tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']}
inventory, roots = set(), set()
paths = sorted(folder.glob('b2ige-*-native-*.cdx.json'))
assert len(paths) == 6, 'Expected one SBOM per workspace crate'
for p in paths:
    bom = json.loads(p.read_text()); validator.validate(bom)
    assert bom['bomFormat'] == 'CycloneDX' and bom['specVersion'] == '1.5'
    components = bom['components'] + [bom['metadata']['component']]
    roots.add(bom['metadata']['component']['name'])
    refs = set()
    def collect(component):
        assert component['bom-ref'] not in refs, 'Duplicate component reference'
        refs.add(component['bom-ref'])
        for child in component.get('components', []): collect(child)
    for component in components:
        collect(component)
        pair = (component['name'], component['version'])
        assert pair in locked, 'Unexpected package'
        inventory.add(pair)
        if 'checksum' in locked[pair]:
            assert {'alg':'SHA-256', 'content':locked[pair]['checksum']} in component.get('hashes', []), 'Missing/wrong registry checksum'
    for edge in bom['dependencies']:
        assert edge['ref'] in refs and set(edge.get('dependsOn', [])) <= refs, 'Dangling dependency reference'
assert inventory == set(locked), 'Missing locked package(s)'
assert roots == {p['name'] for p in locked.values() if 'source' not in p}
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
for p in folder.glob('b2ige-*-npm.cdx.json'):
    bom = json.loads(p.read_text()); validator.validate(bom)
    assert bom['metadata']['component']['name'] == '@b2ige/verify'
    assert bom['metadata']['component']['version'] == version
    assert not bom.get('components'), 'Unexpected npm dependencies'
scan(paths)
print(f'Native CycloneDX 1.5 schema, references and inventory: PASS; {len(paths)} SBOMs, {len(inventory)} locked packages')
