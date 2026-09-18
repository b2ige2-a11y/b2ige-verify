# B2IGE Verify Bench v1

Benchmark measurements below refer only to **B2IGE Verify Bench v1 — 31 explicit cases**, on the benchmark corpus.

P8 is one phase. The benchmark controller is `verify_cli::bench`, alongside the
existing verified report loader, so no circular crate dependency or duplicate
presentation/verdict implementation is needed.

## Run locally

Prerequisites: Rust, a running local Linux Docker engine, and the already available
`node:24.18.1-bookworm-slim` image (or `B2IGE_P6_BASE_IMAGE` naming a local image with
an immutable repository digest). No implicit pull, paid API, or competitor install.

```sh
cargo build --workspace --release
target/release/b2ige bench
target/release/b2ige bench behavior
target/release/b2ige bench sideeffect
target/release/b2ige bench blindtest
target/release/b2ige bench --output json --save /absolute/new-snapshot-directory
target/release/b2ige bench --reverse --output json --save /absolute/another-new-directory
target/release/b2ige bench compare /absolute/new-snapshot-directory/result.json /absolute/another-new-directory/result.json
```

`--case behavior.stdout` selects one case. A selected case cannot grant the full
release gate. Product commands return 0 only for their complete scoped product
gate; their JSON overall release gate remains blocked because other products were
not run. Full benchmark exit: 0 passed, 1 blocked, 3 harness/setup error; misuse 64.
An individual fixture's expected FAIL does **not** make its benchmark fail.
`--save` only creates a new directory; no snapshot is automatically updated.
`b2ige bench schemas NEW_DIRECTORY` exports the four v1 JSON schemas.

The SideEffect fixture executable defaults to `p5-effect-fixture` beside `b2ige`.
`B2IGE_BENCH_EFFECT_FIXTURE` can specify an absolute alternate build; its actual
executable hash is recorded. Build the entire workspace, not just the CLI.

Run both orders and compare, with no automatic baseline overwrite:

```sh
python3 benchmarks/run-local.py /absolute/new-output-directory
```

For reproducible local evidence when Docker is unavailable, explicitly select a
product in a fresh directory for each run:

```sh
python3 benchmarks/run-local.py /absolute/new-behavior-proof --product behavior
python3 benchmarks/run-local.py /absolute/new-sideeffect-proof --product sideeffect
```

These execute actual cases in both orders and compare recomputed semantic hashes.
They require their scoped gates and require the overall release gate to stay
blocked. They do not skip or satisfy the Docker integration test. The default
command still requires all 31 cases. Preflight distinguishes local-engine access,
local base-image availability and missing repository digest; it never pulls an
image. A Docker-capable outer runner must satisfy these prerequisites separately.

The script builds release binaries, runs all 31 cases in both orders, and requires
both full release gates plus equal semantic hashes. It does not run the development
quality gates; run the four commands in `docs/BENCHMARKS.md` for those.

## Storage and reproducibility

- `corpus/cases.v1.json`: independently authored labels, expected verdict sets,
  categories and bounds. These are never generated from observed verifier output.
- `corpus/blindtest.rs`: P6 trusted constructor reused for explicit Docker mutants,
  fresh private suite/canary generation, plus bounded Docker build capture.
- `baseline-v1/`: measured P8 snapshot, schemas, human report and canonical hash.
- Every run creates a new mode-0700 controller directory in the OS temporary area.
  Every case gets fresh scripts, workspace, source SQLite database, store and reset
  state. Every BlindTest case gets a fresh sealed suite and public workspace;
  actual executions use fresh inspected containers. Only immutable image layers
  are reused, never evidence or another case's result.
- Raw hidden suites, Docker evidence and reproduction inputs remain in the private
  controller directory. Benchmark snapshots contain counts, hashes and references;
  they do not embed hidden inputs, oracles or canaries. Human benchmark reports are
  also sanitized. This does not protect against a same-user host process.
- Product result references remain reloadable while that controller directory
  exists. A baseline JSON is a historical measurement, not a portable evidence
  bundle or authenticated certificate. Do not delete its referenced directory
  while you need live evidence verification. Temporary roots may be removed by OS
  cleanup. No raw hidden artifacts are copied into this repository.
- Semantic hashes exclude time, physical paths, per-run hidden randomness and
  runtime artifact hashes; those remain recorded separately. They retain source
  and case definition identities, verdicts, coverage, reduction sizes and metrics.
  This permits cross-run comparison without pretending byte-identical Docker or
  SQLite histories. Different corpus hashes cannot be compared automatically.

Product comparisons, fault schedules and reductions are finite (at most 32 product
execution units per case; each unit has its product-specific child bounds). Process
limits are 5 seconds, with 200 ms for the intentional BlindTest timeout fixture;
Docker controller setup is bounded by the existing product's 30 seconds per call,
and corpus image builds by 120 seconds per Docker command. These are operational
bounds, not performance release thresholds. Keep `RUST_TEST_THREADS=4` for host
contention when running the full tests.

V110 adoption workflow measurements are separately versioned in
[adoption-v1](adoption-v1/README.md); they do not alter P8 labels or baselines.
