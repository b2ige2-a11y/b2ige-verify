# P1A executable conformance map

Scope: P0 Contract Conformance Harness only. No production verifier or product engine.

## Running

From the repository root, with Rust stable and rustfmt/clippy available:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

All integration tests live in `tests/conformance/`, registered by verify-core.
No test uses the network, real target, database, provider, browser, LLM or secret.
Cargo dependency installation may need network; Cargo.lock pins the resolved graph.

## ID reconciliation

P0 IDs are preserved; the user request reused some V IDs with different meanings.

| Request ID | Existing/executable coverage |
| --- | --- |
| V-001 complete execution, coverage and checkers | P0 V-004, V-007, COV-001–003 |
| V-002 missing evidence | P0 V-001, V-005, E-002 |
| V-003 authoritative violation | P0 V-003, V-010 |
| V-004 verifier execution failure | P0 V-002, V-006 |
| V-005 observation without sufficient result | V-005, COV-001 |
| BH-001 baseline safety | P0 B-001–002, BH-003–004 |
| SE-001 attempt versus committed effect | P0 S-001–003, SE-004–009 |
| BT-001 candidate invariant | P0 BT-001 plus BT-005 |

## Test inventory

| IDs | Contract fixed by the tests |
| --- | --- |
| V-001–005 | All P0 verdict cases, plus insufficient evidence after execution |
| V-006 | Runner crash, invalid authoritative config, observer init, corrupted store: ERROR dominates |
| V-007–008 | Missing checker and empty contract never become PASS |
| V-009 | Exit codes 0/1/2/3 |
| V-010 | Supported violation survives unrelated insufficient evidence |
| COV-001 | Partial/unavailable/failed/absent critical coverage blocks PASS |
| COV-002 | Run cannot reclassify required observer as optional |
| COV-003 | Every claim's required observer must be satisfied separately |
| E-001–002 | Advisory/derived and wrong source/run/claim/trust cannot decide |
| E-003 | Corrupt payload or duplicated evidence identity is ERROR |
| E-004 | Advisory-only checker policy is invalid |
| E-005 | Evidence order cannot hide a supported violation |
| E-006 | JCS key-order independence and payload sensitivity |
| B-001–002 | Unapproved or unstable baseline cannot gate |
| BH-003–004 | Forged/replaced/unpinned baseline approval cannot gate |
| S-001–003 | Two attempts/one commit PASS; invisible commit INCONCLUSIVE; two commits FAIL |
| SE-004–005 | Repeated observation deduplicated; wrong identity not sufficient |
| SE-006 | Observed zero differs from absent or partial observation |
| SE-007–008 | Runtime labels cannot prove commits; separate records cannot hide duplicates |
| SE-009 | Unsupported effect semantics rejected |
| BT-001–003 | Original P0 invariant/exposure/hardened-platform cases |
| BT-004–005 | No isolation upgrade; no forged or replaced invariant authority |
| RP-001–002 | Missing or mismatched controllable replay inputs are unavailable |
| RP-003 | FAIL retains evidence, reference and unavailable reason |
| RP-004 | Invalid run identity is ERROR |
| JSON-001 | Six valid golden fixtures pass schema + semantic checking |
| JSON-002 | Four structurally valid adversarial fixtures fail semantic checking |
| JSON-003 | Required metadata, enum and unknown-field rejection |
| JSON-004 | Rust/new schema drift detection |
| JSON-005 | Existing schema negative validation and agent creator rejection |
| JSON-006 | Forged replayability/isolation claims rejected |
| JSON-007 | Invalid identity cannot emit schema-invalid run-result |

Total: 48 test functions. Loops exercise additional adversarial variations; those are
not counted as separate test functions. This is a bounded corpus, not exhaustive proof.

## Trust boundary and verifier-logic review

1. Missing evidence cannot become PASS: trusted policy declares required observers,
   claims, allowed evidence classes and predicates. Completeness labels cannot replace
   evidence. Both are required.
2. Observer failure is not automatically target failure: pre-execution/init/runner
   failures are ERROR; observation insufficiency is INCONCLUSIVE. A supported target
   violation may still yield FAIL despite unrelated evidence loss.
3. Nondeterminism: no Behavior comparison/normalization engine exists. An unstable
   baseline is rejected for gating; no unstable field is silently normalized.
4. Baseline poisoning: no auto-update or auto-approval API. Entire baseline artifacts
   must match trusted pins. Approval does not prove a baseline correct.
5. Secrets: no hidden grader or isolation implementation exists. Preflight constraints
   reject unavailable/exposed modes, but assessments are synthetic trusted input, not
   evidence that this workspace is isolated. Do not use this harness as a sandbox.
6. Reproduction: FAIL points to recorded experiment inputs and evidence. Availability
   is checked against run hashes and deterministic inputs; unavailable cases say why.

`TrustedPolicy`, observer provenance/coverage and preflight assessments are trusted
verifier inputs. Fixture JSON embeds them for testing convenience only. Never accept
that envelope from a target/LLM as authenticated policy/evidence. P1B must establish
these boundaries before any public verification claim. P1A exposes no CLI/service
that accepts external target results. Models do not assign verdicts.

The schema-version decision is recorded in `docs/P1A-SCHEMA-DECISION.md`.
The explicit fixture generator is a maintenance utility, not a test or baseline
approval workflow. It overwrites synthetic golden files only when manually invoked;
review all generated changes and never use regeneration to make failing tests pass.
