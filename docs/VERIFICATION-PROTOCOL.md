# Verification Protocol v1

Protocol v1 freezes the existing local V100 sealed execution boundary. It is not
Agent Protocol v1 (CLI/MCP presentation), a network protocol, or a rename of P0–P9.
CONTRACTS.md, VERDICTS.md and EVIDENCE.md remain authoritative in that order.
This specification adds no product wire fields or verdict semantics.

## Controller lifecycle

1. Independently establish task, verifier and environment-contract identities before
   candidate identity exists. Retain the TaskSeal v1 commitment outside candidate
   control. Local hashes cannot witness this chronology.
2. Independently approve Authorization v1 after candidate creation. It binds the
   retained seal, complete Execution and candidate identity. Never generate approval
   from candidate/model claims, repair a baseline, or refresh pins automatically.
3. Invoke `verify_core::sealed_run::execute` with a fresh receipt ID. It validates
   approval before reservation/execution, invokes the existing product and verified
   loader, records the complete transitive evidence inventory, publishes a receipt,
   and reloads it before exposing `VerifiedReceipt::verdict()`.
4. Retain the resulting receipt commitment independently. Historical
   `sealed_run::load` requires both it and Authorization, repeats product verification
   and recomputes identities. Never fall back to legacy PASS after rejection.

Execution v1 supports exact Behavior, SideEffect and BlindTest only. Other legacy
execution paths remain supported under their own contracts; they do not acquire
sealed claims. Agent/MCP reports cannot be deserialized into VerifiedReceipt.

## Wire and commitment rules

TaskSeal, Authorization, IdentityBundle and Receipt use string `schema_version: "1"`.
Missing required, duplicate and unknown fields are rejected. `runtime_identity`
is required but nullable, and can be null only when runtime evidence is absent
and the product verdict is not PASS. Source identity maps reject duplicate keys.
Execution is tagged by `product` (`behavior`, `sideeffect`, `blindtest`). Nested
product structures retain their own version and validation rules.

Identities are `sha256:` followed by 64 lowercase hex digits, computed with RFC 8785
canonical JSON. TaskSeal uses domain `b2ige.verify.task-seal.v1` and member `seal`;
execution/candidate/evidence-inventory/runtime/receipt use their exact
`b2ige.verify.<name>.v1` domains and member `value`. Source reads use domain
`b2ige.verify.source-read.v1`, `run_id`, `result`, and `evidence`. The exact preimages,
candidate descriptors and runtime inputs are specified in [CONTRACTS.md](CONTRACTS.md).
JSON key order and whitespace do not change commitments; altered bound content does.
Canonical store encoding requirements still apply to stored artifacts.

Receipt contains only version, receipt ID, derived source run ID, full seal,
execution and identity bundle. It contains no authoritative cached verdict.
Receipt evidence is `derived`, not new runtime observation. Valid JSON or a valid
hash alone is not verification. Retaining a receipt without its transitive source
store is insufficient for reload. Missing/corrupt sources or identity/pin mismatch
produce an ERROR-boundary `io::Error`, never a product FAIL or cached PASS.

## Verdict, replay and disclosure

The product loader alone assigns PASS / FAIL / INCONCLUSIVE / ERROR. Existing CLI
exit codes remain 0 / 1 / 2 / 3; integration usage errors are separate. PASS means
required evidence and checkers completed with no violation in declared bounded
scope. Scope, coverage, budget and limitations remain in the product result.
FAIL retains the product counterexample and replay path, or explicit replay
unavailability with reason. Receipt reload rechecks recorded observations; it
does not execute replay, certify the current runtime, or promise byte-identical
external-world behavior. Observer failure is not product failure.

Receipts and source stores are trusted-controller artifacts, not a new public
projection. Local content binding does not establish signatures, timestamps,
independent chronology, environment attestation or reproducible-build provenance.
BlindTest retains the actual Docker inspect and private-store requirements of
[THREAT-MODEL.md](THREAT-MODEL.md). Same-user host compromise, kernel/runtime escape,
perfect secrecy and exhaustive correctness are outside the claim.

Optional query admission v1 requires the controller's one independently pinned
ledger and `Ledger::execute`. Receipt v1 alone does not certify admission.
Unmetered legacy/sealed routes must not be exposed as bypasses by that controller.

See [conformance coverage](../conformance/V100-PROTOCOL.md) and the
[local release procedure](V100-RELEASE.md). Neither establishes private holdout proof.
