#!/usr/bin/env python3
"""Scan the public file inventory; never print potential secret contents."""
import pathlib, re, subprocess, sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
def files():
    out=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT)
    return [ROOT/p for p in out.decode().split('\0') if p and (ROOT/p).is_file()]
def scan(paths):
    bad=[]
    patterns=[rb'/(?:Us' + rb'ers|home)/[^/\s"<>]+/', rb'/(?:private/)?var/folders/[a-z0-9]{2}/',
              rb'\.codex/worktrees/[A-Za-z0-9]+/', rb'BLINDTEST_PRIVATE_CANARY_[a-f0-9]{32}',
              rb'AKIA[A-Z0-9]{16}', rb'gh[pousr]_[A-Za-z0-9]{30,}', rb'github_pat_[A-Za-z0-9_]{30,}',
              rb'sk-(?:proj-|ant-)?[A-Za-z0-9_-]{20,}', rb'[rs]k_(?:live|test)_[A-Za-z0-9]{16,}',
              rb'-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----',
              rb'(?i)authorization["\s:=>]+(?:bearer|basic)\s+[A-Za-z0-9_.~+/-]{12,}']
    for p in paths:
        if p.is_symlink():bad.append(str(p));continue
        if any(part in {'.git','.DS_Store','.b2ige','target','node_modules','__pycache__','.cache','.pytest_cache'}
               or part == '.env' or part.startswith('.env.') for part in p.parts):bad.append(str(p));continue
        if p.suffix in {'.log','.pyc'}:bad.append(str(p));continue
        data=p.read_bytes()
        if any(re.search(pattern,data) for pattern in patterns):bad.append(str(p))
    if bad: raise SystemExit('Public hygiene blocked (paths only):\n'+'\n'.join(bad))
if __name__=='__main__':
    paths=list(pathlib.Path(sys.argv[1]).rglob('*')) if len(sys.argv)>1 else files()
    scan([p for p in paths if p.is_file()]); print('Public artifact hygiene: PASS')
