#!/usr/bin/env python3
"""Tooling-only external pilot. No product authority; no synthetic evidence fallback."""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shlex
import subprocess
import sys
import tempfile
import time
import uuid

sys.dont_write_bytecode = True
import adoption_bench as bench

ROOT = Path(__file__).resolve().parents[1]
PROTOCOL_PATH = ROOT / 'external-pilot/v1/protocol.json'
PROTOCOL = bench.read(PROTOCOL_PATH)
PRODUCTS = [j['product'] for j in PROTOCOL['journeys']]
STAGES = PROTOCOL['stages']
OUTCOMES = bench.OUTCOMES
HEX = r'[a-f0-9]{40}'
HASH = r'sha256:[a-f0-9]{64}'
RANDOM = r'[a-f0-9]{32}'
ATTEST = ['participant_is_b2ige_developer', 'authored_implementation_or_fixtures',
          'saw_private_material', 'own_environment', 'followed_public_instructions',
          'assistance_disclosed', 'publication_consent']
BINARY = ROOT / 'target/release/b2ige'
HELPER = ROOT / 'target/release/examples/adoption_fixture'
MCP = ROOT / 'target/release/b2ige-mcp'


def need(value, message):
    if not value:
        raise ValueError(message)


def shape(value, fields):
    need(type(value) is dict and set(value) == set(fields.split()), 'unknown/missing fields')


def enum(value, values):
    need(type(value) is str and value in values, 'invalid category')


def pattern(value, regex):
    need(type(value) is str and re.fullmatch(regex, value) is not None, 'invalid bounded identifier')


def integer(value, maximum=1000000):
    need(type(value) is int and 0 <= value <= maximum, 'invalid count')


def boolean(value):
    need(type(value) is bool, 'invalid boolean')


def duration(value):
    need(type(value) in (int, float) and math.isfinite(value) and 0 <= value <= 31536000,
         'invalid observational duration')


def items(value, maximum=100):
    need(type(value) is list and len(value) <= maximum, 'invalid bounded list')


def categories(value, allowed):
    items(value)
    need(len(set(value)) == len(value), 'duplicate category')
    for entry in value:
        enum(entry, allowed)


def check_pair(verdict, code):
    enum(verdict, OUTCOMES)
    integer(code, 3)
    need(OUTCOMES[verdict] == code, 'verdict/exit disagreement')


def bindings():
    bench.load_corpus()  # Existing reviewed inventory + P8 constructor/content pins.
    return dict(protocol_identity=bench.digest(PROTOCOL_PATH.read_bytes()),
                corpus_identity=bench.MANIFEST_PIN)


def commit(clean=True):
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT, stderr=subprocess.DEVNULL).decode().strip()
    sha = git('rev-parse', '--verify', 'HEAD^{commit}')
    pattern(sha, HEX)
    if clean:
        need(not git('status', '--porcelain', '--untracked-files=all'), 'clean pinned checkout required')
    return sha


def output_path(path):
    p = Path(path).absolute()
    need(not any(v.is_symlink() for v in [p, *p.parents]), 'symlink output refused')
    p = p.resolve()
    need(p != ROOT and ROOT not in p.parents, 'capture outside the source checkout')
    return p


def save(path, value):
    p = output_path(path)
    data = json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n'
    with p.open('x', encoding='utf-8') as stream:
        os.chmod(p, 0o600)
        stream.write(data)


def load(path):
    p = Path(path)
    need(p.is_file() and p.stat().st_size <= 2 * 1024 * 1024, 'bounded result file required')
    return bench.read(p)


def blank_journey(product):
    return dict(product=product, status='not_started', stages=[], command_count=0,
                manual_trust_checkpoints=0, undocumented_edits=0, assistance=[],
                recovery_attempts=0, first_blocking_stage=None, friction=[], terminology=[],
                first_result_seconds=None, final_verdict=None, final_exit_code=None,
                agent=None, verified_reload=False, controller_separated=False,
                confidence='unanswered', doctor_is_verification='unanswered', note='omitted')


def blank_result(sha, participant, environment):
    return dict(kind='external-adoption-pilot', schema_version='1', authoritative=False,
                pilot_version='external-pilot-v1', pilot_commit=sha, verifier_commit=sha,
                **bindings(), run_id=uuid.uuid4().hex, participant_id=participant,
                provenance='external_operator', test_data=False, attestation=None,
                environment=environment, installation=dict(status='not_started', command_count=0, assistance=[]), journeys=[blank_journey(p) for p in PRODUCTS],
                questions=[], mcp=None, ci=[], denominators=dict(operators=1, journeys=3))


