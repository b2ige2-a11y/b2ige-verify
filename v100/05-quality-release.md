# V100-5 — Verifier Qualification + Protocol + Release

Use measured weaknesses from Proof to improve verifier qualification.

Add only justified techniques such as:
mutation adequacy, property testing, boundary generation, metamorphic checks.

Then stabilize:
- Verification Protocol v1
- conformance suite
- receipt semantics
- reproducible public benchmark
- threat-model documentation
- release gate

Do not build a fully automatic Verifier Forge unless evidence proves it is required.

Risk: advanced/trust-critical.
Primary model: Astra / Medium where needed.
Final release audit: Astra / xhigh.

## Local release foundation

Verification Protocol v1 is consolidated in `docs/VERIFICATION-PROTOCOL.md`, with
the executable requirement map in `conformance/V100-PROTOCOL.md`. Existing TaskSeal,
Authorization, IdentityBundle, Receipt, product and Agent Protocol schemas retain
their versions and semantics. Local gate diagnostics add an independent tooling v1
report only. No verdict-authority, product breadth or isolation level is added.

Qualification follows V100-3's measured availability weakness: 124 deterministic
negative variants over the public 31-row measurement-controller test vector preserve
planned denominators and block incomplete execution. The positive control and row
permutation check prevent a reject-everything implementation from qualifying.
Receipt wire coverage checks all top-level fields of four actual execution
artifacts, plus canonical key-order/whitespace invariance. Existing hash/version,
identity mutation, rehashed evidence, query-budget and original-verdict tests remain.
These are public regression cases, not new real bugs or independent mutants.

`scripts/v100-release-gate.py` runs the bounded local check inventory offline,
retains source/log hashes and refuses failed, missing or zero-test checks. It
cannot advance beyond WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT. The external runner
still owns full workspace tests, fresh Docker/full public fixed/reverse benchmarks,
archive/SBOM/native smoke, external CI and independent audit/private holdout.

Native packaging now includes the conformance map alongside protocol and release
documentation; archive validation requires those documents. Source packaging
already includes gate scripts and tests through the reviewed public inventory.
The macOS partial CI scope now includes TaskSeal/trust-lock/query tests; Docker
requirements remain explicit and unchanged. Historical release claims are scoped
to their historical commits instead of implicitly granting V100 readiness.

Validation commands and exact results are emitted by the local gate outside the
repository. A source change invalidates that run's source binding. No private
holdout access, baseline rewrite, commit, push, publication or independent proof
is part of this implementation. Final local state remains
WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT, subject to deterministic gates and audit.
