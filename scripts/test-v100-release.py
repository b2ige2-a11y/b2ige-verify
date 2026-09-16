#!/usr/bin/env python3
"""Negative controls for the local release decision, without running the full gate."""
import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location('gate', pathlib.Path(__file__).with_name('v100-release-gate.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class LocalGate(unittest.TestCase):
    def test_missing_failed_duplicate_reordered_and_changed_source_block(self):
        expected = gate.commands()
        rows = [{'command': c, 'completed': True} for c in expected]
        self.assertEqual(gate.status(rows, expected, 'a', 'a'), gate.WAITING)
        for bad in [[], rows[:-1], rows + [rows[0]], list(reversed(rows)),
                    [dict(r, completed=False) if i == 0 else r for i, r in enumerate(rows)]]:
            self.assertEqual(gate.status(bad, expected, 'a', 'a'), 'BLOCKED_LOCAL')
        self.assertEqual(gate.status(rows, expected, 'a', 'b'), 'BLOCKED_LOCAL')
        self.assertEqual(gate.status([], [], 'a', 'a'), 'BLOCKED_LOCAL')

    def test_zero_test_success_and_nonzero_exit_never_qualify(self):
        command = ['cargo', 'test']
        self.assertFalse(gate.completed(command, 0, b'test result: ok. 0 passed;'))
        self.assertFalse(gate.completed(command, 0, b''))
        self.assertFalse(gate.completed(command, 1, b'test result: ok. 4 passed;'))
        self.assertFalse(gate.completed(command, None, b'test result: ok. 4 passed;'))
        self.assertTrue(gate.completed(command, 0, b'test result: ok. 4 passed;'))


if __name__ == '__main__':
    unittest.main()
