#!/usr/bin/env python3
"""Bounded native CLI tooling smoke; no product verification or platform promotion."""
import argparse
import json
import pathlib
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True)
a = parser.parse_args()
binary = pathlib.Path(a.binary).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='b2ige-cli-smoke-') as temporary:
    root = pathlib.Path(temporary).resolve()
    def run(*args):
        return subprocess.check_output([str(binary), *args], cwd=root, text=True, timeout=60)
    assert '0.2.0' in run('--version')
    assert 'ci init' in run('--help')
    assert 'verification_performed: false' in run('inspect', str(root))
    run('init')
    registry = root / '.b2ige/project.json'
    before = registry.read_bytes()
    run('init')
    run('setup')
    assert registry.read_bytes() == before
    fixture = root / 'registered.json'
    fixture.write_text(json.dumps({'schema_version': '1', 'entries': {'smoke': {
        'product': 'sideeffect', 'config': 'reviewed.json', 'store': 'runs', 'authorization': None}}}))
    output = run('ci', 'init', '--identity', 'smoke', '--provider', 'github-actions', '--registry', str(fixture),
                 '--verifier-repo', 'b2ige2-a11y/b2ige-verify', '--verifier-ref', 'a' * 40)
    assert 'verification_performed: false' in output
    assert not (root / '.github').exists()
print('Bounded CLI version/help/inspect/idempotent init/setup/CI preview smoke: OK; no product verdict or archive support claim')
