"""Read-only GitHub evidence collection. Called only by an actual external operator.

No network occurs at import or in deterministic tests. gh credentials stay in gh;
responses are never dumped, and downloads are inspected in memory, not extracted.
"""
import base64
import io
import json
from pathlib import Path
import re
import stat
import subprocess
import tempfile
import zipfile


def api(endpoint, *, binary=False):
    # Only constructed repository API paths reach gh; no shell or caller URL.
    result = subprocess.run(['gh', 'api', '--method', 'GET', endpoint], capture_output=True, timeout=120)
    if result.returncode != 0 or len(result.stdout) > 16 * 1024 * 1024:
        raise ValueError('GitHub read unavailable or oversized')
    return result.stdout if binary else json.loads(result.stdout)


def inspect_archive(raw, binary, pilot):
    pilot.need(len(raw) <= 16 * 1024 * 1024, 'oversized artifact')
    verdicts = []
    with zipfile.ZipFile(io.BytesIO(raw)) as archive, tempfile.TemporaryDirectory() as tmp:
        files = archive.infolist()
        pilot.need(0 < len(files) <= 100 and len({f.filename for f in files}) == len(files), 'artifact inventory')
        pilot.need(sum(f.file_size for f in files) <= 8 * 1024 * 1024, 'expanded artifact limit')
        for f in files:
            pilot.pattern(f.filename, r'[A-Za-z0-9_-]{1,64}\.json')
            pilot.need(not f.is_dir() and not f.flag_bits & 1 and not stat.S_ISLNK(f.external_attr >> 16)
                       and stat.S_IFMT(f.external_attr >> 16) in [0, stat.S_IFREG], 'artifact special file')
            data = pilot.bench.decode(archive.read(f))
            pilot.need(data.get('product') == 'blindtest' and data.get('operation') == 'verify', 'CI product boundary')
            verdict = data.get('verdict')
            pilot.enum(verdict, pilot.OUTCOMES)
            # Existing protocol checker rejects malformed/extra fields, readiness and
            # verdict/exit mismatch. Equality prevents accepting its ERROR fallback.
            path = Path(tmp) / f.filename
            pilot.bench.save(path, data)
            result = subprocess.run([str(binary), 'ci-check', str(path), str(pilot.OUTCOMES[verdict])],
                                    capture_output=True, timeout=30)
            pilot.need(result.returncode == pilot.OUTCOMES[verdict]
                       and pilot.bench.decode(result.stdout) == data, 'unvalidated artifact')
            # Defense in depth; not a universal secret detector. The operator must
            # also inspect downloaded contents privately before attesting absence.
            encoded = json.dumps(data)
            for expression in [r"/(?:Us" + r"ers|home)/", r"[A-Za-z]:\\", r"[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}",
                               r"gh[pousr]_[A-Za-z0-9]+", r"github_pat_", r"BLINDTEST_PRIVATE", r"-----BEGIN .*PRIVATE KEY"]:
                pilot.need(re.search(expression, encoded) is None, 'private artifact content')
            verdicts.append(verdict)
    return verdicts, dict(name='b2ige-sanitized-agent-reports', file_count=len(verdicts),
                         kind='validated-agent-protocol-v1-json', archive_sha256=pilot.bench.digest(raw))


