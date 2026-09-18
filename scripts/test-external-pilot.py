#!/usr/bin/env python3
"""Deterministic TEST DATA ONLY. No external operators, CI runs or pilot execution.

In-memory mutations exercise alleged evidence shape; none is written as public
external evidence. Self-attestation cannot detect a dishonest wholesale forgery.
"""
import copy
import importlib.util
import io
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('external_pilot', ROOT / 'scripts/external-pilot.py')
p = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = p
spec.loader.exec_module(p)
import external_pilot_ci as ci
SHA = p.commit(clean=False)


def synthetic():
    """Visibly synthetic; production admission MUST reject this object."""
    r = p.blank_result(SHA, '1234567890abcdef' * 2, dict(os='Linux', architecture='x86_64', docker='available'))
    r['installation'] = dict(status='completed', command_count=2, assistance=['none'])
    r['test_data'] = True
    r['provenance'] = 'TEST DATA'
    r['attestation'] = {k: i >= 3 for i, k in enumerate(p.ATTEST)}
    for j in r['journeys']:
        j.update(status='completed', command_count=12, manual_trust_checkpoints=1,
                 first_result_seconds=1, final_verdict='PASS', final_exit_code=0,
                 verified_reload=True, controller_separated=True, confidence='high', doctor_is_verification='no')
        j['stages'] = [dict(stage=s, started_unix=1700000000, elapsed_seconds=0.1,
                           exit_code=0, completed=True) for s in p.STAGES]
        j['agent'] = dict(product=j['product'], operation='verify', protocol_version='1', verdict='PASS',
                          exit_code=0, source_backed=True, validated=True)
    r['questions'] = [dict(id=q['id'], answer=q['expected'], consulted_docs=False, explanation='omitted') for q in p.PROTOCOL['questions']]
    r['mcp'] = dict(registered_identity='bench', tools_listed=True, response=copy.deepcopy(r['journeys'][0]['agent']), approval_authority='no', assistance='none')
    for index, verdict in enumerate(['PASS', 'FAIL']):
        repo = 'https://github.com/pilot-owner/pilot-repository'
        r['ci'].append(dict(repository_url=repo, base_sha='0123456789' * 4, head_sha='abcdef0123' * 4,
                            verifier_sha=SHA, workflow_path='.github/workflows/b2ige-verify.yml',
                            workflow_sha256='sha256:' + '1' * 64, pr_url=repo + '/pull/1',
                            run_url=repo + '/actions/runs/' + str(index + 1), run_id=index + 1,
                            job_id=index + 10, verdict=verdict, exit_code=p.OUTCOMES[verdict],
                            check_conclusion='success' if verdict == 'PASS' else 'failure',
                            verification_step_conclusion='success' if verdict == 'PASS' else 'failure',
                            independently_provisioned=True, public_consent=True, observed_via='github_api',
                            artifacts=[dict(name='b2ige-sanitized-agent-reports', file_count=1,
                                            kind='validated-agent-protocol-v1-json', archive_sha256='sha256:' + '2' * 64)], no_private_artifacts=True))
    return r


