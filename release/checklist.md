# Final publication checklist — 0.1.0

Current handoff: [FINAL-PUBLICATION-DECISIONS](../FINAL-PUBLICATION-DECISIONS.md).
`DONE` records measured completion; `PUBLICATION_READY=false` records public actions
that have not yet happened.

| Item | State | Evidence / remaining action |
|---|---|---|
| Reviewed product/code commit | DONE | `28315fd`; this final pass changes documentation only |
| P1–P8 semantics, Cargo.lock, evidence schemas and baselines | DONE | No product or verifier logic changed |
| Clean public source inventory and hygiene | DONE | No blocker in the reviewed public candidate |
| License and dependency notices | DONE | Apache-2.0 + Open Core; required notices and SBOM checks passed |
| BehaviorSeal public brand | DONE | Marketing/user-facing name confirmed; `b2ige behavior ...` compatibility CLI retained |
| Official GitHub repository | SELECTED | `https://github.com/b2ige2-a11y/b2ige-verify`; repository remains private until publication |
| Native Ubuntu 22.04 / x86_64 | DONE | Native verified; actual Docker tests and full fixed/reverse benchmark gate PASS |
| Native macOS arm64 | DONE | Native verified; documented partial no-Docker scope PASS |
| Native macOS Intel x86_64 | DONE | Native verified; documented partial no-Docker scope PASS; P3A cleanup-race fix covered |
| Candidate checks | DONE | `34974411043` → SUCCESS |
| Candidate validation and artifacts | DONE | `34991255871` → SUCCESS |
| Release packaging / SBOM / manifest / npm archive checks | DONE | PASS; generated release artifacts were not hand-edited |
| macOS distribution policy | DONE | Unsigned and unnotarized 0.1.0 selected; no Developer ID claim |
| Private Vulnerability Reporting | PENDING_PUBLICATION | Activate and verify immediately after the repository becomes public; not active now |
| npm package | DEFERRED | Final name `@b2ige/verify`; do not publish before actual `@b2ige` scope control |
| `v0.1.0` tag and GitHub Release | PENDING_PUBLICATION | Create in the selected repository after the public conversion |
| Final public operation | PENDING_PUBLICATION | Repository is still private; no public release action has been performed |

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
PUBLICATION_READY=false
```

`PUBLICATION_READY=false` is not a technical blocker. It reflects that the repository is
still private, Private Vulnerability Reporting is not activated, the `v0.1.0` tag/release
has not been created, and npm is intentionally deferred pending scope control.
