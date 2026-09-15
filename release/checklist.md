# Final publication checklist — 0.1.0

Current handoff: [FINAL-PUBLICATION-DECISIONS](../FINAL-PUBLICATION-DECISIONS.md).
DONE records measured local completion, not permission to publish.

| Item | State | Evidence / remaining action |
|---|---|---|
| P1–P8 semantics, Cargo.lock, evidence schemas and baselines | DONE | Original file hashes preserved; no new verifier feature |
| Clean public history | DONE | Unborn HEAD, zero commits; private history not imported or rewritten |
| Public hygiene | DONE | No blockers; Git capture-tree findings are synthetic schemas/tests/scanner rules |
| License and dependency notices | DONE | Apache-2.0; all 137 registry records checked; native Rust runtime notices retained |
| Brand/package separation | DONE | RC_USABLE / OWNER_DECISION / CONFLICT / INCOMPLETE recorded; no unscoped blindtest package |
| BehaviorSeal final use | OWNER_DECISION | Behavior remains working name; no legal clearance claim |
| GitHub organization/repository | OWNER_DECISION | Real owner/name required; remote absent |
| npm scope | OWNER_ACTION_REQUIRED | Control/create @b2ige; scoped wrapper remains private |
| Rust/native CycloneDX SBOM | DONE | Six validated 1.5 documents cover all 143 locked packages; hashes in release index and SHA256SUMS |
| npm CycloneDX SBOM | DONE | Separate wrapper-only SBOM; neither SBOM is a vulnerability scan |
| npm download/integrity/process behavior | DONE | Version/platform archive, trusted dual hashes, bounded safe extraction, atomic cache, fail-closed errors; 28 tests |
| Actual release host/assets/pins | EXTERNAL_EXECUTION | Selected GitHub repository and native CI outputs must supply reviewed host/pins; live delivery untested; publish guards retained |
| Local macOS arm64 | DONE | VERIFIED_NATIVE; final installed product smoke including actual Docker |
| Native macOS x86_64 | EXTERNAL_EXECUTION | NOT_RUN — REMOTE EXECUTION REQUIRED |
| Native Linux x86_64 | EXTERNAL_EXECUTION | NOT_RUN — REMOTE EXECUTION REQUIRED |
| CI matrix preparation | DONE | Read-only pinned actions/compiler, native architecture check, tests/build/package/SBOM/fresh smoke |
| Supported versions / support | DONE | 0.1.x; best-effort community support; no SLA |
| Private security channel | OWNER_ACTION_REQUIRED | Enable GitHub Private Vulnerability Reporting after repository creation and verify report submission |
| Apple distribution policy | OWNER_DECISION | Choose Option A or B below; zero valid identities; neither selected |
| Cryptographic provenance | DEFERRED_ACCEPTABLE | Attestation strategy documented; no signature/attestation claimed |
| Full reproducible build proof | DEFERRED_ACCEPTABLE | No such proof claimed |
| Linux arm64 / Windows / separate crates.io packages | DEFERRED_ACCEPTABLE | Outside 0.1.0 candidate publication scope |
| Initial commit | OWNER_ACTION_REQUIRED | Command in final decisions; no commit made |
| Final public approval | OWNER_DECISION | Required before any public operation; PUBLICATION_READY=false |

## Signing choice — OWNER_DECISION

**Option A: Unsigned 0.1.0.** Owner explicitly accepts unsigned/unnotarized distribution.
Show the install/security warning in [INSTALL](../docs/INSTALL.md), authenticate delivery,
and provide SHA-256 instructions. Never remove quarantine or bypass OS checks automatically.

**Option B: Developer ID signing + notarization.** Requires the owner's Apple developer
setup and real identity. Sign every executable, notarize, verify on clean macOS, then
rehash, repackage and rerun the release checks. No identity is created or purchased here.

## Status meaning

CLEAN_PUBLIC_REPO_READY concerns the clean local source handoff.
TECHNICAL_PUBLICATION_READY remains false until native platform execution, live delivery,
namespace ownership, private reporting and the selected signing policy are fulfilled.
PUBLICATION_READY also requires the owner's explicit final approval.
