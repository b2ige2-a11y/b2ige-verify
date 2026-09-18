#!/usr/bin/env python3
"""Parse YAML and check candidate/bootstrap invariants; no remote run is implied."""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]


def parse(text):
    value = subprocess.check_output(['ruby', '-rjson', '-ryaml', '-rdate', '-e',
        'puts JSON.generate(YAML.safe_load(STDIN.read, permitted_classes: [Date, Time], aliases: false))'], input=text, text=True)
    return json.loads(value)


def validate_bootstrap(doc, example=False):
    # Exact reviewed structure, including every command and upload predicate. A
    # new workflow capability requires deliberate review of this single template.
    assert set(doc.get('on', doc.get('true'))) == {'pull_request_target'}
    assert doc['permissions'] == {'contents': 'read'}
    job = doc['jobs']['verify']
    assert job['env']['B2IGE_TRUSTED_REVISION'] == '${{ github.event.pull_request.base.sha }}'
    assert job['env']['B2IGE_CANDIDATE_REVISION'] == '${{ github.event.pull_request.head.sha }}'
    steps = job['steps']
    checkouts = [s for s in steps if s.get('uses', '').startswith('actions/checkout@')]
    assert len(checkouts) == 2
    assert checkouts[0]['with']['ref'] == '${{ github.event.pull_request.base.sha }}'
    assert all(s['with']['persist-credentials'] is False for s in checkouts)
    assert steps[2]['working-directory'] == 'verifier'
    commands = '\n'.join(s.get('run', '') for s in steps)
    assert '--trusted-revision "$B2IGE_TRUSTED_REVISION"' in commands
    assert '--required-registry .b2ige/project.json' in commands
    assert '--project-root "$GITHUB_WORKSPACE/project"' in commands
    assert '--candidate-approval "$RUNNER_TEMP/b2ige-candidate-approval.json"' in commands
    assert '--trusted-controller' in commands
    assert '--artifact-dir "$RUNNER_TEMP/b2ige-agent-artifacts"' in commands
    assert 'test -f "$RUNNER_TEMP/b2ige-candidate-approval.json"' in commands
    assert 'git fetch --no-tags origin "$B2IGE_CANDIDATE_REVISION"' in commands
    assert not re.search(r'git\s+(checkout|switch|reset|restore|merge|rebase)\b', commands)
    assert not any(s.get('continue-on-error') for s in steps)
    assert not any(v in commands for v in ['trust approve', '--plan-only', '|| true', 'b2ige doctor'])
    upload = steps[-1]
    assert upload['with']['path'] == '${{ runner.temp }}/b2ige-agent-artifacts/*.json'
    assert upload['with']['retention-days'] == 14
    assert upload['if'] == "${{ always() && steps.verification.outputs.artifacts_ready == 'true' }}"
    source = steps[1]['with']
    repo, pin = source['repository'], source['ref']
    assert re.fullmatch(r'[A-Za-z0-9_-]+/[A-Za-z0-9_-]+', repo)
    assert re.fullmatch(r'[a-f0-9]{40}', pin) or (example and pin == 'REPLACE_WITH_REVIEWED_FULL_V110_C_COMMIT_SHA')
    command = steps[3]['run']
    identity = re.search(r'ci init --identity ([A-Za-z0-9_-]{1,64}) --provider', command)[1]
    template = (ROOT / 'crates/verify-cli/src/ci-workflow.yml').read_text()
    expected = re.sub(r'__(IDENTITY|REPO|PIN)__', lambda m: {'IDENTITY': identity, 'REPO': repo, 'PIN': pin}[m[1]], template)
    assert doc == parse(expected), 'generated controller differs from reviewed template'


def validate(path):
    doc = parse(path.read_text())
    events = doc.get('on', doc.get('true'))
    assert events and doc['permissions'] == {'contents': 'read'} and doc['jobs']
    if 'pull_request_target' in events:
        validate_bootstrap(doc, example=path.name.endswith('.example'))
    for job in doc['jobs'].values():
        if 'uses' in job:
            assert job['uses'].startswith('./.github/workflows/')
        for step in job.get('steps', []):
            if 'uses' in step:
                assert re.fullmatch(r'actions/[a-z-]+@[a-f0-9]{40}', step['uses'])
                if step['uses'].startswith('actions/checkout@'):
                    assert step['with']['persist-credentials'] is False
    assert not re.search(r'(?m)^\s*(?:run:.*)?(?:git push|npm publish|cargo publish|gh release create)\b', path.read_text())


if __name__ == '__main__':
    folder = ROOT / '.github/workflows'
    paths = [pathlib.Path(p) for p in sys.argv[1:]] or sorted(folder.glob('*.yml')) + sorted(folder.glob('*.yml.example'))
    for path in paths:
        validate(path)
    print('Workflow YAML/read-only/pinned-source/controller structure: PASS; remote execution NOT implied')
