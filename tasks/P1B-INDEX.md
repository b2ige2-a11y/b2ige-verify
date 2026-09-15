# P1B — Minimal Runner + Evidence Store

Status: implemented; final workspace gate recorded in `state/CURRENT.md`.
Scope: process acquisition only. P1C and product engines are not started.
The explicit P1B user request supersedes the prior automatic-start restriction;
P1A's independent re-review is not claimed.

## Tasks

- [x] Separate executable/args, absolute working directory, cleared/explicit environment.
- [x] Actual Unix child process, byte-exact stdout/stderr, exit code/signal, timestamps.
- [x] Monotonic timeout, kill process group, reap leader, bounded EOF cleanup.
- [x] Distinguish target non-zero status from runner/capture/spawn failure.
- [x] Link existing ExperimentPlan + ProcessSpec to RunContext + EvidenceBundle.
- [x] Local JCS/SHA-256 store, exclusive run reservation, atomic no-replace publication.
- [x] Real executable fixtures and corruption/partial-write integration cases.
- [x] Preserve all 77 P1A regression tests and existing checker semantics.

## Public entry points and supported plan

`verify_core::acquisition::acquire(plan, spec, store, run_id)` reserves a run before
execution and returns AcquisitionResult only after evidence and result commit.
`verify_runner::process::observe` is the low-level process observer.
`verify_evidence::store::EvidenceStore::{reserve,load}` manages persistence.

The existing ExperimentPlan is unchanged. P1B accepts version 1, only required
observer `cli_process`, no optional observers, `reset_strategy: "none"`, isolation
`none`, and a nonzero timeout equal to the plan time budget. Unsupported requests
return an error before execution. Existing product/claim metadata is recorded but
its checker is not run. Explicit environment values are captured in the config.

Normal lifecycle: created -> running -> observed -> completed.
Spawn failure: created -> runner_failed -> completed.
Post-spawn runner/capture failure: created -> running -> runner_failed -> completed.
`completed` means durable acquisition artifact, never product success. Store errors
return `io::Error` and do not produce a successful acquisition return. A crash before
commit leaves an incomplete reserved directory; it cannot be silently reused.

A normal exit, non-zero exit, signal or timeout yields INCONCLUSIVE because no
product checker ran. Runner/capture failure yields ERROR (`RunnerCrash` in the
unchanged coarse ExecutionStatus, with the specific reason in runner_failure).
Timeout coverage is partial; spawn/capture failure coverage is failed. Only leader
exit plus both stream EOFs without a capture failure gives complete CLI coverage.
The evidence relates to `process.acquisition`, not unexecuted product claims.

## Schema version decision

P0 and P1A wire schemas, ExperimentPlan, RunContext, Evidence, evaluator and golden
fixtures remain unchanged at version 1. No compatibility fallback is introduced.
New, separate acquisition/bundle/store envelopes start at version **1**, with
`acquisition_schema_version`, `bundle_schema_version`, `store_schema_version`.
They are not the P0 product run-result schema. ProcessObservation is stored as the
existing `Observation::Value`, with direct_runtime trust and observer version 1.
Its output streams are JSON byte arrays, preserving invalid UTF-8 and NUL bytes.
Evidence integrity_hash is JCS SHA-256 of Observation; bundle/manifest hashes cover
whole Evidence records, including run/source/claim identity. The result commit hashes
its whole manifest. Identical recorded inputs/observations serialize identically;
real timestamps and process behavior are not promised identical across runs.

## Store layout and trust boundary

`<root>/<run-id>/result.json` is a canonical commit envelope containing the result,
evidence references/hashes and manifest hash. Evidence lives in
`<root>/<run-id>/evidence/process-observation.json`. The usual root is `.b2ige/runs`.
Run reservation uses exclusive directory creation. Artifact publication writes and
fsyncs a create-new `.pending` file, hard-links without replacement, removes the
temporary name and fsyncs its parent. Result is written last after rereading and
validating evidence. Readers require the result marker, strict canonical bytes,
valid hash/version/run links and every referenced evidence file. Missing, truncated,
noncanonical, corrupted and symlink artifacts fail closed. Existing run/evidence/
result names are never overwritten. No DB, remote storage or auto-cleanup service.

This is a trusted local filesystem boundary; hashes are not authentication. No
protection against a same-user hostile target rewriting the store and hashes or
racing filesystem access is claimed. Root ancestors must be trusted; symlink checks
are not an openat-based sandbox. Pending crash artifacts can remain for inspection.
Atomic publication requires local filesystem hard-link and directory-fsync support;
unsupported storage operations return errors rather than weakening durability.

## CLI observer quality and limitations

- Sees spawn outcome, leader exit code/signal, stream bytes, deadline and acquisition failures.
- Does not see committed business state, network/DB/filesystem operations, or product correctness.
- Byte order within each stream is retained; no total order between stdout/stderr is claimed.
- Nonblocking pipes prevent a full output pipe from deadlocking the runner. Each
  stream has a 1 MiB limit; overflow is explicit failure, never silent complete coverage.
- Deadlines use Instant; start/end use SystemTime as `unix-ns:<decimal>` strings.
  Wall clocks can jump; durations and ordering must not be inferred from wall time.
- SIGKILL is sent to the dedicated Unix process group, and the direct child is reaped.
  EOF cleanup allows 250 ms; failure to close is explicit runner failure. Deliberately
  detached descendants are outside this non-isolated backend. Non-Unix execution
  returns runner failure; Linux is supported by code but only the recorded host was tested.
- Environment inheritance is disabled; stdin is null. Stored environment values are
  clear text. Target revision is caller-declared, not verified executable provenance.
- Inputs are recorded; executable pinning, replay execution, sandboxing and reset are
  not implemented. Replayability is explicitly unavailable with a reason.

## Required review

1. Missing evidence cannot become PASS: this pipeline never emits PASS; store reads fail closed.
2. Observer failures are ERROR, not product FAIL; non-zero target exits remain observations.
3. No differential comparison or byte-identical replay guarantee is introduced.
4. No baseline or approval is changed or automatically produced.
5. No secrecy/isolation claim is made; trusted local targets are required.
6. Plan/config/seed/revision and actual observations are recorded; replay is unavailable,
   and no FAIL or reproducibility claim is emitted.

Validation commands: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build --workspace --release`.
