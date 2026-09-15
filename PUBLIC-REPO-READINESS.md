# B2IGE Verify — Public repository readiness

```text
CLEAN_PUBLIC_REPO_READY=true
TECHNICAL_PUBLICATION_READY=true
PUBLICATION_READY=true
```

This is the post-release status for the official
[b2ige2-a11y/b2ige-verify](https://github.com/b2ige2-a11y/b2ige-verify) repository.
The repository is public and the [v0.1.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0)
was created from final release source commit `57c6977`. The reviewed product/code
commit is `28315fd`; this post-release pass changes documentation only.

## Source and policy

- Product code, verifier semantics, schemas, benchmark baselines and test logic are unchanged.
- BehaviorSeal is the public Behavior brand. The CLI compatibility command remains
  `b2ige behavior ...`, with internal `behavior` module/schema/API names unchanged.
- Licensing is Apache-2.0 + Open Core.
- The final npm package name is `@b2ige/verify`. Its publication is intentionally deferred
  until the `@b2ige` scope is actually controlled; it is not a GitHub release blocker.
- macOS 0.1.0 is intentionally unsigned and unnotarized. No Developer ID claim is made;
  Gatekeeper warnings may appear.
- `SECURITY.md` remains. GitHub Private Vulnerability Reporting is enabled.

## Technical gates

- Candidate checks run `34974411043`: **SUCCESS**.
- Candidate validation and artifacts run `34991255871`: **SUCCESS**.
- Native verified: Ubuntu 22.04 / `x86_64-unknown-linux-gnu`, macOS arm64 and macOS Intel
  `x86_64`.
- Linux actual Docker tests and full fixed/reverse benchmark gate: **PASS**.
- macOS documented partial no-Docker scope: **PASS**.
- Release packaging, SBOM, manifest and npm archive checks: **PASS**.
- The previous P3A signal fixture flake was a fixture cleanup race; it was fixed and final
  native Intel CI passed.

The existing generated release artifacts, SBOMs, checksums and release manifest were not
hand-edited in this documentation pass.

## Public release state

`PUBLICATION_READY=true` records that the repository is public, Private Vulnerability
Reporting is enabled, and the fixed `v0.1.0` tag/GitHub Release and release assets are
public. The release includes Linux x86_64, macOS arm64 and macOS Intel archives, platform
manifests and SHA256SUMS. npm publication remains intentionally deferred until scope control.
