# Final publication checklist — 0.1.0 (released)

Current status: [FINAL-PUBLICATION-DECISIONS](../FINAL-PUBLICATION-DECISIONS.md).
`DONE` records measured completion; `DEFERRED` records an intentional non-publication
decision such as npm.

| Item | State | Evidence / remaining action |
|---|---|---|
| Reviewed product/code commit | DONE | `28315fd`; this final pass changes documentation only |
| P1–P8 semantics, Cargo.lock, evidence schemas and baselines | DONE | No product or verifier logic changed |
| Clean public source inventory and hygiene | DONE | No blocker in the reviewed public candidate |
| License and dependency notices | DONE | Apache-2.0 + Open Core; required notices and SBOM checks passed |
| BehaviorSeal public brand | DONE | Marketing/user-facing name confirmed; `b2ige behavior ...` compatibility CLI retained |
| Official GitHub repository | DONE | `https://github.com/b2ige2-a11y/b2ige-verify`; repository is public |
| Native Ubuntu 22.04 / x86_64 | DONE | Native verified; actual Docker tests and full fixed/reverse benchmark gate PASS |
| Native macOS arm64 | DONE | Native verified; documented partial no-Docker scope PASS |
| Native macOS Intel x86_64 | DONE | Native verified; documented partial no-Docker scope PASS; P3A cleanup-race fix covered |
| Candidate checks | DONE | `34974411043` → SUCCESS |
| Candidate validation and artifacts | DONE | `34991255871` → SUCCESS |
| Release packaging / SBOM / manifest / npm archive checks | DONE | PASS; generated release artifacts were not hand-edited |
| macOS distribution policy | DONE | Unsigned and unnotarized 0.1.0 selected; no Developer ID claim |
| Private Vulnerability Reporting | DONE | Enabled for the public repository |
| npm package | DEFERRED | Final name `@b2ige/verify`; do not publish before actual `@b2ige` scope control |
| `v0.1.0` tag and GitHub Release | DONE | Fixed at final release source commit `57c6977`; [release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0) |
| Public release assets | DONE | Linux x86_64, macOS arm64, macOS Intel archives; platform manifests; SHA256SUMS |
| Final public operation | DONE | Public repository, enabled PVR and released v0.1.0 are complete |

## Signing decision

**Selected: unsigned 0.1.0.** macOS binaries are intentionally unsigned and unnotarized.
Installation documentation must identify the possible Gatekeeper warning and must not claim
Developer ID signing or notarization. No installation script removes quarantine or bypasses
OS controls automatically. Developer ID signing and notarization remain a future release
improvement.

## Status meaning

```text
CLEAN_PUBLIC_REPO_READY=true
TECHNICAL_PUBLICATION_READY=true
PUBLICATION_READY=true
```

`PUBLICATION_READY=true` reflects that the repository is public, Private Vulnerability
Reporting is enabled, and the fixed `v0.1.0` tag/release and release assets are public.
npm remains intentionally deferred pending scope control.
