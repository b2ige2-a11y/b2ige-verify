# V100-1 — Seal + Identity + Receipt Foundation

Bind verification intent and identities before candidate implementation.

Required design surface:
task identity, verifier identity, candidate identity, environment identity,
evidence identity, pre-implementation seal, mutation detection, receipt-ready binding.

Do not build the full presentation/signing layer yet.
Missing seal-critical evidence or identity mismatch must never become PASS.

Risk: trust-critical.
Primary implementation model: Astra / Medium.
Critical final audit only: Astra / xhigh.

V100-1A boundary: independent Task Seal v1 binds only canonical task, verifier,
and declared environment-contract identities, supplied before candidate identity
exists and without deriving from candidate implementation. Its domain-separated
commitment provides deterministic content binding, not independently witnessed
chronology, authentication, or actual environment attestation. Local existence
cannot prove when a seal was created. See `docs/CONTRACTS.md` for the version and
commitment contract.

V100-1 local implementation: `verify_core::sealed_run` provides mandatory
trusted seal/execution/candidate approval, a receipt-ready identity bundle,
transitive verified source binding, recorded runtime linkage, exclusive execution
reservation, and pinned verified reload. Existing product loaders retain verdict
authority. The independent v1 version/commitment contract is in CONTRACTS.md.
TaskSeal v1 is preserved. Legacy P0–P9 results do not imply a sealed V100 claim.
Receipt presentation/signing remains deferred.

Validation and limitations are recorded in STATE.md. Independent external CI and
private holdout evidence cannot be manufactured by the implementation worker;
the outer deterministic runner owns the full workspace gate.

Local trust review:
1. Missing seal/approval/source identities cannot return a VerifiedReceipt; a
   runtime identity is mandatory for PASS and product coverage checks still apply.
2. Binding/load failures are ERROR-boundary errors; observer/product verdicts are
   preserved and an integrity failure is never mislabeled a product FAIL.
3. Reload hashes recorded data in deterministic store/plan order; it does not
   rerun targets or compare fresh clocks, process IDs, or nondeterministic output.
4. Execution identity pins Behavior authorization and baseline; no production
   helper approves or updates them after candidate changes.
5. P6 private path checks apply before receipt reservation; no new public raw
   evidence projection exists. Host-user compromise remains excluded.
6. Receipts reference the original verified product experiments and complete
   source inventory. Existing recorded replay inputs and replay-unavailability
   reasons remain authoritative; the receipt does not invent replay guarantees.
