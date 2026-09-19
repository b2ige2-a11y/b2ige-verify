# Contributing

Use a Rust stable toolchain and a running local Docker Linux engine with the documented base image.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
RUST_TEST_THREADS=4 cargo test --workspace
cargo build --workspace --release
python3 scripts/readme-bench.py
```

Start with the [documentation index](docs/README.md), [architecture](docs/ARCHITECTURE.md),
and the [v0.3.0 final release record](docs/RELEASE-0.3.0-FINAL.md). Read the authority order
in [AGENTS.md](AGENTS.md).
No verdict semantic changes without deliberate contract/schema version review. Missing evidence
must never become PASS; false-PASS repairs take priority. Observer failure is not a product defect.
Never automatically approve a Behavior baseline or let an LLM assign a verdict.

New benchmark cases need independent expected classifications and reviewed source/config identities
in the corpus, a version decision and real execution/reload/reproduction measurements. Never infer
expected labels from candidate output. Keep claims bounded and record unsupported reproduction.

PR checklist:

- Scope stays narrow; contracts and schema decisions recorded.
- Missing evidence, observer failures, nondeterminism and baseline poisoning considered.
- Hidden artifacts unavailable to target workspace; synthetic secrets identified.
- FAIL reproducible from retained inputs, or unavailable reason explicit.
- Relevant tests, full checks and release build pass; prior tests preserved.
- Documentation/examples work; no local paths, runtime artifacts or secrets in the patch.

Do not attach private BlindTest evidence to a public PR. Use the [release record](docs/RELEASE-0.3.0-FINAL.md)
for the current public release; historical release procedures remain in the documentation index.
Support is best-effort community support with no SLA unless a future separate commercial agreement provides one. The public channel remains unconfigured; see SECURITY.md.
