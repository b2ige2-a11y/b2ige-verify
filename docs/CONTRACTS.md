# Core Contracts

Schema version starts at `1`.

## ExperimentPlan
Must identify:
- plan id/version
- target revision(s)
- seed
- required observers
- optional observers
- reset strategy
- time budget
- exploration budget
- checker contract
- isolation mode

## RunContext
Must record:
- run id
- tool version
- schema version
- platform/runtime
- target revision
- config hash
- plan hash
- seed
- observer versions
- start/end timestamps

## ObservationCoverage
Per observer:
- `complete`
- `partial`
- `unavailable`
- `failed`

Each record includes reason and whether that observer is verdict-critical.

## Counterexample
Must contain:
- violated contract id
- expected
- observed
- minimal reproduction when available
- evidence references
- replay reference

## Verdict
Exactly one of:
- PASS
- FAIL
- INCONCLUSIVE
- ERROR

## Exit codes
- 0 PASS
- 1 FAIL
- 2 INCONCLUSIVE
- 3 ERROR

## PASS contract
PASS is legal only if:
1. the experiment executed according to plan;
2. all verdict-critical observers satisfy their minimum coverage requirement;
3. every authoritative checker completed;
4. no violation was found in the declared explored space.

## Replay contract
Replay guarantees reuse of recorded controllable experiment inputs: plan, seed, fault schedule, normalized configuration, and target identity when available.
Replay does NOT promise external-world byte identity.
Replay result must say whether the original failure/result reproduced.

## P5 SideEffect / P4 projection version decision

SideEffectContract, SideEffectHistory, SideEffectExperimentResult and FaultScheduleResult
are independent v1 artifacts. Attempt identities and provider committed-effect identities
are distinct. Their contract, observer, fault execution, history and load verification
requirements are defined in [SIDEEFFECT.md](SIDEEFFECT.md). Unsupported semantics/configs
fail closed. P0/P1/P2/P3 artifact contracts are unchanged.

ReportDocument, AgentReport and report projection move to v2 for SideEffect support and
product-neutral expected/observed values. Prior v1 schemas are retained separately.
These exports remain non-authoritative and cannot be loaded as source artifacts.

## P6 BlindTest version decision

BlindTest adds independent v1 requirement/suite/case/oracle/config/isolation/result/
quality artifacts and executable InvariantArtifact **v2**. P0 invariant v1 remains
unchanged. ReportDocument/AgentReport/projection move to v3; prior v1/v2 schemas are
retained. P0–P5 authoritative verdict/evidence contracts are unchanged.

[BLINDTEST.md](BLINDTEST.md) defines the approved checker binding, sealed storage,
Docker inspect requirements, complete hidden coverage and verified-load semantics.
Missing hidden evidence, partial hidden suites or unattested isolation cannot PASS.

## P7 Agent integration version decision

Agent Protocol response/request v1 and project registry v1 are independent integration
contracts. Existing ReportDocument/AgentReport v3 and P1–P6 authoritative artifacts and
verdict semantics remain unchanged. CLI legacy `--output agent` remains v3; explicit
`--protocol 1` and MCP use the new v1 projection. Integration invokes existing execution
and verified loaders, never assigns verification verdicts. Doctor PASS is readiness only.
See [AGENT-PROTOCOL.md](AGENT-PROTOCOL.md), [MCP.md](MCP.md), and [CI.md](CI.md).

## P8 Benchmark version decision

BenchmarkCase, BenchmarkCaseResult, BenchmarkSummary and BenchmarkRunResult are
independent **v1** measurement artifacts. `Classification` is a separate enum from
product Verdict. P1–P7 authoritative schemas, checker semantics and report schemas
remain unchanged. The `verify_cli::bench` module invokes public execution and verified
loaders; it cannot correct or override a product verdict. A missing/corrupt artifact
produces a measured verified-loader ERROR boundary, never a fabricated product run.
See [BENCHMARKS.md](BENCHMARKS.md) for denominators, source identities and release gates.

## P9 packaging version decision

Product/package version remains 0.1.0 (local release candidate). The new release manifest is
independent schema v1 and does not change any authoritative product/evidence/report schema.
Benchmark comparison rejects benchmark/schema/corpus mismatches and recomputes canonical semantic
content hashes; stored hash fields are not comparison authority. P1–P8 verdict semantics remain unchanged.

## V100-1A independent Task Seal v1 version decision

Task Seal is an independent V100 artifact with string `schema_version: "1"`.
It does not rename or reinterpret P0–P9 artifacts or change existing schemas,
verdicts, evidence requirements, or LLM authority restrictions.

Its only other fields are `task_identity`, `verifier_identity`, and
`environment_contract_identity`. These are identities of the task, verifier, and
declared environment contract, produced using the existing RFC 8785 canonical JSON
SHA-256 rules (`verify_evidence::canonical_hash`), encoded as `sha256:` plus exactly
64 lowercase hexadecimal digits. The trusted caller must establish these inputs
before candidate identity exists; none may contain or derive from candidate
implementation identity. Syntax validation alone cannot establish their provenance.

