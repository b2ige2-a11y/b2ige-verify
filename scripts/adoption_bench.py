"""V110 adoption measurements only. Never imported by product verdict loaders."""
import copy
import hashlib
import json
import os
from pathlib import Path
import platform
import pty
import select
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / 'benchmarks/adoption-v1'
MANIFEST_PIN = 'sha256:11b7d744c9b4af5ec698b2c92b94023f052ce2f8764b225acc7b569146ae4a5d'
OUTCOMES = {'PASS': 0, 'FAIL': 1, 'INCONCLUSIVE': 2, 'ERROR': 3}
STAGES = ['project', 'inspect', 'init', 'materialize', 'prepare', 'approve',
          'doctor', 'verify', 'agent', 'report']
CONTROLS = ['missing-evidence', 'unapproved-baseline', 'stale-reference',
            'baseline-poisoning', 'failed-observer', 'missing-observer', 'attempt-log-substitution',
            'stale-executable', 'malformed-registry', 'duplicate-registry',
            'doctor-as-verification', 'private-overlap', 'private-copy', 'symlink-controller',
            'expected-label-tampering', 'authority-tampering', 'missing-authority-case',
            'source-product-mismatch', 'omitted-scenario', 'stale-output',
            'corrupt-previous-result', 'FAIL-exit-zero', 'INCONCLUSIVE-exit-zero',
            'ERROR-exit-zero', 'noninteractive-approval', 'tooling-as-evidence',
            'stale-evidence-store', 'stale-project-registry']


def require(value, message):
    if not value:
        raise ValueError(message)


def pairs(items):
    result = {}
    for key, value in items:
        require(key not in result, 'ambiguous JSON')
        result[key] = value
    return result


def decode(data):
    return json.loads(data, object_pairs_hook=pairs)


def read(path):
    return decode(Path(path).read_bytes())


def save(path, value):
    with Path(path).open('x', encoding='utf-8') as f:
        json.dump(value, f, indent=2, sort_keys=True)
        f.write('\n')


def digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def inventory(paths):
    result = {}
    for path in paths:
        for p in sorted(path.rglob('*')):
            require(not p.is_symlink(), 'symlink in pinned inventory')
            if p.is_file():
                result[str(p.relative_to(ROOT))] = digest(p.read_bytes())
    return result


def p8_inventory():
    return inventory([ROOT / 'benchmarks/corpus', ROOT / 'benchmarks/baseline-v1'])


def load_corpus():
    path = CORPUS / 'scenarios.json'
    require(digest(path.read_bytes()) == MANIFEST_PIN, 'adoption manifest changed')
    manifest = read(path)
    validate_authority(manifest, (ROOT / manifest['authority_source_path']).read_bytes())
    for name, pin in manifest['trusted_constructors'].items():
        require(digest((ROOT / name).read_bytes()) == pin, 'trusted constructor changed')
    actual = inventory([CORPUS / 'projects'])
    prefix = 'benchmarks/adoption-v1/projects/'
    require({k.removeprefix(prefix): v for k, v in actual.items()} == manifest['project_inventory'], 'project corpus changed')
    return manifest


def validate_authority(manifest, data):
    require(manifest['schema_version'] == '1' and manifest['corpus_version'] == 'adoption-v1', 'corpus version')
    require(manifest['authority_source_path'] == 'benchmarks/corpus/cases.v1.json', 'authority path')
    require(digest(data) == manifest['authority_source_identity'], 'P8 authority changed')
    cases = decode(data)
    ids = [c['benchmark_case_id'] for c in cases]
    require(len(ids) == len(set(ids)), 'duplicate P8 case')
    cases = dict(zip(ids, cases))
    rows = manifest['scenarios']
    require(len(rows) >= 9 and len({s['scenario_id'] for s in rows}) == len(rows), 'scenario inventory')
    for row in rows:
        require(row['authority_source_case_id'] in cases, 'missing authority case')
        source = cases[row['authority_source_case_id']]
        for field in ['product', 'expected_classification', 'allowed_verdicts']:
            require(row[field] == source[field], 'inherited expectation mismatch')
        for field in ['authority_source_path', 'authority_source_identity']:
            require(row[field] == manifest[field], 'source binding mismatch')
        require(row['product'] in ['behavior', 'sideeffect', 'blindtest'], 'unknown product')
        require(row['allowed_verdicts'] and all(v in OUTCOMES for v in row['allowed_verdicts']), 'unknown outcome')


def rate(n, d):
    return {'numerator': n, 'denominator': d, 'fraction': n / d if d else None}


def aggregate(rows, manifest, controls):
    ids = [r['scenario_id'] for r in rows]
    require(ids == [r['scenario_id'] for r in manifest['scenarios']], 'missing, duplicate or reordered scenario')
    total = len(manifest['scenarios'])
    return {
        'completed': rate(sum(r['completed'] for r in rows), total),
        'first_verification': rate(sum(r['first_verification_success'] for r in rows), total),
        'classification_agreement': rate(sum(r['actual_verdict'] in r['allowed_verdicts'] for r in rows), total),
        'undocumented_edits': rate(sum(r['undocumented_json_edits'] for r in rows), total),
        'unsafe_shortcuts_rejected': rate(sum(v is True for v in controls.values()), len(CONTROLS)),
        'recovery_success': rate(sum(e['recovered'] for r in rows for e in r['recovery_events']),
                                 sum(len(r['recovery_events']) for r in rows)),
    }