def attestation(value):
    shape(value, ' '.join(ATTEST))
    for v in value.values():
        boolean(v)
    need(not any(value[k] for k in ATTEST[:3]), 'non-external participant cannot qualify')
    need(all(value[k] for k in ATTEST[3:]), 'externality/consent attestation required')


def agent_fields(value):
    shape(value, 'product operation protocol_version verdict exit_code source_backed validated')
    enum(value['product'], PRODUCTS)
    need(value['operation'] == 'verify' and value['protocol_version'] == '1', 'verify protocol required')
    check_pair(value['verdict'], value['exit_code'])
    boolean(value['source_backed'])
    need(value['validated'] is True, 'Agent transport must validate')
    if value['verdict'] != 'ERROR':
        need(value['source_backed'], 'missing evidence')


def validate_journey(j):
    shape(j, ' '.join(blank_journey('behavior')))
    enum(j['product'], PRODUCTS)
    enum(j['status'], ['not_started', 'incomplete', 'completed'])
    for k in ['command_count', 'undocumented_edits', 'recovery_attempts']:
        integer(j[k])
    integer(j['manual_trust_checkpoints'], 1)
    for k in ['verified_reload', 'controller_separated']:
        boolean(j[k])
    enum(j['note'], ['omitted', 'instructions_clear', 'instructions_unclear', 'needed_help', 'withheld_for_privacy'])
    enum(j['confidence'], ['unanswered', 'low', 'medium', 'high'])
    enum(j['doctor_is_verification'], ['unanswered', 'yes', 'no', 'unsure'])
    items(j['stages'], len(STAGES))
    names = [s['stage'] for s in j['stages']]
    need(names == STAGES[:len(names)], 'stages must preserve planned order')
    for index, s in enumerate(j['stages']):
        shape(s, 'stage started_unix elapsed_seconds exit_code completed')
        integer(s['started_unix'], 10000000000)
        duration(s['elapsed_seconds'])
        boolean(s['completed'])
        if s['exit_code'] is not None:
            integer(s['exit_code'], 255)
        if s['completed']:
            need(s['exit_code'] in (range(3) if s['stage'] == 'human' else range(4) if s['stage'] in ['agent', 'report'] else [0]),
                 'unsuccessful stage counted as complete')
        if index < len(names) - 1:
            need(s['completed'], 'cannot skip a blocking stage')
    if j['first_blocking_stage'] is not None:
        enum(j['first_blocking_stage'], STAGES)
        need(names and names[-1] == j['first_blocking_stage'] and not j['stages'][-1]['completed'],
             'blocking stage inconsistent')
    categories(j['friction'], PROTOCOL['friction'])
    categories(j['terminology'], PROTOCOL['friction'])
    items(j['assistance'])
    for event in j['assistance']:
        shape(event, 'stage kind')
        enum(event['stage'], STAGES)
        enum(event['kind'], PROTOCOL['assistance'])
        need(event['stage'] in names, 'assistance stage not attempted')
    if j['final_verdict'] is None:
        need(j['final_exit_code'] is None, 'orphan exit code')
    else:
        check_pair(j['final_verdict'], j['final_exit_code'])
        need(j['agent'] is not None, 'no Agent result for verifier verdict')
    if j['agent'] is not None:
        agent_fields(j['agent'])
        need('agent' in names and j['agent']['product'] == j['product'], 'Agent product/stage mismatch')
        need(j['final_verdict'] == j['agent']['verdict'] and j['final_exit_code'] == j['agent']['exit_code'],
             'Agent/result mismatch')
        need(j['stages'][names.index('agent')]['exit_code'] == j['final_exit_code'], 'Agent stage exit mismatch')
    if j['agent'] is None:
        need(not j['verified_reload'] and j['first_result_seconds'] is None, 'missing Agent result')
    if j['manual_trust_checkpoints']:
        need('approve' in names and j['stages'][names.index('approve')]['completed'], 'manual checkpoint missing')
    if j['verified_reload']:
        need('report' in names and j['stages'][-1]['completed'] and j['agent'] is not None,
             'verified loader not completed')
        need(j['stages'][-1]['exit_code'] == j['final_exit_code'], 'reload exit mismatch')
    if j['first_result_seconds'] is not None:
        duration(j['first_result_seconds'])
        need(j['agent'] is not None and j['agent']['source_backed'] and j['verified_reload']
             and j['final_verdict'] != 'ERROR', 'doctor/error is not evidence-backed first result')
    if j['status'] == 'not_started':
        need(j == blank_journey(j['product']), 'unstarted journey has fabricated measurements')
    else:
        need(names, 'attempted stages missing')
    if j['status'] == 'completed':
        need(names == STAGES and all(s['completed'] for s in j['stages'])
             and j['first_blocking_stage'] is None and j['manual_trust_checkpoints'] == 1
             and j['controller_separated'] and j['verified_reload']
             and j['first_result_seconds'] is not None, 'incomplete journey')
        need(j['stages'][STAGES.index('human')]['exit_code'] == j['final_exit_code'], 'human/Agent outcomes differ')
        need(j['confidence'] != 'unanswered' and j['doctor_is_verification'] != 'unanswered', 'operator answers missing')


