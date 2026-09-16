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

Remaining V100-1 work: subsequent candidate/evidence identity linkage and
receipt-ready binding, including integration of seal mismatch/missing-evidence
checks at the authoritative boundary. Receipt presentation/signing remains
deferred; V100-1A introduces no verdict or execution integration.
