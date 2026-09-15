# B2IGE Verify 0.1.0 — Publication Readiness Result

Final pre-publication status for the reviewed source at code commit `28315fd`.
This documentation pass does not change product code, verifier semantics, schemas,
benchmark baselines or test logic.

```text
CLEAN_PUBLIC_REPO_READY=true
TECHNICAL_PUBLICATION_READY=true
PUBLICATION_READY=false
```

## Technical result

| Check | Result |
|---|---|
| Candidate checks `34974411043` | SUCCESS |
| Candidate validation and artifacts `34991255871` | SUCCESS |
| Native targets | Ubuntu 22.04 / `x86_64-unknown-linux-gnu`, macOS arm64, macOS Intel `x86_64` verified native |
| Linux verification | Actual Docker tests and full fixed/reverse benchmark gate PASS |
| macOS verification | Documented partial no-Docker scope PASS |
| Release verification | Packaging, SBOM, manifest and npm archive checks PASS |
| P3A signal fixture | Cleanup race fixed; final native Intel CI PASS |

The existing release manifest, generated release artifacts, SBOMs and checksums were
not manually modified.

## Final distribution decisions

- The public Behavior brand is **BehaviorSeal**. The compatibility CLI remains
  `b2ige behavior ...`; internal module, schema and API names are unchanged.
- 0.1.0 targets [the official GitHub repository](https://github.com/b2ige2-a11y/b2ige-verify).
- Licensing remains **Apache-2.0 + Open Core**.
- macOS 0.1.0 is unsigned and unnotarized. Developer ID signing/notarization is
  not claimed; Gatekeeper warnings are possible and signing is deferred to a future release.
- The final npm package name is `@b2ige/verify`, but npm publication is deferred until
  the `@b2ige` scope is actually controlled. It does not block GitHub 0.1.0.
- `SECURITY.md` remains. Private Vulnerability Reporting is planned immediately after
  the repository becomes public and is not active at this stage.

## Why publication is not ready

The remaining state is the result of public operations not yet being performed:

1. The repository is still private.
2. Private Vulnerability Reporting is not activated.
3. The `v0.1.0` tag and GitHub Release do not yet exist.
4. npm is intentionally deferred pending scope control.

No push or publication was performed by this task.
