# P2 — Behavior Differential MVP

Scope: one explicit BehaviorCase, BEFORE and AFTER through real P1 acquisition.
P1C's recorded gate authorizes P2. P3 is not started. No commit/push/deploy.

## Implementation contract

- `verify_core::behavior::execute` takes a trusted `BehaviorAuthorization`, a
  versioned experiment, a comparison ID, an EvidenceStore and an absolute workspace
  root. It runs only the supplied case; there is no discovery or generation.
- A single case supplies literal args, cleared/explicit environment, timeout,
  null stdin, local fixture snapshot and `process_byte_exact_v1`. Both targets
  must declare that same computed input identity. Only executable identity/path
  and fresh physical cwd differ. Seed is shared in both acquisition plans.
- Both executable-byte SHA-256 identities and baseline binding are checked before
  either run. Each target is hashed again immediately before its acquisition.
  Mutable labels, Git-only identities and mismatches are rejected.
- Reuse P1A Baseline, approval status/actor/reason, externally pinned canonical
  artifact hashes and `checker_binding_hash`. The baseline observation-contract
  hash binds the process policy/observer set. Trusted policy must assert baseline
  stability as in P1A; P2 neither measures it nor updates approvals.
- The P1A Claim used for approval binding describes the versioned P2 comparator;
  the synthetic P1A Equals evaluator does not execute the Behavior comparison.

## Reset and execution

- Capture the fixture once into a bounded in-memory snapshot (64 MiB, 4096 entries,
  depth 64). Content hashes, relative paths, directories and Unix permission bits
  identify it; symlinks and special files are rejected. None means a fresh empty
  directory. Both access and modified times are fixed to Unix epoch on restoration.
- Exclusively create `<workspace_root>/<comparison_id>/before`, execute BEFORE,
  then create `after` from the original snapshot. Never reuse BEFORE's cwd or
  reread a source that BEFORE may have changed. Existing workspaces are refused.
- `acquisition::acquire` executes and atomically persists each P1 result using
  distinct `<comparison_id>-before` and `-after` IDs. The outer P2 experiment
  performs reset; embedded P1 plans truthfully retain `reset_strategy: none`.
- Load both committed runs again. Reconstruct the P1 envelope with existing
  `assemble`, verify raw evidence and source/trust/coverage/context/run/hash links,
  and compare with the exact issued plans/specs and returned acquisition hashes.
  No public comparison API accepts caller-provided process observations.

## Outcome and evidence

| Behavior outcome | Verdict | Requirement |
| --- | --- | --- |
| DIVERGENCE_PROVEN | FAIL | Two complete acquisitions; exit, signal or raw bytes differ |
| NO_DIVERGENCE_FOUND | PASS | Two complete acquisitions; all four observables match |
| INCONCLUSIVE | INCONCLUSIVE | Timeout/incomplete capture, unsupported required observer or unasserted stability |
| ERROR | ERROR | Invalid identity/approval/input, spawn/acquisition/reset/store failure or corrupt evidence |

PASS wording is **NO DIVERGENCE FOUND within tested behavior space**. One explicit
case with one execution per target is not equivalence, exhaustive proof or measured
stability. Completion is required for all four observables before any divergence.
Other runner errors take precedence over incomplete capture. Identical nonzero
exit codes are ordinary observations and may yield NO_DIVERGENCE_FOUND.

P1 v1 reports capture overflow and stream-not-closed-after-timeout as acquisition
ERROR. P2 recognizes only these two exact P1 v1 reasons as incomplete capture and
returns INCONCLUSIVE; the P1 evidence, coverage and verdict remain unchanged.
This is a product-specific classification, not a relaxation of acquisition.

Each divergence contains case identity, both run IDs and target hashes, observable,
status values or raw byte SHA-256/length, and both direct-runtime evidence refs.
Raw bytes remain in the P1 evidence. No difference is made from a hash alone.

## Artifact/version decision

- New `BehaviorComparisonResult` schema v1 and
  `schemas/behavior-comparison-result.schema.json`; no prior schema changes.
  `cargo run -p verify-core --example behavior_schema` emits this schema only.
- New versioned BehaviorCase/BehaviorExperiment input contracts. Unknown comparison
  policies and missing fields are rejected; explicit environment rejects duplicate
  keys. Runtime validates supported versions and identities.
