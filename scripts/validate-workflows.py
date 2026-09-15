#!/usr/bin/env python3
"""Parse YAML and check candidate workflow invariants; not a remote Actions run."""
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
for p in sorted((root / '.github/workflows').glob('*.yml')):
    value = subprocess.check_output(['ruby', '-rjson', '-ryaml', '-rdate', '-e',
        'puts JSON.generate(YAML.safe_load(File.read(ARGV[0]), permitted_classes: [Date, Time], aliases: false))', str(p)], text=True)
    doc = json.loads(value)
    assert doc.get('on', doc.get('true')), p.name
    assert doc['permissions'] == {'contents': 'read'}, p.name
    assert doc['jobs'], p.name
    for job in doc['jobs'].values():
        if 'uses' in job: assert job['uses'].startswith('./.github/workflows/')
        for step in job.get('steps', []):
            if 'uses' in step: assert re.fullmatch(r'actions/[a-z-]+@[a-f0-9]{40}', step['uses'])
    assert not re.search(r'(?m)^\s*(?:run:.*)?(?:git push|npm publish|cargo publish|gh release create)\b', p.read_text())
print('Workflow YAML parsing and read-only/pinned-action structure: PASS; remote execution NOT implied')
