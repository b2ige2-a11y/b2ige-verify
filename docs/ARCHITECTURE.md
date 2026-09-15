# Architecture

## Core flow

`ExperimentPlan -> Preflight -> Environment Reset -> Execute -> Observe -> Normalize -> Check -> Reduce -> EvidenceBundle -> Verdict -> Receipt`

## Components
- Discovery: target/runtime/spec detection.
- Runner: process lifecycle, timeouts, environment control, deterministic seeds.
- Isolation: selectable execution boundaries.
- Observers: HTTP, CLI, filesystem, SQLite, PostgreSQL for initial scope.
- Generator: deterministic boundary/property/stateful candidate generation.
- Noise model: identifies baseline instability; never suppresses a difference without recorded rationale.
- Checker: deterministic contract evaluation.
- Reducer: minimizes reproductions while preserving failure.
- Replay: re-runs the recorded experiment plan.
- Evidence store: canonical machine-readable artifacts.
- Receipt: hashes identities and result artifacts.

## Product adapters
- Behavior: compares baseline and candidate observations under equivalent experiment plans.
- SideEffect: schedules faults and validates committed-effect contracts.
- BlindTest: runs sealed verifier plans against an agent-produced target.

## Architectural constraint
Products may add domain semantics but may not redefine core verdict meanings.

## P6 BlindTest

`public config + sealed approved suite -> canonical path/integrity checks -> immutable
Docker image -> inspect-before -> bounded P1 attach capture -> inspect-after -> private
runtime evidence -> host checker -> verified loader -> Human / Agent projections`.

The target receives only its current input and no host mounts. Private full evidence
and optional actual-corpus quality receipts remain in controller storage. See
[BLINDTEST.md](BLINDTEST.md). P7 integration is not part of P6.
