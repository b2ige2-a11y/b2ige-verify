# B2IGE Verify — Public repository readiness

CLEAN_PUBLIC_REPO_READY = true
TECHNICAL_PUBLICATION_READY = false
PUBLICATION_READY = false

This is the local `B2IGE-Verify-Public` candidate. No public action is authorized.
[Final decisions and initial commit command](FINAL-PUBLICATION-DECISIONS.md).

## Source and history

The snapshot uses a new public Git history, with **zero commits and unborn HEAD**.
Private development history was not imported, rewritten or modified. Existing Codex
capture metadata contains trees/blobs, not commits; no claim of zero Git objects is made.
The current release manifest records `git_commit: null` and an exact source-inventory
hash instead of attributing this snapshot to a private development commit.
The initial commit is reserved for the owner.

## Automatic gates

- Public inventory hygiene: **BLOCKER = 0**. Synthetic schema fields, scanner rules
  and negative-test paths are SAFE_SYNTHETIC. Ignored build artifacts, raw evidence,
  caches and local logs are excluded from the initial commit inventory.
- Apache-2.0 metadata and all 137 registry license/notice records checked.
- Brand candidates and package identity separated. B2IGE Verify, BlindTest and
  SideEffect Proof are RC_USABLE; BehaviorSeal is OWNER_DECISION. Unscoped npm
  blindtest is CONFLICT and prohibited. `@b2ige/verify` scope ownership is INCOMPLETE.
- Rust/native SBOM: **GENERATED**, six CycloneDX 1.5 documents covering all **143**
  locked packages, validated against official schemas, graph references and registry
  hashes. Separate npm SBOM covers only the wrapper. Neither is a vulnerability scan.
- npm distribution implemented and tested: exact version/platform GitHub archive,
  package-carried SHA-256 pins, bounded safe extraction, atomic cache, version check
  and process launch. No PATH fallback. Real repository and pins remain unset;
  private/prepublish guards remain active.
- Release metadata is schema v3; npm distribution manifest is v2. Verifier and evidence
  schemas are unchanged. Archives retain source binding, binary/notice hashes and checksums.

## Final local validation

- `cargo fmt --check`: PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `RUST_TEST_THREADS=4 cargo test --workspace --locked`: PASS, **376 passed / 0 failed**.
- `cargo build --workspace --release --locked`: PASS.
- npm wrapper: PASS, **28 passed / 0 failed**, including synthetic download failure,
  archive/binary tampering, version mismatch and offline cache behavior.
- README Behavior/SideEffect commands, fresh installed CLI/init/doctor/product reports,
  BlindTest using actual Docker, MCP and packed npm execution: PASS.
- Manifest/schema, native/source/npm archive safety, checksums, source inventory,
  SBOM/schema/inventory, workflow structure and final hygiene: PASS.
- No separate complete benchmark rerun: verifier and baseline files are unchanged.
  The required workspace test suite still executes its embedded benchmark tests.

## Remaining external gates

| Platform | Status | Evidence scope |
|---|---|---|
| macOS arm64 | VERIFIED_NATIVE | Current local native binaries and fresh installed product smoke; actual Docker Desktop engine |
| macOS x86_64 | NOT_RUN | REMOTE EXECUTION REQUIRED; previous cross-build/Rosetta is supplemental only |
| Linux x86_64 | NOT_RUN | REMOTE EXECUTION REQUIRED; local Docker engine is arm64, not this target |

No Git remote is configured. Workflow readiness is not execution evidence.
Owner actions: BehaviorSeal decision, GitHub owner/repository, @b2ige scope, unsigned
versus Developer ID/notarization choice, Private Vulnerability Reporting, remote native
CI and final publication approval. Actual host/assets/pins and report submission require
those external accounts and authorized execution. See [checklist](release/checklist.md).

`release/artifacts/` contains local generated archives, SBOMs, external manifest and
SHA256SUMS, all ignored by Git. Local build/cache/evidence output remains ignored;
none is silently included by the initial `git add -A` command.
