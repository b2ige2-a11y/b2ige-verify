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