def blank_row(scenario):
    return dict(scenario, actual_verdict=None, exit_code=None, verification_verdict=None,
                stages=[], trust_checkpoints=0, undocumented_json_edits=False,
                recovery_events=[], first_blocking_stage=None, first_verification_success=False,
                agent_output_validated=False, controller_separated=False,
                evidence_status='not_observed', verification_coverage={}, completed=False)


def validate_result(result, manifest=None):
    manifest = manifest or load_corpus()
    require(set(result) == {'schema_version', 'kind', 'authoritative', 'corpus_version',
            'manifest_identity', 'scenarios', 'negative_controls', 'aggregate', 'p8_before',
            'p8_after', 'observations', 'ci_bootstrap'}, 'result fields')
    require(result['schema_version'] == '1' and result['kind'] == 'adoption-benchmark-tooling'
            and result['authoritative'] is False and result['corpus_version'] == 'adoption-v1', 'result version/kind')
    require(result['manifest_identity'] == MANIFEST_PIN, 'manifest binding')
    require(result['ci_bootstrap'] == 'DEFERRED_TO_V110_C', 'CI scope')
    require(result['p8_before'] == result['p8_after'] == p8_inventory(), 'P8 mutation')
    require(set(result['negative_controls']) == set(CONTROLS), 'negative control inventory')
    require(all(type(v) is bool for v in result['negative_controls'].values()), 'control type')
    obs = result['observations']
    require(set(obs) == {'environment', 'preparation', 'scenario_seconds'}, 'observation fields')
    require(set(obs['environment']) == {'system', 'release', 'machine', 'versions', 'docker_engine'}, 'environment fields')
    require(set(obs['preparation']) == {'mode', 'seconds', 'binary_identity', 'helper_identity'}, 'preparation fields')
    require(obs['preparation']['mode'] in ['source-build', 'prebuilt-source-checkout'], 'preparation mode')
    require(type(obs['preparation']['seconds']) in [int, float] and obs['preparation']['seconds'] >= 0, 'preparation timing')
    require(set(obs['scenario_seconds']) == {r['scenario_id'] for r in manifest['scenarios']}, 'timing inventory')
    for timing in obs['scenario_seconds'].values():
        require(set(timing).issubset(STAGES) and all(type(v) in [float, int] and v >= 0 for v in timing.values()), 'stage timings')
    rows = result['scenarios']
    aggregate(rows, manifest, result['negative_controls'])
    for row, expected in zip(rows, manifest['scenarios']):
        require(set(row) == set(blank_row(expected)), 'scenario fields')
        require(all(row[k] == v for k, v in expected.items()), 'changed expectation/product')
        require(row['actual_verdict'] is None or row['actual_verdict'] in OUTCOMES, 'unknown verdict')
        require(row['verification_verdict'] is None or row['verification_verdict'] in OUTCOMES, 'unknown first verdict')
        require(row['exit_code'] is None if row['actual_verdict'] is None else
                type(row['exit_code']) is int and row['exit_code'] == OUTCOMES[row['actual_verdict']], 'verdict/exit mismatch')
        require(row['evidence_status'] in ['not_observed', 'verified', 'loader_rejected'], 'evidence status')
        coverage = row['verification_coverage']
        require(type(coverage) is dict and set(coverage).issubset({'cases', 'baseline_repetitions', 'required_schedules', 'executed_schedules', 'required_hidden_cases', 'completed_hidden_cases', 'reduction_executions', 'verified_evidence_count'}) and all(type(v) is int and v >= 0 for v in coverage.values()), 'coverage fields')
        for key in ['completed', 'first_verification_success', 'agent_output_validated',
                    'controller_separated', 'undocumented_json_edits']:
            require(type(row[key]) is bool, 'metric type')
        require(type(row['trust_checkpoints']) is int and row['trust_checkpoints'] in [0, 1], 'trust checkpoint')
        names = [s['stage'] for s in row['stages']]
        require(names == STAGES[:len(names)], 'stage ordering')
        for s in row['stages']:
            require(set(s) == {'stage', 'exit_code', 'completed'} and type(s['completed']) is bool
                    and (s['exit_code'] is None or type(s['exit_code']) is int), 'stage shape')
        for stage in row['stages']:
            if stage['completed']:
                allowed = OUTCOMES.values() if stage['stage'] in ['verify', 'report'] else [0]
                require(stage['exit_code'] in allowed, 'completed stage has unsuccessful exit')
        require(row['first_blocking_stage'] is None or row['first_blocking_stage'] in STAGES, 'blocking stage')
        require(all(set(e) == {'stage', 'recovered'} and e['stage'] in STAGES
                    and type(e['recovered']) is bool for e in row['recovery_events']), 'recovery event')
        if row['verification_verdict'] is not None:
            require('verify' in names and row['stages'][names.index('verify')]['exit_code'] == OUTCOMES[row['verification_verdict']], 'first verdict/exit mismatch')
        if row['actual_verdict'] is not None:
            require('report' in names and row['stages'][names.index('report')]['exit_code'] == row['exit_code'], 'report verdict/exit mismatch')
        if row['evidence_status'] == 'loader_rejected':
            require(row['actual_verdict'] == 'ERROR', 'rejected evidence cannot establish success')
        if row['first_verification_success']:
            require(row['verification_verdict'] in ['PASS', 'FAIL', 'INCONCLUSIVE'] and
                    row['agent_output_validated'] and 'verify' in names and
                    row['stages'][names.index('verify')]['completed'], 'readiness cannot be verification')
        if row['completed']:
            require(names == STAGES and all(s['completed'] for s in row['stages']) and
                    row['controller_separated'] and row['agent_output_validated'] and
                    row['first_blocking_stage'] is None and row['actual_verdict'] is not None
                    and row['trust_checkpoints'] == 1 and row['evidence_status'] != 'not_observed'
                    and row['verification_verdict'] is not None
                    and row['verification_coverage'].get('verified_evidence_count', 0) > 0
                    and row['first_verification_success'] == (row['verification_verdict'] != 'ERROR'), 'incomplete journey')
    require(result['aggregate'] == aggregate(rows, manifest, result['negative_controls']), 'aggregate tampering')
    # Public semantic output is an allowlisted projection: no raw reports, paths or suite values.
    text = json.dumps({k: v for k, v in result.items() if k != 'observations'})
    require('BLINDTEST_PRIVATE_' not in text and str(Path.home()) not in text, 'private leakage')
    return result


