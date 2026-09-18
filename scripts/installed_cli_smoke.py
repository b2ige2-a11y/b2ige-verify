"""Bounded installed command checks; readiness and help are not product verdicts."""
import json
import pathlib
import subprocess


def check_cli(binary, version, work, env=None):
    binary = pathlib.Path(binary)
    assert binary.is_absolute() and binary.is_file(), 'installed CLI missing; fallback forbidden'

    def run(*args, expected=0):
        result = subprocess.run([str(binary), *args], cwd=work, env=env,
                                stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=60)
        assert result.returncode == expected, (args, result.returncode, result.stderr)
        return result

    assert run('--version').stdout.strip() == f'verify-cli {version}', 'installed CLI version mismatch'
    help_text = run('--help').stdout
    for surface in ['inspect', 'init', 'prepare', 'trust approve', 'doctor', 'verify ID', 'ci init',
                    'behavior', 'sideeffect', 'blindtest', 'report', 'bench']:
        assert surface in help_text, surface
    assert 'verification_performed: false' in run('inspect', str(work)).stdout
    for args in [('prepare', '--help'), ('trust', 'approve', '--help'), ('verify', '--help')]:
        assert 'b2ige prepare --product' in run(*args).stdout
    for args in [('bench', '--help'), ('ci', 'init', '--help')]:
        run(*args)
    run('init', '--dry-run')
    registry = pathlib.Path(work) / '.b2ige/project.json'
    assert not registry.exists(), 'dry-run wrote registry'
    run('init')
    before = registry.read_bytes()
    run('init')
    assert registry.read_bytes() == before, 'init changed existing registry'
    approval = run('trust', 'approve', 'missing-draft', expected=3)
    assert 'non-interactive approval refused' in approval.stderr
    assert registry.read_bytes() == before
    doctor = json.loads(run('doctor', expected=3).stdout)
    assert doctor['kind'] == 'readiness' and doctor['verification_performed'] is False
    assert doctor['ready'] is False and 'verdict' not in doctor
    missing = run('verify', 'missing', '--output', 'agent', '--protocol', '1', expected=3)
    assert 'ERROR: registered identity unavailable; no product can be routed' in missing.stderr
    check = json.loads(run('ci-check', 'missing-agent.json', '0', expected=3).stdout)
    assert check['verdict'] == 'ERROR'