def ci_fields(c, sha):
    shape(c, 'repository_url base_sha head_sha verifier_sha workflow_path workflow_sha256 pr_url run_url run_id job_id verdict exit_code check_conclusion verification_step_conclusion independently_provisioned public_consent observed_via artifacts no_private_artifacts')
    pattern(c['repository_url'], r'https://github\.com/[A-Za-z0-9_-]{1,100}/[A-Za-z0-9_.-]{1,100}')
    repo = c['repository_url']
    need(not any(v.lower() in ['example', 'test', 'fake', 'simulated', 'localhost'] for v in repo.split('/')[-2:]), 'placeholder CI repository')
    for k in ['base_sha', 'head_sha', 'verifier_sha']:
        pattern(c[k], HEX)
        need(len(set(c[k])) > 1, 'placeholder SHA')
    need(c['base_sha'] != c['head_sha'] and c['verifier_sha'] == sha, 'CI commit binding')
    pattern(c['workflow_path'], r'\.github/workflows/[A-Za-z0-9_-]{1,64}\.yml')
    pattern(c['workflow_sha256'], HASH)
    pattern(c['pr_url'], re.escape(repo) + r'/pull/[1-9][0-9]{0,11}')
    integer(c['run_id'], 10**15)
    integer(c['job_id'], 10**15)
    need(c['run_id'] > 0 and c['job_id'] > 0 and c['run_url'] == repo + '/actions/runs/' + str(c['run_id']), 'CI run binding')
    check_pair(c['verdict'], c['exit_code'])
    conclusion = 'success' if c['verdict'] == 'PASS' else 'failure'
    need(c['check_conclusion'] == c['verification_step_conclusion'] == conclusion, 'PASS-only-green boundary')
    need(c['independently_provisioned'] is True and c['public_consent'] is True
         and c['observed_via'] == 'github_api' and c['no_private_artifacts'] is True, 'external CI/control missing')
    items(c['artifacts'], 1)
    need(len(c['artifacts']) == 1, 'sanitized artifact inventory missing')
    a = c['artifacts'][0]
    shape(a, 'name file_count kind archive_sha256')
    need(a['name'] == 'b2ige-sanitized-agent-reports' and a['kind'] == 'validated-agent-protocol-v1-json', 'artifact category')
    integer(a['file_count'], 100)
    need(a['file_count'] > 0, 'empty artifact')
    pattern(a['archive_sha256'], HASH)


