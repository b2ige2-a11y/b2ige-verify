# License Review Result

> Historical result. Current publication decisions supersede open items below: [final readiness sweep](PUBLICATION-READINESS-RESULT.md).

- **Applied license:** Repository code is Apache License 2.0. The business
  model is Open Core; current Core, CLI, BlindTest, Behavior, SideEffect Proof,
  MCP, Skill, and Bench are in the OSS scope. Future team, enterprise, or
  managed features may be separate commercial offerings.
- **Modified manifests:** Workspace `Cargo.toml` and all six public crate
  manifests now resolve to `Apache-2.0`; `npm/b2ige/package.json` declares the
  same license. Cargo `publish = false`, npm `private`, and the prepublish
  guard remain. No repository URL was added.
- **NOTICE:** No root `NOTICE` was added. The review found no present need to
  create one; third-party license/attribution obligations still apply.
- **Dependency licenses:** See [dependency-license-review](release/dependency-license-review.md).
  No GPL/AGPL package was found in the locked Cargo metadata; artifact-level
  third-party notice/SBOM review remains **REVIEW REQUIRED**. The npm wrapper
  has no dependencies.
- **Trademarks:** [TRADEMARKS.md](TRADEMARKS.md) separates B2IGE, B2IGE Verify,
  BlindTest, BehaviorSeal (tentative), and SideEffect Proof from the code
  license without asserting registration or exclusivity.
- **Closed publication blocker:** public license selection.
- **Remaining blockers:** final naming collision clearance; publication
  authorization; macOS Intel/Linux verification; signing, notarization, and
  provenance; security reporting/support policy; npm hosting/binary integrity;
  Git history secret scan; and dependency license `REVIEW REQUIRED`.
