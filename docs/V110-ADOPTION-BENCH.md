# V110 adoption benchmark — measured report

Corpus: adoption-v1; tooling schema: 2; non-authoritative V110 UX/adoption evidence.

V110-B measures bounded reproducibility of the adoption workflow against semantic expectations inherited from the already-reviewed P8 corpus.
It does not establish independent new correctness labels, third-party project compatibility, external-user usability, real hidden secrecy, or exhaustive correctness. External operator evidence remains V110-D.

Source checkout/build preparation is separate from operation; historical v0.2.0 archives do not contain V110-A. CI bootstrap: DEFERRED_TO_V110_C.

Environment: {"docker_engine": "linux/aarch64", "machine": "arm64", "release": "27.0.0", "system": "Darwin", "versions": {"docker": "29.5.2", "node": "v24.18.1", "python3": "Python 3.14.3", "rustc": "rustc 1.98.1 (48a229cea 2026-09-01)"}}
Required Docker scope: actual local Linux engine; digest-pinned preloaded image, no network build or runtime service.

| Metric | Numerator / denominator |
|---|---:|
| classification_agreement | 9 / 9 |
| completed | 9 / 9 |
| first_verification | 9 / 9 |
| recovery_success | 0 / 0 (N/A) |
| undocumented_edits | 0 / 9 |
| unsafe_shortcuts_rejected | 31 / 31 |

| Scenario / P8 authority | Runtime / product | Allowed → measured | Verify / exit | Report / exit | Outcome stage | Trust checkpoints | Blocking stage |
|---|---|---|---|---|---|---:|---|
| rust-preserving / behavior.preserving | Rust / behavior | PASS → PASS | PASS / 0 | PASS / 0 | verify | 1 | none |
| node-regression / behavior.stdout | Node.js / behavior | FAIL → FAIL | FAIL / 1 | FAIL / 1 | verify | 1 | none |
| python-preserving / behavior.preserving | Python / behavior | PASS → PASS | PASS / 0 | PASS / 0 | verify | 1 | none |
| sqlite-safe / sideeffect.normal | Python / SQLite / sideeffect | PASS → PASS | PASS / 0 | PASS / 0 | verify | 1 | none |
| sqlite-duplicate / sideeffect.unsafe_retry | Python / SQLite / sideeffect | FAIL → FAIL | FAIL / 1 | FAIL / 1 | verify | 1 | none |
| sqlite-corrupt / sideeffect.corrupt | Python / SQLite / sideeffect | ERROR → ERROR | PASS / 0 | ERROR / 3 | report | 1 | none |
| docker-correct / blindtest.correct | Node.js / Docker / blindtest | PASS → PASS | PASS / 0 | PASS / 0 | verify | 1 | none |
| docker-mutant / blindtest.mutant_a | Node.js / Docker / blindtest | FAIL → FAIL | FAIL / 1 | FAIL / 1 | verify | 1 | none |
| docker-partial / blindtest.partial | Node.js / Docker / blindtest | INCONCLUSIVE → INCONCLUSIVE | INCONCLUSIVE / 2 | INCONCLUSIVE / 2 | verify | 1 | none |

Measured scenario outcomes: {"ERROR": 1, "FAIL": 3, "INCONCLUSIVE": 1, "PASS": 4}.
Initial verify outcomes: {"ERROR": 0, "FAIL": 3, "INCONCLUSIVE": 1, "PASS": 5}.

Measured inventory: 9 / 9 selected scenarios retained, including blocked scenarios.
First verification success means a source-backed PASS, FAIL or INCONCLUSIVE response after verify, not doctor readiness and not product PASS alone.
The corrupt SideEffect scenario first verifies intact evidence, then removes evidence and measures the report loader ERROR boundary, matching P8. Generic report errors may carry the CLI’s Behavior product hint; they grant no product success.
One benchmark PTY checkpoint per approval replays a pre-existing reviewed fixture decision. No output-derived approval or production noninteractive bypass exists.
Recovery events are explicitly counted; zero recovery attempts are N/A. Repeated init is tested separately without counting it as a recovery.

Public P8 BlindTest fixtures are mechanically materialized outside the project. This measures operational path separation, not secrecy from readers of the public corpus or unrestricted same-user host processes.
Timing is observational and never gates success; preparation and stage timings are stored separately in result.json. Docker layers and compiler caches may be reused; project state, approvals and evidence stores are fresh.
P8 corpus and baseline byte inventories are identical before/after. Expected source file and trusted constructor identities are pinned. No private holdout was run.

## Negative controls

- ERROR-exit-zero: rejected
- FAIL-exit-zero: rejected
- INCONCLUSIVE-exit-zero: rejected
- attempt-log-substitution: rejected
- authority-tampering: rejected
- baseline-poisoning: rejected
- corrupt-previous-result: rejected
- doctor-as-verification: rejected
- duplicate-registry: rejected
- expected-label-tampering: rejected
- failed-observer: rejected
- malformed-agent: rejected
- malformed-registry: rejected
- missing-authority-case: rejected
- missing-evidence: rejected
- missing-observer: rejected
- noninteractive-approval: rejected
- omitted-scenario: rejected
- private-copy: rejected
- private-overlap: rejected
- protected-output: rejected
- pty-boundary: rejected
- source-product-mismatch: rejected
- stale-evidence-store: rejected
- stale-executable: rejected
- stale-output: rejected
- stale-project-registry: rejected
- stale-reference: rejected
- symlink-controller: rejected
- tooling-as-evidence: rejected
- unapproved-baseline: rejected

