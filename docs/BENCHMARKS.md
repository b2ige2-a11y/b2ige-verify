# Benchmark Methodology — B2IGE Verify Bench v1

Benchmark measurements below refer only to **B2IGE Verify Bench v1 — 31 explicit cases**, on the benchmark corpus.

B2IGE Verify Bench measures an independently labeled finite corpus using the real
product execution and verified load APIs. It does not assign or repair product
verdicts. A benchmark PASS means the measured corpus met its gate; it is separate
from an individual product PASS/FAIL/INCONCLUSIVE/ERROR.

## Versioned framework and corpus

Implementation: `verify_cli::bench` (a module in the current CLI crate). This location
reuses its verified report/Agent projection without circular dependencies. Four
independent JSON schemas are v1: BenchmarkCase, BenchmarkCaseResult,
BenchmarkSummary, BenchmarkRunResult. Existing product schemas remain unchanged.

`benchmarks/corpus/cases.v1.json` defines labels **before execution**. Expected
classification uses its own CORRECT / BUGGY / NONDETERMINISTIC / INCOMPLETE /
INFRA_ERROR_EXPECTED enum. Allowed product verdict sets remain separate. Case
identities bind that independent definition and embedded fixture source; actual
config/executable/image hashes are also recorded. Updating a product output never
rewrites a label, baseline approval, or snapshot.

| Product | Cases | Independently authored expectations |
|---|---:|---|
| Behavior | 9 | Preserving PASS; stdout/stderr/exit/signal changes FAIL; deterministic evolving counter baseline INCONCLUSIVE; stable stderr change despite unstable stdout FAIL; generated-only and reducible bugs FAIL |
| SideEffect | 13 | SAFE normal/retry/delivery/kill+retry PASS; UNSAFE retry/delivery/kill duplicates FAIL; ordered-after, atomic-with and lost effect FAIL; observer/fault incomplete INCONCLUSIVE; corrupted stored evidence ERROR |
| BlindTest | 9 | Correct and leakage probe PASS; mutant A, mutant B and no-op FAIL; partial suite and timeout INCONCLUSIVE; unattested unavailable-image execution ERROR; removed hidden evidence ERROR |
| **Total** | **31** | 7 correct, 16 buggy, 1 nondeterministic, 6 incomplete, 1 expected infrastructure error |

Behavior generation verifies the base input first, generates the same identities
twice, executes the generated suite, and reduces two explicit assignments to the
necessary assignment through the existing reducer. Counter-based instability is
intentional fixture state, unique to the case; it does not depend on lucky random
sampling or timestamp resolution.

SideEffect detection requires actual verified SQLite committed-effect identities
and violation evidence references. Attempts alone never earn duplicate-detection
success. The lost and relationship cases also require the exact violation kind.
All schedules use fresh snapshots. Reduction uses the product's fresh execution
and committed-violation signature, including its local-minimality definition.

BlindTest uses actual immutable Docker images from explicit P6 targets: removed
validation (mutant A), inverted condition (mutant B), no-op and a bounded filesystem /
process-environment leakage probe. Concrete hidden IDs, logical inputs, oracle and
canary are created in private controller storage. The Agent measurement scans the
existing typed allowlist projection for private canary/metadata, sealed path, hidden
case IDs and concrete hidden argument/environment values. No mutation generator is present.
The isolation-unavailable case requests an absent immutable image, so no required
isolation can be attested. Other isolation bypass attacks remain covered by existing
P6 negative tests, not falsely counted as additional P8 corpus cases.

The quality receipt executes a correct control plus both mutants and no-op against
one sealed suite and reloads every child. The benchmark compares the receipt's
observed/expected fields to actual verified child verdicts. It does not accept a
self-reported suite quality boolean alone.

## Measurement definitions

Rates always record numerator, denominator and fractional value. Zero denominator
is JSON `null` and human `N/A`; 0/N remains 0%. Product totals and all four verdict
counts are explicit. A harness execution failure has a null actual verdict and a
harness error, blocks the gate, and is not silently counted as product ERROR.
Late harness failures also clear provisional verdict/reload success, retaining
source references for diagnosis. Planned label denominators are retained when
rows are absent; duplicate rows never inflate counts and always block the gate.
The `measured_verdicts` rate reports completed verdicts / planned cases. Both
`hidden_leakage_measurement` and `agent_leakage_measurement` report measurements
present / planned BlindTest cases. Zero leakage with zero measurements is **not**
evidence of no leakage. Read false-verdict rates together with measured coverage
and blockers; unexecuted cases establish neither correctness nor false verdicts.

- **False PASS:** non-correct cases observed PASS (BUGGY, NONDETERMINISTIC,
  INCOMPLETE or expected infrastructure error). Denominator: non-correct cases.
  Known-bug and known-unsafe false PASS are also reported separately.
- **False FAIL:** CORRECT cases observed FAIL / CORRECT cases. ERROR and
  INCONCLUSIVE are not called false FAIL, but wrong expected verdicts block the gate.
- **True bug detection:** BUGGY cases observed FAIL / BUGGY cases. ERROR never kills
  a mutant. **Correct acceptance:** CORRECT PASS / CORRECT cases.