def validate(r, *, complete=False, expected_commit=None):
    shape(r, 'kind schema_version authoritative pilot_version pilot_commit verifier_commit protocol_identity corpus_identity run_id participant_id provenance test_data attestation environment installation journeys questions mcp ci denominators')
    need(r['kind'] == 'external-adoption-pilot' and r['schema_version'] == '1'
         and r['pilot_version'] == 'external-pilot-v1' and r['authoritative'] is False, 'tooling kind/version')
    need(r['test_data'] is False and r['provenance'] == 'external_operator', 'synthetic/internal evidence forbidden')
    pattern(r['participant_id'], RANDOM)
    pattern(r['run_id'], RANDOM)
    pattern(r['pilot_commit'], HEX)
    need(r['pilot_commit'] == r['verifier_commit'] == (expected_commit or commit()), 'stale/mismatched code commit')
    for k, v in bindings().items():
        need(r[k] == v, 'protocol/corpus identity mismatch')
    attestation(r['attestation'])
    shape(r['environment'], 'os architecture docker')
    enum(r['environment']['os'], ['Darwin', 'Linux'])
    enum(r['environment']['architecture'], ['arm64', 'aarch64', 'x86_64', 'amd64'])
    enum(r['environment']['docker'], ['available', 'unavailable'])
    shape(r['installation'], 'status command_count assistance')
    enum(r['installation']['status'], ['not_started', 'incomplete', 'completed'])
    integer(r['installation']['command_count'], 2)
    categories(r['installation']['assistance'], PROTOCOL['assistance'])
    if r['installation']['status'] == 'completed':
        need(r['installation']['command_count'] == 2, 'source build incomplete')
    items(r['journeys'], 3)
    need([j['product'] for j in r['journeys']] == PRODUCTS, 'exactly three distinct planned journeys required')
    for j in r['journeys']:
        validate_journey(j)
        if j['product'] == 'blindtest' and j['status'] == 'completed':
            need(r['environment']['docker'] == 'available', 'actual Docker required')
    items(r['questions'], 6)
    need([q['id'] for q in r['questions']] == ['q' + str(i + 1) for i in range(len(r['questions']))], 'question inventory')
    for q in r['questions']:
        shape(q, 'id answer consulted_docs explanation')
        enum(q['answer'], ['yes', 'no', 'unsure'])
        boolean(q['consulted_docs'])
        enum(q['explanation'], ['omitted', 'evidence_required', 'observer_boundary', 'nondeterminism_risk',
                                 'baseline_trust', 'same_user_limit', 'bounded_replay', 'unsure'])
    if r['mcp'] is not None:
        m = r['mcp']
        shape(m, 'registered_identity tools_listed response approval_authority assistance')
        need(m['registered_identity'] == 'bench' and m['tools_listed'] is True, 'MCP registered identity/list missing')
        agent_fields(m['response'])
        need(m['response']['product'] == 'behavior', 'MCP exercise product')
        enum(m['approval_authority'], ['yes', 'no', 'unsure'])
        enum(m['assistance'], PROTOCOL['assistance'])
    items(r['ci'], 2)
    for c in r['ci']:
        ci_fields(c, r['pilot_commit'])
    need(len({(c['repository_url'], c['run_id']) for c in r['ci']}) == len(r['ci']), 'duplicate CI run')
    shape(r['denominators'], 'operators journeys')
    need(type(r['denominators']['operators']) is int and type(r['denominators']['journeys']) is int
         and r['denominators'] == {'operators': 1, 'journeys': 3}, 'denominator tampering')
    if complete:
        need(r['installation']['status'] == 'completed', 'installation incomplete')
        need(all(j['status'] == 'completed' for j in r['journeys']), 'incomplete journeys retained; gate pending')
        need(len(r['questions']) == 6, 'six questions required')
        need(r['mcp'] is not None and r['mcp']['response']['source_backed']
             and r['mcp']['response']['verdict'] != 'ERROR', 'actual MCP evidence required')
        need(len(r['ci']) == 2 and sum(c['verdict'] == 'PASS' for c in r['ci']) == 1, 'CI PASS/non-PASS pair required')
    return r


def classification(j):
    if j['status'] != 'completed':
        return 'incomplete/drop-off'
    return ('assisted completion' if any(a['kind'] in ['ai_assistant', 'external_human', 'b2ige_developer']
                                         for a in j['assistance']) else 'unassisted/doc-only completion')


def report(results, *, expected_commit=None):
    for r in results:
        validate(r, expected_commit=expected_commit)
    need(len({r['run_id'] for r in results}) == len(results), 'copied run rejected')
    # All attempts stay in denominators; repeat operators are not new operators.
    operators = len({r['participant_id'] for r in results})
    qualified = set()
    for r in results:
        try:
            validate(r, complete=True, expected_commit=expected_commit)
            qualified.add(r['participant_id'])
        except ValueError:
            pass
    lines = ['# External adoption pilot', '',
             'EVIDENCE_PENDING' if not qualified else 'READY_FOR_INDEPENDENT_REVIEW (V110-D remains pending)', '',
             PROTOCOL['claim_boundary'], '',
             f'External operators meeting collection gate: {len(qualified)}/{operators}; runs: {len(results)}.',
             f'Journey completion: {sum(j["status"] == "completed" for r in results for j in r["journeys"])}/{3 * len(results)}.',
             'Zero denominator means N/A, never success. Timing is observational only.',
             'Independent final audit, public report review and external B2IGE PR CI are still required.', '']
    for r in sorted(results, key=lambda v: (v['participant_id'], v['run_id'])):
        lines += [f'Participant {r["participant_id"]}; run {r["run_id"]}; commit {r["pilot_commit"]}.',
                  'Platform: ' + json.dumps(r['environment'], sort_keys=True),
                  'Installation: ' + json.dumps(r['installation'], sort_keys=True)]
        for j in r['journeys']:
            metrics = {k: j[k] for k in ['product', 'status', 'command_count', 'first_result_seconds',
                       'manual_trust_checkpoints', 'undocumented_edits', 'recovery_attempts', 'first_blocking_stage',
                       'friction', 'terminology', 'assistance', 'final_verdict', 'final_exit_code', 'confidence',
                       'doctor_is_verification', 'controller_separated', 'note']}
            metrics['classification'] = classification(j)
            metrics['agent_validated'] = j['agent'] is not None
            lines.append(json.dumps(metrics, sort_keys=True))
        matches = sum(q['answer'] == PROTOCOL['questions'][i]['expected'] for i, q in enumerate(r['questions']))
        lines += [f'Conceptual answers matching documented context: {matches}/6; answered {len(r["questions"])}/6.',
                  'Answers (never rewritten): ' + json.dumps(r['questions'], sort_keys=True),
                  'MCP: ' + json.dumps(r['mcp'], sort_keys=True),
                  'CI PASS-green/non-PASS-not-green and artifact inspections: ' + json.dumps(r['ci'], sort_keys=True), '']
    return '\n'.join(lines) + '\n'


