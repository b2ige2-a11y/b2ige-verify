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
    r = p.blank_result(SHA, '1234567890ab4def8234567890abcdef', dict(os='Linux', architecture='x86_64', docker='available'))
    r['installation'] = dict(status='completed', command_count=2, assistance=['none'])
    r['test_data'] = True
    r['provenance'] = 'TEST DATA'
    r['attestation'] = {k: i >= 4 for i, k in enumerate(p.ATTEST)}
    for j in r['journeys']:
        j.update(status='completed', command_count=12, manual_trust_checkpoints=1,
                 first_result_seconds=1, final_verdict='PASS', final_exit_code=0,
                 verified_reload=True, controller_separated=True, confidence='high', doctor_is_verification='no')
        j['stages'] = [dict(stage=s, started_unix=1700000000, elapsed_seconds=0.1,
                           exit_code=0, completed=True) for s in p.STAGES]
        j['agent'] = dict(product=j['product'], operation='verify', protocol_version='1', verdict='PASS',
                          exit_code=0, source_backed=True, validated=True,
                          source_identity=p.bench.digest(('TEST DATA ' + j['product']).encode()))
    r['questions'] = [dict(id=q['id'], answer=q['expected'], consulted_docs=False, explanation='omitted') for q in p.PROTOCOL['questions']]
    r['mcp'] = dict(registered_identity='bench', tools_listed=True, response=copy.deepcopy(r['journeys'][0]['agent']), approval_authority='no', assistance='none')
    r['mcp']['response']['source_identity'] = p.bench.digest(b'TEST DATA MCP')
    r['integration_attempts'] = [dict(stage=s, product='behavior' if s == 'mcp' else 'blindtest',
                                    status='completed', assistance='none', friction=[], recovery_attempts=0) for s in ['mcp', 'ci']]
    for index, verdict in enumerate(['PASS', 'FAIL']):
        repo = 'https://github.com/pilot-owner/pilot-repository'
        r['ci'].append(dict(repository_url=repo, base_sha='0123456789' * 4, head_sha='abcdef0123' * 4,
                            verifier_sha=SHA, workflow_path='.github/workflows/b2ige-verify.yml',
                            workflow_sha256='sha256:' + '1' * 64, pr_url=repo + '/pull/1',
                            run_url=repo + '/actions/runs/' + str(index + 1), run_id=index + 1,
                            job_id=index + 10, verdict=verdict, exit_code=p.OUTCOMES[verdict],
                            response=dict(product='blindtest', operation='verify', protocol_version='1', verdict=verdict,
                                          exit_code=p.OUTCOMES[verdict], source_backed=True, validated=True,
                                          source_identity=p.bench.digest(('TEST DATA CI ' + verdict).encode())), assistance='none',
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

    def test_copied_observation_cannot_be_relabelled(self):
        def copied(r):
            r['journeys'][1] = copy.deepcopy(r['journeys'][0])
            r['journeys'][1]['product'] = 'sideeffect'
            r['journeys'][1]['agent']['product'] = 'sideeffect'
        self.reject(copied)
        self.reject(lambda r: r['mcp'].update(response=copy.deepcopy(r['journeys'][0]['agent'])))
        self.reject(lambda r: r['journeys'][0]['agent'].pop('source_identity'))
        self.reject(lambda r: r['journeys'][0]['agent'].update(product='blindtest'))

    def test_anonymous_ids_cannot_be_internal_metadata_or_missing(self):
        for value in ['', None, 'internal_developer', 'adoption-v1', 'rust-preserving', '0' * 32,
                      '1234567890abcdef' * 2]:
            self.reject(lambda r: r.update(participant_id=value))
        self.reject(lambda r: r.pop('participant_id'))

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

    def test_installation_and_integration_help_cannot_disappear(self):
        self.reject(lambda r: r['installation'].update(assistance=['ai_assistant']))
        self.r['installation']['assistance'] = ['ai_assistant']
        for j in self.r['journeys']:
            j['assistance'].append(dict(stage='setup', kind='ai_assistant'))
        self.valid()
        self.assertTrue(p.assisted(self.r))
        self.assertTrue(all(p.classification(j) == 'assisted completion' for j in self.r['journeys']))
        for field in ['friction', 'terminology']:
            self.reject(lambda r: r['journeys'][0].update({field: ['unknown']}))
        self.reject(lambda r: r['journeys'][0]['assistance'].append(dict(stage='invented', kind='ai_assistant')))
        self.reject(lambda r: r['ci'][0].update(assistance='unknown'))
        self.reject(lambda r: r['mcp'].update(assistance='none-of-the-above'))
        with mock.patch.object(p, 'choose', return_value='none'):
            self.assertEqual(p.strongest_help('b2ige_developer', 'TEST DATA'), 'b2ige_developer')

    def test_dropoff_retains_stage_product_help_and_recovery(self):
        j = p.blank_journey('sideeffect')
        j.update(status='incomplete', command_count=1, first_blocking_stage='setup',
                 friction=['installation'], assistance=[dict(stage='setup', kind='external_human')], recovery_attempts=1)
        j['stages'] = [dict(stage='setup', started_unix=1700000000, elapsed_seconds=1, exit_code=3, completed=False)]
        self.r['journeys'][1] = j
        self.valid(complete=False)
        text = p.report([self.r], expected_commit=SHA)
        for value in ['EVIDENCE_PENDING', '2/3', 'sideeffect', 'external_human', '"recovery_attempts": 1']:
            self.assertIn(value, text)
        self.assertIn('"blocking_category": "installation"', text)
        self.reject(lambda r: r['journeys'][1].update(first_blocking_stage=None), complete=False)
        self.reject(lambda r: r['journeys'][1].update(status='completed'))
        j['stages'][0]['exit_code'] = -9
        self.valid(complete=False)
        self.assertIn('2/3', p.report([self.r], expected_commit=SHA))

    def test_six_fixed_questions_and_unmodified_wrong_answers(self):
        self.assertEqual([q['id'] for q in p.PROTOCOL['questions']], ['q1', 'q2', 'q3', 'q4', 'q5', 'q6'])
        self.assertEqual([q['expected'] for q in p.PROTOCOL['questions']], ['no', 'no', 'yes', 'yes', 'yes', 'yes'])
        for answer in ['yes', 'no', 'unsure']:
            for q in self.r['questions']:
                q.update(answer=answer, consulted_docs=True)
            before = copy.deepcopy(self.r)
            self.valid()
            self.assertEqual(before, self.r)
            self.assertIn('"consulted_docs": true', p.report([self.r], expected_commit=SHA))
        self.reject(lambda r: r['questions'][0].update(answer='correct'))
        self.reject(lambda r: r['questions'][0].update(id='q2'))

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

    def test_ci_error_is_reportable_but_never_a_completed_control(self):
        c = self.r['ci'][1]
        c.update(verdict='ERROR', exit_code=3)
        c['response'].update(verdict='ERROR', exit_code=3, source_backed=False, source_identity=None)
        self.valid(complete=False)
        with self.assertRaisesRegex(ValueError, 'infrastructure ERROR'): self.valid()
        self.assertIn('EVIDENCE_PENDING', p.report([self.r], expected_commit=SHA))

    def test_ci_reference_mutation_matrix(self):
        for repo in ['https://example.com/owner/repo', 'https://localhost/owner/repo',
                     'https://github.com/owner/simulated-pilot', 'https://github.com/mock-owner/repo',
                     'https://github.com/owner/synthetic_fixture', 'https://github.com/owner/ghp_secret',
                     'https://github.com/owner/' + 'AKIA' + 'ABCDEFGHIJKLMNOP', 'https://github.com/owner/192.0.2.1']:
            self.reject(lambda r: r['ci'][0].update(repository_url=repo))
        for key in ['base_sha', 'head_sha', 'verifier_sha']:
            for value in [None, '', 'main', '0' * 40]:
                self.reject(lambda r: r['ci'][0].update({key: value}))
        self.reject(lambda r: r['ci'][1].update(run_id=1, run_url=r['ci'][0]['run_url']))
        self.reject(lambda r: r['ci'][0].update(check_conclusion='failure'))
        self.reject(lambda r: r['ci'][0].update(verification_step_conclusion='skipped'))
        self.reject(lambda r: r['ci'][0]['artifacts'][0].update(file_count=2))
        for name in ['store', '.b2ige', 'authorization', 'approval', 'sealed', 'oracle', 'canary', 'human', 'docker.sock', 'environment']:
            self.reject(lambda r: r['ci'][0]['artifacts'][0].update(name=name))

    def test_incomplete_integration_retains_assistance(self):
        self.r['mcp'] = None
        self.r['integration_attempts'][0].update(status='incomplete', assistance='ai_assistant', friction=['mcp'], recovery_attempts=1)
        self.valid(complete=False)
        text = p.report([self.r], expected_commit=SHA)
        self.assertIn('Run assistance: assisted', text)
        self.assertIn('EVIDENCE_PENDING', text)
        self.reject(lambda r: r['integration_attempts'][0].update(status='completed'), complete=False)
        self.reject(lambda r: r['integration_attempts'][0].update(friction=[]), complete=False)

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

    def test_public_privacy_attack_matrix(self):
        for private in ['Alice Example', '@participant_handle', 'person@example.invalid',
                        '/ho' + 'me/person/work', '192.0.2.1', 'ghp_TESTSECRET', 'Bearer TESTSECRET',
                        'AKIA' + 'ABCDEFGHIJKLMNOP', '-----BEGIN OPENSSH ' + 'PRIVATE KEY-----',
                        'SECRET_KEY=TESTSECRET', '/private/controller/sealed', '/var/run/docker.sock']:
            self.assertEqual(p.sanitize_note(private), 'withheld_for_privacy')
            for mutate in [lambda r:r.update(participant_id=private),
                           lambda r:r['journeys'][0].update(note=private),
                           lambda r:r['questions'][0].update(explanation=private),
                           lambda r:r['environment'].update(os=private)]:
                self.reject(mutate, complete=False)
        self.valid()  # Consented public GitHub links remain possible.

    def test_schema_stale_and_bindings(self):
        self.reject(lambda r: r.update(schema_version='1'))
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
        self.reject(lambda r: r.update(pilot_commit='main'))
        with mock.patch.object(p.subprocess, 'check_output', side_effect=subprocess.CalledProcessError(128, ['git'])):
            with self.assertRaises(subprocess.CalledProcessError): p.commit()

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
        r2['run_id'] = p.uuid.uuid4().hex
        with self.assertRaisesRegex(ValueError, 'reused observations'):
            p.report([self.r, r2], expected_commit=SHA)
        # A distinct hypothetical TEST DATA execution has distinct source identities.
        for value in [j['agent'] for j in r2['journeys']] + [r2['mcp']['response']] + [c['response'] for c in r2['ci']]:
            value['source_identity'] = p.bench.digest(('TEST DATA second ' + value['source_identity']).encode())
        for c in r2['ci']:
            c['run_id'] += 100
            c['run_url'] = c['repository_url'] + '/actions/runs/' + str(c['run_id'])
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
            commands += [['prepare', '--product', product, '--config', str(path), '--out', str(root / product)] for product in p.PRODUCTS]
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

    def test_interactive_execution_never_supplies_approval_input(self):
        with mock.patch.object(p.subprocess, 'run', return_value=subprocess.CompletedProcess([], 3)) as run, mock.patch.object(p, 'show'):
            result = p.execute([p.BINARY, 'trust', 'approve', 'TEST-DATA'], cwd=ROOT, env={}, interactive=True)
        self.assertEqual(result.returncode, 3)
        self.assertFalse(run.call_args.kwargs['capture_output'])
        for key in ['input', 'stdin', 'stdout']:
            self.assertNotIn(key, run.call_args.kwargs)
        self.reject(lambda r:r['journeys'][0].update(command_count=0))
        self.reject(lambda r:r['journeys'][0]['stages'][5].update(completed=False, exit_code=3))

    def test_real_transport_validation_rejects_substitutions(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(p, 'show'):
            for data, code in [({}, 0), (dict(verdict='PASS'), 0), (self.r, 0),
                               (dict(product='behavior', operation='doctor', verdict='PASS'), 0),
                               (dict(product='behavior', operation='verify', verdict='PASS'), 0),
                               (dict(product='behavior', operation='verify', verdict='PASS'), 1),
                               (dict(product='behavior', operation='verify', verdict='FAIL'), 0),
                               (dict(product='behavior', operation='verify', verdict='INCONCLUSIVE'), 0),
                               (dict(product='behavior', operation='verify', verdict='ERROR'), 0)]:
                with self.assertRaises(ValueError):
                    p.check_agent(data, code, Path(tmp), {}, 'behavior')

    def test_complete_cli_must_reinspect_github(self):
        with mock.patch.object(sys, 'argv', ['external-pilot', 'validate', 'TEST-DATA', '--complete']), \
             mock.patch.object(p, 'load', return_value=self.r), mock.patch.object(p, 'commit', return_value=SHA), \
             mock.patch.object(ci, 'recheck', side_effect=ValueError('TEST DATA unavailable')) as recheck:
            with self.assertRaisesRegex(ValueError, 'unavailable'): p.main()
        recheck.assert_called_once()

    def test_ci_partial_observation_and_inspection_are_preserved(self):
        # In-memory validator/collector unit test only; no network, files or pilot run.
        self.r['ci'] = []
        attempt = self.r['integration_attempts'][1]
        attempt.update(status='incomplete', friction=['ci'])
        first = copy.deepcopy(synthetic()['ci'][0])
        first['no_private_artifacts'] = False
        snapshots = []
        def snapshot():
            self.valid(complete=False)
            snapshots.append(copy.deepcopy(self.r))
        for inspection_answer in ['yes', 'no']:
            self.r['ci'] = []
            snapshots.clear()
            inputs = ['pilot-owner/pilot-repository', '.github/workflows/b2ige-verify.yml', 'sha256:' + '1' * 64, '1', '1', '2', '2']
            with mock.patch('builtins.input', side_effect=inputs), mock.patch('builtins.print'), \
                 mock.patch.object(p, 'choose', side_effect=['yes', 'yes', 'none', inspection_answer]), \
                 mock.patch.object(ci, 'observe', side_effect=[copy.deepcopy(first), ValueError('TEST DATA second unavailable')]):
                with self.assertRaises(ValueError):
                    ci.collect_ci(SHA, p.BINARY, p, self.r['ci'], attempt, snapshot)
            self.assertEqual(len(self.r['ci']), 1)
            self.assertFalse(snapshots[0]['ci'][0]['no_private_artifacts'])
            self.assertEqual(self.r['ci'][0]['no_private_artifacts'], inspection_answer == 'yes')
            self.assertIn('EVIDENCE_PENDING', p.report([self.r], expected_commit=SHA))

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
        responses, inspected = ci.inspect_archive(zipped(data), p.BINARY, p)
        self.assertEqual([v['verdict'] for v in responses], ['ERROR'])
        self.assertFalse(responses[0]['source_backed'])
        self.assertEqual(inspected['file_count'], 1)
        for mutation in [lambda v:v.update(operation='doctor'), lambda v:v.update(extra=True),
                         lambda v:v.update(summary='person@example.invalid')]:
            altered = copy.deepcopy(data)
            mutation(altered)
            with self.assertRaises(ValueError): ci.inspect_archive(zipped(altered), p.BINARY, p)
        for private in ['Bearer TESTSECRET', 'AKIA' + 'ABCDEFGHIJKLMNOP', 'SECRET_KEY=TESTSECRET',
                        '/opt/private/controller', '/var/run/docker.sock', '192.0.2.1',
                        '-----BEGIN OPENSSH ' + 'PRIVATE KEY-----']:
            altered = dict(data, summary=private)
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
            'repos/' + repo + '/actions/runs/1': dict(id=1, run_attempt=1, event='pull_request_target', status='completed', repository={'full_name': repo}, path=workflow, head_sha=base, pull_requests=[dict(number=1, base={'sha': base}, head={'sha': head})], conclusion='failure', html_url=url + '/actions/runs/1'),
            'repos/' + repo + '/contents/' + workflow + '?ref=' + base: dict(encoding='base64', content=base64.b64encode(body).decode()),
            'repos/' + repo + '/actions/runs/1/jobs?per_page=100': dict(total_count=1, jobs=[dict(name='verify', id=10, run_id=1, run_attempt=1, conclusion='failure', steps=[dict(name='Verify all registered contracts (only PASS is green)', status='completed', conclusion='failure')])]),
            'repos/' + repo + '/actions/runs/1/artifacts?per_page=100': dict(total_count=1, artifacts=[dict(id=20, workflow_run=dict(id=1, head_sha=base), expired=False, name='b2ige-sanitized-agent-reports', size_in_bytes=2)]),
            'repos/' + repo + '/actions/artifacts/20/zip': b'TEST DATA'
        }
        def observe(values):
            return ci.observe(repo, 1, 1, SHA, workflow, p.bench.digest(body), p.BINARY, p, lambda endpoint, **kwargs: copy.deepcopy(values[endpoint]))
        inspection = self.r['ci'][1]['artifacts'][0]
        with mock.patch.object(ci, 'inspect_archive', return_value=([self.r['ci'][1]['response']], inspection)):
            observed = observe(responses)
            observed['assistance'] = 'none'
            p.ci_fields(observed, SHA)
            self.assertEqual(observed['head_sha'], head)
            self.assertEqual(observed['base_sha'], base)
            for endpoint, mutation in [
                ('/actions/runs/1', lambda v:v.update(pull_requests=[])),
                ('/actions/runs/1', lambda v:v.update(id=2)),
                ('/actions/runs/1', lambda v:v.update(run_attempt=2)),
                ('/actions/runs/1', lambda v:v.update(head_sha=head)),
                ('/actions/runs/1', lambda v:v.update(conclusion='success')),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v.update(total_count=2)),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v['artifacts'][0].update(expired=True)),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v['artifacts'][0]['workflow_run'].update(id=2)),
                ('/actions/runs/1/artifacts?per_page=100', lambda v:v['artifacts'][0]['workflow_run'].update(head_sha=head)),
                ('/actions/runs/1/jobs?per_page=100', lambda v:v['jobs'][0].update(run_id=2)),
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