def semantic(result):
    validate_result(result)
    value = copy.deepcopy(result)
    # Only stage durations and preparation duration are observational. Environment,
    # build mode and binary identities remain part of the comparison.
    value['observations'].pop('scenario_seconds')
    value['observations']['preparation'].pop('seconds')
    return value


def accepted(result):
    validate_result(result)
    return (all(r['completed'] and r['actual_verdict'] in r['allowed_verdicts']
                and not r['undocumented_json_edits'] for r in result['scenarios'])
            and all(result['negative_controls'].values()))


def run(command, *, cwd, env, timeout=120):
    return subprocess.run([str(v) for v in command], cwd=cwd, env=env,
                          capture_output=True, timeout=timeout, check=False)


def lanes(project, controller):
    for p in (project, controller):
        require(p.is_absolute(), 'absolute lane required')
        require(not any(x.is_symlink() for x in [p, *p.parents]), 'symlink lane')
    a, b = project.resolve(), controller.resolve()
    require(a != b and a not in b.parents and b not in a.parents, 'lane overlap')


def private_separation(project, controller, values):
    lanes(project, controller)
    for path in project.rglob('*'):
        require(not path.is_symlink(), 'project symlink')
        if path.is_file():
            data = path.read_bytes()
            require(all(value.encode() not in data for value in values), 'private material copied into project')


def fresh_project(source):
    require(not (source / '.b2ige').exists() and not (source / '.b2ige').is_symlink(), 'stale project registry')


def fresh_store(store):
    require(not store.exists() and not store.is_symlink(), 'stale evidence store')


def _replay_fixture_confirmation(command, *, project, controller, scenario, env):
    """Private benchmark PTY, fixed 'APPROVE bench' only; no public arbitrary command option."""
    require(scenario in load_corpus()['scenarios'], 'only pinned benchmark scenarios')
    lanes(project, controller)
    require(project.parent == controller.parent and project.parent.parent.name == 'scenarios', 'benchmark sandbox required')
    require(command[1:3] == ['trust', 'approve'] and command[command.index('--identity') + 1] == 'bench', 'fixed benchmark approval')
    master, slave = pty.openpty()
    child = subprocess.Popen([str(v) for v in command], cwd=controller, env=env,
                             stdin=slave, stdout=slave, stderr=slave)
    os.close(slave)
    data = bytearray()
    sent = False
    deadline = time.monotonic() + 30
    try:
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                data.extend(chunk)
                require(len(data) < 1024 * 1024, 'bounded terminal output')
                if not sent and b'Type APPROVE ' in data:
                    os.write(master, b'APPROVE bench\n')
                    sent = True
            if child.poll() is not None:
                break
        return subprocess.CompletedProcess(command, child.wait(timeout=2), bytes(data), b'')
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)


