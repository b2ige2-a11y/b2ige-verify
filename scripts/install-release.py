#!/usr/bin/env python3
"""Install exact native archives. Integrity is not publisher authentication."""
import argparse
import hashlib
import io
import json
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu')
# Historical release inventory. Unknown future online releases require review.
RELEASES = {'0.2.0': TARGETS}
BINARIES = ('b2ige', 'b2ige-mcp', 'b2ige-demo', 'b2ige-demo-effect', 'p5-effect-fixture')
MAX_ARCHIVE = 512 * 1024 * 1024
MAX_CONTENT = 2 * 1024 * 1024 * 1024
REPOSITORY = 'https://github.com/b2ige2-a11y/b2ige-verify/releases/download'
WINDOWS_DEVICES = {'con', 'prn', 'aux', 'nul', *(f'{p}{n}' for p in ('com', 'lpt') for n in range(1, 10))}


def host_target():
    value = {('Darwin', 'arm64'): TARGETS[0], ('Darwin', 'x86_64'): TARGETS[1],
             ('Linux', 'x86_64'): TARGETS[2]}.get((platform.system(), platform.machine().lower()))
    if value is None:
        raise ValueError('unsupported native platform; use reviewed source/build instructions; Linux arm64 deferred, Windows source/build only')
    return value


def filename(version, target):
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', version) or target not in TARGETS:
        raise ValueError('unsupported version/target; use an explicitly supported native archive')
    return f'b2ige-{version}-{target}.tar.gz'


def urls(version, target):
    if version not in RELEASES or target not in RELEASES[version]:
        raise ValueError('unsupported release target/version; review the release inventory (no latest fallback)')
    return (f'{REPOSITORY}/v{version}/{filename(version, target)}',
            f'{REPOSITORY}/v{version}/SHA256SUMS')


def checksum_document(data):
    if len(data) > 1024 * 1024:
        raise ValueError('checksum document too large')
    entries = {}
    for line in data.decode('ascii').splitlines():
        match = re.fullmatch(r'([a-f0-9]{64})  ([A-Za-z0-9][A-Za-z0-9_.-]*)', line)
        if not match or match[2] in entries or match[2] in ('.', '..'):
            raise ValueError('malformed or duplicate checksum entry; obtain the original trusted SHA256SUMS')
        entries[match[2]] = match[1]
    if not entries:
        raise ValueError('missing checksums')
    return entries


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate manifest key')
        result[key] = value
    return result


def safe_destination(destination):
    p = pathlib.Path(destination).expanduser().absolute()
    if '..' in p.parts:
        raise ValueError('destination traversal refused')
    for ancestor in (p, *p.parents):
        if ancestor.is_symlink() and not (platform.system() == 'Darwin' and str(ancestor) in ('/tmp', '/var')):
            raise ValueError('symlink destination/parent refused; choose a real new directory')
    if p.exists() or p.is_symlink():
        raise ValueError('destination already exists; select a new directory; existing state is preserved')
    if not p.parent.is_dir():
        raise ValueError('destination parent must already exist')
    return p


def validated_members(tar, root, version, target):
    members = []
    seen = set()
    total = 0
    for m in tar:
        members.append(m)
        path = pathlib.PurePosixPath(m.name)
        if (path.is_absolute() or not path.parts or path.parts[0] != root
                or any(part in ('', '.', '..') for part in m.name.rstrip('/').split('/'))
                or any(part.rstrip(' .') != part or part.split('.')[0].casefold() in WINDOWS_DEVICES for part in path.parts)
                or '\\' in m.name or ':' in m.name or any(ord(c) < 32 or ord(c) == 127 for c in m.name)
                or path.as_posix().casefold() in seen or not (m.isfile() or m.isdir())
                or m.mode & 0o7000 or m.size < 0):
            raise ValueError('unsafe archive member/layout (links, traversal, duplicates and special files refused)')
        seen.add(path.as_posix().casefold())
        total += m.size
        if total > MAX_CONTENT or len(members) > 100000:
            raise ValueError('archive exceeds extraction limits')
    files = {m.name: m for m in members if m.isfile()}
    for m in members:
        if any(str(parent) in files for parent in pathlib.PurePosixPath(m.name).parents):
            raise ValueError('archive file/directory collision')
    def read(name, limit):
        member = files.get(f'{root}/{name}')
        if member is None or member.size > limit:
            raise ValueError('missing/oversized installed file: ' + name)
        return tar.extractfile(member).read()
    manifest = json.loads(read('release-manifest.json', 4 * 1024 * 1024), object_pairs_hook=unique_object)
    if not isinstance(manifest, dict):
        raise ValueError('manifest must be an object')
    if (manifest.get('schema_version') != '3' or manifest.get('manifest_role') != 'embedded'
            or manifest.get('version') != version or manifest.get('built_target') != target
            or not isinstance(manifest.get('supported_platforms'), list)
            or target not in manifest['supported_platforms']):
        raise ValueError('manifest version/target/layout mismatch')
    hashes = manifest.get('binary_sha256')
    if not isinstance(hashes, dict) or set(hashes) != set(BINARIES):
        raise ValueError('missing/unexpected installed binary inventory')
    for name, expected in hashes.items():
        if hashlib.sha256(read('bin/' + name, MAX_ARCHIVE)).hexdigest() != expected:
            raise ValueError('installed binary checksum mismatch')
        if not files[f'{root}/bin/{name}'].mode & 0o111:
            raise ValueError('installed binary is not executable')
    for name, key in [('THIRD-PARTY-NOTICES.txt', 'license_notices_sha256'),
                      ('RUST-RUNTIME-NOTICES.html', 'rust_runtime_notices_sha256')]:
        if hashlib.sha256(read(name, 16 * 1024 * 1024)).hexdigest() != manifest.get(key):
            raise ValueError('notice checksum mismatch')
    for name in ('LICENSE', 'TRADEMARKS.md', 'SECURITY.md'):
        read(name, 4 * 1024 * 1024)
    return members


