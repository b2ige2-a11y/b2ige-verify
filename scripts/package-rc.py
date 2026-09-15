#!/usr/bin/env python3
"""Local RC archives only; no uploads, tags, releases, or registry publication."""
import datetime
import hashlib
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from hygiene import ROOT, files, scan


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dump(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def public_member(info):
    # tar headers otherwise expose the local account even after content remapping.
    info.uid = info.gid = 0
    info.uname = info.gname = ''
    info.pax_headers = {}
    info.mtime = 0
    info.mode = 0o755 if info.isdir() or info.mode & 0o111 else 0o644
    return info


def main():
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
    info = subprocess.check_output(['rustc', '-vV'], text=True)
    target = next(line.split(': ', 1)[1] for line in info.splitlines() if line.startswith('host:'))
    targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu']
    if target not in targets: raise SystemExit('Unsupported candidate target')
    out = ROOT / 'release/artifacts'; out.mkdir(exist_ok=True)
    env = dict(os.environ)
    env['RUSTFLAGS'] = f'--remap-path-prefix={pathlib.Path.home()}=/build/user --remap-path-prefix={ROOT}=/build/b2ige'
    subprocess.run(['cargo', 'build', '--workspace', '--release', '--locked'], cwd=ROOT, env=env, check=True)
    subprocess.run(['python3', 'scripts/license-notices.py', '--check'], cwd=ROOT, check=True)
    name = f'b2ige-{version}-{target}'
    with tempfile.TemporaryDirectory(prefix='b2ige-package-') as temp:
        stage = pathlib.Path(temp) / name; (stage / 'bin').mkdir(parents=True)
        # npm sbom derives root component.name from this directory, even when
        # package.json has a scoped name. Preserve the full scope/name layout.
        npm_sbom_stage = pathlib.Path(temp) / '@b2ige/verify'; npm_sbom_stage.mkdir(parents=True)
        shutil.copy2(ROOT / 'npm/b2ige/package.json', npm_sbom_stage / 'package.json')
        npm_sbom = out / f'b2ige-{version}-npm.cdx.json'
        npm_sbom.write_bytes(subprocess.check_output(['npm', 'sbom', '--offline', '--sbom-format=cyclonedx'], cwd=npm_sbom_stage))
        npm_bom = json.loads(npm_sbom.read_text())
        assert npm_bom['bomFormat'] == 'CycloneDX' and npm_bom['metadata']['component']['version'] == version
        assert npm_bom['metadata']['component']['name'] == '@b2ige/verify'
        scan([npm_sbom])
        binaries = ['b2ige', 'b2ige-mcp', 'b2ige-demo', 'b2ige-demo-effect', 'p5-effect-fixture']
        for binary in binaries: shutil.copy2(ROOT / 'target/release' / binary, stage / 'bin' / binary)
        # Stage from the reviewed public inventory, never a recursive workspace copy.
        public = files(); scan(public)
        for p in public:
            rel = p.relative_to(ROOT)
            if rel.parts[0] in {'docs', 'examples', 'schemas', 'skills', 'npm', 'benchmarks', 'release'} or (len(rel.parts) == 1 and p.suffix in {'.md', '.txt'}) or p.name in {'LICENSE', 'demo.py', 'demo-all.sh', 'demo-behavior.sh', 'demo-sideeffect.sh', 'demo-blindtest.sh', 'readme-bench.py'}:
                if rel.as_posix() == 'release/release-manifest.json': continue
                dest = stage / rel; dest.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(p, dest)
        # std is statically linked; use the exact installed toolchain's library notices.
        sysroot = pathlib.Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
        runtime_notice = sysroot / 'share/doc/rust/COPYRIGHT-library.html'
        if not runtime_notice.is_file(): raise SystemExit('Rust library attribution bundle missing')
        shutil.copy2(runtime_notice, stage / 'RUST-RUNTIME-NOTICES.html')
        shutil.copy2(ROOT / 'Cargo.lock', stage / 'Cargo.lock')
        benchmark = json.loads((ROOT / 'benchmarks/baseline-v1/result.json').read_text())
        spec = importlib.util.spec_from_file_location('native_sbom', ROOT / 'scripts/native-sbom.py')
        native_sbom = importlib.util.module_from_spec(spec); spec.loader.exec_module(native_sbom)
        sbom = native_sbom.generate(out)
        for filename in sbom['files']: shutil.copy2(out / filename, stage / filename)
        sbom['npm'] = {'status': 'GENERATED', 'file': npm_sbom.name, 'sha256': sha(npm_sbom), 'scope': 'npm wrapper only; no Rust/native dependencies', 'generator': 'npm sbom'}
        manifest = {
            'schema_version': '3', 'manifest_role': 'embedded', 'version': version,
            'git_commit': (subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=ROOT, text=True, capture_output=True).stdout.strip() or None),
            'working_tree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)),
            'rust_toolchain': info, 'built_target': target, 'supported_platforms': targets,
            'actually_verified_platforms': [],
            'platform_validation_scope': {t: 'No runtime execution recorded for this artifact' for t in targets},
            'platform_validation': {t: 'NOT_RUN' for t in targets},
            'binary_sha256': {b: sha(stage / 'bin' / b) for b in binaries},
            'artifact_sha256': {}, 'license': 'Apache-2.0',
            'license_notices_sha256': sha(ROOT / 'THIRD-PARTY-NOTICES.txt'),
            'rust_runtime_notices_sha256': sha(runtime_notice),
            'signing': {'publisher_signed': False, 'apple_notarized': False, 'status': 'OWNER_DECISION'},
            'sbom': sbom,
            'name_clearance': {'status': 'INCOMPLETE', 'review': 'release/name-clearance-review.md', 'final_package_names': None},
            'benchmark': {k: benchmark[k] for k in ['benchmark_version', 'corpus_hash', 'semantic_hash', 'summary']},
            'benchmark_scope': 'B2IGE Verify Bench v1; 31 explicit cases; metrics on the benchmark corpus',
            'build_timestamp_nonsemantic': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'local_release_candidate_ready': False, 'clean_public_repo_ready': True, 'technical_publication_ready': False, 'publication_ready': False, 'owner_publication_authorized': False,
            'known_limitations': ['Packager alone does not attest runtime validation; fresh gates must pass',
                                  'Remote native platforms, real release host/pins, namespace ownership, private security reporting and owner decisions pending',
                                  'Unsigned by a publisher and not notarized; checksums do not authenticate',
                                  'Bounded corpus; trusted controller; no same-host-user/admin/root secrecy',
                                  'Linux arm64 and Windows unsupported in this candidate'],
            'source_inventory_sha256': hashlib.sha256(b''.join(str(p.relative_to(ROOT)).encode() + b'\0' + hashlib.sha256(p.read_bytes()).digest()
                                        for p in sorted(public) if p.relative_to(ROOT).as_posix() != 'release/release-manifest.json')).hexdigest(),
            'ci_provenance': {k: os.environ.get(k) for k in ['GITHUB_REPOSITORY', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_WORKFLOW']},
        }
        dump(stage / 'release-manifest.json', manifest)
        dump(ROOT / 'release/release-manifest.json', manifest)
        dump(stage / 'dependency-inventory.json', tomllib.loads((ROOT / 'Cargo.lock').read_text())['package'])
        scan([p for p in stage.rglob('*') if p.is_file()])
        native = out / f'{name}.tar.gz'
        with tarfile.open(native, 'w:gz', format=tarfile.USTAR_FORMAT) as t: t.add(stage, arcname=name, filter=public_member)
        # npm is staged separately so ignored local native/ files cannot sneak into npm pack.
        npm_stage = pathlib.Path(temp) / 'npm'; npm_stage.mkdir()
        for p in files():
            if p.is_relative_to(ROOT / 'npm/b2ige'):
                rel = p.relative_to(ROOT / 'npm/b2ige'); dest = npm_stage / rel
                dest.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(p, dest)
        for license_file in ['LICENSE', 'TRADEMARKS.md']: shutil.copy2(ROOT / license_file, npm_stage / license_file)
        packed = json.loads(subprocess.check_output(['npm', 'pack', '--ignore-scripts', '--json', '--pack-destination', str(out)], cwd=npm_stage, text=True))
        npm_archive = out / packed[0]['filename']
        source = out / f'b2ige-{version}-source.tar.gz'
        with tarfile.open(source, 'w:gz') as t:
            for p in files(): t.add(p, arcname=f'b2ige-{version}-source/{p.relative_to(ROOT)}', recursive=False, filter=public_member)
        manifest['manifest_role'] = 'release-index'
        manifest['artifact_sha256'] = {p.name: sha(p) for p in [native, source, npm_archive, npm_sbom]}
        manifest['artifact_sha256'].update(sbom['files'])
        dump(out / f'{name}.manifest.json', manifest); dump(ROOT / 'release/release-manifest.json', manifest)
        artifacts = [out / n for n in manifest['artifact_sha256']] + [out / f'{name}.manifest.json']
        (out / 'SHA256SUMS').write_text(''.join(sha(p) + '  ' + p.name + '\n' for p in sorted(artifacts)))
        print('Local archives created; runtime readiness NOT granted by packaging:', native.name)


if __name__ == '__main__':
    main()
