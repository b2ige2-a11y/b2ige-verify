#!/usr/bin/env python3
"""Execute the local V100 foundation gate; never grants publication or holdout proof."""
import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
from hygiene import ROOT, files

WAITING = 'WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT'


def commands():
    core = ['cargo', 'test', '--locked', '-p', 'verify-core']
    cli = ['cargo', 'test', '--locked', '-p', 'verify-cli']
    return [
        ['cargo', 'fmt', '--check'],
        *[core + ['--test', name] for name in
          ['conformance', 'task_seal', 'trust_lock', 'adversarial_query']],
        *[core + ['--test', name, 'sealed_'] for name in
          ['behavior_differential', 'sideeffect_proof']],
        cli + ['--lib', 'bench::tests'],
        cli + ['--test', 'agent_protocol'],
        cli + ['--test', 'benchmark', '--', '--skip', 'actual_docker_benchmark_corpus'],
        ['python3', 'scripts/test-release.py'],
        ['python3', 'scripts/test-v100-release.py'],
        ['python3', 'scripts/test-adoption.py'],
        ['python3', 'scripts/validate-workflows.py'],
        ['python3', 'scripts/hygiene.py'],
        ['cargo', 'clippy', '--locked', '-p', 'verify-core', '-p', 'verify-cli',
         '--all-targets', '--', '-D', 'warnings'],
        ['cargo', 'build', '--workspace', '--release', '--locked'],
    ]


def source_identity():
    # Same public inventory used by packaging; includes uncommitted source bytes.
    digest = hashlib.sha256()
    for path in sorted(files()):
        if path.is_symlink():
            raise ValueError('Symlink in source inventory')
        digest.update(path.relative_to(ROOT).as_posix().encode() + b'\0')
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def completed(command, code, output):
    if code != 0:
        return False
    if command[:2] == ['cargo', 'test']:
        counts = re.findall(rb'test result: ok\. ([0-9]+) passed;', output)
        return bool(counts) and sum(map(int, counts)) > 0
    return True


def status(rows, expected, before, after):
    # Exact inventory/order prevents an empty or partial run granting readiness.
    ready = (bool(expected) and [r['command'] for r in rows] == expected
             and all(r['completed'] is True for r in rows) and before == after)
    return WAITING if ready else 'BLOCKED_LOCAL'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output_directory', help='New controller directory outside the repository')
    args = parser.parse_args()
    output = pathlib.Path(args.output_directory).resolve()
    if output.is_relative_to(ROOT):
        parser.error('Keep gate logs outside the public repository')
    output.mkdir(mode=0o700)  # No reuse of historical/partial success.
    expected = commands()
    before = source_identity()
    report = {
        'schema_version': '1', 'scope': 'V100-5 local foundation only',
        'status': 'BLOCKED_LOCAL', 'source_sha256_before': before,
        'source_sha256_after': None, 'checks': [],
        'external_ci': 'NOT_RUN', 'private_holdout': 'NOT_ACCESSED',
        'publication_ready': False,
        'pending': ['authoritative full workspace gate', 'actual Docker qualification',
                    'fresh full fixed/reverse public benchmark', 'archive/SBOM/native smoke',
                    'external CI', 'independent audit and private holdout'],
    }

    def save():
        (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')

    save()
    env = dict(os.environ, CARGO_NET_OFFLINE='true', RUST_TEST_THREADS='4')
    for index, command in enumerate(expected):
        log = output / f'{index:02d}.log'
        print('Running:', ' '.join(command), flush=True)
        with log.open('xb') as stream:
            try:
                code = subprocess.run(command, cwd=ROOT, env=env, stdout=stream,
                                      stderr=subprocess.STDOUT, timeout=900).returncode
            except (OSError, subprocess.TimeoutExpired) as error:
                stream.write((type(error).__name__ + '\n').encode())
                code = None
        content = log.read_bytes()
        report['checks'].append({'command': command, 'exit_code': code,
                                 'completed': completed(command, code, content),
                                 'log': log.name, 'log_sha256': hashlib.sha256(content).hexdigest()})
        save()
        if not report['checks'][-1]['completed']:
            break
    report['source_sha256_after'] = source_identity()
    report['status'] = status(report['checks'], expected, before, report['source_sha256_after'])
    save()
    print(report['status'], flush=True)
    return 0 if report['status'] == WAITING else 1


if __name__ == '__main__':
    raise SystemExit(main())
