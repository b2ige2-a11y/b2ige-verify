#!/usr/bin/env python3
"""Bounded synthetic tests of privileged CI admission and sanitized artifact staging."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import pathlib
import shlex
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
BINARY = pathlib.Path(os.environ.get('B2IGE_TEST_BINARY', ROOT / 'target/release/b2ige')).resolve()
spec = importlib.util.spec_from_file_location('controller_diff_verify', ROOT / 'scripts/diff-verify.py')
diff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diff)


def payload(verdict='PASS', product='blindtest'):
    return {'protocol_version': '1', 'product': product, 'operation': 'verify',
            'verdict': verdict, 'kind': 'synthetic_transport', 'summary': 'Transport fixture only',
            'expected': None, 'observed': None, 'reproduction': None, 'evidence_refs': [],
            'source': {'artifact_id': 'synthetic', 'integrity_hash': 'sha256:' + 'a' * 64},
            'scope': {'description': 'transport only', 'coverage': {}, 'budget': {}},
            'limitations': ['Not product verification evidence'], 'next_action': 'review',
            'other_failure_count': 0}


def sideeffect_config(executable):
    empty = {'snapshot_version': '1', 'timestamps': 'unix_epoch',
             'entries': {'': {'mode': 448, 'content_hash': None}}}
    snapshot = 'sha256:' + hashlib.sha256(json.dumps(empty, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return {'schema_version': '1', 'contract_id': 'synthetic',
            'operation': {'operation_id': 'op', 'idempotency_identity': 'key', 'correlation_identity': 'cor'},
            'trigger': {'executable': str(executable), 'executable_hash': diff.file_identity(executable),
                        'args': [], 'environment': {}, 'fixture': {'source': None, 'snapshot_identity': snapshot},
                        'timeout_ms': 1000},
            'effects': [{'effect_id': 'effect', 'provider': 'synthetic', 'adapter': 'sqlite', 'operation': 'op',
                         'identity': {'external_identity': 'provider_operation_external_id',
                                      'idempotency_identity': 'key', 'correlation_identity': 'cor'},
                         'expectation': 'exactly_once', 'authoritative_observer': 'ledger'}],
            'relations': [], 'fault_schedules': [{'schedule_id': 'single', 'primitives': ['NONE']}],
            'exploration_budget': {'max_schedules': 1, 'max_attempts': 1, 'reduction_executions': 0},
            'required_observers': [{'observer_id': 'ledger', 'db_path': 'ledger.sqlite', 'table': 'effects',
                                    'external_effect_id_column': 'external', 'idempotency_column': 'idem',
                                    'correlation_column': 'correlation', 'operation_column': 'operation',
                                    'commit_order_column': 'ordering',
                                    'authoritative_source': 'durable_append_only_committed_state'}]}


class ControllerTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix='b2ige-ci-control-')
        self.addCleanup(temp.cleanup)
        self.root = pathlib.Path(temp.name).resolve()

    def test_approved_native_target_blocked_before_execution_and_upload(self):
        if os.name != 'posix':
            self.skipTest('native local runtime is Unix only')
        project = self.root / 'project'
        project.mkdir()
        (project / '.b2ige').mkdir()
        marker = self.root / 'host-write'
        reports = project / 'b2ige-agent-reports'
        executable = self.root / 'candidate'
        executable.write_text('#!/bin/sh\nprintf synthetic > ' + shlex.quote(str(marker)) +
                              '\nprintf synthetic-private-material > ' + shlex.quote(str(reports / 'raw.json')) + '\n')
        executable.chmod(0o700)
        config = project / 'reviewed.json'
        config.write_text(json.dumps(sideeffect_config(executable)))
        registry = {'schema_version': '1', 'entries': {'synthetic': {
            'product': 'sideeffect', 'config': 'reviewed.json', 'store': str(self.root / 'store'),
            'authorization': None}}}
        (project / '.b2ige/project.json').write_text(json.dumps(registry))
        def git(*args):
            return subprocess.check_output(['git', '-C', str(project), *args], text=True).strip()
        git('init', '-q')
        git('config', 'user.name', 'Synthetic Test')
        git('config', 'user.email', 'synthetic@example.invalid')
        git('add', '.')
        git('commit', '-qm', 'trusted fixture')
        base = git('rev-parse', 'HEAD')
        (project / 'scripts').mkdir()
        (project / 'scripts/diff-verify.py').write_text('raise RuntimeError("candidate adapter executed")')
        (project / 'build.rs').write_text('compile_error!("candidate build executed");')
        (project / 'package.json').write_text('{"scripts":{"postinstall":"exit 99"}}')
        (project / '.gitattributes').write_text('* diff=synthetic filter=synthetic\n')
        git('add', '.')
        git('commit', '-qm', 'candidate executable surfaces')
        head = git('rev-parse', 'HEAD')
        git('checkout', '-q', base)
        with mock.patch.object(diff, 'ROOT', project):
            paths = diff.changed_files(base, head)
        self.assertIn('build.rs', paths)
        self.assertIn('scripts/diff-verify.py', paths)
        self.assertFalse(marker.exists())
        approval = self.root / 'approval.json'
        approval.write_text(json.dumps({'schema_version': '1', 'head': head, 'contracts': {
            'synthetic': {'config_sha256': diff.file_identity(config), 'authorization_sha256': None,
                          'target_identity': diff.file_identity(executable)}}}))
        command = [sys.executable, str(ROOT / 'scripts/diff-verify.py'), '--project-root', str(project),
                   '--base', base, '--head', head, '--trusted-revision', base,
                   '--candidate-approval', str(approval), '--b2ige', str(BINARY)]
        output = self.root / 'github-output'
        artifacts = self.root / 'artifacts'
        guarded = subprocess.run([*command, '--trusted-controller', '--artifact-dir', str(artifacts)],
                                 capture_output=True, text=True, timeout=20,
                                 env={**os.environ, 'GITHUB_OUTPUT': str(output)})
        self.assertEqual(guarded.returncode, 3, guarded.stderr)
        self.assertFalse(json.loads(guarded.stdout)['gate_pass'])
        self.assertIn('reviewed isolated runtime', guarded.stderr)
        self.assertFalse(marker.exists())
        self.assertFalse(reports.exists())
        self.assertFalse(artifacts.exists())
        self.assertFalse(output.exists())
        # Local behavior deliberately remains compatible. This bounded marker also
        # reproduces why hash approval alone cannot safely admit a native CI target.
        local = subprocess.run(command, capture_output=True, text=True, timeout=20)
        self.assertNotEqual(local.returncode, 0)  # no committed SQLite evidence
        self.assertTrue(marker.exists(), local.stdout + local.stderr)
        self.assertTrue((reports / 'raw.json').exists())
        self.assertFalse(artifacts.exists())

    def test_isolation_guard_covers_all_entries_and_requires_exact_level(self):
        config = self.root / 'config.json'
        entry = {'product': 'blindtest', 'config': str(config), 'store': 'store', 'authorization': None}
        for level in [None, 'NONE', 'WORKSPACE_SEPARATION', 'HARDENED_LINUX', 'container_isolation']:
            config.write_text(json.dumps({'required_isolation': level}))
            with self.assertRaises(ValueError):
                diff.check_controller_isolation({'contract': entry})
        config.write_text(json.dumps({'required_isolation': 'DOCKER_ISOLATION'}))
        diff.check_controller_isolation({'contract': entry})
        for product in ['behavior', 'sideeffect']:
            with self.assertRaises(ValueError):
                diff.check_controller_isolation({'safe': entry, 'native': dict(entry, product=product)})

    def test_option_like_identity_and_repository_remain_literal_preview_values(self):
        (self.root / '.b2ige').mkdir()
        registry = self.root / '.b2ige/project.json'
        for identity in ['--write', '--', 'true', '__PIN__']:
            registry.write_text(json.dumps({'schema_version': '1', 'entries': {identity: {
                'product': 'sideeffect', 'config': 'reviewed.json', 'store': 'runs', 'authorization': None}}}))
            command = [str(BINARY), 'ci', 'init', '--identity', identity,
                       '--provider', 'github-actions', '--registry', str(registry),
                       '--verifier-repo', '-owner/-repo', '--verifier-ref', '1' * 40, '--root', str(self.root)]
            preview = subprocess.run(command, capture_output=True, text=True, timeout=20)
            self.assertEqual(preview.returncode, 0, preview.stderr)
            self.assertIn('Mode: preview (no writes)', preview.stdout)
            self.assertFalse((self.root / '.github').exists())
            # The generated command's custom parser consumes option-like values
            # as data; the identity must never activate --write.
            generated = next(line.strip() for line in preview.stdout.splitlines() if ' ci init --identity ' in line)
            argv = shlex.split(generated)
            argv[0] = str(BINARY)
            argv[-1] = str(self.root)
            called = subprocess.run(argv, cwd=self.root, capture_output=True, text=True, timeout=20)
            self.assertEqual(called.returncode, 0, called.stderr)
            self.assertIn('Mode: preview (no writes)', called.stdout)
            self.assertFalse((self.root / '.github').exists())

    def test_upload_is_allowlisted_serialization_not_report_directory_glob(self):
        reports = self.root / 'reports'
        reports.mkdir()
        expected = payload('FAIL')
        checked = reports / 'contract.json'
        checked.write_text(json.dumps(expected))
        validated = diff.validate_report(checked, 'blindtest', 1)
        self.assertIsNotNone(validated)
        (reports / 'unrelated.json').write_text('synthetic private raw data')
        (reports / 'symlink.json').symlink_to(reports / 'unrelated.json')
        self.assertIsNone(diff.validate_report(reports / 'symlink.json', 'blindtest', 0))
        checked.write_text('replaced after parsing; must never be copied')
        output = self.root / 'github-output'
        artifacts = self.root / 'artifacts'
        with mock.patch.dict(os.environ, {'GITHUB_OUTPUT': str(output)}):
            diff.publish_reports(artifacts, [('contract', validated)])
        self.assertEqual([p.name for p in artifacts.iterdir()], ['contract.json'])
        self.assertEqual(json.loads((artifacts / 'contract.json').read_text()), expected)
        self.assertEqual(output.read_text(), 'artifacts_ready=true\n')
        output.unlink()
        with mock.patch.dict(os.environ, {'GITHUB_OUTPUT': str(output)}), self.assertRaises(OSError):
            diff.publish_reports(artifacts, [('contract', validated)])
        self.assertFalse(output.exists())
        link = self.root / 'link'
        link.symlink_to(artifacts, target_is_directory=True)
        with self.assertRaises(ValueError):
            diff.publish_reports(link / 'nested', [('contract', validated)])

    def test_transport_exit_matrix_and_missing_malformed_readiness(self):
        raw = self.root / 'raw.json'
        for verdict, code in [('PASS', 0), ('FAIL', 1), ('INCONCLUSIVE', 2), ('ERROR', 3)]:
            raw.write_text(json.dumps(payload(verdict)))
            for reported in [0, 1, 2, 3, 64]:
                with self.subTest(verdict=verdict, reported=reported):
                    checked = subprocess.run([str(BINARY), 'ci-check', str(raw), str(reported)], capture_output=True)
                    self.assertEqual(checked.returncode, code if code == reported else 3)
                    self.assertEqual(json.loads(checked.stdout)['verdict'], verdict if code == reported else 'ERROR')
        for value in ['not JSON', '{}', json.dumps(dict(payload(), kind='readiness'))]:
            raw.write_text(value)
            self.assertEqual(subprocess.run([str(BINARY), 'ci-check', str(raw), '0'], capture_output=True).returncode, 3)
        raw.unlink()
        self.assertEqual(subprocess.run([str(BINARY), 'ci-check', str(raw), '0'], capture_output=True).returncode, 3)

    def test_empty_selection_and_stale_reports_cannot_execute_or_upload(self):
        config = self.root / 'config.json'
        config.write_text(json.dumps({'required_isolation': 'DOCKER_ISOLATION',
                                      'target': {'image': 'sha256:' + 'a' * 64}}))
        entries = {'contract': {'product': 'blindtest', 'config': str(config), 'store': 'store', 'authorization': None}}
        approval = self.root / 'approval.json'
        approval.write_text(json.dumps({'schema_version': '1', 'head': 'b' * 40, 'contracts': {
            'contract': {'config_sha256': diff.file_identity(config), 'authorization_sha256': None,
                         'target_identity': 'sha256:' + 'a' * 64}}}))
        report_dir = self.root / 'reports'
        report_dir.mkdir()
        (report_dir / 'stale.json').write_text(json.dumps(payload()))
        for selection in [[], ['contract']]:
            argv = ['diff-verify', '--base', 'a' * 40, '--head', 'b' * 40, '--trusted-revision', 'a' * 40,
                    '--candidate-approval', str(approval), '--report-dir', str(report_dir),
                    '--trusted-controller', '--artifact-dir', str(self.root / 'artifacts')]
            with mock.patch.object(diff, 'checked_entries', return_value=entries), \
                    mock.patch.object(diff, 'trusted_revision', side_effect=lambda value, label: value), \
                    mock.patch.object(diff, 'changed_files', return_value=['src/change']), \
                    mock.patch.object(diff, 'select', return_value=(selection, 'synthetic')), \
                    mock.patch.object(diff.subprocess, 'run') as execute, \
                    mock.patch.object(sys, 'argv', argv), \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(diff.main(), 3)
            execute.assert_not_called()
            self.assertFalse((self.root / 'artifacts').exists())


if __name__ == '__main__':
    unittest.main()
