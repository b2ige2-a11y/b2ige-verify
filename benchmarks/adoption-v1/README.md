# V110 Real-World Adoption Bench v1

This is **non-authoritative V110 UX/adoption evidence**, separate from B2IGE Verify
Bench/P8. It measures the real source-checkout path:

`inspect → init → prepare → pre-existing trusted inputs → trust approve → doctor → verify ID → report`

It tests bounded reproducibility against expectations inherited from the reviewed
P8 corpus. It does **not** establish new independent correctness labels, third-party
project compatibility, external-user usability, real hidden secrecy, exhaustive
correctness, or market success. External project/operator evidence belongs to V110-D.
CI bootstrap remains `DEFERRED_TO_V110_C`; this does not implement `b2ige ci init`.
The historical v0.2.0 archive does not include V110-A; use this source checkout.

## Reproduce

Prerequisites: Unix, Rust, Python 3.9+, Node.js, and a local Linux Docker engine.
The Dockerfile pins an existing local Node image by its exact content identity.
The runner refuses a missing image and never pulls a mutable tag. Provision the
pinned image in your controller environment before measuring; see
`projects/docker-credentials/Dockerfile`. No paid API, service, credentials,
telemetry or LLM is used. Normal installed compilers/runtimes and image layers may
be reused; adoption state and evidence may not.

From the repository root, choose two **new, real (not symlinked)** output paths:

```sh
python3 scripts/adoption-bench.py ./adoption-run-1 --build
python3 scripts/adoption-bench.py ./adoption-run-2 --build --reverse
python3 scripts/adoption-bench.py --compare ./adoption-run-1/result.json ./adoption-run-2/result.json
python3 scripts/test-adoption-bench.py
```

`--build` measures the locked source workspace and benchmark fixture helper build
separately. Without it, first build both:

```sh
cargo build --workspace --release --locked
cargo build -p verify-cli --release --locked --example adoption_fixture
```

Each run reserves its output directory exclusively, then creates a temporary HOME,
project/controller lanes and fresh evidence stores. Preexisting output, symlinked
roots, source mutation and authority mismatch fail closed. It never deletes a
user output directory to retry. Temporary private controller evidence is destroyed
at completion, including on exceptions; only the allowlisted measurement summary
and report survive. Keep output directories outside the public source inventory.
Failed/incomplete scenarios remain in the planned denominator.
The runner refuses output under the adoption corpus or either protected P8 tree
before creating anything. Reverse execution retains the canonical planned inventory
in summaries. Comparing two identical blocked runs still exits 3.

## Semantic authority and fixture reuse

`scenarios.json` binds each of nine scenarios to an exact P8 source ID, product,
classification, allowed verdicts and SHA-256 of `benchmarks/corpus/cases.v1.json`.
The tooling separately pins the manifest bytes. It rejects missing/duplicate IDs,
changed source bytes, changed inherited labels, product mismatches and constructor
changes. Updating these pins is a deliberate corpus revision, never a runtime
operation. No result file is a source for expected classifications.

The standalone Cargo **example**, `adoption_fixture`, includes these unchanged
pre-V110-B sources directly:

- `crates/verify-mcp/tests/support/behavior.rs`: exact `printf same` reference,
  approved baseline, checker binding and explicit stability policy.
- `crates/verify-mcp/tests/support/sideeffect.rs`: disposable SQLite schema,
  safe/unsafe transaction, retries and durable committed-state observer.
- `benchmarks/corpus/blindtest.rs`: P8 credential suite, invariants and approvals.

It relocates paths, recomputes physical fixture/candidate identities, and copies
existing authorization. It does not execute candidates or derive trust from their
outputs. New Rust, Node and Python candidates implement the preserving/stdout
cases; the reference is unchanged. SQLite normal/unsafe/corrupt scenarios reuse
the reviewed transaction. Docker candidates reproduce P8 correct/mutant predicates;
partial scope uses P8's existing one-case budget. The corrupt scenario measures
verified report rejection after evidence removal, as P8 does.

