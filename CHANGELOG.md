# Changelog

## 0.1.0 — pre-publication (GitHub Release pending; npm deferred)

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
- Distribution: native/source candidate archives, release manifest v3, license notice bundles,
  wrapper-only CycloneDX SBOM, fresh-install smoke and checksum/version-enforced npm launcher.
  The final npm package name is `@b2ige/verify`; npm publication is intentionally deferred until
  the `@b2ige` scope is controlled.
- Publication review: public inventory hygiene, cross-platform native CI, security/support policy,
  and unsigned macOS release provenance. The GitHub publication action remains pending.

Product version 0.1.0 does not rename independent schema v1/v2/v3 or promise API stability.