def choose(prompt, options):
    while True:
        print(prompt + ' [' + '/'.join(options) + ']')
        answer = input('> ').strip()
        if answer in options:
            return answer
        print('Choose a listed category; do not enter personal information.')


def sanitize_note(text):
    # No arbitrary prose is published or persisted. Only exact common statements
    # map to public categories; everything else is discarded, never regex-guessed.
    need(type(text) is str and len(text) <= 240, 'note must be at most 240 characters')
    return {'': 'omitted', 'instructions clear': 'instructions_clear',
            'instructions unclear': 'instructions_unclear', 'needed help': 'needed_help'}.get(
                text.strip().lower(), 'withheld_for_privacy')


def count(prompt):
    while True:
        answer = input(prompt + ' (0..1000): ').strip()
        if answer.isdecimal() and 0 <= int(answer) <= 1000:
            return int(answer)


def show(command, cwd):
    print('Working directory:', cwd)
    print('$', shlex.join([str(a) for a in command]), flush=True)


def execute(command, *, cwd, env, interactive=False, timeout=600):
    show(command, cwd)
    result = subprocess.run([str(a) for a in command], cwd=cwd, env=env,
                            capture_output=not interactive, timeout=timeout, check=False)
    # Local trusted terminal only: expose the product's actual discovery/readiness
    # guidance. Do not retain raw output in snapshots or echo Agent/private reports.
    words = [str(a) for a in command]
    if not interactive and 'agent' not in words and 'ci-check' not in words:
        if result.stdout:
            print(result.stdout.decode(errors='replace'), end='')
        if result.stderr:
            print(result.stderr.decode(errors='replace'), end='', file=sys.stderr)
    return result


def check_agent(data, code, cwd, env, product, operation='verify'):
    need(type(data) is dict and data.get('product') == product and data.get('operation') == operation,
         'wrong Agent operation/product')
    check_pair(data.get('verdict'), code)
    # ci-check accepts only verify. Compare the entire report transport after an explicit
    # operation-only adaptation; product stored-result reload has already executed.
    transport = dict(data, operation='verify')
    with tempfile.TemporaryDirectory(dir=cwd) as tmp:
        path = Path(tmp) / 'transport.json'
        bench.save(path, transport)
        checked = execute([BINARY, 'ci-check', path, str(code)], cwd=cwd, env=env)
        need(checked.returncode == code and bench.decode(checked.stdout) == transport,
             'Agent protocol validation failed (no fallback accepted)')
    backed = (data.get('source') is not None and data.get('scope') is not None
              and data['scope']['coverage'].get('verified_evidence_count', 0) > 0)
    need(code == 3 or backed, 'non-ERROR response lacks verified source evidence')
    return dict(product=product, operation='verify', protocol_version='1', verdict=data['verdict'],
                exit_code=code, source_backed=backed, validated=True)


def mcp_exercise(j):
    command = [MCP, '--registry', j.registry]
    show(command, j.controller)
    request = dict(protocol_version='1', product='behavior', operation='verify',
                   identity='bench', output='agent', execution_budget=None)
    messages = [dict(jsonrpc='2.0', id=1, method='initialize', params=dict(protocolVersion='2025-11-25', capabilities={}, clientInfo=dict(name='external-pilot', version='1'))),
                dict(jsonrpc='2.0', method='notifications/initialized'),
                dict(jsonrpc='2.0', id=2, method='tools/list'),
                dict(jsonrpc='2.0', id=3, method='tools/call', params=dict(name='b2ige_behavior_verify', arguments=request))]
    print('Exact stdio messages:', json.dumps(messages))
    need(choose('Run this registered-identity MCP exercise?', ['yes', 'no']) == 'yes', 'MCP declined')
    response = subprocess.run([str(v) for v in command], input=''.join(json.dumps(v) + '\n' for v in messages).encode(),
                              cwd=j.controller, env=j.env, capture_output=True, timeout=600)
    need(response.returncode == 0, 'MCP server failed')
    lines = [bench.decode(v) for v in response.stdout.splitlines()]
    need(len(lines) == 3 and [v['id'] for v in lines] == [1, 2, 3], 'MCP response sequence')
    need(lines[0]['result']['protocolVersion'] == '2025-11-25', 'MCP initialization')
    names = [v['name'] for v in lines[1]['result']['tools']]
    need(names == ['b2ige_doctor', 'b2ige_behavior_verify', 'b2ige_sideeffect_verify', 'b2ige_blindtest_verify', 'b2ige_report'], 'MCP tool inventory')
    payload = lines[2]['result']
    data = payload['structuredContent']
    need(bench.decode(payload['content'][0]['text']) == data and payload['isError'] == (data['verdict'] != 'PASS'), 'MCP envelope')
    fixed = check_agent(data, OUTCOMES[data['verdict']], j.controller, j.env, 'behavior')
    print('Sanitized MCP result:', json.dumps(fixed))
    return dict(registered_identity='bench', tools_listed=True, response=fixed,
                approval_authority=choose('Does receiving an MCP result give MCP approval authority?', ['yes', 'no', 'unsure']),
                assistance=choose('MCP assistance used (strongest category)', PROTOCOL['assistance']))


