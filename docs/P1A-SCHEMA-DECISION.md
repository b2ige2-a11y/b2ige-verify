# P1A schema version decision

## 2026-09-14 repair decision (R-01 through R-08)

This is a pre-publication correction of the synthetic harness v1, not a released
protocol migration. **Keep harness_schema_version `1` and the v1 schema ID**;
version `2` remains unsupported. The four P0 artifact schemas and run-result wire
version remain unchanged. This explicit decision supersedes the original harness
snapshot below. Old fixture policies without checker bindings are accepted
structurally but cannot authorize Behavior/BlindTest checkers. No old artifact-only
approval is silently promoted to checker approval.

Schema-visible changes: optional `TrustedPolicy.approved_checker_bindings` (default
empty, fail closed); non-null optional baseline notes/stability_runs, invariant
hidden, and effect eventual_window_ms; explicit safe-integer maxima (2^53-1) for
exact uint64 metadata in the harness schema and semantic validation.
Duplicate coverage/observer-version/budget map keys are rejected during typed raw
deserialization. A previously parsed Value cannot recover discarded duplicate keys.

A checker binding is the RFC 8785 SHA-256 of an object containing
`binding_version: "1"`, `domain: "b2ige.verify.approved-checker"`, `artifact_hash`
(the entire baseline/invariant, including provenance and observation contract hash),
and `claim_hash` (the entire executable Claim, including ID, observer, accepted trust
and predicate). Both artifact approval and the exact binding must be separately
trusted. Checks without bindings cannot produce PASS or FAIL, but do not suppress
an independently authorized violation in another claim. Evaluating or rebinding an
experiment never creates an approval. This binds the approved artifact/checker pair;
it does not infer the meaning of natural-language invariant text or authenticate a human.

P1A SideEffect supports only `correlation_fields: ["idempotency_identity"]` with
the existing fixed provider/operation domain. Other lists are ERROR. Mixed snapshots
retain matching effects, record insufficient domain coverage and can still prove
duplicate commits; they cannot turn an observed violation into PASS.

Replay Available requires valid run metadata and required observer versions as well
as matching plan/config/seed/fault inputs. Target identities must be `sha256:<64 lower
hex>` or `git:<40 or 64 lower hex>`; unresolved names, tags and short refs are not
accepted as replay identities. These remain trusted synthetic content addresses,
not evidence that a real target was fetched or executed.

Canonical bytes and JSON comparison now share serde_json_canonicalizer 0.3.2.
A serde-value validation step preserves/rejects nonfinite floats recursively before
JSON can erase them. Exact metadata integers (seeds, counters, budgets, orders and
windows) outside 0..2^53-1 are rejected, not rounded into the same experiment identity.
Arbitrary finite JSON observation/predicate numbers use JCS binary64 semantics;
applications requiring larger exact numbers must encode them as strings.
All hashes and bindings use this one path. No compatibility fallback to incorrect
serde_jcs hashes is provided; affected artifacts need explicit re-identification and
re-approval outside evaluation.

Checked-in synthetic fixtures are explicitly migrated to content-addressed target
labels and pre-approved bindings. Their target verdict expectations are unchanged.
The error fixture now records unavailable replay metadata because evaluation stops
before declaring a valid experiment. The shared Behavior fixture explicitly approves
the boolean `Equals(true)` and numeric `Equals(1)` controls; the latter permits the
unchanged independent numeric test to exercise `1`/`1.0` equality. `Equals(false)`,
`Equals(2)` and other changes are not approved. This fixed test setup is not an
automatic baseline/invariant approval mechanism. No independent test is edited.

## Original P1A decision (historical)

Date: 2026-09-14

The four P0 JSON Schemas remain unchanged at version 1. Their permissive structural
validation is not a verdict authorization mechanism; semantic conformance checking
is also required.

A new, separately identified `conformance-fixture.v1.json` schema describes the
synthetic harness envelope. `harness_schema_version` is `1`. It does not extend or
silently reinterpret `run-result.v1.json`. The checked-in schema is the contract;
JSON-004 compares it with the Rust-derived schema and fails on drift. Regeneration
requires an explicit schema version decision and review, never automatic acceptance
of a changed snapshot by the test suite.

The envelope contains trusted test policy, a run and the expected evaluated output.
RunContext adds the metadata required by CONTRACTS: tool version, target revision,
platform/runtime, observer versions and start/end timestamps. It also has
`experiment_hash`, an equal alias of existing `plan_hash`; mismatches are ERROR.
Evidence records observer-supplied order and an inline canonical payload with a JCS
SHA-256 integrity hash. Observation discriminants distinguish runtime attempts from
committed-state snapshots. A snapshot is scoped to its claim, source, run, provider,
operation and idempotency identity; complete coverage attests the entire declared
observation domain. External effect IDs deduplicate observations within that domain.

Legacy Behavior and BlindTest provenance enums are deliberately retained:
`unapproved/approved` and `candidate/authoritative/rejected`. Intermediate workflow
states such as recorded/reviewed are not needed for P1A. An approved/authoritative
label alone is insufficient: the entire artifact must match a separately trusted
human/policy authorization hash. Modifying the artifact invalidates the pin. These
pins are synthetic policy inputs, not an implemented authentication system.

The SideEffect v1 schema can represent additional semantics. P1A only checks the
required exactly-once examples. Unsupported semantics are ERROR rather than ignored.
A provider source or explicitly configured authoritative target-state substitute is
required for committed counts. `direct_runtime` cannot establish committed count,
even though the original schema permits describing that trust class.

Replay availability means the controllable inputs are present and bound to the
recorded plan/config/seed/target. An explicit empty fault schedule records no faults;
a missing schedule does not. Fault schedules are bound into config_hash. There is
no replay execution, external-world identity guarantee or reproduction-success claim.
A FAIL includes evidence references and a `run:<id>:replay_inputs` reference into the
envelope; missing replay inputs yield `unavailable` with a reason.

P0 run-result output is created by deterministic evaluation, not a supplied Verdict.
It is checked against the unchanged run-result schema. Invalid RunContext cannot be
serialized into a result artifact. No receipt hash is manufactured; receipts are not
implemented in P1A.