class PilotTests(unittest.TestCase):
    def setUp(self):
        self.network = mock.patch.object(socket, 'socket', side_effect=AssertionError('network forbidden'))
        self.network.start()
        self.addCleanup(self.network.stop)
        # Shape checks on a hypothetical self-attestation, isolated to test memory.
        # This is NOT an external result and is never saved or reported publicly.
        self.r = synthetic()
        self.r.update(test_data=False, provenance='external_operator')

    def valid(self, value=None, complete=True):
        return p.validate(value or self.r, complete=complete, expected_commit=SHA)

    def reject(self, mutate, complete=True):
        value = copy.deepcopy(self.r)
        mutate(value)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            self.valid(value, complete=complete)

    def test_protocol_inventory_and_shape(self):
        self.assertEqual(p.PRODUCTS, ['behavior', 'sideeffect', 'blindtest'])
        self.assertEqual([v['scenario'] for v in p.PROTOCOL['journeys']], ['rust-preserving', 'sqlite-safe', 'docker-correct'])
        self.assertEqual(len(p.PROTOCOL['questions']), 6)
        self.valid()

    def test_test_data_cannot_qualify(self):
        with self.assertRaises(ValueError): self.valid(synthetic())
        for source in ['internal_developer', 'localhost', 'Codex', 'ChatGPT', 'adoption-benchmark-tooling', 'TEST DATA']:
            self.reject(lambda r: r.update(provenance=source))
        self.reject(lambda r: r.update(test_data=True))
        with self.assertRaises(ValueError): self.valid(p.bench.read(p.bench.CORPUS / 'measured-result.json'))

    def test_attestation_and_developer(self):
        self.reject(lambda r: r.update(attestation=None))
        for key in p.ATTEST:
            self.reject(lambda r: r['attestation'].update({key: not r['attestation'][key]}))
        self.reject(lambda r: r['attestation'].update(participant_is_b2ige_developer=1))

    def test_exact_journeys_and_copied_results(self):
        self.reject(lambda r: r['journeys'].pop())
        self.reject(lambda r: r['journeys'].__setitem__(1, copy.deepcopy(r['journeys'][0])))
        self.reject(lambda r: r['journeys'][0].update(product='unknown'))
        with self.assertRaises(ValueError): p.report([self.r] * 3, expected_commit=SHA)

    def test_incomplete_denominator(self):
        self.r['journeys'][1] = p.blank_journey('sideeffect')
        self.valid(complete=False)
        with self.assertRaises(ValueError): self.valid()
        report = p.report([self.r], expected_commit=SHA)
        self.assertIn('Journey completion: 2/3', report)
        self.assertIn('incomplete/drop-off', report)
        self.assertIn('EVIDENCE_PENDING', report)

    def test_verdict_exit_and_doctor(self):
        for v, code in [('PASS', 1), ('FAIL', 0), ('INCONCLUSIVE', 0), ('ERROR', 0)]:
            self.reject(lambda r: r['journeys'][0].update(final_verdict=v, final_exit_code=code))
        self.reject(lambda r: r['journeys'][0]['agent'].update(operation='doctor'))
        self.reject(lambda r: r['journeys'][0].update(stages=r['journeys'][0]['stages'][:7]))
        self.reject(lambda r: r['journeys'][0]['agent'].update(source_backed=False))
        self.reject(lambda r: r['journeys'][0].update(verified_reload=False))
        self.reject(lambda r: r['journeys'][0].update(manual_trust_checkpoints=0))

    def test_assistance_and_answers_preserved(self):
        self.r['journeys'][0]['assistance'] = [dict(stage='approve', kind='ai_assistant')]
        self.r['questions'][0]['answer'] = 'yes'
        before = copy.deepcopy(self.r)
        self.valid()
        self.assertEqual(self.r, before)
        self.assertEqual(p.classification(self.r['journeys'][0]), 'assisted completion')
        self.assertIn('"answer": "yes"', p.report([self.r], expected_commit=SHA))
        self.r['journeys'][0]['assistance'][0]['kind'] = 'public_docs'
        self.assertEqual(p.classification(self.r['journeys'][0]), 'unassisted/doc-only completion')

    def test_required_integrations(self):
        self.reject(lambda r: r['questions'].pop())
        self.reject(lambda r: r['journeys'][0].update(agent=None))
        self.reject(lambda r: r.update(mcp=None))
        self.reject(lambda r: r['mcp'].update(registered_identity='/local/config'))
        self.reject(lambda r: r['ci'].pop(0))
        self.reject(lambda r: r['ci'].pop(1))
        self.reject(lambda r: r['ci'][0].update(no_private_artifacts=False))
        self.reject(lambda r: r['ci'][0].update(artifacts=[]))
        self.reject(lambda r: r['ci'][1].update(check_conclusion='success'))
        self.reject(lambda r: r['ci'][0].update(observed_via='simulated'))
        self.reject(lambda r: r['ci'][0].update(run_url='http://localhost/run/1'))
        self.reject(lambda r: r['ci'][0].update(repository_url='https://github.com/example/test'))
        self.reject(lambda r: r['ci'][0].update(independently_provisioned=False))

    def test_private_unknown_and_nonfinite_fields(self):
        for target in [lambda r:r, lambda r:r['environment'], lambda r:r['journeys'][0],
                       lambda r:r['questions'][0], lambda r:r['mcp'], lambda r:r['ci'][0],
                       lambda r:r['ci'][0]['artifacts'][0], lambda r:r['journeys'][0]['agent']]:
            self.reject(lambda r: target(r).update(raw_path='/private/data'), complete=False)
        self.reject(lambda r: r.update(participant_id='personal-name'))
        self.reject(lambda r: r['questions'][0].update(explanation='person@example.invalid'))
        for value in [float('nan'), float('inf'), -1, True]:
            self.reject(lambda r: r['journeys'][0].update(first_result_seconds=value))
        for value in ['{"a": NaN}', '{"a": Infinity}', '{"a": 1,"a": 2}']:
            with self.assertRaises(ValueError): p.bench.decode(value)

    def test_notes_are_sanitized_not_retained(self):
        self.assertEqual(p.sanitize_note('instructions clear'), 'instructions_clear')
        self.assertEqual(p.sanitize_note('person@example.invalid'), 'withheld_for_privacy')
        self.assertEqual(p.sanitize_note('/private/controller'), 'withheld_for_privacy')
        self.assertEqual(p.sanitize_note(''), 'omitted')
        self.reject(lambda r:r['journeys'][0].update(note='arbitrary raw note'), complete=False)

    def test_schema_stale_and_bindings(self):
        self.reject(lambda r: r.update(schema_version='2'))
        self.reject(lambda r: r.update(authoritative=True))
        self.reject(lambda r: r.update(pilot_commit='1' * 40))
        self.reject(lambda r: r.update(verifier_commit='1' * 40))
        self.reject(lambda r: r.update(protocol_identity='sha256:' + '1' * 64))
        self.reject(lambda r: r.update(corpus_identity='sha256:' + '1' * 64))
        self.reject(lambda r: r['denominators'].update(journeys=2))
        self.reject(lambda r: r['denominators'].update(operators=0))
        self.reject(lambda r: r['denominators'].update(operators=True))
        with mock.patch.object(p.subprocess, 'check_output', side_effect=[SHA.encode(), b' M file']):
            with self.assertRaises(ValueError): p.commit()

    def test_create_new_protected_and_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp).resolve() / 'test-data.json'
            p.save(path, {'TEST DATA': True})
            old = path.read_bytes()
            with self.assertRaises(FileExistsError): p.save(path, {})
            self.assertEqual(old, path.read_bytes())
            link = path.parent / 'link'
            link.symlink_to(path)
            with self.assertRaises(ValueError): p.save(link, {})
        with self.assertRaises(ValueError): p.save(ROOT / 'external-pilot/v1/result.json', {})

    def test_report_determinism_and_zero(self):
        report = p.report([], expected_commit=SHA)
        self.assertIn('EVIDENCE_PENDING', report)
        self.assertIn('0/0', report)
        self.assertNotIn('READY_FOR_INDEPENDENT_REVIEW', report)
        self.assertEqual(p.report([self.r], expected_commit=SHA), p.report([self.r], expected_commit=SHA))
        r2 = copy.deepcopy(self.r)
        r2['run_id'] = 'abcdef0123456789' * 2
        text = p.report([self.r, r2], expected_commit=SHA)
        self.assertIn('1/1; runs: 2', text)
        self.assertIn('6/6', text)

    def test_tooling_rejected_by_actual_product_surfaces(self):
        self.assertTrue(p.BINARY.is_file(), 'build release workspace first')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            path = root / 'TEST-DATA.json'
            p.bench.save(path, synthetic())
            commands = [['verify', str(path), '--registry', str(path)], ['report', str(path)],
                        ['doctor', '--registry', str(path)], ['trust', 'approve', str(path)]]
            commands += [[product, 'verify', str(path)] for product in p.PRODUCTS]
            for args in commands:
                response = subprocess.run([str(p.BINARY), *args], cwd=root, capture_output=True, timeout=30)
                self.assertEqual(response.returncode, 3, args)
            response = subprocess.run([str(p.MCP), '--registry', str(path)], input=b'', cwd=root, capture_output=True, timeout=30)
            self.assertNotEqual(response.returncode, 0)

    def test_no_automatic_approval_or_network_selftest(self):
        source = (ROOT / 'scripts/external-pilot.py').read_text()
        self.assertNotIn('_replay_fixture_confirmation(', source)
        self.assertNotIn('pty.openpty', source)
        self.assertIn("interactive=name in ['approve', 'human']", source)

    def test_workflow_reuses_gate_and_create_new(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            registry = root / 'registry.json'
            p.bench.save(registry, dict(schema_version='1', entries={'bench': dict(product='blindtest', config='/opt/b2ige-pilot/controller/bench/config.json', store='/opt/b2ige-pilot/controller/store', authorization=None)}))
            destination = root / '.github/workflows/b2ige-verify.yml'
            with mock.patch.object(p, 'commit', return_value=SHA), mock.patch('builtins.print'):
                ci.workflow_file(root, registry, 'b2ige2-a11y/b2ige-verify', '/opt/b2ige-pilot/controller', destination, p)
                before = destination.read_bytes()
                with self.assertRaises(FileExistsError):
                    ci.workflow_file(root, registry, 'b2ige2-a11y/b2ige-verify', '/opt/b2ige-pilot/controller', destination, p)
                self.assertEqual(destination.read_bytes(), before)
            template = (ROOT / 'crates/verify-cli/src/ci-workflow.yml').read_text()
            # Exact reviewed verification/upload tail unchanged.
            marker = '      - name: Verify all registered contracts (only PASS is green)'
            self.assertEqual(before.decode().split(marker)[1], template.split(marker)[1])
            self.assertIn('runs-on: [self-hosted, linux, x64, b2ige-pilot]', before.decode())
            self.assertIn(SHA, before.decode())
            self.assertNotIn('continue-on-error', before.decode())

    def test_ci_approval_refuses_noninteractive(self):
        with mock.patch.object(p.sys.stdin, 'isatty', return_value=False):
            with self.assertRaises(ValueError): ci.approval_file('missing', SHA, 'missing', p)

    def test_actual_protocol_checker_on_archive(self):
        result = subprocess.run([str(p.BINARY), 'blindtest', 'verify', 'missing-config', '--output', 'agent', '--protocol', '1'], capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 3)
        data = p.bench.decode(result.stdout)
        self.assertEqual(data['product'], 'blindtest')
        def zipped(value):
            buf = io.BytesIO()
            with zipfile.ZipFile(buf, 'w') as archive: archive.writestr('bench.json', json.dumps(value))
            return buf.getvalue()
        verdicts, inspected = ci.inspect_archive(zipped(data), p.BINARY, p)
        self.assertEqual(verdicts, ['ERROR'])
        self.assertEqual(inspected['file_count'], 1)
        for mutation in [lambda v:v.update(operation='doctor'), lambda v:v.update(extra=True),
                         lambda v:v.update(summary='person@example.invalid')]:
            altered = copy.deepcopy(data)
            mutation(altered)
            with self.assertRaises(ValueError): ci.inspect_archive(zipped(altered), p.BINARY, p)

    def test_github_observation_binding_and_recheck(self):
        # Fully in-memory TEST DATA, fetched via a bounded fake transport. This tests
        # interpretation only, never declares an external run or emits evidence.
        repo = 'pilot-owner/pilot-repository'
        url = 'https://github.com/' + repo
        base, head = '0123456789' * 4, 'abcdef0123' * 4
        workflow = '.github/workflows/b2ige-verify.yml'
        body = (ROOT / 'crates/verify-cli/src/ci-workflow.yml').read_text().replace('__PIN__', SHA).replace('__REPO__', 'b2ige2-a11y/b2ige-verify').replace('__IDENTITY__', 'bench').encode()
        import base64
        responses = {
            'repos/' + repo: dict(private=False, html_url=url),
            'repos/' + repo + '/pulls/1': dict(base={'repo': {'full_name': repo}}, html_url=url + '/pull/1'),
            'repos/' + repo + '/actions/runs/1': dict(event='pull_request_target', status='completed', repository={'full_name': repo}, path=workflow, head_sha=base, pull_requests=[dict(number=1, base={'sha': base}, head={'sha': head})], conclusion='failure', html_url=url + '/actions/runs/1'),
            'repos/' + repo + '/contents/' + workflow + '?ref=' + base: dict(encoding='base64', content=base64.b64encode(body).decode()),
            'repos/' + repo + '/actions/runs/1/jobs?per_page=100': dict(total_count=1, jobs=[dict(name='verify', id=10, conclusion='failure', steps=[dict(name='Verify all registered contracts (only PASS is green)', status='completed', conclusion='failure')])]),
            'repos/' + repo + '/actions/runs/1/artifacts?per_page=100': dict(total_count=1, artifacts=[dict(id=20, expired=False, name='b2ige-sanitized-agent-reports', size_in_bytes=2)]),
            'repos/' + repo + '/actions/artifacts/20/zip': b'TEST DATA'
        }
        def observe(values):
            return ci.observe(repo, 1, 1, SHA, workflow, p.bench.digest(body), p.BINARY, p, lambda endpoint, **kwargs: copy.deepcopy(values[endpoint]))
        inspection = self.r['ci'][1]['artifacts'][0]
        with mock.patch.object(ci, 'inspect_archive', return_value=(['ERROR'], inspection)):
            observed = observe(responses)
            p.ci_fields(observed, SHA)
            self.assertEqual(observed['head_sha'], head)
            self.assertEqual(observed['base_sha'], base)
            for endpoint, mutation in [
                ('/actions/runs/1', lambda v:v.update(pull_requests=[])),
                ('/actions/runs/1', lambda v:v.update(head_sha=head)),
                ('/actions/runs/1', lambda v:v.update(conclusion='success')),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v.update(total_count=2)),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v['artifacts'][0].update(expired=True)),
                ('/actions/runs/1/jobs?per_page=100', lambda v:v['jobs'][0]['steps'][0].update(conclusion='skipped')),
            ]:
                changed = copy.deepcopy(responses)
                mutation(changed['repos/' + repo + endpoint])
                with self.assertRaises(ValueError): observe(changed)
            record = dict(pilot_commit=SHA, ci=[observed])
            ci.recheck(record, p.BINARY, p, lambda endpoint, **kwargs: copy.deepcopy(responses[endpoint]))
            record['ci'][0]['head_sha'] = SHA
            with self.assertRaises(ValueError):
                ci.recheck(record, p.BINARY, p, lambda endpoint, **kwargs: copy.deepcopy(responses[endpoint]))

    def test_archive_path_and_raw_artifact_rejected(self):
        for name in ['../raw.json', 'store/result.json', 'human.txt', 'sealed.json']:
            buf = io.BytesIO()
            with zipfile.ZipFile(buf, 'w') as archive: archive.writestr(name, '{}')
            with self.assertRaises((ValueError, KeyError)):
                ci.inspect_archive(buf.getvalue(), p.BINARY, p)

    def test_github_fake_missing_and_job_failure(self):
        # TEST DATA network transport, never emitted as an external result.
        def fetch(endpoint, **kwargs):
            if endpoint.endswith('/pulls/1'):
                return {'base': {'repo': {'full_name': 'pilot-owner/pilot-repository'}}, 'html_url': 'https://github.com/pilot-owner/pilot-repository/pull/1'}
            if '/actions/runs/' in endpoint:
                return dict(event='push', status='completed', repository={'full_name': 'pilot-owner/pilot-repository'}, path='.github/workflows/b2ige-verify.yml')
            return dict(private=False, html_url='https://github.com/pilot-owner/pilot-repository')
        with self.assertRaises(ValueError):
            ci.observe('pilot-owner/pilot-repository', 1, 1, SHA, '.github/workflows/b2ige-verify.yml', 'sha256:'+'1'*64, p.BINARY, p, fetch)


if __name__ == '__main__':
    unittest.main()
