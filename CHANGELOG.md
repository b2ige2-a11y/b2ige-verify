# Changelog

## 0.2.0 — release candidate (not released)

- Trust: Trust Lock and the seal/identity/receipt foundation add executable trust boundaries without assigning verdicts.
- Adversarial trust hardening: fail-closed admission, identity, evidence and receipt checks strengthen the bounded verifier boundary.
- Proof and adoption: bounded RealBench/RedBench proof plus adoption and diff-aware verification safeguards reuse verified product execution.
- Qualification and release: verifier qualification, protocol/conformance and release-foundation tooling make the local candidate stop at an explicit publication boundary.
- Final publication still requires fresh external CI and an independent private holdout; npm publication remains deferred until the `@b2ige` scope is controlled.

## 0.1.0 — released (npm deferred)

- Core: deterministic verdicts, canonical evidence, verified loading and bounded process replay.
- BehaviorSeal (CLI compatibility command: `behavior`): approved before/after comparisons,
  stability, deterministic generation and local counterexample reduction.
- SideEffect Proof: committed SQLite ledger observations under retry, duplicate delivery and
  bounded kill-after-commit schedules; recorded reproduction.
- BlindTest: approved hidden credential/process invariants in attested Docker isolation,
  sanitized Agent results and finite suite-quality receipts.
- Agent/MCP/CI: project registration and readiness, local stdio MCP, protocol v1 and exit guards.
- Bench: independently labeled 31-case v1 corpus; compare rejects version mismatch and recomputes
  semantic hashes from actual content instead of trusting stored hashes.
- Distribution: native/source release archives, release manifest v3, license notice bundles,
  wrapper-only CycloneDX SBOM, fresh-install smoke and checksum/version-enforced npm launcher.
  The final npm package name is `@b2ige/verify`; npm publication is intentionally deferred until
  the `@b2ige` scope is controlled.
- Publication review: public inventory hygiene, cross-platform native CI, security/support policy,
  unsigned macOS release provenance, public release assets and enabled GitHub Private Vulnerability
  Reporting. The GitHub Release is available at
  https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0.

Product version 0.1.0 does not rename independent schema v1/v2/v3 or promise API stability.
