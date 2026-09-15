#!/usr/bin/env python3
"""Execute fresh installed smoke before recording target runtime evidence."""
import argparse
import hashlib
import json
import pathlib
import platform
import subprocess
from hygiene import ROOT, files

parser = argparse.ArgumentParser()
parser.add_argument('--without-docker', action='store_true')
parser.add_argument('--skip-benchmark', action='store_true', help='Use only when verifier source/baseline preservation was checked')
args = parser.parse_args()
manifest_path = ROOT / 'release/release-manifest.json'
manifest = json.loads(manifest_path.read_text())
target = manifest['built_target']
host = subprocess.check_output(['rustc', '-vV'], text=True)
assert f'host: {target}\n' in host, 'Cross build cannot grant native platform validation'
machine = platform.machine().lower()
assert machine in ({'arm64', 'aarch64'} if target.startswith('aarch64') else {'x86_64', 'amd64'}), 'Runtime architecture mismatch'
if platform.system() == 'Darwin':
    translated = subprocess.run(['sysctl', '-in', 'sysctl.proc_translated'], capture_output=True, text=True)
    assert translated.stdout.strip() != '1', 'Rosetta cannot grant native verification'
elif platform.system() != 'Linux':
    raise SystemExit('Unsupported native runtime host')
inventory = hashlib.sha256(b''.join(str(p.relative_to(ROOT)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest()
                          for p in sorted(files()) if p != manifest_path)).hexdigest()
assert inventory == manifest['source_inventory_sha256'], 'Source changed after packaging; rebuild first'
subprocess.run(['python3', 'scripts/validate-archive.py', 'release/artifacts'], cwd=ROOT, check=True)
archive = ROOT / 'release/artifacts' / f'b2ige-{manifest["version"]}-{target}.tar.gz'
command = ['python3', 'scripts/fresh-smoke.py', str(archive)]
if args.without_docker: command.append('--without-docker')
if args.skip_benchmark: command.append('--skip-benchmark')
subprocess.run(command, cwd=ROOT, check=True)
subprocess.run(['python3', 'scripts/npm-archive-smoke.py'], cwd=ROOT, check=True)
# Refuse to stamp the current checkout if it changed while the gate was running.
current_inventory = hashlib.sha256(b''.join(str(p.relative_to(ROOT)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest()
                                  for p in sorted(files()) if p != manifest_path)).hexdigest()
assert current_inventory == inventory, 'Source changed during smoke; rebuild before recording readiness'
manifest['platform_validation'][target] = 'VERIFIED_NATIVE'
manifest['platform_validation_scope'][target] = ('Native CLI/init/doctor/Behavior/SideEffect/reports/MCP; Docker NOT RUN' if args.without_docker else 'Native CLI/init/doctor/Behavior/SideEffect/BlindTest/reports/MCP; Docker actual engine') + ('; full benchmark NOT RERUN, unchanged baseline retained' if args.skip_benchmark else '; benchmark executed')
manifest['actually_verified_platforms'] = [target]
manifest['local_release_candidate_ready'] = not args.without_docker
manifest['known_limitations'][0] = ('Fresh native non-Docker gate passed; BlindTest and full benchmark NOT RUN'
                                    if args.without_docker else 'Fresh native full smoke passed for built_target only; other targets require native CI')
data = json.dumps(manifest, indent=2) + '\n'
manifest_path.write_text(data)
external = ROOT / 'release/artifacts' / f'b2ige-{manifest["version"]}-{target}.manifest.json'
external.write_text(data)
checksums = external.parent / 'SHA256SUMS'
names = [line.split('  ', 1)[1] for line in checksums.read_text().splitlines()]
checksums.write_text(''.join(hashlib.sha256((external.parent / name).read_bytes()).hexdigest() + '  ' + name + '\n' for name in names))
print('Recorded fresh runtime scope:', manifest['platform_validation'][target])
