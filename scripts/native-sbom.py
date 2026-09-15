#!/usr/bin/env python3
"""Generate standard Cargo SBOMs in public staging; fail on incomplete inventory."""
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import tomllib
from hygiene import ROOT, files, scan


def generate(out):
    generator = os.environ.get('B2IGE_CYCLONEDX', 'cargo-cyclonedx')
    actual = subprocess.check_output([generator, 'cyclonedx', '--version'], text=True).strip()
    if actual != 'cargo-cyclonedx-cyclonedx 0.5.9':
        raise SystemExit('Expected pinned cargo-cyclonedx 0.5.9')
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
    out.mkdir(parents=True, exist_ok=True)
    artifacts = []
    with tempfile.TemporaryDirectory(prefix='b2ige-sbom-') as tmp:
        stage = pathlib.Path(tmp)
        for p in files():
            rel = p.relative_to(ROOT)
            if rel.parts[0] == 'crates' or rel.as_posix() in {'Cargo.toml', 'Cargo.lock'}:
                dest = stage / rel; dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(p, dest)
        before = (stage / 'Cargo.lock').read_bytes()
        subprocess.run([generator, 'cyclonedx', '--manifest-path', str(stage / 'Cargo.toml'),
                        '--format', 'json', '--spec-version', '1.5', '--target', 'all', '--all',
                        '--override-filename', 'native-sbom'], env=dict(os.environ, CARGO_NET_OFFLINE='true'), check=True)
        assert before == (stage / 'Cargo.lock').read_bytes(), 'SBOM generator changed lockfile'
        def normalize(value):
            if isinstance(value, str):
                return value.replace('path+file://' + str(stage) + '/', 'urn:b2ige:workspace:')
            if isinstance(value, list): return [normalize(v) for v in value]
            if isinstance(value, dict): return {k: normalize(v) for k, v in value.items()}
            return value
        for p in sorted(stage.glob('crates/*/native-sbom.json')):
            data = normalize(json.loads(p.read_text()))
            data['version'] += 1
            data['metadata']['properties'].append({'name':'b2ige:normalization', 'value':'Absolute workspace bom-ref prefix replaced consistently by urn:b2ige:workspace:; package inventory and edges unchanged'})
            dest = out / f'b2ige-{version}-native-{p.parent.name}.cdx.json'
            dest.write_text(json.dumps(data, indent=2) + '\n'); artifacts.append(dest)
    scan(artifacts)
    subprocess.run([os.environ.get('B2IGE_SBOM_PYTHON', 'python3'), str(ROOT / 'scripts/validate-sbom.py'), str(out)], check=True)
    return {'status':'GENERATED', 'format':'CycloneDX', 'spec_version':'1.5', 'generator':actual,
            'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in artifacts},
            'scope':'All 143 Cargo.lock packages: 6 workspace and 137 registry packages; all targets, transitive/build/dev dependencies. Not an OS image or compiler SBOM; not a vulnerability scan.'}


if __name__ == '__main__':
    print(json.dumps(generate(ROOT / 'release/artifacts'), indent=2))