def observe(repository, pr_number, run_id, sha, workflow, workflow_hash, binary, pilot, fetch=api):
    pilot.pattern(repository, r'[A-Za-z0-9_-]{1,100}/[A-Za-z0-9_.-]{1,100}')
    prefix = 'repos/' + repository
    repo = fetch(prefix)
    pilot.need(repo.get('private') is False and repo.get('html_url') == 'https://github.com/' + repository, 'public consented repository required')
    pr = fetch(prefix + '/pulls/' + str(pr_number))
    run = fetch(prefix + '/actions/runs/' + str(run_id))
    pilot.need(run['event'] == 'pull_request_target' and run['status'] == 'completed'
               and run['repository']['full_name'] == repository and run['path'] == workflow,
               'external workflow run mismatch')
    linked = [p for p in run['pull_requests'] if p['number'] == pr_number]
    pilot.need(len(linked) == 1, 'run must link the supplied PR (no inferred association)')
    link = linked[0]
    base, head = link['base']['sha'], link['head']['sha']
    pilot.need(pr['base']['repo']['full_name'] == repository and pr['html_url'] == 'https://github.com/' + repository + '/pull/' + str(pr_number), 'PR repository mismatch')
    # pull_request_target uses base as workflow head, not candidate HEAD. Historical
    # pair comes from the run's linked PR, not the PR's mutable latest SHA.
    pilot.need(run['head_sha'] == base, 'trusted base SHA mismatch')
    content = fetch(prefix + '/contents/' + workflow + '?ref=' + base)
    pilot.need(content['encoding'] == 'base64', 'workflow encoding')
    workflow_bytes = base64.b64decode(content['content'], validate=False)
    pilot.need(pilot.bench.digest(workflow_bytes) == workflow_hash, 'reviewed workflow identity differs')
    text = workflow_bytes.decode()
    repositories = re.findall(r'^          repository: ([A-Za-z0-9_-]{1,100}/[A-Za-z0-9_-]{1,100})$', text, re.MULTILINE)
    pilot.need(len(repositories) == 1, 'pinned verifier repository missing')
    template = (pilot.ROOT / 'crates/verify-cli/src/ci-workflow.yml').read_text()
    expected = template.replace('__REPO__', repositories[0]).replace('__PIN__', sha).replace('__IDENTITY__', 'bench')
    pilot.need(text in [expected, pilot_workflow(expected, pilot)], 'workflow differs from reviewed generated gate/provisioning')
    jobs = fetch(prefix + '/actions/runs/' + str(run_id) + '/jobs?per_page=100')
    pilot.need(jobs['total_count'] == len(jobs['jobs']) <= 100, 'uninspected job pagination')
    selected = [j for j in jobs['jobs'] if j['name'] == 'verify']
    pilot.need(len(selected) == 1, 'exact verification check required')
    job = selected[0]
    steps = [s for s in job['steps'] if s['name'] == 'Verify all registered contracts (only PASS is green)']
    pilot.need(len(steps) == 1 and steps[0]['status'] == 'completed', 'actual verification step required')
    inventory = fetch(prefix + '/actions/runs/' + str(run_id) + '/artifacts?per_page=100')
    pilot.need(inventory['total_count'] == len(inventory['artifacts']) == 1, 'all run artifacts must be inspected; only sanitized artifact allowed')
    artifact = inventory['artifacts'][0]
    pilot.need(not artifact['expired'] and artifact['name'] == 'b2ige-sanitized-agent-reports'
               and artifact['size_in_bytes'] <= 16 * 1024 * 1024, 'sanitized artifact missing/expired')
    raw = fetch(prefix + '/actions/artifacts/' + str(artifact['id']) + '/zip', binary=True)
    verdicts, inspection = inspect_archive(raw, binary, pilot)
    # The minimum kit is one identity. Avoid inventing aggregate verdict precedence.
    pilot.need(len(verdicts) == 1, 'minimum pilot CI expects exactly one registered identity')
    verdict = verdicts[0]
    conclusion = 'success' if verdict == 'PASS' else 'failure'
    pilot.need(job['conclusion'] == steps[0]['conclusion'] == run['conclusion'] == conclusion, 'PASS-only-green disagreement')
    return dict(repository_url='https://github.com/' + repository, base_sha=base, head_sha=head,
                verifier_sha=sha, workflow_path=workflow, workflow_sha256=workflow_hash,
                pr_url=pr['html_url'], run_url=run['html_url'], run_id=run_id, job_id=job['id'],
                verdict=verdict, exit_code=pilot.OUTCOMES[verdict], check_conclusion=conclusion,
                verification_step_conclusion=conclusion, independently_provisioned=True,
                public_consent=True, observed_via='github_api', artifacts=[inspection], no_private_artifacts=True)


def collect_ci(sha, binary, pilot):
    pilot.need(pilot.choose('Consent to publishing supplied repository/PR/run references?', ['yes', 'no']) == 'yes', 'CI consent required')
    pilot.need(pilot.choose('Was candidate approval independently provisioned by the trusted operator for EACH head, outside candidate control?', ['yes', 'no']) == 'yes', 'independent approval required')
    repository = input('Public repository OWNER/REPO: ').strip()
    workflow = input('Reviewed workflow path (.github/workflows/NAME.yml): ').strip()
    pilot.pattern(workflow, r'\.github/workflows/[A-Za-z0-9_-]{1,64}\.yml')
    workflow_hash = input('SHA-256 of the exact independently reviewed deployed workflow (sha256:...): ').strip()
    pilot.pattern(workflow_hash, pilot.HASH)
    results = []
    for label in ['PASS green', 'non-PASS non-green']:
        print('Supply the actual external ' + label + ' run. Never use local or simulated runs.')
        pr = input('PR number: ').strip()
        run = input('Actions run ID: ').strip()
        pilot.pattern(pr, r'[1-9][0-9]{0,11}')
        pilot.pattern(run, r'[1-9][0-9]{0,14}')
        c = observe(repository, int(pr), int(run), sha, workflow, workflow_hash, binary, pilot)
        pilot.ci_fields(c, sha)
        results.append(c)
    pilot.need(pilot.choose('After privately inspecting ALL downloaded artifacts in both runs: are raw stores, sealed material, private paths, oracle/canary values, human reports, credentials and logs absent?', ['yes', 'no', 'unsure']) == 'yes', 'private-artifact human inspection required')
    pilot.need(results[0]['verdict'] == 'PASS' and results[1]['verdict'] != 'PASS', 'CI pair missing')
    return results


