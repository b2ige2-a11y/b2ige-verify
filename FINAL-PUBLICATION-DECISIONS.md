# Final publication decisions — 0.1.0

This records the final release state for B2IGE Verify 0.1.0. The release was
created from final release source commit `57c6977`. The product code, verifier
semantics, schemas, benchmark baseline and test logic are unchanged by this
post-release documentation pass.

## Confirmed owner decisions

1. **BehaviorSeal** is the public brand name for the Behavior product. Marketing
   and user-facing documentation use BehaviorSeal. The compatible CLI command
   `b2ige behavior ...` and internal module, schema and API names remain unchanged.
2. The official GitHub repository for 0.1.0 is
   [b2ige2-a11y/b2ige-verify](https://github.com/b2ige2-a11y/b2ige-verify).
   A future organization move is not a 0.1.0 blocker.
3. The license remains **Apache-2.0 + Open Core**.
4. macOS 0.1.0 is an **unsigned, unnotarized** binary release. No Developer ID
   signing or notarization claim is made. Gatekeeper warnings are expected and
   signing is a future release improvement.
5. The final npm package name is **`@b2ige/verify`**. It must not be published
   until the `@b2ige` scope is actually controlled. npm is intentionally deferred
   and is not a GitHub 0.1.0 release blocker.
6. `SECURITY.md` remains in the repository. GitHub Private Vulnerability Reporting
   is enabled for the public repository.

## Technical verification

Reviewed product/code commit: `28315fd`. This documentation handoff does not alter
that code commit.

| Evidence | Result |
|---|---|
| Candidate checks run `34974411043` | SUCCESS |
| Candidate validation and artifacts run `34991255871` | SUCCESS |
| Native verified targets | Ubuntu 22.04 / `x86_64-unknown-linux-gnu`; macOS arm64; macOS Intel `x86_64` |
| Linux scope | Actual Docker tests and full fixed/reverse benchmark gate PASS |
| macOS scope | Documented partial no-Docker scope PASS on native arm64 and Intel CI |
| Release checks | Packaging, SBOM, manifest and npm archive checks PASS |
| P3A signal fixture | Fixture cleanup race fixed; final native Intel CI PASS |
| Final release source | `57c6977` |
| Public GitHub Release | [B2IGE Verify 0.1.0](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0) |
| Public release assets | Linux x86_64, macOS arm64, macOS Intel archives; platform manifests; SHA256SUMS |
| GitHub Private Vulnerability Reporting | Enabled |

The existing generated release artifacts, SBOMs and checksums are not hand-edited
by this task.

## Publication state

```text
CLEAN_PUBLIC_REPO_READY=true
TECHNICAL_PUBLICATION_READY=true
PUBLICATION_READY=true
```

`PUBLICATION_READY=true` records that the public release operations are complete:

- the official repository is public;
- GitHub Private Vulnerability Reporting is enabled;
- the fixed `v0.1.0` tag and GitHub Release are public with their release assets; and
- npm publication remains intentionally deferred until the `@b2ige` scope is controlled.

## Post-release state

The official repository is public, the `v0.1.0` tag remains fixed at the released
snapshot, and the GitHub Release and release assets are available at the link above.
npm remains deferred until its scope prerequisite is satisfied. This documentation
commit does not modify the tag, GitHub Release, generated release artifacts, or npm.
