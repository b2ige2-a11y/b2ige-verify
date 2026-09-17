# V110 adoption benchmark — measured report

Corpus: adoption-v1; tooling schema: 1; non-authoritative V110 UX/adoption evidence.

V110-B measures bounded reproducibility of the adoption workflow against semantic expectations inherited from the already-reviewed P8 corpus.
It does not establish independent new correctness labels, third-party project compatibility, external-user usability, real hidden secrecy, or exhaustive correctness. External operator evidence remains V110-D.

Source checkout/build preparation is separate from operation; historical v0.2.0 archives do not contain V110-A. CI bootstrap: DEFERRED_TO_V110_C.

Environment: {"docker_engine": "linux/aarch64", "machine": "arm64", "release": "27.0.0", "system": "Darwin", "versions": {"docker": "29.5.2", "node": "v26.0.0", "python3": "Python 3.14.3", "rustc": "rustc 1.98.1 (48a229cea 2026-09-01)"}}
Docker scope: actual local Linux engine; digest-pinned preloaded image, no network build or runtime service.

| Metric | Numerator / denominator |
|---|---:|
| classification_agreement | 9 / 9 |
| completed | 9 / 9 |
| first_verification | 9 / 9 |
| recovery_success | 0 / 0 (N/A) |
| undocumented_edits | 0 / 9 |
| unsafe_shortcuts_rejected | 28 / 28 |

| Scenario / P8 authority | Runtime / product | Allowed → actual | Trust checkpoints | Blocking stage |
|---|---|---|---:|---|
| rust-preserving / behavior.preserving | Rust / behavior | PASS → PASS | 1 | none |
| node-regression / behavior.stdout | Node.js / behavior | FAIL → FAIL | 1 | none |
| python-preserving / behavior.preserving | Python / behavior | PASS → PASS | 1 | none |
| sqlite-safe / sideeffect.normal | Python / SQLite / sideeffect | PASS → PASS | 1 | none |
| sqlite-duplicate / sideeffect.unsafe_retry | Python / SQLite / sideeffect | FAIL → FAIL | 1 | none |
| sqlite-corrupt / sideeffect.corrupt | Python / SQLite / sideeffect | ERROR → ERROR | 1 | none |
| docker-correct / blindtest.correct | Node.js / Docker / blindtest | PASS → PASS | 1 | none |
| docker-mutant / blindtest.mutant_a | Node.js / Docker / blindtest | FAIL → FAIL | 1 | none |
| docker-partial / blindtest.partial | Node.js / Docker / blindtest | INCONCLUSIVE → INCONCLUSIVE | 1 | none |

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
- malformed-registry: rejected
- missing-authority-case: rejected
- missing-evidence: rejected
- missing-observer: rejected
- noninteractive-approval: rejected
- omitted-scenario: rejected
- private-copy: rejected
- private-overlap: rejected
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

See [runner instructions](../benchmarks/adoption-v1/README.md), [measured result](../benchmarks/adoption-v1/measured-result.json), and [fresh rerun comparison](../benchmarks/adoption-v1/repeatability.json).

Two clean executions matched semantically and produced identical human reports.

## Validation and scope audit

The final locked workspace gate passed: format, Clippy with warnings denied,
447 Rust tests, and release build. README benchmark consistency, public hygiene,
existing adoption tests, release tests, V100 release tests, workflow validation and
Git whitespace checks passed. The adoption harness's 14 tests passed, including
fresh repeated execution and forged-completion/report-order controls.
`python3 scripts/platform-tests.py` passed against the actual local Linux/aarch64
Docker engine. No private holdout was run.

The complete diff against main contains only the new adoption corpus/tooling,
benchmark fixture example, this report and the benchmark discoverability link.
Existing P8 corpus/baselines, workflows, workspace Cargo versions/lockfile,
authoritative schemas, V100 trust documents/semantics, V110-A approval code,
V110 state and release/tag state are unchanged. Build outputs and developer home
paths are absent from the public diff. V110-B is deliberately not marked COMPLETE.

## Eight benchmark review answers

1. Missing evidence cannot establish success: real loader rejection and recorded
   evidence coverage are required; the corrupt case's first result predates deletion.
2. Failed scenarios cannot disappear: exact ordered inventory and planned denominators
   are validated, even when an execution blocks.
3. Candidate output cannot author expectations or approvals: pinned P8 labels and
   existing reviewed constructors supply them before candidate execution.
4. Doctor cannot count as verification: a separate verify stage and source-backed
   response are mandatory; the real CI checker rejects readiness.
5. Previous results cannot be reused as a current run: create-new output, fresh lanes
   and fresh stores are mandatory; the runner has no previous-result input.
6. Public summaries withhold private paths/values: only fixed measurement fields and
   numeric coverage are exported; Agent output and project copies are leak-checked.
7. Semantic comparison detects meaningful changes: inventory, expectations, products,
   verdicts, exits, stages, coverage, controls and denominators remain in comparison.
8. Adoption evidence is explicitly separate from correctness claims: inherited public
   fixtures provide bounded workflow measurements, not new independent qualification.

These answers apply to the tested trusted-controller boundary, not a malicious
same-user host or wholesale replacement of verifier, tooling and retained pins.