def collect(output, sha):
    need(commit() == sha, 'checkout does not match supplied exact commit')
    need(sys.stdin.isatty() and sys.stdout.isatty(), 'real interactive operator terminal required')
    run_root = output_path(output)
    run_root.mkdir(mode=0o700)  # Refuse reuse even for interrupted/failed sessions.
    participant = input('Generate a random participant ID with the id command; paste its 32 hex characters (never a name): ').strip()
    pattern(participant, RANDOM)
    answers = {}
    print('Externality is self-attestation, not cryptographic identity proof. No personal data is requested.')
    for key in ATTEST:
        answers[key] = choose(key.replace('_', ' ') + '?', ['yes', 'no']) == 'yes'
    attestation(answers)
    env = {k: v for k, v in os.environ.items() if k in ['PATH', 'SYSTEMROOT', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'RUSTUP_HOME', 'CARGO_HOME', 'HOME']}
    env.update(LANG='C', LC_ALL='C')
    try:
        docker = execute(['docker', 'info', '--format', '{{.OSType}}'], cwd=run_root, env=env, timeout=30)
    except (OSError, subprocess.SubprocessError):
        docker = None
    r = blank_result(sha, participant, dict(os=platform.system(), architecture=platform.machine(),
                     docker='available' if docker and docker.returncode == 0 and docker.stdout.strip() == b'linux' else 'unavailable'))
    r['attestation'] = answers
    sequence = 0

    def snapshot():
        nonlocal sequence
        need(commit() == sha, 'source changed during capture; preserve previous snapshots')
        validate(r, expected_commit=sha)
        sequence += 1
        save(run_root / f'measurement-{sequence:04d}.json', r)

    snapshot()
    # Build exact checked-out source, never trust stale binaries from another checkout.
    install_help = choose('Assistance needed during installation?', PROTOCOL['assistance'])
    r['installation']['assistance'] = [install_help]
    for command in [['cargo', 'build', '--workspace', '--release', '--locked'],
                    ['cargo', 'build', '-p', 'verify-cli', '--release', '--locked', '--example', 'adoption_fixture']]:
        r['installation']['status'] = 'incomplete'
        r['installation']['command_count'] += 1
        snapshot()
        p = execute(command, cwd=ROOT, env=env, interactive=True)
        need(p.returncode == 0, 'installation blocked; initial planned denominator retained')
    r['installation']['status'] = 'completed'
    snapshot()
    scenarios = {s['scenario_id']: s for s in bench.load_corpus()['scenarios']}
    for planned, row in zip(PROTOCOL['journeys'], r['journeys']):
        print('\nJourney:', row['product'])
        if choose('Start journey?', ['yes', 'no']) == 'no':
            continue
        j = bench.Journey(run_root, scenarios[planned['scenario']], BINARY, HELPER, env)
        j.draft = j.root / 'draft'
        began = time.monotonic()
        verified = None
        human_exit = None
        # Only the unchanged bounded fixture setup methods are reused. Never execute(),
        # approve(), fix_confirmation(), or the benchmark PTY helper.
        old_run = bench.run
        def visible_setup(command, *, cwd, env, timeout=120):
            row['command_count'] += 1
            return execute(command, cwd=cwd, env=env, timeout=timeout)
        try:
            for name in STAGES:
                help_kind = choose('Assistance needed at ' + name + '?', PROTOCOL['assistance'])
                if name == 'setup' and install_help != 'none':
                    row['assistance'].append(dict(stage=name, kind=install_help))
                if help_kind != 'none' and (name != 'setup' or help_kind != install_help):
                    row['assistance'].append(dict(stage=name, kind=help_kind))
                stage = dict(stage=name, started_unix=int(time.time()), elapsed_seconds=0,
                             exit_code=None, completed=False)
                row['status'] = 'incomplete'
                row['stages'].append(stage)
                row['first_blocking_stage'] = name
                snapshot()  # An interrupted command remains an attempted incomplete stage.
                started = time.monotonic()
                try:
                    if name in ['setup', 'materialize']:
                        bench.run = visible_setup
                        p = j.project_setup() if name == 'setup' else j.materialize()
                        bench.run = old_run
                        code = p.returncode
                        stage['exit_code'] = code
                        need(code == 0, 'fixture setup failed')
                        bench.lanes(j.project, j.controller)
                        row['controller_separated'] = True
                    else:
                        commands = {
                            'inspect': [BINARY, 'inspect', j.project],
                            'init': [BINARY, 'init', j.project],
                            'prepare': [BINARY, 'prepare', '--product', row['product'], '--config', j.controller / 'input.json', '--out', j.draft],
                            'approve': j.approval_command(),
                            'doctor': [BINARY, 'doctor', '--registry', j.registry],
                            'human': [BINARY, 'verify', 'bench', '--registry', j.registry],
                            'agent': [BINARY, 'verify', 'bench', '--registry', j.registry, '--output', 'agent', '--protocol', '1'],
                        }
                        if name == 'prepare':
                            if row['product'] == 'sideeffect': commands[name] += ['--fixture', j.project / 'fixture']
                            if row['product'] == 'blindtest': commands[name] += ['--workspace', j.project]
                        if name == 'report':
                            row['controller_separated'] = False
                            bench.lanes(j.project, j.controller)
                            if row['product'] == 'blindtest':
                                bench.private_separation(j.project, j.sealed, j.private)
                            row['controller_separated'] = True
                            need(verified is not None and verified.get('source') is not None, 'source required for reload')
                            source = verified['source']['artifact_id']
                            if row['product'] == 'blindtest':
                                paths = sorted(j.store.glob('*/result.json'))
                                need(len(paths) == 2, 'expected separate human and Agent runs')
                                # Public BlindTest source IDs are aliases. Match by actual loader projection.
                                matching = []
                                for path in paths:
                                    row['command_count'] += 1
                                    probe = execute([BINARY, 'report', path, '--store', j.store, '--output', 'agent', '--protocol', '1'], cwd=j.controller, env=j.env)
                                    if bench.decode(probe.stdout).get('source') == verified['source']:
                                        matching.append(path)
                                need(len(matching) == 1, 'ambiguous stored source')
                                source = matching[0]
                            commands[name] = [BINARY, 'report', source, '--store', j.store, '--output', 'agent', '--protocol', '1']
                            if row['product'] == 'behavior': commands[name] += ['--authorization', j.auth]
                        if name == 'approve':
                            print('Read the displayed REVIEW.md and identities. You alone decide whether to approve. No automatic answer is sent.')
                        row['command_count'] += 1
                        p = execute(commands[name], cwd=j.controller, env=j.env, interactive=name in ['approve', 'human'])
                        code = p.returncode
                        stage['exit_code'] = code
                        if name in ['agent', 'report']:
                            data = bench.decode(p.stdout)
                            row['command_count'] += 1
                            projection = check_agent(data, code, j.controller, j.env, row['product'], 'report' if name == 'report' else 'verify')
                            if name == 'agent':
                                verified = data
                                row['agent'] = projection
                                row['final_verdict'], row['final_exit_code'] = data['verdict'], code
                                print('Sanitized Agent result:', json.dumps(projection))
                                need(code == human_exit, 'human/Agent outcomes differ; preserve both, do not retry')
                            else:
                                need(data == dict(verified, operation='report'), 'stored reload disagrees')
                                row['verified_reload'] = True
                                if projection['source_backed'] and code != 3:
                                    row['first_result_seconds'] = round(time.monotonic() - began, 4)
                        elif name == 'human':
                            human_exit = code
                            need(code in [0, 1, 2], 'human verification could not complete')
                        else:
                            need(code == 0, 'stage command failed')
                            if name == 'doctor':
                                need(bench.decode(p.stdout)['verification_performed'] is False, 'doctor is readiness only')
                            if name == 'approve':
                                row['manual_trust_checkpoints'] = 1
                    stage['exit_code'] = code
                    stage['completed'] = True
                    row['first_blocking_stage'] = None
                except (ValueError, OSError, subprocess.SubprocessError, KeyError):
                    print('Stage blocked; no raw output is exported. Preserve local controller files for private review.')
                    row['first_blocking_stage'] = name
                    break
                finally:
                    bench.run = old_run
                    stage['elapsed_seconds'] = round(time.monotonic() - started, 4)
                    snapshot()
        finally:
            bench.run = old_run
            row['confidence'] = choose('Confidence in what the result means', ['low', 'medium', 'high'])
            row['doctor_is_verification'] = choose('Is doctor readiness product verification?', ['yes', 'no', 'unsure'])
            row['undocumented_edits'] = count('Undocumented manual edits')
            row['recovery_attempts'] = count('Manual recovery attempts (no silent retries)')
            print('Optional note (max 240 chars, no personal information). Only instructions clear / instructions unclear / needed help become public categories; other text is discarded.')
            row['note'] = sanitize_note(input('Note or Enter to omit: ')[:240])
            for field in ['friction', 'terminology']:
                for category in PROTOCOL['friction']:
                    if choose(field + ': ' + category + '?', ['no', 'yes']) == 'yes':
                        row[field].append(category)
            if len(row['stages']) == len(STAGES) and all(s['completed'] for s in row['stages']) and row['first_result_seconds'] is not None:
                row['status'] = 'completed'
            snapshot()
        if row['product'] == 'behavior' and row['status'] == 'completed':
            try:
                r['mcp'] = mcp_exercise(j)
            except (ValueError, OSError, KeyError, subprocess.SubprocessError):
                print('MCP incomplete; collection gate remains pending.')
            snapshot()
    for question in PROTOCOL['questions']:
        print(question['question'])
        r['questions'].append(dict(id=question['id'], answer=choose('Your answer', ['yes', 'no', 'unsure']),
                                   consulted_docs=choose('Consulted documentation?', ['yes', 'no']) == 'yes',
                                   explanation=choose('Optional bounded explanation', ['omitted', 'evidence_required', 'observer_boundary', 'nondeterminism_risk', 'baseline_trust', 'same_user_limit', 'bounded_replay', 'unsure'])))
        snapshot()
    print('Local capture saved. CI evidence and independent review remain required. Retain the latest snapshot and all earlier attempts privately.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    sub.add_parser('id', help='generate a random anonymous participant ID')
    run = sub.add_parser('run', help='interactive real operator only; never a self-test')
    run.add_argument('--commit', required=True)
    run.add_argument('--out', required=True)
    val = sub.add_parser('validate')
    val.add_argument('result')
    val.add_argument('--complete', action='store_true', help='require collection gate, not final V110-D approval')
    rep = sub.add_parser('report')
    rep.add_argument('results', nargs='*')
    ci = sub.add_parser('ci', help='read actual GitHub runs and inspect sanitized archives')
    ci.add_argument('result')
    ci.add_argument('--out', required=True)
    approve = sub.add_parser('ci-approval', help='interactive independent CI candidate admission; never approves product inputs')
    approve.add_argument('--registry', required=True)
    approve.add_argument('--head', required=True)
    approve.add_argument('--out', required=True)
    workflow = sub.add_parser('ci-workflow', help='generate reviewed gate for an ephemeral operator-owned public pilot runner')
    workflow.add_argument('--root', required=True)
    workflow.add_argument('--registry', required=True)
    workflow.add_argument('--verifier-repo', required=True)
    workflow.add_argument('--out', required=True)
    args = parser.parse_args()
    if args.action == 'id': print(uuid.uuid4().hex)
    elif args.action == 'run': collect(args.out, args.commit)
    elif args.action == 'validate':
        record = validate(load(args.result), complete=args.complete)
        if args.complete:
            from external_pilot_ci import recheck
            recheck(record, BINARY, sys.modules[__name__])
        print('Collection gate valid; independent review still required.' if args.complete else 'Valid bounded record; may remain incomplete.')
    elif args.action == 'report': print(report([load(p) for p in args.results]), end='')
    elif args.action == 'ci-approval':
        from external_pilot_ci import approval_file
        approval_file(args.registry, args.head, args.out, sys.modules[__name__])
    elif args.action == 'ci-workflow':
        from external_pilot_ci import workflow_file
        workflow_file(args.root, args.registry, args.verifier_repo, '/opt/b2ige-pilot/controller', args.out, sys.modules[__name__])
    elif args.action == 'ci':
        from external_pilot_ci import collect_ci
        r = validate(load(args.result))
        need(not r['ci'], 'CI already captured; never rewrite it')
        r['ci'] = collect_ci(r['pilot_commit'], BINARY, sys.modules[__name__])
        validate(r)
        save(args.out, r)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError, EOFError, KeyboardInterrupt):
        print('BLOCKED: invalid/incomplete input or interrupted operation. Existing snapshots are preserved; no completion claimed.', file=sys.stderr)
        sys.exit(3)
