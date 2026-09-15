#!/usr/bin/env python3
"""Bounded, read-only scan of ALL locally stored Git objects; no matched values output.

This is pattern evidence, not a credential-validity or exhaustive secrecy proof.
JSON stays in the private local audit directory. Classification requires review.
"""
import collections
import hashlib
import json
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
RULES = {
    'host_home': rb'/(?:Us' + rb'ers|home)/[^/\s"<>]+',
    'windows_home': rb'[A-Za-z]:\\(?:Users|Documents and Settings)\\[^\\\s]+',
    'codex_worktree': rb'\.codex/worktrees[^\s"<>]*',
    'temp_path': rb'/(?:private/)?(?:tmp/|var/folders/)[^\s"<>]*',
    'private_canary_value': rb'BLINDTEST_PRIVATE_CANARY_[a-fA-F0-9]{32,}',
    'provider_secret': rb'(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|sk-(?:proj-|ant-)?[A-Za-z0-9_-]{20,}|[rs]k_(?:live|test)_[A-Za-z0-9]{16,}|AKIA[A-Z0-9]{16})',
    'private_key': rb'-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----',
    'authorization': rb'(?i)(?:authorization["\s:=>]+(?:bearer|basic)\s+[^\s"\']+|bearer\s+[A-Za-z0-9_.~+/-]{12,})',
    'credential_assignment': rb'(?i)(?:api[_-]?key|(?:access[_-]?)?token|secret(?:[_-]?key)?|password)\s*["\']?\s*[:=]\s*["\'][^"\'\r\n]{6,}["\']',
    'private_evidence_marker': rb'(?i)(?:"(?:private_canary|hidden_details|raw_hidden_evidence|private_metadata)"\s*:|\.b2ige/[^\s"\']*(?:sealed|private|store))',
}


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def main():
    commits = git('rev-list', '--all', '--reflog').decode().splitlines()
    locations = collections.defaultdict(set)
    first_commit = {}
    for commit in commits:
        for item in git('ls-tree', '-rz', '--full-tree', commit).split(b'\0'):
            if not item:
                continue
            meta, path = item.split(b'\t', 1)
            _, kind, oid = meta.decode().split()
            if kind == 'blob':
                locations[oid].add(path.decode())
                first_commit.setdefault(oid, commit)
    inventory = git('cat-file', '--batch-all-objects', '--batch-check=%(objectname) %(objecttype) %(objectsize)').decode().splitlines()
    results, skipped, total_bytes, kinds = [], [], 0, collections.Counter()
    for item in inventory:
        oid, kind, size = item.split(); size = int(size); kinds[kind] += 1
        if size > 32 * 1024 * 1024 or total_bytes + size > 512 * 1024 * 1024:
            skipped.append({'object': oid, 'bytes': size}); continue
        data = git('cat-file', kind, oid); total_bytes += size
        paths = sorted(locations.get(oid, []))
        for rule, pattern in RULES.items():
            for match in re.finditer(pattern, data):
                results.append({'object': oid, 'kind': kind, 'commit': first_commit.get(oid, oid if kind == 'commit' else None),
                                'paths': paths, 'line': data[:match.start()].count(b'\n') + 1,
                                'rule': rule, 'match_sha256': hashlib.sha256(match.group()).hexdigest(),
                                'reachable_from_refs_or_reflogs': oid in locations or oid in commits})
        for path in paths:
            parts = pathlib.PurePosixPath(path).parts
            if any(p in {'.b2ige', '.git', 'target', '.DS_Store'} or p == '.env' or p.startswith('.env.') for p in parts):
                results.append({'object': oid, 'kind': kind, 'commit': first_commit[oid], 'paths': [path],
                                'line': 0, 'rule': 'private_file_path', 'reachable_from_refs_or_reflogs': True})
    report = {'schema_version': '2', 'head': (subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=ROOT, text=True, capture_output=True).stdout.strip() or None),
              'commits_refs_and_reflogs': len(commits), 'objects': dict(kinds), 'scanned_bytes': total_bytes,
              'skipped': skipped, 'findings': results, 'classification': 'REVIEW_REQUIRED'}
    out = ROOT / '.b2ige/publication/history-scan.json'; out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'findings'}))
    print('Finding counts:', dict(collections.Counter(x['rule'] for x in results)))
    if skipped:
        raise SystemExit('History scan incomplete: object bounds reached')


if __name__ == '__main__':
    main()
