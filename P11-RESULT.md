# P11 — Agent Distribution result

Date: 2026-09-16. Gate: **LOCAL DISTRIBUTION ASSETS VALIDATED; NOT PUBLISHED**.

Starting HEAD: `a39792c97494c0268646c27f9c88c9d80b972959` (pre-existing checkout).
Excluded P10 commit `13539e165a5170003162ec0c2015af4537e61573` was not applied and is not
an ancestor of that HEAD. Root README and its inherited content were left unchanged.
P9 public release remains intact. No next product phase is started.

## Changed scope

- `plugin.json`: portable Agent Plugins 1.0.0 for Cursor; shared `skills/` discovery.
- `.codex-plugin/plugin.json`: B2IGE Verify / Developer Tools / requested messages.
- `server.json`: Official Registry name, release MCPB URL, stdio and actual final hash.
- `skills/b2ige-verify/SKILL.md`: six requested discovery intents plus setup reference;
  existing verdict/authorization rules retained.
- `skills/b2ige-verify/references/distribution.md`: explicit local setup prerequisites.
- `distribution/mcpb/{manifest.json,launch.sh,README.md}`: MCPB 0.4 and portable launcher.
- `distribution/{release-inputs.json,build_mcpb.py,test_distribution.py,bundle.sha256}`:
  pinned provenance, reproducible packaging and distribution-only regression checks.
- `distribution/{README.md,metadata.md}`: shared setup, metadata and platform handoff.
- `tasks/P11-INDEX.md`, `state/CURRENT.md`, this result: scope and local asset gate.

No changes to Rust core/product code, product schemas, benchmark baselines, verdict
contracts, original release scripts/assets, root README, LICENSE, SECURITY.md or
TRADEMARKS.md. No npm publish. Full Rust tests/CI and product build were deliberately
not run, as requested. The new packaging scripts package release binaries only.

## Artifact and provenance

Generated (git-ignored):
`release/artifacts/agent-distribution/b2ige-verify-0.1.0.mcpb`
and `b2ige-verify-0.1.0.mcpb.sha256`.

Final SHA-256:
`f205d467a639747ce684bb32c4a560e6f63ade73cddeddbb822aafd00a48c884`.

Bundle payload: manifest, launcher, operator README, pinned input provenance, LICENSE,
SECURITY.md, TRADEMARKS.md, THIRD-PARTY-NOTICES.txt, and exactly three `b2ige-mcp` binaries:
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`.

Inputs were downloaded from GitHub v0.1.0 and checked against the pinned published
SHA256SUMS, per-target release manifests and individual binary hashes. All three binary
payloads remained byte-identical. Source commit is
`57c69773c019b6dbc3b6ac730269c6233ec5e523`; local v0.1.0 tag object remains
`3c4583a3bf9e7ec5cdc319dc2489830053a98941`. No compilation or upload took place.

## Validation evidence

| Check | Result |
|---|---|
| `git diff --check` | PASS |
| All new JSON parsed | PASS |
| Agent Plugin 1.0.0 canonical JSON Schema | PASS |
| Registry 2025-12-11 canonical JSON Schema + format checker | PASS |
| MCPB 0.4 JSON Schema shipped with official CLI | PASS |
| `sh -n distribution/mcpb/launch.sh` | PASS |
| Python packaging/test syntax compilation | PASS |
| `mcpb validate`, `mcpb pack`, final archived manifest validation | PASS; CLI 2.1.2 |
| Final ZIP CRC, exact entry set, native bytes and executable permissions | PASS |
| Two independent packaging runs, normalized ZIP compared with `cmp` | PASS; identical final SHA-256 |
| `mcp-publisher validate server.json` | PASS; v1.8.1, official registry validation endpoint returned “server.json is valid” |
| Codex `plugin-creator/scripts/validate_plugin.py .` | PASS |
| Skill `skill-creator/scripts/quick_validate.py skills/b2ige-verify` | PASS |
| `python3 distribution/test_distribution.py -v` | PASS; 5 distribution-only tests |
| Product/protected paths compared against starting HEAD | Unchanged |

Test coverage includes all three dispatch routes with test doubles, unsupported
OS/architectures, missing/relative registry, missing binary, invalid sealed-root input,
literal paths containing spaces/semicolon, exit propagation, empty sealed-root handling,
corrupted cached release rejection, and final bundle hash consistency with server.json.
Native macOS arm64 smoke launched the actual extracted release binary, initialized MCP,
listed the five expected tools, and confirmed an unregistered identity yields structured
ERROR with `isError: true`. A malformed registry failed without stdout success output.
These are transport/packaging checks, not evidence that any user product passed verification.

Validation fixes: shortened the registry description to its 100-character limit;
corrected a test expectation for macOS `/var` → `/private/var` path canonicalization.
Both affected checks were rerun successfully. No product patch was needed.

Tooling: Node v24.18.1; Python 3.14.3; jsonschema 4.26.0; PyYAML 6.0.3.
Official schemas were fetched for validation only, not added to product `schemas/`.

## Readiness and limits

- **Official Registry:** server.json validation-ready. Its MCPB URL is not live until the
  owner uploads the additional release asset. Validation does not prove publication,
  name ownership, or future download availability.
- **Cursor:** manifest/skill/schema preflight-ready, using the more portable root
  Agent Plugin format. Complete Marketplace submission-ready status is **pending** a
  live client import/intent-selection check and repository push. No UI installation
  or marketplace review was performed. The checklist is in the distribution guide.
- **Codex:** local manifest/skill-ready and validated. Fresh-client installation was
  not performed. Public-directory readiness is **not claimed**: current OpenAI guidance
  requires contact/product-specific review for local execution and file access.
- **Smithery / cursor.directory:** common metadata and local bundle prepared. No listing
  or authenticated publication has been created.
- Only macOS arm64 ran natively in P11. Intel macOS and Linux bytes are the previously
  verified v0.1.0 release assets; P11 tests their dispatch and byte integrity, not native
  product execution on those hosts. No new cross-platform/product guarantee is made.
- MCPB is unsigned; original macOS signing/notarization status is unchanged.
- Plugin manifests intentionally omit automatic MCP configuration. Users must install
  the verified bundle and register trusted absolute paths; no nonexistent PATH binary,
  automatic baseline, hidden-suite isolation, or hosted service is assumed.

Schema decision: product schemas/protocol remain unchanged. New external metadata uses
MCPB 0.4, Agent Plugins 1.0.0, Registry 2025-12-11, and the installed Codex ingestion
contract. Distribution version 0.1.0 does not rewrite the fixed product tag.

The only remaining external actions are documented in the
[platform handoff](distribution/README.md#submission-checklist-and-owner-actions).
Push, asset upload, registry publish, marketplace/public-directory submission were
**not performed**. Final commit identity is reported in the task handoff (avoids a
self-referential commit hash inside this file).
