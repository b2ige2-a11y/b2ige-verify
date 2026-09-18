#!/usr/bin/env python3
"""Test tooling admission, contamination controls and two actual fresh journeys."""
import copy
import json
import subprocess
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import adoption_bench as b


class HarnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = b.load_corpus()
        cls.temp = tempfile.TemporaryDirectory(prefix='adoption-self-test-')
        cls.root = Path(cls.temp.name).resolve()
        cls.before = b.p8_inventory()
        cls.first = b.execute(cls.root / 'first')
        cls.second = b.execute(cls.root / 'second', reverse=True)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def bad_result(self, change):
        value = copy.deepcopy(self.first)
        change(value)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            b.validate_result(value)

    def test_clean_complete_fresh_and_repeat_semantics(self):
        self.assertTrue(b.accepted(self.first), json.dumps(self.first['aggregate']))
        self.assertTrue(b.accepted(self.second), json.dumps(self.second['aggregate']))
        self.assertEqual(b.semantic(self.first), b.semantic(self.second))
        self.assertEqual(b.report(self.first), b.report(self.second))
        self.assertEqual(self.before, b.p8_inventory())
        for r in self.first['scenarios']:
            self.assertEqual(r['stages'][0]['stage'], 'project')
            self.assertEqual(r['stages'][-1]['stage'], 'report')
            self.assertTrue(r['controller_separated'])
            self.assertTrue(r['agent_output_validated'])
            self.assertEqual(r['trust_checkpoints'], 1)

    def test_human_report_does_not_depend_on_json_key_order(self):
        changed = copy.deepcopy(self.first)
        for key in ['aggregate', 'negative_controls']:
            changed[key] = dict(reversed(list(changed[key].items())))
        self.assertEqual(b.report(self.first), b.report(changed))

    def test_inventory_and_source_binding(self):
        rows = self.manifest['scenarios']
        self.assertGreaterEqual(len(rows), 9)
        self.assertEqual(len(rows), len({r['scenario_id'] for r in rows}))
        self.assertEqual({r['product'] for r in rows}, {'behavior', 'sideeffect', 'blindtest'})
        self.assertTrue({'Rust', 'Node.js', 'Python'}.issubset({r['runtime'] for r in rows}))
        authority = (b.ROOT / self.manifest['authority_source_path']).read_bytes()
        for mutation in [lambda m: m['scenarios'][0].update(authority_source_case_id='unknown'),
                         lambda m: m['scenarios'][0].update(product='blindtest'),
                         lambda m: m['scenarios'][0].update(expected_classification='BUGGY'),
                         lambda m: m['scenarios'][0].update(allowed_verdicts=['FAIL']),
                         lambda m: m['scenarios'].append(m['scenarios'][0]),
                         lambda m: m['scenarios'].pop()]:
            bad = copy.deepcopy(self.manifest)
            mutation(bad)
            with self.assertRaises(ValueError): b.validate_authority(bad, authority)
        with self.assertRaises(ValueError): b.validate_authority(self.manifest, authority + b' ')
        cases = b.decode(authority)
        cases.append(cases[0])
        data = json.dumps(cases).encode()
        bad = copy.deepcopy(self.manifest)
        bad['authority_source_identity'] = b.digest(data)
        with self.assertRaisesRegex(ValueError, 'duplicate P8'): b.validate_authority(bad, data)

    def test_zero_denominator_and_incomplete_denominator(self):
        self.assertEqual(b.rate(0, 0), {'numerator': 0, 'denominator': 0, 'fraction': None})
        rows = copy.deepcopy(self.first['scenarios'])
        rows[0] = b.blank_row(self.manifest['scenarios'][0])
        metrics = b.aggregate(rows, self.manifest, self.first['negative_controls'])
        self.assertEqual(metrics['completed']['denominator'], len(rows))
        self.assertEqual(metrics['completed']['numerator'], len(rows) - 1)
        bad = copy.deepcopy(self.first)
        bad['scenarios'] = rows
        bad['aggregate'] = metrics
        self.assertFalse(b.accepted(bad))

    def test_version_product_outcome_and_unknown_fields(self):
        for change in [lambda r: r.update(schema_version='1'),
                       lambda r: r.update(authoritative=True),
                       lambda r: r.update(kind='product-evidence'),
                       lambda r: r.update(extra='ignored'),
                       lambda r: r['scenarios'][0].update(product='auto'),
                       lambda r: r['scenarios'][0].update(actual_verdict='READY')]:
            self.bad_result(change)

    def test_missing_duplicate_scenario_expected_and_denominator_tampering(self):
        for change in [lambda r: r['scenarios'].pop(),
                       lambda r: r['scenarios'].append(r['scenarios'][0]),
                       lambda r: r['scenarios'].reverse(),
                       lambda r: r['scenarios'][0].update(allowed_verdicts=['FAIL']),
                       lambda r: r['aggregate']['completed'].update(denominator=1)]:
            self.bad_result(change)

    def test_doctor_never_becomes_verification(self):
        def change(r):
            row = r['scenarios'][0]
            row['stages'] = row['stages'][:7]
            row['completed'] = False
            row['verification_verdict'] = None
        self.bad_result(change)

    def test_forged_completion_requires_successful_stages_and_verified_evidence(self):
        for change in [lambda r: r['scenarios'][0]['stages'][6].update(exit_code=3),
                       lambda r: r['scenarios'][0]['verification_coverage'].update(verified_evidence_count=0),
                       lambda r: r['scenarios'][0].update(verification_verdict=None, first_verification_success=False)]:
            self.bad_result(change)

    def test_verdict_exit_zero_coercions(self):
        for verdict in ['FAIL', 'INCONCLUSIVE', 'ERROR']:
            index = next(i for i, r in enumerate(self.first['scenarios']) if r['actual_verdict'] == verdict)
            self.bad_result(lambda r: r['scenarios'][index].update(exit_code=0))
            self.assertTrue(self.first['negative_controls'][verdict + '-exit-zero'])

    def test_stale_output_refused_and_never_consumed(self):
        previous = self.root / 'stale'
        previous.mkdir()
        (previous / 'result.json').write_text('{corrupted previous success')
        with mock.patch.object(b, 'run') as run:
            with self.assertRaises(FileExistsError): b.execute(previous)
            run.assert_not_called()
        self.assertEqual((previous / 'result.json').read_text(), '{corrupted previous success')
        with self.assertRaises(ValueError): b.read(previous / 'result.json')

    def test_all_negative_controls_are_exercised(self):
        self.assertEqual(set(self.first['negative_controls']), set(b.CONTROLS))
        for name, ok in self.first['negative_controls'].items():
            with self.subTest(control=name): self.assertTrue(ok)
        self.bad_result(lambda r: r['negative_controls'].pop(b.CONTROLS[0]))

    def test_semantic_comparison_preserves_meaningful_fields(self):
        timing = copy.deepcopy(self.first)
        timing['observations']['scenario_seconds'] = {s['scenario_id']: {} for s in self.manifest['scenarios']}
        timing['observations']['preparation']['seconds'] += 999
        self.assertEqual(b.semantic(self.first), b.semantic(timing))
        for field, value in [('evidence_status', 'loader_rejected'), ('verification_verdict', 'FAIL')]:
            changed = copy.deepcopy(self.first)
            changed['scenarios'][0][field] = value
            with self.assertRaises(ValueError): b.semantic(changed)
        coverage = copy.deepcopy(self.first)
        coverage['scenarios'][0]['verification_coverage']['verified_evidence_count'] += 1
        self.assertNotEqual(b.semantic(self.first), b.semantic(coverage))
        changed = copy.deepcopy(self.first)
        changed['negative_controls']['stale-output'] = False
        changed['aggregate'] = b.aggregate(changed['scenarios'], self.manifest, changed['negative_controls'])
        self.assertNotEqual(b.semantic(self.first), b.semantic(changed))
        self.assertFalse(b.accepted(changed))

    def test_p8_files_unmodified_and_public_output_has_no_private_values(self):
        self.assertEqual(self.before, self.first['p8_before'])
        self.assertEqual(self.before, self.first['p8_after'])
        text = json.dumps(self.first)
        for value in [str(self.root), str(Path.home()), 'BLINDTEST_PRIVATE_', 'private_canary']:
            self.assertNotIn(value, text)
        self.bad_result(lambda r: r['scenarios'][0].update(private_path='/private/value'))

    def test_duplicate_json_and_symlink_paths(self):
        with self.assertRaises(ValueError): b.decode('{"schema_version":"1","schema_version":"1"}')
        target = self.root / 'link-target'
        target.mkdir()
        link = self.root / 'link'
        link.symlink_to(target, target_is_directory=True)
        with self.assertRaises(ValueError): b.execute(link / 'output')
        with self.assertRaises(ValueError): b.lanes(self.root, self.root / 'nested')

    def test_verify_report_and_corrupt_outcome_are_distinct_and_bound(self):
        corrupt = next(r for r in self.first['scenarios'] if r['scenario_id'] == 'sqlite-corrupt')
        self.assertEqual((corrupt['verification_verdict'], corrupt['verification_exit_code']), ('PASS', 0))
        self.assertEqual((corrupt['report_verdict'], corrupt['report_exit_code']), ('ERROR', 3))
        self.assertEqual(corrupt['measured_outcome_stage'], 'report')
        self.assertTrue(corrupt['first_verification_success'])
        for change in [lambda r: r['scenarios'][5].update(evidence_status='verified'),
                       lambda r: r['scenarios'][5].update(measured_outcome_stage='verify'),
                       lambda r: r['scenarios'][0].update(report_verdict='FAIL'),
                       lambda r: r['scenarios'][0].update(report_exit_code=False),
                       lambda r: r['scenarios'][0].update(verification_exit_code=1)]:
            self.bad_result(change)
        def inconsistent(r):
            r['scenarios'][0].update(verification_verdict='FAIL', verification_exit_code=1)
            r['scenarios'][0]['stages'][7]['exit_code'] = 1
        self.bad_result(inconsistent)

    def test_public_observations_reject_private_text_and_nonfinite_numbers(self):
        for change in [lambda r: r['observations']['environment']['versions'].update(node='private-token'),
                       lambda r: r['observations']['environment'].update(release='/controller/private'),
                       lambda r: r['observations']['preparation'].update(binary_identity='/controller/private'),
                       lambda r: r['observations']['preparation'].update(seconds=float('nan')),
                       lambda r: r['observations']['scenario_seconds']['rust-preserving'].update(project=float('inf'))]:
            self.bad_result(change)
        for value in ['NaN', 'Infinity', '-Infinity']:
            with self.assertRaises(ValueError): b.decode(value)

    def test_protected_output_refused_before_any_write(self):
        for protected in [b.ROOT / 'benchmarks/corpus', b.ROOT / 'benchmarks/baseline-v1', b.CORPUS]:
            output = protected / 'never-create-audit-output'
            with mock.patch.object(b, 'run') as run:
                with self.assertRaises(ValueError): b.execute(output)
                run.assert_not_called()
            self.assertFalse(output.exists())

    def test_crashes_and_failed_controls_remain_in_full_denominator(self):
        with mock.patch.object(b.Journey, 'project_setup', side_effect=RuntimeError('private crash text')):
            result = b.execute(self.root / 'crashed')
        self.assertEqual(result['aggregate']['completed'], b.rate(0, 9))
        self.assertEqual(result['aggregate']['first_verification'], b.rate(0, 9))
        self.assertEqual(result['aggregate']['classification_agreement'], b.rate(0, 9))
        self.assertEqual(result['aggregate']['unsafe_shortcuts_rejected']['denominator'], len(b.CONTROLS))
        self.assertFalse(b.accepted(result))
        self.assertEqual(len(b.read(self.root / 'crashed/result.json')['scenarios']), 9)
        self.assertNotIn('private crash text', json.dumps(result))
        compared = subprocess.run(['python3', str(b.ROOT / 'scripts/adoption-bench.py'), '--compare',
                                   str(self.root / 'crashed/result.json'), str(self.root / 'crashed/result.json')], capture_output=True)
        self.assertEqual(compared.returncode, 3)
        self.assertEqual(b.decode(compared.stdout), {'semantic_equal': True, 'both_runs_accepted': False})
        controls = dict(self.first['negative_controls'])
        controls['extra-success'] = True
        with self.assertRaises(ValueError): b.aggregate(self.first['scenarios'], self.manifest, controls)

    def test_semantic_mutation_matrix(self):
        for change in [lambda r: r['scenarios'][0].update(actual_verdict='FAIL'),
                       lambda r: r['scenarios'][0].update(expected_classification='BUGGY'),
                       lambda r: r['scenarios'].pop(),
                       lambda r: r['scenarios'].append(r['scenarios'][0]),
                       lambda r: r['scenarios'][0].update(product='blindtest'),
                       lambda r: r['scenarios'][0]['stages'].pop(),
                       lambda r: r['aggregate']['completed'].update(denominator=8),
                       lambda r: r['scenarios'][0].update(runtime='Python')]:
            changed = copy.deepcopy(self.first)
            change(changed)
            with self.assertRaises(ValueError): b.semantic(changed)

    def test_terminal_replay_refuses_arbitrary_or_mutated_inputs_before_spawn(self):
        with b.benchmark_sandbox() as root:
            j = b.Journey(root / 'scenarios', self.manifest['scenarios'][0],
                          b.ROOT / 'target/release/b2ige', b.ROOT / 'target/release/examples/adoption_fixture', {})
            command = j.approval_command()
            pin_file = j.controller / 'reviewed-input'
            pin_file.write_text('fixed input')
            j.approval_replays['primary'] = (command, {pin_file: b.digest(pin_file.read_bytes())}, {})
            mutations = [lambda: command.__setitem__(0, '/bin/echo'),
                         lambda: command.__setitem__(3, root / 'arbitrary-draft'),
                         lambda: command.extend(['--yes']),
                         lambda: j.env.update(UNREVIEWED='value'),
                         lambda: pin_file.write_text('changed input'),
                         lambda: j.row['stages'].append({'stage': 'verify'}),
                         lambda: j.responses.update(verify={'verdict': 'PASS'}),
                         lambda: pin_file.unlink()]
            for mutate in mutations:
                with self.subTest(mutation=mutate), mock.patch.object(b.subprocess, 'Popen') as spawn:
                    command[:] = j.approval_command()
                    j.env.clear(); j.responses.clear(); j.row['stages'].clear()
                    pin_file.write_text('fixed input')
                    mutate()
                    with self.assertRaises((ValueError, OSError)): b._replay_fixture_confirmation(j)
                    spawn.assert_not_called()
        with mock.patch.object(b.subprocess, 'Popen') as spawn:
            with self.assertRaises(ValueError): b._replay_fixture_confirmation(j)
            spawn.assert_not_called()

    def test_malformed_error_agent_cannot_hide_in_validator_error_fallback(self):
        j = b.Journey(self.root, self.manifest['scenarios'][0],
                      b.ROOT / 'target/release/b2ige', b.ROOT / 'target/release/examples/adoption_fixture', {})
        # Real registered error response, then remove/extend fields without changing ERROR/3.
        raw = subprocess.run([str(j.binary), 'report', 'missing', '--store', str(j.store),
                              '--output', 'agent', '--protocol', '1'], capture_output=True)
        original = b.decode(raw.stdout)
        for mutate in [lambda r: r.pop('summary'), lambda r: r.update(unexpected=True)]:
            value = copy.deepcopy(original); mutate(value)
            response = subprocess.CompletedProcess([], 3, json.dumps(value).encode(), b'')
            with self.assertRaises(ValueError): j.validate_agent(response, 'report')
            (j.controller / 'agent-0.json').unlink()


if __name__ == '__main__':
    unittest.main()