class Journey:
    def __init__(self, root, scenario, binary, helper, env):
        self.s = scenario
        self.row = blank_row(scenario)
        self.binary, self.helper, self.env = binary, helper, dict(env)
        self.root = root / scenario['scenario_id']
        self.root.mkdir()
        self.project, self.controller = self.root / 'project', self.root / 'controller'
        self.project.mkdir()
        self.controller.mkdir(mode=0o700)
        self.store = self.controller / 'store'
        self.draft = self.project / 'draft'
        self.registry = self.controller / 'registry.json'
        self.auth = self.controller / 'authorization.json'
        self.mode = scenario['authority_source_case_id'].split('.')[1]
        self.private = ['secret-do-not-export', 'provider-private'] if scenario['product'] == 'sideeffect' else []
        self.timings = {}
        self.responses = {}
        self.controls = {}

    def cli(self, *args):
        return run([self.binary, *args], cwd=self.controller, env=self.env)

    def stage(self, name, action, valid=lambda p: p.returncode == 0):
        started = time.monotonic()
        self.row['stages'].append({'stage': name, 'exit_code': None, 'completed': False})
        record = self.row['stages'][-1]
        try:
            response = action()
            record['exit_code'] = response.returncode
            require(valid(response), 'stage failed: ' + name)
            record['completed'] = True
            return response
        except (OSError, ValueError, KeyError, subprocess.SubprocessError):
            self.row['first_blocking_stage'] = name
            raise
        finally:
            self.timings[name] = round(time.monotonic() - started, 4)

    def project_setup(self):
        source = CORPUS / 'projects' / self.s['project']
        fresh_project(source)
        shutil.copytree(source, self.project, dirs_exist_ok=True)
        if self.s['runtime'] == 'Rust':
            self.target = self.project / 'status-cli'
            return run(['rustc', '--edition=2021', self.project / 'src/main.rs', '-o', self.target], cwd=self.project, env=self.env)
        if self.s['product'] == 'behavior':
            runtime = shutil.which('node' if self.s['runtime'] == 'Node.js' else 'python3')
            require(runtime, 'candidate runtime unavailable')
            source = self.project / ('src/cli.js' if self.s['runtime'] == 'Node.js' else 'src/cli.py')
            self.target = self.project / 'status-cli'
            self.target.write_text('#!/bin/sh\nexec ' + shlex.quote(runtime) + ' ' + shlex.quote(str(source)) + ' "$@"\n')
            self.target.chmod(0o700)
        elif self.s['product'] == 'sideeffect':
            self.target = self.project / 'fixture'
        else:
            mode = 'mutant_a' if self.mode == 'mutant_a' else 'correct'
            shutil.copyfile(self.project / (mode + '.js'), self.project / 'target.js')
            base = (self.project / 'Dockerfile').read_text().splitlines()[0].split()[1]
            check = run(['docker', 'image', 'inspect', base], cwd=self.controller, env=self.env)
            require(check.returncode == 0, 'preloaded pinned Docker base required; no pull performed')
            endpoint = run(['docker', 'context', 'inspect', '--format', '{{.Endpoints.docker.Host}}'], cwd=self.controller, env=self.env)
            host = self.env.get('DOCKER_HOST') or endpoint.stdout.decode().strip()
            require(endpoint.returncode == 0 and host.startswith('unix://'), 'local Docker required')
            docker_config = self.controller / 'docker-config'
            docker_config.mkdir()
            built = run(['docker', '--config', docker_config, '--host', host, 'build', '--pull=false', '--network=none', '--quiet', self.project],
                        cwd=self.controller, env=dict(self.env, DOCKER_BUILDKIT='0'))
            require(built.returncode == 0, 'Docker candidate build failed')
            self.target = built.stdout.decode().strip()
            require(self.target.startswith('sha256:') and len(self.target) == 71, 'immutable image required')
        return subprocess.CompletedProcess([], 0)

    def materialize(self):
        p = run([self.helper, self.s['product'], self.mode, self.project, self.controller, self.target], cwd=self.controller, env=self.env)
        if p.returncode == 0 and self.s['product'] == 'blindtest':
            self.sealed = self.controller / 'material/sealed'
            self.env['B2IGE_BLINDTEST_SEALED_ROOT'] = str(self.sealed)
            suite = read(self.sealed / 'suite.json')
            self.private = [str(self.sealed), suite['private_canary'], suite['private_metadata']]
            for c in suite['cases']:
                self.private += [c['case_id'], *c['args'], *c['environment'].values()]
            self.row['controller_separated'] = False
            private_separation(self.project, self.sealed, self.private)
            self.row['controller_separated'] = True
        return p

    def prepare(self, config=None, output=None):
        args = ['prepare', '--product', self.s['product'], '--config', config or self.controller / 'input.json', '--out', output or self.draft]
        if self.s['product'] == 'sideeffect': args += ['--fixture', self.project / 'fixture']
        if self.s['product'] == 'blindtest': args += ['--workspace', self.project]
        return self.cli(*args)

    def approval_command(self):
        args = [str(self.binary), 'trust', 'approve', self.draft, '--product', self.s['product'],
                '--identity', 'bench', '--project-root', self.project, '--controller', self.controller,
                '--registry', self.registry, '--store', self.store]
        if self.s['product'] == 'behavior': args += ['--authorization', self.auth]
        return args

    def approve(self):
        fresh_store(self.store)
        self.row['trust_checkpoints'] = 1
        return _replay_fixture_confirmation(self.approval_command(), project=self.project,
                    controller=self.controller, scenario=self.s, env=self.env)

    def validate_agent(self, response, operation):
        data = decode(response.stdout)
        require((data['product'] == self.s['product'] or (operation == 'report' and response.returncode == 3 and data['product'] == 'behavior' and data['kind'] == 'infrastructure_error' and data['source'] is None)) and data['operation'] == operation
                and data['protocol_version'] == '1' and data['verdict'] in OUTCOMES
                and OUTCOMES[data['verdict']] == response.returncode, 'invalid Agent transport')
        for private in [str(self.root), *self.private]:
            require(private not in response.stdout.decode(), 'Agent private leakage')
        raw = self.controller / ('agent-' + str(len(self.responses)) + '.json')
        save(raw, data)
        if operation == 'verify':
            checked = self.cli('ci-check', raw, str(response.returncode))
            require(checked.returncode == response.returncode and decode(checked.stdout)['verdict'] == data['verdict'], 'Agent validator disagreement')
        elif response.returncode != 3:
            original = dict(self.responses['verify'], operation='report')
            require(data == original, 'report projection differs from verified source')
        else:
            require(data['source'] is None and data['kind'] == 'infrastructure_error' and data['evidence_refs'] == [], 'invalid report error')
        self.responses[operation] = data
        return data

    def report(self, source):
        args = ['report', source, '--store', self.store, '--output', 'agent', '--protocol', '1']
        if self.s['product'] == 'behavior': args += ['--authorization', self.auth]
        return self.cli(*args)

    def execute(self):
        try:
            self.stage('project', self.project_setup)
            lanes(self.project, self.controller)
            self.row['controller_separated'] = True
            self.stage('inspect', lambda: self.cli('inspect', self.project))
            self.stage('init', lambda: self.cli('init', self.project))
            retained_registry = (self.project / '.b2ige/project.json').read_bytes()
            repeated = self.cli('init', self.project)
            self.controls['repeat-init'] = repeated.returncode == 0 and (self.project / '.b2ige/project.json').read_bytes() == retained_registry
            require(self.controls['repeat-init'], 'repeat init changed registry')
            self.stage('materialize', self.materialize)
            # Blind workspace snapshot must exclude preparation output, which is adjacent.
            if self.s['product'] == 'blindtest': self.draft = self.root / 'draft'
            self.stage('prepare', self.prepare)
            self.stage('approve', self.approve)
            doctor = self.stage('doctor', lambda: self.cli('doctor', '--registry', self.registry))
            require(decode(doctor.stdout)['verification_performed'] is False, 'doctor semantics')
            self.doctor = doctor
            verified = self.stage('verify', lambda: self.cli('verify', 'bench', '--registry', self.registry,
                                  '--output', 'agent', '--protocol', '1'), lambda p: p.returncode in OUTCOMES.values())
            def agent_stage():
                data = self.validate_agent(verified, 'verify')
                self.row['verification_verdict'] = data['verdict']
                self.row['verification_coverage'] = data['scope']['coverage'] if data['scope'] else {}
                self.row['agent_output_validated'] = True
                self.row['first_verification_success'] = data['source'] is not None and data['verdict'] != 'ERROR'
                return subprocess.CompletedProcess([], 0)
            self.stage('agent', agent_stage)
            original = self.responses['verify']
            require(original['source'] is not None, 'missing verified source')
            source = original['source']['artifact_id']
            if self.s['product'] == 'blindtest':
                # Agent source IDs are opaque aliases. Resolve only inside the controller.
                sources = [p.parent.name for p in self.store.glob('*/result.json')
                           if 'blindtest_result_id' in read(p)['manifest']['result']]
                require(len(sources) == 1, 'ambiguous private source')
                source = sources[0]
            if self.mode == 'corrupt':
                remove_evidence(self.store, source)
            def reporting():
                if self.s['product'] == 'blindtest':
                    self.row['controller_separated'] = False
                    private_separation(self.project, self.sealed, self.private)
                    self.row['controller_separated'] = True
                response = self.report(source)
                data = self.validate_agent(response, 'report')
                if self.mode != 'corrupt': require(data['verdict'] == original['verdict'], 'reload disagreement')
                self.row['actual_verdict'], self.row['exit_code'] = data['verdict'], response.returncode
                self.row['evidence_status'] = 'loader_rejected' if self.mode == 'corrupt' else 'verified'
                return response
            self.stage('report', reporting, lambda p: p.returncode in OUTCOMES.values())
            self.row['completed'] = True
        except (OSError, ValueError, KeyError, subprocess.SubprocessError):
            if self.row['first_blocking_stage'] is None:
                self.row['first_blocking_stage'] = self.row['stages'][-1]['stage']
                self.row['stages'][-1]['completed'] = False
        return self.row