def install(archive, checksums, destination, version=None, target=None, smoke=False):
    archive = pathlib.Path(archive)
    match = re.fullmatch(r'b2ige-([0-9]+\.[0-9]+\.[0-9]+)-(.+)\.tar\.gz', archive.name)
    if not match:
        raise ValueError('malformed native archive filename')
    version = version or match[1]
    target = target or host_target()
    if archive.name != filename(version, target):
        raise ValueError('archive filename version/target mismatch')
    destination = safe_destination(destination)
    entries = checksum_document(pathlib.Path(checksums).read_bytes())
    if archive.name not in entries:
        raise ValueError('missing archive checksum; obtain trusted SHA256SUMS')
    # Immutable in-process snapshot: validation and extraction use the same bytes.
    with archive.open('rb') as stream:
        data = stream.read(MAX_ARCHIVE + 1)
    if len(data) > MAX_ARCHIVE or hashlib.sha256(data).hexdigest() != entries[archive.name]:
        raise ValueError('archive checksum mismatch; preserve destination and reacquire trusted archive/checksums')
    root = archive.name[:-7]
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
        members = validated_members(tar, root, version, target)
        if smoke and target != host_target():
            raise ValueError('smoke requires the native host target')
        safe_destination(destination)
        destination.mkdir(mode=0o700)
        try:
            for m in members:
                path = destination / m.name
                if m.isdir():
                    path.mkdir(parents=True, exist_ok=True)
                else:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    with path.open('xb') as output, tar.extractfile(m) as source:
                        shutil.copyfileobj(source, output)
                    path.chmod(0o755 if m.mode & 0o111 else 0o644)
        except Exception:
            shutil.rmtree(destination)
            raise
    package = destination / root
    if smoke:
        # Absolute installed paths and an empty cwd prevent source-tree fallback.
        with tempfile.TemporaryDirectory(prefix='b2ige-smoke-') as work:
            for binary, option in [('b2ige', '--version'), ('b2ige-mcp', '--help')]:
                subprocess.run([str(package / 'bin' / binary), option], cwd=work,
                               check=True, timeout=60)
    return package


class HTTPSOnly(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if not newurl.startswith('https://'):
            raise ValueError('insecure release redirect refused')
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def download(url, path, limit):
    with urllib.request.build_opener(HTTPSOnly()).open(url, timeout=60) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError('release download exceeds limit')
    path.write_bytes(data)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version')
    parser.add_argument('--target', choices=TARGETS)
    parser.add_argument('--destination', required=True)
    parser.add_argument('--archive')
    parser.add_argument('--checksums')
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--smoke', action='store_true')
    a = parser.parse_args(argv)
    try:
        safe_destination(a.destination)
        if a.offline:
            if not a.archive or not a.checksums:
                raise ValueError('offline mode requires --archive and --checksums; no network is used')
            package = install(a.archive, a.checksums, a.destination, a.version, a.target, a.smoke)
        else:
            if not a.version or a.archive or a.checksums:
                raise ValueError('online mode requires exact --version; local inputs require --offline')
            target = a.target or host_target()
            archive_url, checksums_url = urls(a.version, target)
            with tempfile.TemporaryDirectory(prefix='b2ige-download-') as tmp:
                archive = pathlib.Path(tmp) / filename(a.version, target)
                sums = pathlib.Path(tmp) / 'SHA256SUMS'
                download(checksums_url, sums, 1024 * 1024)
                checksum_document(sums.read_bytes())
                download(archive_url, archive, MAX_ARCHIVE)
                package = install(archive, sums, a.destination, a.version, target, a.smoke)
        print(f'Installed native archive: {package}\nAdd its bin directory to PATH manually. No product verification performed.\nChecksums establish integrity, not publisher authentication. macOS Developer ID/notarization unavailable.\nBlindTest requires a local Docker engine; run b2ige blindtest doctor and restore missing prerequisites.\nFor broken registry/setup state, preserve it and recover reviewed inputs; never auto-approve.\nnpm and Cargo registry publication remain deferred.')
        return 0
    except (OSError, EOFError, ValueError, KeyError, TypeError, tarfile.TarError, subprocess.SubprocessError) as error:
        print(f'Installation refused: {error}', file=sys.stderr)
        return 3


if __name__ == '__main__':
    raise SystemExit(main())
