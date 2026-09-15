# P4 — Evidence / Result Experience

Status: COMPLETE / LOCAL GATE PASS (2026-09-15)
Prerequisite: P3 LOCAL COMPLETE, recorded in the preceding state/CURRENT.md.

- [x] ReportDocument v1: four verified product loaders, source identity/hash,
  projection version, verdict/outcome, expected/observed, reproduction, coverage,
  evidence/run references, limitations, replayability and raw artifact reference.
- [x] Human terminal + allowlisted compact AgentReport v1. Projection never assigns
  authoritative verdicts. Reducer status stays separate from final comparison verdict.
- [x] `b2ige report`: ID/store-path input, human/json/agent, --open,
  --store, --authorization. Verdict exit codes 0/1/2/3; argument misuse 64.
- [x] Bundled local HTML: primary failure first, collapsed other suite failures,
  evidence/runs/coverage/limitations/raw, recorded and locally minimized reproduction.
- [x] Verified navigation: reducer → suite/final comparison → suite children.
  Loopback capability URL, Host validation, no-store, CSP, escaped content; each
  navigation revalidates the root and selected artifact. No arbitrary file endpoint.
- [x] Integrity tests: source missing/corrupt, suite child corrupt, forged report,
  mismatched authorization, path evidence missing, HTTP root deletion.
- [x] Output tests: all four verdicts, stability, suite failures, reducer MINIMIZED
  and BUDGET_EXHAUSTED, deterministic compact output, schemas, terminal/HTML escaping,
  progressive disclosure, CLI exit codes and misuse.
- [x] Existing 206 tests preserved; 22 new = 228 passed, 0 failed/ignored.
- [x] cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings;
  RUST_TEST_THREADS=4 cargo test --workspace; cargo build --workspace --release.
- [x] Final renderer tests rerun after browser fixes: 22 passed. Bundled frontend
  typecheck/lint/build are Rust compiler/clippy/release build; no JS pipeline.
- [x] agent-browser smoke: real --open release URL; 1280px desktop and 390px mobile,
  suite → child navigation, native disclosure, no console errors, no forms,
  no horizontal overflow; expanded evidence/raw axe check: 0 violations/incomplete.

Review: missing evidence cannot yield a report; observer ERROR stays distinct from
FAIL; no new nondeterminism rules, baseline writes, or verifier-secret products;
recorded experiments and unavailable paired replay remain explicit. Hashes are not
authentication; trusted local boundary remains. No generator/reducer semantics changed.

P5 may be planned after this gate; P5 is NOT STARTED. No commit/push/deploy.
See [P4-RESULT.md](../P4-RESULT.md) and [CLI usage](../crates/verify-cli/README.md).
