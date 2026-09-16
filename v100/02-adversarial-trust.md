# V100-2 — Adversarial Trust

Strengthen resistance to verifier reading, verifier/evidence tampering,
result forgery, hidden-data leakage, repeated-query oracle extraction,
and boundary confusion.

Claims must remain inside docs/THREAT-MODEL.md.
Do not claim same-host-user secrecy that is not actually provided.

Risk: trust/security-critical.
Primary implementation model: Astra / Medium.
Critical audit: Astra / xhigh only when the security boundary changes.

## Local implementation package (2026-09-16; acceptance pending)

- Added independent v1 hidden-query Policy and persistent Ledger in
  `crates/verify-core/src/sealed_run/query.rs`. Trusted controller admission binds
  task seal/suite, revalidates retained policy pins, and exclusively reserves a
  durable attempt before sealed execution. Concurrent requests, interrupted slots,
  new candidates/run IDs and verifier errors do not reset/refund the budget.
  Ordinary execution/CLI/MCP remain unmetered; controllers claiming a bound must
  expose only this admission route and retain the same ledger/pin.
- Reused authoritative sealed/product loaders for verdicts. Added full rehashed
  policy/evidence substitution attacks and real ERROR receipt reload coverage;
  existing Behavior seal/verifier/source-identity attacks remain intact.
- Rejected private canary in fixed target input, disabled image healthchecks, and
  checked explicit proc protection sets in actual before/after inspect evidence.
  Added rehashed actual-loader attacks against both stages and missing fields.
- Extended the real public Docker probe with root/proc writes, socket discovery
  and a bounded TCP attempt; added a target printing forged PASS JSON and writing
  its own result file. These attacks cannot supply authoritative host evidence.
- Extended public-label filtering for literal suite IDs, fixture names and case
  environment keys; expanded the real Agent projection regression.
- Recorded independent schema v1 decisions and exact compatibility/boundary
  limitations in CONTRACTS, VERDICTS, EVIDENCE, BLINDTEST and THREAT-MODEL.
  Existing P0–P9 names/schemas and V100 receipt/seal schemas remain unchanged.

## Targeted validation evidence

| Command | Observed result |
| --- | --- |
| `cargo test --offline -p verify-core --test adversarial_query` | 6 passed, including concurrent admission, restart, fully rehashed policy/evidence substitution, strict wire parsing, scope/zero budget, and real verifier ERROR receipt reload |
| `cargo test --offline -p verify-core --test behavior_differential sealed_` | 5 passed; real seal/identity/candidate tampering and product verdict preservation |
| `cargo test --offline -p verify-core --test trust_lock` | 8 passed |
| `cargo test --offline -p verify-core --test blindtest adversarial_fixed_input` | 1 passed |
| `cargo test --offline -p verify-core --test blindtest` | Attempt before final added forgery test: 24 passed, 35 failed at Docker initialization; no Docker execution evidence obtained |
| `cargo test --offline -p verify-cli --test blindtest_reports malicious_public_labels_and_run_names_are_sanitized` | Failed at Docker initialization, before projection assertions |
| `cargo check --offline -p verify-core -p verify-cli --all-targets` | Passed, including final added integration tests |
| `cargo build --offline --release -p verify-core -p verify-cli` | Passed |
| `git diff --check` | Passed |

Docker diagnostic: `docker version --format '{{json .}}'` reported Server null
and permission denied connecting to the local user's Docker Desktop Unix socket.
No permission bypass or test skip was introduced. The outer runner must execute
the final Docker integration tests (core BlindTest and CLI BlindTest reports),
including the new forged-result target, real probes, rehashed inspect attacks and
bounded successful execution/reload. Compilation is not runtime proof.

## Required verifier review / remaining limits

1. Missing critical policy/source/runtime evidence still rejects or produces the
   original ERROR/INCONCLUSIVE; admission alone never enables PASS.
2. Policy exhaustion and observer failure are verifier errors, not product FAIL.
3. Query accounting is exclusive persistent reservation, independent of scheduling;
   network/probe observations remain bounded and require actual Docker attestation.
4. No baseline or approval pin is created/refreshed by execution.
5. Controller files, pins, ledger and Human view must remain outside candidate
   authority. No secrecy against an unrestricted same-host user, rollback-proof
   storage, encoded information-flow protection or exhaustive extraction resistance.
6. Existing product counterexamples/replay-unavailability semantics are preserved;
   admission records contain no invented product counterexample or verdict.

New strict Docker observations can reject historical v1 artifacts that lack them;
no stored evidence is upgraded or rewritten. A Receipt v1 does not attest query
admission; the separate controller ledger must be retained for that operational
claim. Independent external CI/private holdout evidence is unavailable in this
worker context and has not been invented. Full workspace tests, phase advancement,
commit/push/merge and acceptance remain with the outer deterministic runner and
independent audit. `v100/STATE.md` Current phase is unchanged.
