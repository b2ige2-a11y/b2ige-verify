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