def remove_evidence(store, source):
    evidence = sorted((store / source / 'evidence').glob('*.json'))
    require(evidence, 'no evidence to remove')
    evidence[0].unlink()


def rejected(action):
    try:
        action()
    except (ValueError, OSError, KeyError, TypeError):
        return True
    return False


def structural_controls(result, manifest, scratch):
    """Attack the actual admission/summary validators; never manufacture successful verdicts."""
    controls = {}
    def attack_result(name, change):
        bad = copy.deepcopy(result)
        change(bad)
        controls[name] = rejected(lambda: validate_result(bad, manifest))
    attack_result('expected-label-tampering', lambda r: r['scenarios'][0].update(allowed_verdicts=['FAIL']))
    attack_result('omitted-scenario', lambda r: r['scenarios'].pop())
    attack_result('corrupt-previous-result', lambda r: r['scenarios'][1].update(actual_verdict='PASS'))
    data = (ROOT / manifest['authority_source_path']).read_bytes()
    controls['authority-tampering'] = rejected(lambda: validate_authority(manifest, data + b' '))
    for name, key, value in [('missing-authority-case', 'authority_source_case_id', 'absent'),
                             ('source-product-mismatch', 'product', 'blindtest')]:
        bad = copy.deepcopy(manifest)
        bad['scenarios'][0][key] = value
        controls[name] = rejected(lambda: validate_authority(bad, data))
    output = scratch / 'previous-output'
    output.mkdir()
    (output / 'result.json').write_text('{"prior_failure":true}')
    controls['stale-output'] = rejected(lambda: output.mkdir())
    controls['stale-evidence-store'] = rejected(lambda: fresh_store(output))
    (output / '.b2ige').mkdir()
    save(output / '.b2ige/project.json', {'schema_version': '1', 'entries': {}})
    controls['stale-project-registry'] = rejected(lambda: fresh_project(output))
    controls['private-overlap'] = rejected(lambda: lanes(scratch, scratch / 'nested'))
    leaked = scratch / 'accidental-project-copy'
    leaked.mkdir()
    (leaked / 'suite.json').write_text('benchmark-private-marker')
    controls['private-copy'] = rejected(lambda: private_separation(leaked, output, ['benchmark-private-marker']))
    link = scratch / 'linked-controller'
    link.symlink_to(output, target_is_directory=True)
    controls['symlink-controller'] = rejected(lambda: lanes(scratch / 'project', link))
    return controls