## Reproduce and inspect

See [runner instructions](../benchmarks/adoption-v1/README.md),
[measured result](../benchmarks/adoption-v1/measured-result.json), and
[fresh rerun comparison](../benchmarks/adoption-v1/repeatability.json).

## Independent audit repairs and version decision

The final audit rejected v1 result mutations that had previously passed: inconsistent
verify/report verdicts, an unmarked corrupt-evidence loader boundary, and arbitrary
public environment strings. Tooling schema v2 explicitly records both verdict/exit
pairs and the measured outcome stage; v1 results are refused. Authoritative schemas
and P8 expectations remain unchanged.

Additional controls bind PTY replay to an active disposable benchmark, the fixed CLI
command, reviewed input bytes and environment before candidate execution. Missing or
changed authority, late replay and arbitrary commands are refused before spawning.
Malformed ERROR responses cannot be mistaken for the protocol checker's fallback.
Protected corpus output paths are refused before any directory is created. Crashed
journeys retain all nine planned rows; equal failed runs cannot pass comparison.

The project implementations are public archetypes. Behavior retains the reviewed
reference and authorization while changing only the candidate language. SideEffect
uses the pre-existing reviewed Python/SQLite retry fixture and the same durable
committed-ledger authority as P8; its normal/corrupt setup has two attempts instead
of P8's single NONE attempt. The corrupt scenario inherits the evidence-removal
loader case, not a claim of byte-identical product execution. BlindTest uses the
unchanged P8 suite/invariant constructor with fresh physical materialization.

## Eight benchmark review answers

1. Missing evidence cannot establish product success. Real loaders reject it; the
   corrupt scenario's initial PASS is explicitly recorded before evidence deletion.
2. Failed or incomplete scenarios cannot disappear from a successful summary: exact
   planned inventory, full denominators and control inventory are validated.
3. Candidate output cannot supply expectation or approval authority: pinned P8
   labels and existing reviewed constructors provide it before candidate execution.
4. Doctor readiness cannot count as first verification: a separate verify response,
   successful transport validation and verified evidence coverage are required.
5. Prior output cannot become a current execution: output reservation is create-new;
   projects, controllers, registries, approvals and stores are fresh.
6. Public summaries are bounded projections with runtime private-value/path scans,
   strict environment formats and complete Agent transport validation.
7. Semantic comparison retains inventory, authority, runtime/product, expectation,
   verdict/exit pairs, measured stage, stage completion, controls and denominators.
   Mutation tests reject meaningful changes; only durations are excluded.
8. Adoption metrics remain tooling evidence about bounded public workflows, separate
   from verifier correctness, external users/projects, private holdout qualification,
   actual hidden secrecy, historical archive support and V110-C CI bootstrap.

These answers assume the tested trusted-controller boundary. They do not authenticate
a wholesale replacement of verifier/tooling/pins or protect against unrestricted
same-user host access. External adoption evidence remains V110-D. No private holdout
is run by this benchmark; V110-B remains NEXT until separately reviewed completion.

## Final independent validation and scope audit

Audited implementation: `0fb4c21f2b0c1d548d8183514a699a5a5517fba0`.
Merge-base/main at audit: `7f4fe89f2b731454bf17af2d99590416edc2ef69`
(V110-A complete). The independent audit repaired benchmark tooling only.

Exactly one final comprehensive pass completed after code repairs:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --all-targets --locked` — 447 passed, none failed/ignored
- `cargo build --workspace --release --locked`
- `python3 scripts/readme-bench.py`
- `python3 scripts/hygiene.py`
- `python3 scripts/test-adoption.py` — 8 tests
- `python3 scripts/test-adoption-bench.py` — 21 tests
- `python3 scripts/test-release.py` — 3 tests
- `python3 scripts/test-v100-release.py` — 2 tests
- `python3 scripts/validate-workflows.py`
- `git diff --check`

Two additional complete fresh executions (forward and reverse) both passed 9/9
classification agreement, 9/9 completed, 9/9 first verification, 0/9 undocumented
edits, and 31/31 rejected negative controls. Their semantic contents and generated
human reports matched. Only stage/preparation durations were excluded. The original
28 controls remain; malformed Agent, PTY boundary and protected-output controls add
three. The measured outcomes are PASS 4, FAIL 3, INCONCLUSIVE 1, ERROR 1; initial
verify outcomes are PASS 5, FAIL 3, INCONCLUSIVE 1, ERROR 0.

`python3 scripts/platform-tests.py` then passed once against the actual local
Linux/aarch64 Docker engine on the recorded macOS arm64 host. This is local evidence,
not a new external multi-platform CI or private holdout qualification.

All nine P8 corpus/baseline files remained byte-identical from before the audit
through the final executions. Their expected labels and baselines were never updated.
The complete branch diff is confined to adoption corpus/projects/tooling, the fixture
example, the measured report and the benchmark discoverability link. V100/P8 authority,
V110-A approval semantics, workflows, existing Cargo versions/lockfile, authoritative
schemas, V110 completion state and release/tag state are unchanged. No private holdout
or generated build outputs are included; public files pass private-path/secret hygiene.
V110-B is not marked COMPLETE. No tag, release publication or merge is part of this audit.
