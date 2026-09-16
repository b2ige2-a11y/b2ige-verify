# V100-3 — Proof

Prove value before adding broad new machinery.

Expand reproducible RealBench and RedBench evidence using bounded methodology.

Track explicitly:
false PASS, false FAIL, detection, reproduction, leakage and limitations.

Prefer historical/realistic failures and actual AI-coding scenarios.
Do not manufacture favorable denominators.
Do not represent bounded results as exhaustive correctness.

Risk: normal to advanced.
Primary model: Luna / Max.
Use Astra / Medium only for difficult analysis.

## Local implementation package

The public proof corpus reuses the real product execution and verified loaders;
it does not replace them with metric-only mocks. The 31 independently labeled
cases cover executable Behavior programs, a SQLite committed-effect fixture,
and the existing P6 Docker corpus. The cases include preserving behavior,
observable process regressions, controlled noise, generated/reduced failures,
retry/fault/relationship side-effect failures, missing evidence, hidden
mutants, partial/timeout/unavailable execution, and bounded leakage probes.

The benchmark controller now keeps false-verdict denominators tied to the
planned independent case inventory even when result rows are missing, ignores
duplicate rows for measured numerators while blocking the run, and rejects
comparison snapshots whose corpus, selection, or case identities do not match.
Stored summaries and semantic hashes remain derived; product verdicts still
come only from verified product loaders. No schema version or P0-P9 meaning
changed.

Late harness failures clear provisional verdict/reload success while retaining
diagnostic references. Availability rates expose unmeasured verdicts and leakage;
missing measurements cannot be interpreted as a clean run. Docker preflight
distinguishes engine access, base-image availability, and repository digest
failures without downloading anything. The existing real Docker test remains
mandatory and unchanged. `benchmarks/run-local.py --product` provides an explicit
fixed/reverse scoped runner; its default still requires the entire corpus.

## Observed local evidence

The unchanged historical `benchmarks/baseline-v1/` snapshot records a prior complete
fixed/reverse corpus run: 31/31 cases, false PASS 0/24 non-correct cases,
false FAIL 0/7 correct cases, bug detection 16/16, verified reproduction 16/16,
hidden leakage 0, Agent private-value leakage 0, and named-mutant adequacy
2/2. These are bounded corpus measurements, not historical-bug, user, or
holdout denominators. The V100-2 adversarial tests additionally exercise
forged target results, rehashed evidence, missing hidden material, fixed-input
canary overlap, and query-boundary controls.

Current worker measurements (2026-09-16, macOS/aarch64) executed each public
non-Docker corpus in fresh fixed and reverse runs. The two orders are repeated
experiments on the same cases, not twice as many independent cases.

| Scope | Measured verdicts | False PASS | False FAIL | Bug detection | Verified reproduction |
| --- | --- | --- | --- | --- | --- |
| Behavior | 9/9 | 0/8 | 0/1 | 7/7 | 7/7 |
| SideEffect | 13/13 | 0/9 | 0/4 | 6/6 | 6/6 |
| BlindTest | 0/9 | Unmeasured | Unmeasured | Unmeasured | Unmeasured |

Both measured product gates passed in both orders, with recomputed semantic
equality; each overall release gate stayed blocked. No new Docker leakage or
named-mutant adequacy measurement is available. BlindTest leakage measurement
availability is 0/9 for both hidden and Agent projections, not evidence of zero
leakage. These are constructed realistic process/SQLite cases, not newly found
historical bugs, actual external AI-agent usage, or independent holdout outcomes.

Reproduction commands (each destination must be new):

```sh
python3 benchmarks/run-local.py /private/tmp/b2ige-v100-3-behavior-proof --product behavior
python3 benchmarks/run-local.py /private/tmp/b2ige-v100-3-sideeffect-proof --product sideeffect
```

Each output directory retains `fixed/result.json`, `reverse/result.json`, their
canonical hashes and `comparison.json`. Raw source stores remain at their recorded
temporary references; OS cleanup can remove them. Fixed/reverse semantic hashes:

- Behavior: `sha256:b5fdc4f87da62f2003f0919d358947b08718712ddd5ac1195bf11344e98a9a1a`
- SideEffect: `sha256:e2bfe83c583200282dbef3e11841ff8aa2ffd51ffce75ac03f2c2aa8bcc8e424`

Targeted checks performed:

- `cargo test -p verify-cli --lib bench::tests --locked`: 20 passed.
- `cargo test -p verify-cli --test benchmark --locked -- --skip actual_docker_benchmark_corpus`: 3 passed, including unavailable-engine reporting.
- `cargo test -p verify-core --test adversarial_query --locked`: 6 passed.
- `cargo test -p verify-core --test blindtest adversarial_fixed_input_cannot_disclose_canary --locked`: 1 passed.
- `cargo clippy -p verify-cli --all-targets --locked -- -D warnings`: passed.
- `cargo build --workspace --release --locked`: passed via the scoped runner.
- `cargo fmt --check` and `git diff --check`: passed.
- `cargo test -p verify-cli --test benchmark actual_docker_benchmark_corpus --locked`: **failed**, all nine cases blocked at local-engine preflight. Direct Docker CLI access returned permission denied for the local socket. No product verdict was fabricated.

The Docker test is not ignored, relaxed, or replaced by the unavailable-engine
test. The outer runner still needs an accessible local Linux Docker engine and
the documented local digest-bearing Node image. The full workspace test was not
run by this worker. Independent external CI/private holdout and the final phase
gate remain unproven; this report does not advance the Current phase or decide
acceptance.

## Limitations

The corpus is small, explicit, and public; it is not exhaustive, statistically
representative, a generalized mutation score, or proof of production
correctness. Docker leakage absence is bounded canary/probe evidence inside the
documented boundary, not perfect information-flow secrecy. Same-user host
access, controller compromise, kernel/runtime escape, and private holdout
qualification remain outside this phase. FAIL replay/reduction claims retain
the product-specific availability and local-minimality limits recorded in the
benchmark result.