def live_controls(journeys, result, manifest, scratch):
    controls = structural_controls(result, manifest, scratch)
    byid = {j.s['scenario_id']: j for j in journeys}
    b = byid['rust-preserving']
    if b.row['completed']:
        review_controller = b.root / 'noninteractive-controller'
        review_controller.mkdir()
        shutil.copyfile(b.auth, review_controller / 'authorization.json')
        command = b.approval_command()
        for key, value in [('--controller', review_controller), ('--registry', review_controller / 'registry.json'), ('--store', review_controller / 'store'), ('--authorization', review_controller / 'authorization.json')]:
            command[command.index(key) + 1] = value
        noninteractive = run(command, cwd=b.controller, env=b.env)
        replay = _replay_fixture_confirmation(command, project=b.project, controller=review_controller, scenario=b.s, env=b.env)
        controls['noninteractive-approval'] = noninteractive.returncode == 3 and replay.returncode == 0
        config = read(b.controller / 'input.json')
        stale_reference = b.controller / 'stale-reference'
        stale_reference.write_text('#!/bin/sh\nprintf changed\n')
        stale_reference.chmod(0o700)
        for name, mutate in [
            ('unapproved-baseline', lambda c: c['baseline']['approval'].update(status='unapproved')),
            ('stale-reference', lambda c: c['before'].update(executable=str(stale_reference))),
            ('baseline-poisoning', lambda c: c.update(before=c['after'])),
        ]:
            changed = copy.deepcopy(config)
            mutate(changed)
            path = b.controller / (name + '.json')
            save(path, changed)
            if name == 'unapproved-baseline':
                # prepare may legitimately retain an unapproved draft; approval must refuse it.
                out = b.root / name
                prepared = b.prepare(path, out)
                if prepared.returncode == 0:
                    negative_controller = b.root / 'unapproved-controller'
                    negative_controller.mkdir()
                    shutil.copyfile(b.auth, negative_controller / 'authorization.json')
                    command = b.approval_command()
                    for key, value in [('--controller', negative_controller), ('--registry', negative_controller / 'registry.json'), ('--store', negative_controller / 'store'), ('--authorization', negative_controller / 'authorization.json')]:
                        command[command.index(key) + 1] = value
                    command[3] = out
                    command[command.index('--identity') + 1] = 'bench'
                    response = _replay_fixture_confirmation(command, project=b.project,
                                    controller=negative_controller, scenario=b.s, env=b.env)
                    controls[name] = response.returncode == 3
                else:
                    controls[name] = prepared.returncode == 3
            else:
                controls[name] = b.prepare(path, b.root / name).returncode == 3
        for name, value in [('malformed-registry', '{bad'),
                            ('duplicate-registry', '{"schema_version":"1","entries":{},"entries":{}}')]:
            path = b.controller / (name + '.json')
            path.write_text(value)
            controls[name] = b.cli('verify', 'bench', '--registry', path, '--output', 'agent', '--protocol', '1').returncode == 3
        ready = b.controller / 'readiness.json'
        ready.write_bytes(b.doctor.stdout)
        controls['doctor-as-verification'] = b.cli('ci-check', ready, '0').returncode == 3
        # Simulate a replaced executable after the operator has pinned it.
        with b.target.open('ab') as f: f.write(b'changed executable')
        original_response = b.responses['verify']
        stale = b.cli('verify', 'bench', '--registry', b.registry, '--output', 'agent', '--protocol', '1')
        controls['stale-executable'] = stale.returncode == 3 and b.validate_agent(stale, 'verify')['verdict'] == 'ERROR'
        # Preserve the negative response independently; the primary source stays intact.
        error_response = decode(stale.stdout)
        b.responses['verify'] = original_response
        b.responses['negative-error'] = error_response
        # The copied store is disposable. Remove required evidence, then ask the real loader.
        source = b.responses['verify']['source']['artifact_id']
        remove_evidence(b.store, source)
        controls['missing-evidence'] = b.report(source).returncode == 3
    s = byid['sqlite-safe']
    if s.row['completed']:
        config = read(s.controller / 'input.json')
        changed = copy.deepcopy(config)
        changed['required_observers'][0]['external_effect_id_column'] = 'absent_column'
        path = s.controller / 'failed-observer.json'
        save(path, changed)
        controls['failed-observer'] = s.prepare(path, s.root / 'failed-observer').returncode == 3
        changed = copy.deepcopy(config)
        changed['required_observers'][0]['db_path'] = 'missing.db'
        path = s.controller / 'missing-observer.json'
        save(path, changed)
        controls['missing-observer'] = s.prepare(path, s.root / 'missing-observer').returncode == 3
        changed = copy.deepcopy(config)
        # Keep reviewed committed-state contract intact; a log-only candidate cannot satisfy it.
        changed['trigger']['args'] = ['-c', 'print("payment committed")']
        path = s.controller / 'attempt-only.json'
        save(path, changed)
        response = s.cli('sideeffect', 'verify', path, '--store', s.controller / 'attempt-only-store', '--output', 'agent', '--protocol', '1')
        controls['attempt-log-substitution'] = response.returncode == 1 and decode(response.stdout)['verdict'] == 'FAIL'
    for verdict in ['FAIL', 'INCONCLUSIVE', 'ERROR']:
        candidates = [(j, response) for j in journeys for response in j.responses.values() if response['verdict'] == verdict and response['operation'] == 'verify']
        if candidates:
            j, response = candidates[0]
            path = j.controller / ('coerced-' + verdict + '.json')
            save(path, response)
            controls[verdict + '-exit-zero'] = j.cli('ci-check', path, '0').returncode == 3
    tooling = scratch / 'tooling.json'
    save(tooling, result)
    controls['tooling-as-evidence'] = all(b.cli(*args).returncode == 3 for args in [
        ['verify', tooling, '--registry', b.registry, '--output', 'agent', '--protocol', '1'],
        ['report', tooling, '--output', 'agent', '--protocol', '1'],
    ])
    require(set(controls).issubset(CONTROLS), 'unexpected control key')
    return {name: controls.get(name, False) for name in CONTROLS}