- INCONCLUSIVE and ERROR rates: observed verdict count / selected case count.
- **Evidence validity:** schema and product evidence checks passed / completed
  artifacts checked. **Verified reload:** execute/load equality / completed valid
  artifacts. Loader errors and absent measurements block the gate.
- The two intentional missing/corrupt-store controls first execute, validate and
  reload an intact artifact, then remove required evidence and require product and
  report loader rejection. The recorded ERROR describes this loader boundary;
  no successful product artifact is forged. Intact validation and negative evidence
  rejection are separate measurements. These now-invalid artifacts do not count as
  completed *valid* artifacts expected to reload after corruption.
- **Observation coverage:** Behavior complete measured observables/pairs; SideEffect
  complete schedules; BlindTest complete hidden cases. Overall aggregation counts
  these product-specific units, not a common completeness probability.
- **Requested fault coverage:** fully evidenced actions / requested schedule actions.
  A schedule with an incomplete fault supplies no confirmed complete action set.
- **Committed confirmation coverage:** verified initial/final committed-state
  observation availability, including evidence of absence; never request count.
- **Mutation adequacy on benchmark corpus:** killed explicitly named known mutants /
  total named known mutants (**2** in v1). No-op is measured as a known bug, separately.
  This is not a general mutation score or proof of suite completeness.
- Setup and execution durations are recorded per case; shared Docker build time is
  charged to the first BlindTest case. Total elapsed includes framework work.
  Timings are measurements, not release thresholds.

## Reproduction and reduction

Each FAIL records existence, verified reproduction, replay availability/result,
reduction availability, local-minimality status, original/reduced size and references.

| Product/path | Measured semantics |
|---|---|
| Exact/generated Behavior | Fresh recorded input pair preserves divergence observable signature; separate P1 process replay reproduces exact streams/status |
| Stability Behavior | Fresh candidate against the recorded profile preserves stable divergence signature; unstable bytes are not claimed reproducible |
| Behavior reducer | P3 assignment deletion with verified final signature; local neighborhood only; size unit is explicit assignments |
| SideEffect | Fresh SQLite/reset and retained fault schedule preserve committed violation signature; size unit is schedule actions; no automatic replay CLI |
| BlindTest | Trusted rerun of the identical private suite/image preserves hidden predicate violations; exact private inputs retained; no reducer or automatic replay CLI |

Unsupported/unmeasured replay and reduction values are null/unavailable, not false
0% or claims of a common cross-product replay guarantee. Local minimization is not
global minimality; BlindTest reproduction is never called minimal.

## Release gate (blocking)

- Complete, nonempty independent inventory; exactly one result per case; no foreign,
  duplicate, missing or altered case definitions; no harness errors or mismatches.
- Known bug false PASS = 0; known unsafe SideEffect false PASS = 0.
- Missing required evidence PASS = 0; partial hidden suite PASS = 0;
  isolation unattested PASS = 0.
- Correct/safe-control false FAIL = 0; correct BlindTest false FAIL = 0.
- Evidence/schema validity = 100%; verified reload of completed valid artifacts = 100%.
- Hidden canary leakage = 0; Agent hidden field/value leakage = 0. Missing leakage
  measurements for completed PASS/FAIL Docker cases also block the gate.
- Required generator/noise/detection/reduction/quality-receipt measurements must
  exist and pass; known failures in this corpus must reproduce with verified evidence.
- Partial product or case selections cannot grant the full release gate. A complete
  selected product has a separately labeled scoped gate.

This is the first release gate on a **small, explicitly constructed corpus**. It is
not exhaustive, representative of all production workloads, or an external-product
ranking. Performance is not a blocker in v1.

## Artifacts, comparison and self-checks

See [benchmarks/README.md](../benchmarks/README.md) for runnable commands, temporary
store retention and the fixed/reverse local runner. Snapshots use canonical JSON and
SHA-256; CLI JSON can be pretty printed. Canonical integrity is not host authentication.
Semantic comparison requires matching corpus/selection identities and ignores clocks,
paths and randomized hidden identities, retaining measured semantic differences.
Comparison labels are `unchanged`, `improvement`, `regression`; mixed or otherwise
unclassified semantic changes conservatively require regression review. Timing alone
cannot make a regression or improvement. Snapshot references require their original
private local stores for live product revalidation.

Harness self-checks include wrong expected label on an actual preserving process,
known false-PASS candidate, missing result, duplicate result, zero suite, incomplete
execution, missing evidence, benchmark verdict tamper, rehashed product verdict tamper,
invalid JSON enum/unknown fields and time-insensitive comparisons. Full integration
tests execute all three products' corpus; metrics-only mocks are insufficient.

Required development and release verification:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
RUST_TEST_THREADS=4 cargo test --workspace
cargo build --workspace --release
python3 benchmarks/run-local.py /absolute/new-output-directory
```

Baseline location: [baseline-v1](../benchmarks/baseline-v1/).
The committed baseline directory contains the measured fixed/reverse snapshots and
canonical hash; its bounded results are not a substitute for product evidence.

## Future external comparison

Schemathesis, Keploy, equiv and TestSprite may be evaluated in future work with
appropriate workload/contract alignment. This benchmark installs none of these
products and requires no paid API keys. Their execution is not a benchmark gate
dependency. External-product comparison, hosting, publishing and marketing remain
out of scope for this methodology.
