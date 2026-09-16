#!/usr/bin/env python3
"""Parse YAML and check candidate workflow invariants; not a remote Actions run."""
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
workflow_dir = root / '.github/workflows'
workflows = sorted(workflow_dir.glob('*.yml')) + sorted(workflow_dir.glob('*.yml.example'))
for p in workflows:
    value = subprocess.check_output(['ruby', '-rjson', '-ryaml', '-rdate', '-e',
        'puts JSON.generate(YAML.safe_load(File.read(ARGV[0]), permitted_classes: [Date, Time], aliases: false))', str(p)], text=True)
    doc = json.loads(value)
    assert doc.get('on', doc.get('true')), p.name
    assert doc['permissions'] == {'contents': 'read'}, p.name
    assert doc['jobs'], p.name
    if p.name == 'b2ige-verify.yml.example':
        events = doc.get('on', doc.get('true'))
        assert set(events) == {'pull_request_target'}
        job = doc['jobs']['verify']
        assert job['env']['B2IGE_TRUSTED_REVISION'] == '${{ github.event.pull_request.base.sha }}'
        assert job['env']['B2IGE_CANDIDATE_REVISION'] == '${{ github.event.pull_request.head.sha }}'
        checkout = [step for step in job['steps'] if step.get('uses', '').startswith('actions/checkout@')]
        assert len(checkout) == 1
        assert checkout[0]['with']['ref'] == '${{ github.event.pull_request.base.sha }}'
        assert checkout[0]['with']['persist-credentials'] is False
        commands = '\n'.join(step.get('run', '') for step in job['steps'])
        assert '--trusted-revision "$B2IGE_TRUSTED_REVISION"' in commands
        assert '--required-registry .b2ige/project.json' in commands
        assert '--candidate-approval "$RUNNER_TEMP/b2ige-candidate-approval.json"' in commands
        assert 'test -f "$RUNNER_TEMP/b2ige-candidate-approval.json"' in commands
        assert '--head "$B2IGE_CANDIDATE_REVISION"' in commands
        assert 'git fetch --no-tags origin "$B2IGE_CANDIDATE_REVISION"' in commands
        assert not re.search(r'git\s+(checkout|switch|reset|restore|merge|rebase)\b', commands)
    for job in doc['jobs'].values():
        if 'uses' in job: assert job['uses'].startswith('./.github/workflows/')
        for step in job.get('steps', []):
            if 'uses' in step: assert re.fullmatch(r'actions/[a-z-]+@[a-f0-9]{40}', step['uses'])
    assert not re.search(r'(?m)^\s*(?:run:.*)?(?:git push|npm publish|cargo publish|gh release create)\b', p.read_text())
print('Workflow YAML parsing and read-only/pinned-action structure: PASS; remote execution NOT implied')
