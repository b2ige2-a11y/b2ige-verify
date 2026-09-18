# Changelog

## 0.3.0 — release candidate (not published)

- Easy Adoption: inspect, idempotent init, prepare, interactive trusted approval, readiness-only doctor and registered identity verification.
- Public adoption workflow benchmark: 9 reproducible scenarios with tooling and bounded historical measurements; no P8 label or baseline changes.
- CI bootstrap: `b2ige ci init`, immutable verifier commit pinning and hardened fail-closed controller generation.
- Distribution: exact-version online and checksum-backed offline installer, safe archive extraction, extracted version checks and fresh install/package smoke.
- Windows x64: source/build plus bounded CLI runtime smoke evidence from V110-C; not VERIFIED_NATIVE and no native archive. Fresh release CI remains pending.
- External pilot framework included; V110-D actual genuine external participant evidence remains DEFERRED. No AI/internal run is external evidence; external adoption is a post-release validation objective.
- macOS unsigned/unnotarized; npm/Cargo unpublished; Linux arm64 deferred. Bounded verification is not exhaustive proof.

See [candidate procedure and release-note draft](docs/RELEASE-0.3.0.md).

## 0.2.0 — released (npm deferred)

- Trust: Trust Lock and the seal/identity/receipt foundation add executable trust boundaries without assigning verdicts.
- Adversarial trust hardening: fail-closed admission, identity, evidence and receipt checks strengthen the bounded verifier boundary.
- Proof and adoption: bounded RealBench/RedBench proof plus adoption and diff-aware verification safeguards reuse verified product execution.
- Qualification and release: verifier qualification, protocol/conformance and release-foundation tooling document the local qualification boundary and completed publication record.
- Final external CI and the final fresh isolated holdout completed before publication. The exact-main GitHub Actions run `35159158561` passed Windows source/build, macOS Apple Silicon, macOS Intel and Linux x86_64 actual-Docker/full benchmark checks. The holdout passed with the sanitized aggregate RealBench 14/14, RedBench/trust 35/35, leakage 6/6, regression 6/6, total 61/61.
- Holdout authoritative checking was deterministic; an LLM did not directly assign product verdicts. The holdout used a dedicated workspace and isolated Docker-in-Docker candidate execution under the same macOS user account; no OS-account-level separation is claimed.
- Npm publication remains deferred/private.

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