def environment(env, cwd):
    versions = {}
    for name, args in [('rustc', ['--version']), ('node', ['--version']),
                       ('python3', ['--version']), ('docker', ['version', '--format', '{{.Server.Version}}'])]:
        try:
            p = run([name, *args], cwd=cwd, env=env, timeout=15)
            text = p.stdout.decode().strip().splitlines()
            # Do not copy arbitrary version stderr/paths into public observations.
            versions[name] = text[0] if p.returncode == 0 and text and '/' not in text[0] and len(text[0]) < 150 else 'unavailable'
        except (OSError, subprocess.SubprocessError): versions[name] = 'unavailable'
    docker_engine = 'unavailable'
    try:
        p = run(['docker', 'info', '--format', '{{.OSType}}/{{.Architecture}}'], cwd=cwd, env=env, timeout=15)
        if p.returncode == 0 and p.stdout.decode().strip() in ['linux/aarch64', 'linux/arm64', 'linux/x86_64', 'linux/amd64']:
            docker_engine = p.stdout.decode().strip()
    except (OSError, subprocess.SubprocessError):
        pass
    return {'system': platform.system(), 'release': platform.release(), 'machine': platform.machine(),
            'versions': versions, 'docker_engine': docker_engine}


def execute(output, *, build=False):
    output = Path(output).absolute()
    require(not any(p.is_symlink() for p in [output, *output.parents]), 'symlink output refused')
    output.mkdir()  # Create-new, including previous failed or partial output.
    manifest = load_corpus()
    before = p8_inventory()
    corpus_before = inventory([CORPUS])
    binary, helper = ROOT / 'target/release/b2ige', ROOT / 'target/release/examples/adoption_fixture'
    prep_start = time.monotonic()
    if build:
        for command in [['cargo', 'build', '--workspace', '--release', '--locked'],
                        ['cargo', 'build', '-p', 'verify-cli', '--release', '--locked', '--example', 'adoption_fixture']]:
            p = run(command, cwd=ROOT, env=os.environ.copy(), timeout=600)
            require(p.returncode == 0, 'source preparation failed')
    require(binary.is_file() and helper.is_file(), 'build source CLI and fixture example first')
    preparation = {'mode': 'source-build' if build else 'prebuilt-source-checkout',
                   'seconds': round(time.monotonic() - prep_start, 4),
                   'binary_identity': digest(binary.read_bytes()), 'helper_identity': digest(helper.read_bytes())}
    controls = dict.fromkeys(CONTROLS, False)
    rows, journeys = [], []
    with tempfile.TemporaryDirectory(prefix='b2ige-adoption-') as temporary:
        root = Path(temporary).resolve()
        (root / 'scenarios').mkdir()
        (root / 'home').mkdir()
        # Never inherit developer B2IGE overrides, startup hooks or project/evidence state.
        env = {k: v for k, v in os.environ.items() if k in ['PATH', 'SYSTEMROOT', 'DOCKER_HOST', 'DOCKER_CONTEXT']}
        env.update(HOME=str(root / 'home'), TMPDIR=str(root), LANG='C', LC_ALL='C',
                   RUSTUP_HOME=os.environ.get('RUSTUP_HOME', str(Path.home() / '.rustup')))
        # Discover the user's local Docker endpoint read-only before isolating Docker config.
        try:
            endpoint = run(['docker', 'context', 'inspect', '--format', '{{.Endpoints.docker.Host}}'], cwd=ROOT, env=os.environ.copy())
            if endpoint.returncode == 0 and endpoint.stdout.decode().strip().startswith('unix://'):
                env['DOCKER_HOST'] = endpoint.stdout.decode().strip()
                env.pop('DOCKER_CONTEXT', None)
        except (OSError, subprocess.SubprocessError):
            pass  # Docker scenarios still occupy their planned denominator.
        obs = {'environment': environment(env, root), 'preparation': preparation, 'scenario_seconds': {}}
        for scenario in manifest['scenarios']:
            j = Journey(root / 'scenarios', scenario, binary, helper, env)
            rows.append(j.execute())
            journeys.append(j)
            obs['scenario_seconds'][scenario['scenario_id']] = j.timings
        result = dict(schema_version='1', kind='adoption-benchmark-tooling', authoritative=False,
                      corpus_version='adoption-v1', manifest_identity=MANIFEST_PIN, scenarios=rows,
                      negative_controls=controls, aggregate=aggregate(rows, manifest, controls),
                      p8_before=before, p8_after=p8_inventory(), observations=obs, ci_bootstrap='DEFERRED_TO_V110_C')
        result['negative_controls'] = live_controls(journeys, result, manifest, root)
        result['aggregate'] = aggregate(rows, manifest, result['negative_controls'])
        require(inventory([CORPUS]) == corpus_before, 'corpus changed during execution')
        load_corpus()
        result['p8_after'] = p8_inventory()
        validate_result(result, manifest)
        public = json.dumps(result)
        for j in journeys:
            require(all(value not in public for value in [str(root), *j.private]), 'public summary private leakage')
        save(output / 'result.json', result)
        (output / 'report.md').write_text(report(result), encoding='utf-8')
    return result


