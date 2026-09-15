# P1C — Minimal Replay Executor

Scope: persisted P1B process acquisitions only. P2 is not started.
Implementation complete; final local gate is recorded in `state/CURRENT.md`.

## Tasks

- [x] Load the original through the integrity-checking Evidence Store.
- [x] Validate plan/experiment, config, seed, target, process inputs, versions and evidence links before execution.
- [x] Verify a content-addressed executable against its recorded SHA-256 identity.
- [x] Reuse P1B acquisition and the existing Process Runner for fresh observations/evidence.
- [x] Reserve a distinct run identity and atomically commit the replay relationship with fresh evidence.
- [x] Keep reproduction status separate from product Verdict.
- [x] Exercise requirements A–H with real executable integration tests.
- [x] Preserve existing P1A 77 and P1B 18 tests without edits or weakening.

## Entry point and preflight

`verify_core::acquisition::replay::execute(store, original_run_id, replay_run_id)`
returns `ReplayResult`. The caller supplies a fresh run ID. Duplicate identities
cannot spawn: the original ID is rejected and acquisition reserves any new ID
exclusively before execution.

Only committed P1B AcquisitionResult v1 artifacts are accepted. Store loading
checks canonical bytes, the manifest hash, evidence integrity and run identity.
Typed preflight reconstructs the P1B envelope from its recorded observation and
requires exact agreement: plan identity/hash and experiment alias, config hash,
seed, executable, args, absolute cwd, explicit environment, timeout, target,
observer versions, run IDs, coverage, claims, evidence source and hashes. Missing
fields, unsupported plans/versions and inconsistent links are UNAVAILABLE before
reservation. The supported tool/platform/observer versions must match this backend.
The unchanged P1B plan has no fault schedule, reset, optional observer or isolation
execution support; unsupported extensions are rejected rather than ignored.

P1C supports `sha256:<lowercase hex>` as the SHA-256 of the executable's raw bytes
(not its JSON representation). The current executable is streamed and checked
against that recorded identity before acquisition. Missing or changed files are
UNAVAILABLE. Mutable labels remain forbidden under P1A. Although P1A recognizes
full `git:` identities as immutable, this executor rejects them as UNAVAILABLE:
a commit alone does not establish which arbitrary executable was built from it.
No checkout/build/provenance system is added.

## Execution and persistence

`acquire` and replay share the same private acquisition operation: validate inputs,
reserve run, call the existing `verify_runner::process::observe`, build fresh
Evidence, write evidence, commit result last. Replay changes only the commit
envelope. It never writes original evidence into the replay run.

The replay run's `result.json` is the existing Store v1 commit wrapping a new
ReplayResult v1. It contains original_run_id, replay_run_id, the canonical hash of
the complete original AcquisitionResult, original evidence record hashes, replay
status/reason and the fresh AcquisitionResult. That acquisition contains its own
run/source/observation/evidence hashes. Store integrity covers the relationship
and the actual replay evidence together; there is no second, non-atomic sidecar.
Preflight rejection is returned without creating a replay directory. Runner
failure is committed with new failure evidence and ERROR. Storage/reservation/
commit failure returns ERROR, no successful acquisition, and may leave an
incomplete reserved directory under the existing P1B crash policy.

The embedded acquisition bundle marks replay-of-replay unavailable. To retry,
use the original P1B run and another fresh ID; preflight always runs again.

## Status and schema decision

A **new replay envelope starts at version 1** (`replay_schema_version`).
`verify_replay::ReplayStatus` is a new type, serialized as:

- `REPRODUCED`: the recorded process result matches: started, exit code, signal,
  stdout bytes, stderr bytes, timeout and runner-failure fields. Wall timestamps
  are excluded only from this comparison; evidence still retains and hashes them.
- `EXECUTED_BUT_DIVERGED`: acquisition succeeded but that process result differs.
  Output variation is not an infrastructure ERROR or product FAIL.
- `UNAVAILABLE`: original/preflight invalid or a controllable input cannot be verified.
- `ERROR`: replay runner or storage failure, including spawn failure.

This is bounded process-result reproduction, not a Behavior comparator or proof
that an original product failure reproduced. Equal timeout observations mean only
equal captured prefixes/status, never complete output. No product checker runs:
normal replay acquisition remains INCONCLUSIVE and runner failure remains ERROR.
P0/P1A schemas, ReplayInputs/Replayability/ReproductionOutcome, product Verdict,
P1B acquisition/bundle and Store wire shapes remain at v1 without fallback.
The new wrapper is intentionally not accepted as a P1B original.

## Trust boundary and remaining limits

Store hashes are integrity checks, not authentication. A raw Store caller can
forge internally consistent original metadata; replay assumes trusted P1B
acquisition artifacts and callers. P1B's declared target hash cannot retroactively
prove which bytes ran originally. P1C verifies current bytes against that trusted
recorded identity, and replay success requires a new normal acquisition execution.
Raw Store writes alone are never used as evidence that replay ran.

Trusted local Unix processes/filesystem only. The preflight hash and subsequent
path-based spawn are not atomic against a hostile same-user executable replacement.
Interpreters, linked libraries, cwd contents and external state are not snapshotted
or reset. Environment values are stored in clear text. Existing 1 MiB stream limits,
process-group cleanup and detached-descendant limitations apply. No sandbox,
secrecy or universal determinism is claimed. This host's validation does not prove
Linux/Windows support. No new dependency package is introduced; SHA-256 reuses the
existing workspace dependency.

## Required verifier review

1. Missing evidence never becomes PASS: preflight rejects; this path emits no PASS.
2. Observer/spawn failures are replay ERROR, not product FAIL.
3. Exact output variation can yield divergence, but never a product regression claim.
4. No baseline, approval or checker state is changed.
5. No secrecy/isolation claim is made; trusted local targets are required.
6. Both run IDs, original hashes and fresh evidence are durably linked to the
   recorded inputs; unavailable inputs explicitly prevent execution.

Validation: `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build --workspace --release`.