- Comparison artifact includes the full experiment, case/baseline/candidate
  identities, input/snapshot/experiment/config/authorization hashes, optional durable
  run records, both acquisitions' config/plan/result hashes, evidence references,
  outcome/verdict, scope/budget, coverage, limitations and replayability.
- Store v1 remains unchanged. Reserve the comparison ID before execution and
  atomically publish its result with a derived receipt via existing RunStore.
  The receipt points to direct-runtime acquisition evidence; it is not independently
  authoritative evidence. Existing results, pending runs and workspaces are never
  overwritten. An interrupted comparison without a commit is not a result.
- Preflight/execution errors persist as ERROR if storage permits. An `io::Err`
  means ERROR without a promised comparison commit (e.g. duplicate ID or I/O failure).
- `behavior::load` verifies the receipt and actual linked acquisitions on disk;
  it recalculates any positive comparison. Missing/corrupt artifacts cannot be
  returned as a verified positive result. Generic `EvidenceStore::load` alone is
  an integrity reader, not the Behavior validator. Hashes are not authentication.

## Limits and replay

Trusted local Unix only. No hostile same-user hash-to-spawn protection, sandbox,
secret protection, dynamic-library/runtime pinning, external state reset, network,
DB, HTTP or filesystem behavior comparison. Absolute cwd paths, inode, ctime and
host dependencies may differ; these are not normalized. Permission bits/content
are copied; extended attributes, ownership and other host metadata are not pinned.
Targets that depend on these uncontrolled inputs can produce differences. Both
workspaces are retained for inspection. Explicit environment is stored in clear text.

No paired snapshot replay executor is added in P2. Every result explicitly records
`replayability: unavailable` with a reason and instructions for a fresh explicit
comparison using matching input/fixture/executable hashes. P1C replay by itself
does not restore these workspaces. No normalization, noise learner, generator,
mutation, fuzzing, shrinking, reducer, stateful exploration, SideEffect, BlindTest,
UI, MCP, LLM or cloud work is included.

## Required validation

Real integration suite: `cargo test -p verify-core --test behavior_differential`.
Tests cover A–L plus corruption, one-sided acquisition, changed executables/source
fixtures, provenance/binding rejection, unsupported observers, input schema, and
no-overwrite behavior. The original 110 tests are preserved.

## Verifier-logic review

1. Missing evidence cannot become PASS: both durable run links, typed P1 reconstruction,
   direct-runtime evidence and complete terminal/stream observations are required.
   Coherently rehashed false-PASS reports are rejected by comparison recomputation.
2. Observer failure cannot become product FAIL: incomplete capture/timeout is
   INCONCLUSIVE; other runner/acquisition errors are ERROR before checking differences.
3. Uncontrolled nondeterminism is not learned away. Reset removes sequential local
   fixture contamination; trusted stability is an external assertion. DIVERGENCE
   proves the sampled observable difference, not its cause or a general regression.
4. Baseline poisoning cannot silently update approval: artifact and checker pins,
   provenance, contract hash and actual BEFORE executable identity must all match.
5. There is no secrecy claim. P1 isolation remains `none`; trusted local targets can
   access host files. No hidden grader/oracle or isolation implementation is added.
6. Controllable inputs, target identities, snapshot identity and evidence are recorded.
   Automatic paired replay is unavailable and explicitly explained on every FAIL.

This is an implementation self-review plus bounded tests, not an independent audit.

## Recorded local gate — 2026-09-14

**PASS** on Darwin arm64 / Rust 1.98.1.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, zero warnings |
| `cargo test --workspace` | PASS, 138 tests; zero failed/ignored |
| `cargo build --workspace --release` | PASS |
| Real Behavior integration suite | 28/28 PASS, including A–L |
| Existing P1 suite | 110/110 PASS; no test/fixture/schema changes |

P1 modifications are limited to crate-internal visibility for `validate_inputs`
and `assemble`, allowing exact P1 reconstruction without a second implementation.
No acquisition/runner/evidence/replay execution or verdict semantics changed.
The core module export and two new fixture binary registrations are additive.
No new dependencies or Cargo.lock changes.

Results: [P2-RESULT.md](../P2-RESULT.md). Gate: [state/CURRENT.md](../state/CURRENT.md).
P3's local prerequisite gate is satisfied; P3 remains unstarted and requires a new
explicit request. No commit, push or deployment.