The seal identity/commitment is `canonical_hash({"domain":
"b2ige.verify.task-seal.v1", "seal": <entire TaskSeal>})`. Validation rejects
unsupported versions and malformed identities; commitment computation validates
first. Missing, duplicate, and unknown wire fields are rejected. Every bound-field
mutation either fails validation or changes the commitment. Mutation detection
requires comparison with an independently retained commitment; this does not
authenticate replacement of both the artifact and its commitment.

Local hashing establishes deterministic content binding only. Seal existence does
not prove creation time or independently witnessed pre-candidate chronology,
authentication, actual environment conformance, or correctness. A valid seal
assigns no verdict and cannot substitute for verdict-critical evidence.
V100-1A adds no candidate/evidence binding, signatures, timestamps, attestations,
receipt presentation, CLI, network service, or cloud feature.

## V100-1 sealed execution and receipt foundation version decision

V100 adds independent string-version **v1** `sealed_run::Authorization`,
`IdentityBundle`, and `Receipt` contracts. TaskSeal v1 and all existing P0–P9
schemas/terms remain unchanged. `Execution` is a tagged member of Receipt v1,
covering existing exact Behavior, SideEffect, and BlindTest execution. Other
legacy products/projections do not thereby acquire a V100 sealed claim.

The authoritative V100 entry points are `verify_core::sealed_run::execute` and
`load`. A `VerifiedReceipt` is not deserializable or externally constructible;
its verdict comes only from the original product's verified loader. Raw receipts,
identity bundles, and their derived storage evidence never assign a verdict.
Legacy entry points retain their existing contracts; callers requiring V100 must
use this boundary and must never fall back to a legacy PASS after its rejection.
No CLI/report/signing surface is introduced by this foundation.

The trusted controller must retain the pre-candidate TaskSeal commitment outside
candidate control. After candidate implementation it independently approves an
Authorization containing the seal commitment, execution identity, and candidate
identity. This approval associates the complete execution with the sealed task,
verifier and environment contract; hashes alone cannot establish that semantic
approval or the provenance of the seal inputs. The execution identity includes
all configuration and, for Behavior, the unchanged baseline/checker authorization.
Production code never creates, repairs, or refreshes these approval pins.

All new commitments use RFC 8785 `canonical_hash({"domain": D, "value": V})`:

| Identity | Domain D | Value V |
| --- | --- | --- |
| Execution | `b2ige.verify.execution.v1` | Entire tagged Execution |
| Candidate | `b2ige.verify.candidate.v1` | Array `[product, descriptor]` |
| Evidence | `b2ige.verify.evidence-inventory.v1` | Sorted map of source run IDs to source-read identities |
| Runtime | `b2ige.verify.runtime.v1` | Ordered recorded runtime objects described below |
| Receipt | `b2ige.verify.receipt.v1` | Entire Receipt |

Candidate descriptors are `{executable, input}` for Behavior (actual executable
byte hash and canonical input identity), `{executable}` for SideEffect, and
`{image, workspace, build}` for BlindTest. BlindTest requires an immutable
`sha256:` image ID, not a mutable tag; the verified image must match it.
Execution identity additionally binds the complete target configuration. Original
product execution/load checks remain responsible for executable/snapshot/runtime
binding. Historical reload does not require live source files or rerun targets.

Each source-read identity is `canonical_hash({"domain":
"b2ige.verify.source-read.v1", "run_id": ID, "result": RAW_RESULT,
"evidence": ALL_STORE_VERIFIED_EVIDENCE})`. The evidence array uses store order
(sorted evidence IDs). A scoped store records every successful load, including
transitive acquisitions, reduction/quality references, and their complete raw
results and evidence. Repeated reads of one run must agree. Thus a root result
hash alone cannot substitute for the transitive inventory.

Runtime identity binds Behavior's verified before/after RunContexts; SideEffect's
verified primary attempts (`identity`, `process`, `runtime`) in schedule order;
or BlindTest's verified image and complete executions including Docker
attestations. Runtime evidence in other transitive reads is also covered by the
evidence inventory. A runtime identity may be null only when runtime evidence is
absent and the product verdict is not PASS. The declared environment-contract
identity is copied from the seal, separately from observed runtime identity;
recording a platform or runtime hash does not attest environment conformance.

Receipt includes its schema version, receipt ID, derived `<receipt-id>-source`
run ID, full seal, execution, and IdentityBundle. The bundle includes its version,
seal commitment, task/verifier/environment-contract/candidate/execution/evidence/
runtime identities, and the source inventory. Unknown, missing required, and
ambiguous duplicate wire fields are rejected (runtime identity is nullable).
Stored artifacts must satisfy the existing canonical store encoding rules.

Execution checks Authorization before reservation or target execution. Receipt
reservation is exclusive; retries cannot reuse a completed or partial receipt ID.
A receipt is published only after verified product load and read back before
returning a verdict. Reload requires both the independently retained Authorization
and the independently retained receipt commitment, validates the seal and links,
repeats product verification, and recomputes the entire bundle. Missing/corrupt
seal, identity, source, or runtime evidence, unsupported versions, and identity
mismatches produce an ERROR-boundary `io::Error`, never PASS or an invented product
FAIL. Receipt removal cannot turn a V100 load into legacy verification.

This is deterministic local content binding. It adds no independently witnessed
chronology, signatures, authenticated controller identity, external timestamps,
cloud attestation, reproducible-build proof, or exhaustive correctness proof.
