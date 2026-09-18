#!/usr/bin/env python3
"""Exercise the actual npm tarball with the packaged native executable, offline."""
import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import subprocess
import tarfile
import tempfile

root = pathlib.Path(__file__).resolve().parents[1]


def smoke(out):
    manifests = list(out.glob('*.manifest.json'))
    assert len(manifests) == 1, 'Expected one candidate target manifest'
    m = json.loads(manifests[0].read_text())
    spec = importlib.util.spec_from_file_location('validate_archive', root / 'scripts/validate-archive.py')
    validator = importlib.util.module_from_spec(spec); spec.loader.exec_module(validator)
    version = validator.product_metadata(root)
    validator.target_artifacts(m, version)
    checksums = validator.checksums(out)
    with tempfile.TemporaryDirectory(prefix='b2ige-npm-smoke-') as tmp:
        stage = pathlib.Path(tmp)
        npm_name = f'b2ige-verify-{version}.tgz'
        for name in [npm_name, f'b2ige-{m["version"]}-{m["built_target"]}.tar.gz']:
            assert hashlib.sha256((out / name).read_bytes()).hexdigest() == checksums[name]
            if name != npm_name:
                assert checksums[name] == m['artifact_sha256'][name]
            with tarfile.open(out / name) as t:
                validator.members_safe(t.getmembers()); t.extractall(stage, filter='data')
        binary = stage / f'b2ige-{m["version"]}-{m["built_target"]}/bin/b2ige'
        env = dict(os.environ, B2IGE_BINARY=str(binary), B2IGE_BINARY_SHA256=m['binary_sha256']['b2ige'])
        wrapper = stage / 'package/bin/b2ige.cjs'
        r = subprocess.run(['node', str(wrapper), '--version'], env=env, text=True, capture_output=True, check=True)
        assert r.stdout.strip() == f'verify-cli {m["version"]}'
        r = subprocess.run(['node', str(stage / 'package/bin/blindtest.cjs'), '--help'], env=env, capture_output=True)
        assert r.returncode == 0
        env['B2IGE_BINARY_SHA256'] = '0' * 64
        r = subprocess.run(['node', str(wrapper), '--version'], env=env, text=True, capture_output=True)
        assert r.returncode == 3 and not r.stdout and 'checksum mismatch' in r.stderr
        # Exercise the real packaged USTAR bytes through the actual downloader too.
        # Only this disposable test process substitutes a local transport and cache.
        launcher_platform = {'aarch64-apple-darwin':'darwin-arm64', 'x86_64-apple-darwin':'darwin-x64',
                             'x86_64-unknown-linux-gnu':'linux-x64'}[m['built_target']]
        archive = out / f'b2ige-{m["version"]}-{m["built_target"]}.tar.gz'
        (stage / 'package/native-manifest.json').write_text(json.dumps({
            'schema_version':'2', 'version':m['version'], 'release_repository':'synthetic-owner/synthetic-repository',
            'platforms':{launcher_platform:{'sha256':m['binary_sha256']['b2ige'], 'archive_sha256':m['artifact_sha256'][archive.name]}}}))
        preload = stage / 'transport.cjs'
        preload.write_text("const fs=require('node:fs'),{Readable}=require('node:stream'),{EventEmitter}=require('node:events');"
            "require('node:os').homedir=()=>__dirname;require('node:https').get=(url,opts,cb)=>{"
            "const req=new EventEmitter();req.destroy=()=>{};queueMicrotask(()=>{const r=Readable.from(["
            + 'fs.readFileSync(' + json.dumps(str(archive)) + ')'
            + "]);r.statusCode=200;r.headers={};cb(r)});return req;};")
        env.pop('B2IGE_BINARY', None); env.pop('B2IGE_BINARY_SHA256', None)
        r = subprocess.run(['node','--require',str(preload),str(wrapper),'--version'], env=env, text=True, capture_output=True)
        assert r.returncode == 0, r.stderr
        assert r.stdout.strip() == f'verify-cli {m["version"]}'
    print('Extracted npm/native and real archive downloader/launch/tamper checks: PASS; synthetic local transport, no real GitHub host')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', nargs='?', type=pathlib.Path, default=root / 'release/artifacts')
    smoke(parser.parse_args().directory.resolve())
