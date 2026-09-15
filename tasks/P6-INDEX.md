# P6 — BlindTest MVP (single phase)

Scope: Requirement → approved structured invariant → sealed hidden suite → actual
Docker execution → host oracle → verified evidence/verdict → Human/Agent report/UI.
No P6A/P6B/P6C split. P7, commit, push and deployment are outside this task.

## Implementation

- [x] Requirement v1 and executable invariant v2; P0 v1 preserved.
- [x] Approved requirement/artifact/checker bindings; candidate/reviewed rejected.
- [x] Approved, hash-pinned suite/manifest/cases and host-only oracle.
- [x] Canonical sealed/workspace/store separation; symlink/hard-link protection.
- [x] Immutable Linux Docker image, args/env/fixture policies and public source identity.
- [x] No host mounts/network; non-root, read-only, caps/security/resource bounds.
- [x] Actual before/after Docker inspect attestation and complete process capture.
- [x] Host-side six predicates/AND; full hidden coverage required for PASS.
- [x] Actual evidence storage and verified-load recomputation.
- [x] Separate sanitized Agent and trusted full Human projections; no agent hidden details.
- [x] Doctor / verify / validate-suite / report / local viewer integration.
- [x] Actual correct / two mutants / no-op / probing Docker corpus.
- [x] Actual bounded self-validation receipts; optional ordinary-suite quality evidence.
- [x] False-PASS and canary leakage regression tests.
- [x] Threat model, architecture, evidence and schema version decisions documented.

## Final gate (recorded in P6-RESULT and state/CURRENT)

- [x] `cargo fmt --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=4 cargo test --workspace` (273 original tests preserved)
- [x] `cargo build --workspace --release`
- [x] Actual Docker integration and release CLI/report corpus
- [x] Agent leakage/false-PASS metrics zero
- [x] agent-browser desktop + 390px mobile, navigation, details, console, accessibility
- [x] P6 completion recorded; P7 remains unstarted

[Contract and operation](../docs/BLINDTEST.md) · [Threat model](../docs/THREAT-MODEL.md)