The P8 BlindTest constructor uses fresh case IDs, logical times and canaries.
Those are controller-only physical inputs, not new expected semantics. They are
never exported or normalized into authority. The public fixture is not secret
from a reader of this repository. Same-user unrestricted host access is outside
the secrecy boundary.

## Benchmark terminal replay

`_replay_fixture_confirmation` is private benchmark tooling, with no standalone
arbitrary-command entry point. It only accepts a pinned scenario in a fresh
benchmark layout and replays the fixed `APPROVE bench` decision. All trust inputs
are materialized from existing constructors **before candidate execution**. The
helper never evaluates whether a candidate deserves approval. Primary and negative
approval decisions are fixed before verification. Replay requires an active disposable
benchmark root, the fixed source-built CLI/command, unchanged reviewed input bytes
and environment, fresh controller destinations, and no candidate result. There is
no arbitrary-command parameter. Production approval
still requires stdin and stdout TTYs; no flag, environment, MCP or Agent bypass
was added. Negative-control approvals use separate disposable controller locations.

## Measurements and interpretation

The independent tooling schema is string version `2`, kind
`adoption-benchmark-tooling`, `authoritative: false`. It is validated in Python and
is never a product schema or input to verdict loaders. Real CLI controls confirm
that `verify`, `report`, and all three product preparation paths refuse it as source
evidence/configuration, preserving registry and authorization bytes.

Version decision: v2 adds `verification_exit_code`, `report_verdict`,
`report_exit_code`, and `measured_outcome_stage` to distinguish an execution verdict
from a subsequent loader rejection. V1 measurements are not silently upgraded.
The scenario manifest remains v1; no authoritative product/P8/V100 schema changes.

Each row records source bindings, product/runtime, actual verdict/exit, attempted
and completed stages, one trust checkpoint, undocumented JSON edits, recovery
observations, first blocking stage, first verification success, sanitized Agent
validation and controller separation. A first verification is successful when it
returns a source-backed PASS, FAIL or INCONCLUSIVE, independently of whether the
product passes. Doctor can never satisfy this metric. For the corrupt scenario,
first verification is intact (PASS/0); the final measured outcome is report-loader
ERROR/3. The human table shows both outcomes and their measured stage. Normal rows
must preserve verdict/exit agreement between verify and report. All nine rows remain
in completed/first-verification denominators, including the loader-ERROR scenario.

Report output is compared with the original sanitized verification projection.
BlindTest's public opaque source alias is resolved only in the controller; private
run identifiers never enter the summary. The existing CLI uses a generic Behavior
hint on report-load ERROR; this bounded limitation is recorded, not repaired by
changing product code or reclassifying the outcome.

All rates contain numerators and denominators; zero denominators are null/N/A.
Unexpected blocks cannot disappear. Recovery is N/A when none was attempted.
Source preparation and individual stage timings are observational only. Comparison
ignores **only** preparation seconds and per-stage durations. It retains inventory,
expectations, verdicts, evidence status, stages, negative controls, environment,
binary identities and aggregate counts. No speed threshold gates results.

Public environment observations use bounded version formats; raw version output,
arbitrary environment strings and non-finite numeric fields are refused. Entire
Agent transports, including ERROR responses, must survive the real protocol checker
without silent fallback or dropped fields.

`test-adoption-bench.py` runs two fresh full journeys in forward/reverse order and adversarial validator
checks. Runtime controls exercise missing evidence, unapproved/poisoned/stale
baselines, failed/missing observers, attempt-only output, stale executable,
malformed/duplicate registries, readiness, overlap/symlinks, source/label tampering,
omitted/crashed scenarios, stale/corrupt output, PTY replay restrictions, protected
output paths, malformed Agent responses and exit-zero coercion. P8 corpus/baseline
inventories are hashed before and after every execution.

See [measured report](../../docs/V110-ADOPTION-BENCH.md). These are public project
archetypes, not third-party projects or external operator qualification.