def approval_file(registry, head, output, pilot):
    """Explicit human CI admission input, after existing product trust approve.

    No result is read, and no product registry/baseline is created or updated.
    The human independently attests the image-to-head association; hashing alone
    cannot establish build provenance, exactly as in existing V110-C.
    """
    pilot.need(pilot.sys.stdin.isatty() and pilot.sys.stdout.isatty(), 'interactive controller terminal required')
    pilot.commit()
    pilot.pattern(head, pilot.HEX)
    registry = Path(registry).resolve(strict=True)
    original = registry.read_bytes()
    value = pilot.bench.decode(original)
    pilot.shape(value, 'schema_version entries')
    pilot.need(value['schema_version'] == '1' and set(value['entries']) == {'bench'}, 'one reviewed bench identity required')
    entry = value['entries']['bench']
    pilot.need(entry['product'] == 'blindtest' and entry['authorization'] is None, 'CI is Docker BlindTest only')
    config_path = Path(entry['config'])
    pilot.need(config_path.is_absolute() and not config_path.is_symlink(), 'absolute controller config required')
    config_bytes = config_path.read_bytes()
    config = pilot.bench.decode(config_bytes)
    image = config['target']['image']
    pilot.pattern(image, pilot.HASH)
    print('Candidate SHA:', head, '\nImmutable Docker image:', image,
          '\nConfig byte identity:', pilot.bench.digest(config_bytes))
    print('Independently review the isolated build provenance for this exact head and image. A hash is not proof of that association.')
    pilot.need(input('Type APPROVE-CI ' + head + ' only after independent review: ').strip() == 'APPROVE-CI ' + head,
               'CI candidate approval declined')
    pilot.need(registry.read_bytes() == original and config_path.read_bytes() == config_bytes, 'controller changed during review')
    pilot.save(output, dict(schema_version='1', head=head, contracts={'bench': dict(
        config_sha256=pilot.bench.digest(config_bytes), authorization_sha256=None, target_identity=image)}))


def pilot_workflow(generated, pilot):
    generated = generated.replace('runs-on: ubuntu-22.04', 'runs-on: [self-hosted, linux, x64, b2ige-pilot]')
    marker = '      - name: Require protected controller inputs (no automatic approval)'
    provision = '''      - name: Provision independently approved public pilot inputs
        run: |
          test -f /opt/b2ige-pilot/controller/candidate-approval.json
          test ! -e "$RUNNER_TEMP/b2ige-candidate-approval.json"
          test ! -e "$RUNNER_TEMP/b2ige-sealed"
          cp /opt/b2ige-pilot/controller/candidate-approval.json "$RUNNER_TEMP/b2ige-candidate-approval.json"
          cp -R /opt/b2ige-pilot/controller/material/sealed "$RUNNER_TEMP/b2ige-sealed"
'''
    pilot.need(generated.count(marker) == 1, 'reviewed template changed')
    generated = generated.replace(marker, provision + marker)
    return generated


def workflow_file(root, registry, verifier_repo, controller, output, pilot):
    """Generate a public-pilot-only ephemeral operator runner workflow.

    Uses existing ci init, changing only runner/provisioning, never the gate body.
    The participant must independently review the resulting workflow before use.
    """
    sha = pilot.commit()
    pilot.pattern(verifier_repo, r'[A-Za-z0-9_-]{1,100}/[A-Za-z0-9_-]{1,100}')
    # Neutral fixed controller path prevents publication of participant home/name.
    pilot.need(controller == '/opt/b2ige-pilot/controller', 'use the documented neutral controller path')
    with tempfile.TemporaryDirectory() as tmp:
        command = [pilot.BINARY, 'ci', 'init', '--identity', 'bench', '--provider', 'github-actions',
                   '--registry', registry, '--verifier-repo', verifier_repo, '--verifier-ref', sha,
                   '--root', tmp, '--write']
        response = pilot.execute(command, cwd=pilot.ROOT, env=pilot.os.environ.copy())
        pilot.need(response.returncode == 0, 'ci init failed')
        generated = (Path(tmp) / '.github/workflows/b2ige-verify.yml').read_text()
    generated = pilot_workflow(generated, pilot)
    root = pilot.output_path(root)
    dest = pilot.output_path(output)
    pilot.need(root.is_dir() and root in dest.parents, 'workflow must be in the external repository')
    pilot.need(dest.relative_to(root).as_posix() == '.github/workflows/b2ige-verify.yml', 'fixed workflow destination')
    dest.parent.mkdir(parents=True, exist_ok=True)
    with dest.open('x', encoding='utf-8') as stream:
        stream.write(generated)
    print('Review before publishing. Workflow identity:', pilot.bench.digest(generated.encode()))


def recheck(record, binary, pilot, fetch=api):
    """Completion CLI re-fetches real runs; a URL-shaped assertion is insufficient."""
    for expected in record['ci']:
        repository = expected['repository_url'].removeprefix('https://github.com/')
        pr = int(expected['pr_url'].rsplit('/', 1)[1])
        observed = observe(repository, pr, expected['run_id'], record['pilot_commit'],
                           expected['workflow_path'], expected['workflow_sha256'], binary, pilot, fetch)
        pilot.need(observed == expected, 'external CI recheck differs; do not rewrite previous evidence')
