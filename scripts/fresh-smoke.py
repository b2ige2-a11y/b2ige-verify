#!/usr/bin/env python3
"""Extract an RC archive into a new directory and run installed executables only."""
import argparse, importlib.util, json, os, pathlib, subprocess, sys, tempfile
parser = argparse.ArgumentParser()
parser.add_argument('archive')
parser.add_argument('--skip-benchmark', action='store_true')
parser.add_argument('--without-docker', action='store_true', help='Explicit partial platform smoke, never a full release gate')
args = parser.parse_args()
archive = pathlib.Path(args.archive).resolve()
tool_scripts = pathlib.Path(__file__).resolve().parent
root = pathlib.Path(tempfile.mkdtemp(prefix='b2ige-install-'))
spec = importlib.util.spec_from_file_location('install_release', pathlib.Path(__file__).with_name('install-release.py'))
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)
package = installer.install(archive, archive.parent / 'SHA256SUMS', root / 'installed', smoke=True)
bin_dir = package / 'bin'
spec = importlib.util.spec_from_file_location('installed_demo', package / 'scripts/demo.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
demo, run = module.demo, module.run
os.environ.pop('B2IGE_BENCH_EFFECT_FIXTURE', None)
os.environ.pop('B2IGE_BLINDTEST_SEALED_ROOT', None)
os.environ.pop('B2IGE_BINARY', None)
os.environ['PATH'] = str(bin_dir) + os.pathsep + os.environ['PATH']
work = root / 'new-project'
work.mkdir()
os.chdir(work)
cli = bin_dir / 'b2ige'
help_text = run([cli, '--help'])
for surface in ['inspect', 'init', 'prepare', 'trust approve', 'doctor', 'verify ID', 'ci init',
                'behavior', 'sideeffect', 'blindtest', 'report', 'bench']:
    assert surface in help_text, surface
assert run([cli, '--version']).strip() == 'verify-cli ' + json.loads((package / 'release-manifest.json').read_text())['version']
run([cli, 'bench', '--help'])
# ci-check is an operation-only surface, absent from top-level help.
check = subprocess.run([str(cli), 'ci-check', 'missing-agent.json', '0'],
                       capture_output=True, text=True, timeout=45)
assert check.returncode == 3 and json.loads(check.stdout)['verdict'] == 'ERROR'
assert (package / 'scripts/install-release.py').is_file()
run([cli, 'init', '--dry-run']); run([cli, 'init'])
setup = package / 'scripts' / 'setup.py'
assert setup.is_file()
run([sys.executable, str(setup), '--binary', str(cli), '--skip-build'])
# Empty registry is intentionally not ready; inspect it, never call it verification.
r = subprocess.run([str(cli),'doctor'],capture_output=True,text=True,timeout=45)
d = json.loads(r.stdout)
assert r.returncode == (0 if d['ready'] else 3)
for product in (['behavior','sideeffect'] if args.without_docker else ['behavior','sideeffect','blindtest']):
    demo(product, bin_dir, root / product)
if not args.without_docker:
    run([cli, 'blindtest', 'doctor'])
entry = {'product':'behavior','config':str(root/'behavior/pass/experiment.json'),'store':str(root/'mcp-runs'),'authorization':str(root/'behavior/pass/authorization.json')}
registry = work/'.b2ige/project.json'
registry.write_text(json.dumps({'schema_version':'1','entries':{'hello':entry}}))
assert json.loads(run([cli,'doctor']))['ready']
# Exercise CI bootstrap with a real registered product fixture from the installed demo.
ci_args = [cli, 'ci', 'init', '--identity', 'hello', '--provider', 'github-actions',
           '--registry', str(registry), '--verifier-repo', 'b2ige2-a11y/b2ige-verify',
           '--verifier-ref', 'a' * 40]
preview = run(ci_args)
assert 'verification_performed: false' in preview
assert not (work / '.github').exists()
assert preview == run(ci_args)
run([*ci_args, '--write'])
assert (work / '.github/workflows/b2ige-verify.yml').read_text() == preview.split('--- workflow ---\n')[1].rstrip() + '\n'
subprocess.run([sys.executable, str(tool_scripts / 'validate-workflows.py'),
                str(work / '.github/workflows/b2ige-verify.yml')], check=True)
recovery = root / 'recovery-check'
(recovery / '.b2ige').mkdir(parents=True)
recovery_registry = recovery / '.b2ige/project.json'
recovery_registry.write_text('{malformed')
recovered = subprocess.run([sys.executable, str(setup), '--project-root', str(recovery),
                            '--binary', str(cli), '--skip-build'], capture_output=True, text=True)
assert recovered.returncode == 3 and recovery_registry.read_text() == '{malformed'
messages = [
 {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'fresh-smoke','version':'1'}}},
 {'jsonrpc':'2.0','method':'notifications/initialized'},
 {'jsonrpc':'2.0','id':2,'method':'tools/list'},
 {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'b2ige_behavior_verify','arguments':{'protocol_version':'1','product':'behavior','operation':'verify','identity':'hello','output':'agent','execution_budget':None}}}
]
r = subprocess.run([str(bin_dir/'b2ige-mcp'),'--registry',str(registry)],input=''.join(json.dumps(m)+'\n' for m in messages),text=True,capture_output=True,timeout=60,check=True)
responses = [json.loads(line) for line in r.stdout.splitlines()]
assert responses[0]['result']['protocolVersion']=='2025-11-25'
assert len(responses[1]['result']['tools'])==5
assert responses[2]['result']['structuredContent']['verdict']=='PASS'
# The full 31-case gate is bounded and also exercises the packaged benchmark helper.
if args.skip_benchmark:
    print('Fresh archive CLI/init/doctor/product reports/MCP: PASS; benchmark NOT RERUN')
elif args.without_docker:
    for product in ['behavior', 'sideeffect']:
        bench = json.loads(run([cli,'bench',product,'--output','json']))
        assert bench['products'][product]['gate_pass']
        assert not bench['complete_release_corpus'] and not bench['summary']['gate_pass']
    print('Fresh archive CLI/init/doctor/Behavior/SQLite/reports/MCP/subset benchmarks: PASS; Docker NOT RUN; full release gate NOT claimed')
else:
    bench = json.loads(run([cli,'bench','--output','json']))
    assert bench['summary']['gate_pass'] and bench['complete_release_corpus']
    print('Fresh archive CLI/init/doctor/examples/reports/Docker/MCP/31-case bench: PASS')
print('Development source tree was not used by installed commands; system Docker and /bin/sh remain prerequisites.')
