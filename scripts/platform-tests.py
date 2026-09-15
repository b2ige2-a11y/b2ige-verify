#!/usr/bin/env python3
"""Explicit hosted-macOS subset. Full Docker suite remains mandatory on Linux."""
import argparse
import os
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--without-docker', action='store_true')
args = parser.parse_args()
env = dict(os.environ, RUST_TEST_THREADS='4')


def run(*args):
    subprocess.run(['cargo', 'test', '--locked', *args], env=env, check=True)


if not args.without_docker:
    run('--workspace')
else:
    run('--workspace', '--no-run')
    run('--workspace', '--lib', '--bins')
    run('--workspace', '--doc')
    for target in ['conformance', 'independent_gate', 'repair_regressions', 'process_acquisition',
                   'replay_execution', 'behavior_differential', 'sideeffect_proof']:
        run('-p', 'verify-core', '--test', target)
    for target in ['reports', 'sideeffect_reports', 'agent_protocol']:
        run('-p', 'verify-cli', '--test', target)
    run('-p', 'verify-cli', '--test', 'benchmark', '--', '--skip', 'actual_docker_benchmark_corpus')
    run('-p', 'verify-mcp', '--test', 'workflow', '--', '--skip', 'blindtest_actual_docker_all_verdicts_no_hidden_leakage')
    print('Explicit non-Docker subset passed. BlindTest integration suites NOT RUN; full gate NOT claimed.')
