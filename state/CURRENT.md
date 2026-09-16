# Current State

Date: 2026-09-16
Phase: P11 AGENT DISTRIBUTION (local asset gate complete)
Status: DISTRIBUTION ASSETS VALIDATED — EXTERNAL PUBLICATION NOT PERFORMED

- P11 local gate: MCPB packaging/integrity/reproducibility, Official Registry validation,
  portable Cursor manifest schema, Codex plugin validator, shared skill and distribution
  checks completed. [P11 result](../P11-RESULT.md) · [P11 scope](../tasks/P11-INDEX.md).
- Cursor live-client import and public submission remain pending. Codex local manifest
  is validated; public-directory eligibility for local execution needs OpenAI review.
- No push, release asset upload, registry/marketplace submission or npm publish.
  Existing product code, schemas, baselines, v0.1.0 tag/binaries and root README unchanged.
- P10 commit `13539e165a5170003162ec0c2015af4537e61573` was not applied.

## P9 public product release (preserved)

CLEAN_PUBLIC_REPO_READY=true
TECHNICAL_PUBLICATION_READY=true
PUBLICATION_READY=true

- Reviewed product/code commit: `28315fd`; final release source commit: `57c6977`. This
  post-release pass changes documentation/readiness records only; product code, verifier
  semantics, schemas, benchmark baselines and test logic were not changed.
- Candidate checks run `34974411043` and candidate validation/artifacts run `34991255871`
  both completed with SUCCESS.
- Native verified: Ubuntu 22.04 / `x86_64-unknown-linux-gnu`, macOS arm64 and macOS Intel
  `x86_64`. Linux actual Docker tests and the full fixed/reverse benchmark gate PASS.
  macOS documented partial no-Docker scope PASS.
- Release packaging, SBOM, manifest and npm archive checks PASS. Existing generated release
  artifacts, SBOMs and checksums were not hand-edited.
- The previous P3A signal fixture flake was a fixture cleanup race; the fix was included in
  the final native Intel CI PASS.
- BehaviorSeal is the public Behavior brand. The compatibility CLI remains `b2ige behavior ...`;
  internal module/schema/API names remain unchanged. License remains Apache-2.0 + Open Core.
- Official repository: https://github.com/b2ige2-a11y/b2ige-verify. It is public. The fixed
  `v0.1.0` tag and [GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0)
  were created from final release source commit `57c6977`.
- Public release assets include Linux x86_64, macOS arm64 and macOS Intel archives, platform
  manifests and SHA256SUMS.
- macOS 0.1.0 is intentionally unsigned and unnotarized; no Developer ID signing or
  notarization claim is made. Gatekeeper warning is possible; signing is a future improvement.
- Final npm package name is `@b2ige/verify`; npm publication is intentionally deferred until
  the `@b2ige` scope is actually controlled. It is not a GitHub 0.1.0 blocker.
- `SECURITY.md` remains. GitHub Private Vulnerability Reporting is enabled.
- `PUBLICATION_READY=true` reflects the completed public repository conversion, PVR activation,
  fixed `v0.1.0` tag/release and public release assets. npm publication remains intentionally
  deferred until the `@b2ige` scope is controlled.

[Final decisions](../FINAL-PUBLICATION-DECISIONS.md) · [Checklist](../release/checklist.md) ·
[Readiness](../PUBLIC-REPO-READINESS.md) · [Manifest](../release/release-manifest.json).
