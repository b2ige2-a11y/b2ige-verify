> Historical private-development review. Its history blockers are resolved for this
> clean public candidate by excluding that history; see [current readiness](../PUBLIC-REPO-READINESS.md).

# Public hygiene review — 2026-09-15

## History: completed, publication blocked

Read-only scan of **24 commits, 388 blobs and 251 trees (663 objects; 3,105,855
bytes)**, including every locally stored loose/packed object, all refs and reflog
commits. No object was omitted. Bounds: 32 MiB/object and 512 MiB total; neither was
reached. No gitleaks or trufflehog executable was installed; scripts/history-scan.py
performed direct Git object scanning. No rewrite, pruning, push or repository change
was performed. All findings were reviewed against their blob contents.

Patterns cover home/Windows-home/Codex worktree/temp paths, provider credentials,
Authorization/Bearer data, credential assignments, private keys, runtime canary values,
private evidence markers and forbidden history paths including environment files.
Commit messages/tree names were scanned too. Normal Git author identifiers are retained
as intentional authorship metadata; the owner must approve that public attribution.
This bounded pattern scan cannot prove absence of unknown/encoded secrets.

**No actual credential, environment-file content, private-key value or runtime private
canary was detected by these checks.** Six historical blobs still contain machine-specific
paths (four documents and two captured benchmark snapshots). These remain BLOCKERs for
exposing this existing history. The snapshots contain runtime/reproduction locations,
not raw sealed hidden inputs; schema descriptions and synthetic test fixtures are distinct.
Three historical Finder metadata blobs remain REVIEW items. Current public inventory
contains none of those files or machine-specific path values.

The table groups all 241 matches by immutable blob, with a containing commit (not
necessarily the introducing commit). Full redacted finding/line/rule/classification/risk
and removal recommendations are in [history-scan-review.json](history-scan-review.json).
Potential matched values are not copied into this report.

| Blob | Containing commit | Path | Class | Matches | Finding |
|---|---|---|---|---:|---|
| `0b7a726370247b8a1b9bed80fd2890519b21e3d4` | `83db19326e830b733e6a83393996e7961442bb83` | `schemas/blindtest-sealed-suite.schema.json` | FALSE_POSITIVE | 2 | Schema field declaration or scanner regular expression; no private runtime value |
| `18dcfbdca79d2a20138ca7331322e6ff5e21ac88` | `83db19326e830b733e6a83393996e7961442bb83` | `schemas/report-document.schema.json` | FALSE_POSITIVE | 1 | Schema field declaration or scanner regular expression; no private runtime value |
| `31b451b94b5e0e5fa4d90b783e2edebe76e731cc` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `P1A-REPAIR-RESULT.md` | BLOCKER | 14 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |
| `6e86c0598c88cc9b5174f824e885c83d2a507389` | `83db19326e830b733e6a83393996e7961442bb83` | `docs/BLINDTEST.md` | SAFE_SYNTHETIC | 7 | Documented example path or literal negative-test injection, not a recorded private runtime location |
| `734dcc6212f3b9a39668dc370ad52cc55b415c89` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `benchmarks/baseline-v1/result.json` | BLOCKER | 59 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |
| `76fe13eb1feb0754817d5ed21c0131d0159b0185` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `P1A-RESULT.md` | BLOCKER | 37 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |
| `86cf29981ea0ed0c7d2ac8ff01042dea65b03ca1` | `83db19326e830b733e6a83393996e7961442bb83` | `crates/verify-mcp/tests/workflow.rs` | SAFE_SYNTHETIC | 1 | Documented example path or literal negative-test injection, not a recorded private runtime location |
| `a6d6938448ab4790510c0997dad4451d1f86dc92` | `83db19326e830b733e6a83393996e7961442bb83` | `scripts/hygiene.py` | FALSE_POSITIVE | 1 | Schema field declaration or scanner regular expression; no private runtime value |
| `ad6c006d239a44997cee896ec29526998ae68f68` | `3e94ec87452ca4bfd5210cb78f243e5fa13238ca` | `.DS_Store` | REVIEW | 1 | Historical Finder metadata, not current public inventory; no secret pattern found |
| `af51c0a2c93363a6e795e1b06d40d48fe7b7965b` | `607fec7c3028ce6a36f0f33a721d36d465a06837` | `.DS_Store` | REVIEW | 1 | Historical Finder metadata, not current public inventory; no secret pattern found |
| `afff4703dae17b5df173b114605963f7bd607f05` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `benchmarks/baseline-v1/reverse-result.json` | BLOCKER | 59 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |
| `b164b085b199855af5d17e6be51bf73fb9725257` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `P6-RESULT.md` | BLOCKER | 3 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |
| `ddb55140715c53c19a0bd8dcdb40be560507bc4b` | `83db19326e830b733e6a83393996e7961442bb83` | `schemas/blindtest-run-result.schema.json` | FALSE_POSITIVE | 2 | Schema field declaration or scanner regular expression; no private runtime value |
| `ebeb0b961ec3fcd30c5f1be499472cca9350a545` | `0ead049136079524c98ede0cc67fe1eb042ba813` | `.DS_Store` | REVIEW | 1 | Historical Finder metadata, not current public inventory; no secret pattern found |
| `edfc9db373b692337502d0171e62edbf0aae958a` | `47343edacf49d12a824e2f7cd53f08926474a30f` | `P1A-INDEPENDENT-GATE.md` | BLOCKER | 52 | Historical machine-specific home/worktree or captured runtime location; no credential material observed |