def report(result):
    validate_result(result)
    rows = result['scenarios']
    a = result['aggregate']
    lines = ['# V110 adoption benchmark — measured report', '',
             'Corpus: adoption-v1; tooling schema: 1; non-authoritative V110 UX/adoption evidence.', '',
             'V110-B measures bounded reproducibility of the adoption workflow against semantic expectations inherited from the already-reviewed P8 corpus.',
             'It does not establish independent new correctness labels, third-party project compatibility, external-user usability, real hidden secrecy, or exhaustive correctness. External operator evidence remains V110-D.', '',
             'Source checkout/build preparation is separate from operation; historical v0.2.0 archives do not contain V110-A. CI bootstrap: DEFERRED_TO_V110_C.', '',
             'Environment: ' + json.dumps(result['observations']['environment'], sort_keys=True),
             'Docker scope: actual local Linux engine; digest-pinned preloaded image, no network build or runtime service.', '',
             '| Metric | Numerator / denominator |', '|---|---:|']
    for name, value in sorted(a.items()):
        lines.append(f"| {name} | {value['numerator']} / {value['denominator']}" + (' (N/A)' if not value['denominator'] else '') + ' |')
    lines += ['', '| Scenario / P8 authority | Runtime / product | Allowed → actual | Trust checkpoints | Blocking stage |', '|---|---|---|---:|---|']
    for r in rows:
        lines.append(f"| {r['scenario_id']} / {r['authority_source_case_id']} | {r['runtime']} / {r['product']} | {','.join(r['allowed_verdicts'])} → {r['actual_verdict'] or 'NOT_OBSERVED'} | {r['trust_checkpoints']} | {r['first_blocking_stage'] or 'none'} |")
    lines += ['', f"Measured inventory: {len(rows)} / {len(rows)} selected scenarios retained, including blocked scenarios.",
              'First verification success means a source-backed PASS, FAIL or INCONCLUSIVE response after verify, not doctor readiness and not product PASS alone.',
              'The corrupt SideEffect scenario first verifies intact evidence, then removes evidence and measures the report loader ERROR boundary, matching P8. Generic report errors may carry the CLI’s Behavior product hint; they grant no product success.',
              'One benchmark PTY checkpoint per approval replays a pre-existing reviewed fixture decision. No output-derived approval or production noninteractive bypass exists.',
              'Recovery events are explicitly counted; zero recovery attempts are N/A. Repeated init is tested separately without counting it as a recovery.', '',
              'Public P8 BlindTest fixtures are mechanically materialized outside the project. This measures operational path separation, not secrecy from readers of the public corpus or unrestricted same-user host processes.',
              'Timing is observational and never gates success; preparation and stage timings are stored separately in result.json. Docker layers and compiler caches may be reused; project state, approvals and evidence stores are fresh.',
              'P8 corpus and baseline byte inventories are identical before/after. Expected source file and trusted constructor identities are pinned. No private holdout was run.', '',
              '## Negative controls', '', *[f"- {name}: {'rejected' if ok else 'NOT REJECTED'}" for name, ok in sorted(result['negative_controls'].items())], '']
    return '\n'.join(lines)
