# P8 baseline v1 — measured local snapshot

Benchmark measurements below refer only to **B2IGE Verify Bench v1 — 31 explicit cases**, on the benchmark corpus.

Captured from the actual release binary on 2026-09-15, macOS aarch64,
Rust 1.98.1, Docker Engine 29.5.2. This is an observed measurement, **not** an
expected-result golden or an automatic update to any Behavior baseline.

- `result.json`: fixed-order BenchmarkRunResult v1 (canonical JSON).
- `canonical.sha256`: canonical SHA-256 of that fixed-order result.
- `report.txt`: human report generated from the same run.
- `benchmark-run-result.v1.schema.json`: machine schema used by the runner.
- `reverse-result.json`: independent fresh reverse-order execution.
- `comparison.json`: all 31 cases unchanged; semantic hashes match.

Both full release gates passed. Evidence/schema and verified reload checks occurred
inside the benchmark. Intentionally removed evidence was separately rejected.
Raw product evidence and sealed inputs remain in the private temporary controller
roots. P9 explicitly removed nonsemantic local result/reproduction path references
from both published snapshots and recomputed the canonical content hash. Corpus labels,
metrics and semantic hashes are unchanged. Live evidence references are not distributed; this directory is not a portable evidence bundle or host-authenticated
certificate. It contains no raw hidden suite, oracle, input inventory or canary.

Re-execution uses a new output directory; never overwrite this baseline implicitly.
See [methodology](../../docs/BENCHMARKS.md) and [P8 result](../../P8-RESULT.md).