Risk for historical local paths: **medium privacy/publication hygiene**; they are not
reported as live secrets. Removal recommendation: separately approved history cleanup
using filter-repo/BFG, or owner-approved publication of a new clean repository from the
reviewed snapshot. Do not run either automatically. If future review discovers actual
credentials, revoke/rotate first, then sanitize every affected ref/cache and re-scan.
Current-tree deletion alone does not remove historical content.

## Clean public candidate resolution

**Private development history: NOT INTENDED FOR PUBLICATION**

**Public candidate history: NEW CLEAN HISTORY**

The historical local-path blocker is **resolved for this candidate by the clean
public repository strategy**: the candidate was copied from the reviewed working
tree and initialized without importing any private commits or Git objects. The
private repository remains unchanged; its history was not rewritten, scrubbed or
claimed to be publication-safe. A final candidate file-tree rescan found **0
current public-tree blockers**. Intentional placeholder paths and negative-test
values in public fixtures are classified `SAFE_SYNTHETIC` and are not private
runtime evidence.

## Public artifacts and synthetic boundary

Native/source/npm archives are rebuilt through a fresh temporary staging inventory and
extracted independently for checks. Archive-member validation rejects absolute/traversal
paths, links/devices, duplicate entries, environment files and private/cache directories.
Native/source tar ownership headers are normalized to uid/gid zero and empty user/group
names; archive validation rejects local account metadata in every tarball.
The inventory excludes Git internals, Finder metadata, target outputs, local logs/caches,
private stores, screenshots and private runtime canaries. Native binary bytes are scanned
for provider secrets, host paths and actual canary values in addition to extracted text.

Public BlindTest schemas, example target/controller source and intentionally synthetic
negative-test inputs remain public. They do not establish hidden production oracle
secrecy. Full Human runtime evidence and generated sealed suites are excluded.
Generic Unix system paths and documented synthetic temporary paths are reviewed context,
not evidence of a captured private store. No benchmark baseline is re-approved or changed.

Final extracted archive and binary/checksum validation is recorded in
[PUBLICATION-READINESS-RESULT](../PUBLICATION-READINESS-RESULT.md); history BLOCKERs remain
separate even when current artifacts pass. Archive file hashes live in the external
release manifest and SHA256SUMS; the report avoids embedding self-referential hashes.

## README and public claim review

All public Markdown was searched for unqualified correctness/sandbox/false-positive
slogans. No prohibited positive claim was found. Seventeen uses of the exhaustive-proof
term were reviewed in context; each is a prohibition, explicit exclusion or bounded-test
limitation. Benchmark documentation and the generated README table explicitly label
B2IGE Verify Bench v1, 31 explicit cases, and metrics on the benchmark corpus. Historical
non-benchmark test results remain scoped to their own executed corpora. No baseline JSON
or verifier verdict was changed.
