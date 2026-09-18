# B2IGE Verify 0.3.0 release candidate

This is local release candidate preparation only, not a published release or final
qualification. Intended local result: **V0.3.0 RELEASE CANDIDATE PREPARED LOCALLY**.
Publication, independent release audit, fresh cross-platform PR/exact-main CI and
final isolated private holdout: **PENDING**. No future run IDs or hashes are assigned.

Branch: `release/0.3.0`. Preparation base: `8e4d60dbeba64d8aff6bf328e4ced3174597ffa3`.
The preparation commit is `3448a07835f5f2268b896f7899de042a445cb536`.
If the independent audit requires repairs, one subsequent repair commit becomes the
audited candidate HEAD; resolve it with `git rev-parse HEAD`. Pre-commit local artifacts
record that base plus `working_tree_dirty: true` and their source inventory hash.
They are not clean final-commit artifacts and must not be published. Regenerate the
final artifacts against the exact qualified main commit after merge.

## Scope and version decision

V110-A includes inspect, idempotent init, prepare, interactive `trust approve`,
readiness-only doctor and registered identity verification. V110-B adds the public
9-scenario adoption benchmark and its reproducible tooling. Historical measurements
remain bounded public workflow evidence, not third-party compatibility, universal
usability, external-user validation or an independent hidden holdout.

V110-C includes `ci init`, immutable full-SHA verifier pinning, hardened controller
generation, exact-version/offline installation, archive safety and fresh install
smoke. Candidate approval remains independently provisioned and fail-closed.
V110-D includes the external-pilot framework only. **Actual genuine external-user
evidence is DEFERRED.** No AI/internal run is represented as external evidence.
External adoption is a post-release validation objective, not a correctness gate.

Product/workspace/npm version is 0.3.0. The release manifest stays schema_version
`"3"`; only its product-version constraint advances. Agent Protocol 1, npm native
manifest v2, registry, evidence, approval, adoption benchmark and pilot schemas
remain independent and unchanged. No V100 semantics, P8 labels/baselines or V110
approval/evidence expectations change.

## Platform matrix

| Platform | Candidate scope | Fresh 0.3.0 qualification |
|---|---|---|
| macOS Apple Silicon | Native archive; prior VERIFIED_NATIVE within recorded scope | Local gate records only actual execution; remote PR/final CI PENDING |
| macOS Intel | Native archive; prior VERIFIED_NATIVE within documented partial no-Docker scope | Fresh PR/final native CI PENDING |
| Linux x86_64 | Native archive; prior VERIFIED_NATIVE with actual Docker/runtime/full benchmark scope | Fresh PR/final native CI PENDING |
| Windows x64 | Source/build + bounded CLI runtime smoke; not VERIFIED_NATIVE | Fresh Windows runner CI PENDING; no public native archive |
| Linux arm64 | DEFERRED | No native runner or archive; macOS Docker arm64 is not native Linux evidence |

V110-C's prior bounded Windows smoke is recorded in `v110/STATE.md`; it does not
qualify this new artifact. Candidate workflows remain read-only and unchanged.

macOS remains `publisher_signed=false`, `apple_notarized=false`. No signing,
notarization or automatic quarantine removal. SHA256 provides integrity, not
publisher authentication. npm remains private with its failing prepublish guard;
Cargo retains `publish=false`. Neither registry is a publication target here.

## Artifacts and installation

Expected names (only the actual host native archive is generated locally):

- `b2ige-0.3.0-aarch64-apple-darwin.tar.gz`
- `b2ige-0.3.0-x86_64-apple-darwin.tar.gz`
- `b2ige-0.3.0-x86_64-unknown-linux-gnu.tar.gz`
- `b2ige-0.3.0-source.tar.gz`
- `b2ige-verify-0.3.0.tgz` (private wrapper candidate, not npm publication)
- `b2ige-0.3.0-TARGET.manifest.json`, `SHA256SUMS`, npm/native CycloneDX SBOMs

Native archives contain CLI/MCP and existing demo/benchmark helpers, the installer,
CI adapters, V110 docs, public benchmark/adoption corpus and pilot protocol/guide.
V110 adoption benchmark execution and pilot evidence tooling intentionally remain
**source-checkout workflows**: the source archive contains their Python tools,
Rust example constructors and tests. They require the documented reviewed Git
checkout/history and locked source build; merely unpacking a native archive is
not sufficient to run those studies. The public native CLI's `bench` command is
independently usable. Do not run an external participant pilot during RC preparation.
The separately pinned 0.1.0 plugin/MCPB distribution is historical, not a new 0.3.0
bundle, and is not rebuilt or advertised as current native functionality.

```sh
python3 scripts/install-release.py --offline --version 0.3.0 \
  --archive /trusted/b2ige-0.3.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Use the actual host target and a new destination under an existing parent. All
checksums, members, manifest version/target and binary hashes are checked before
writes; installed absolute CLI/MCP paths run from an empty directory. The CLI must
report exactly `verify-cli 0.3.0`. No source/PATH fallback. Exact online 0.2.0 support
is retained; online `--version 0.3.0` is usable only AFTER public assets exist.
There is no `latest` default or fallback.

## Local gates and generated-file handling

Prerequisite: build `cargo build --locked -p verify-cli --release --example adoption_fixture`.
Run one stabilized comprehensive pass: fmt, locked workspace/all-target clippy
with denied warnings, locked workspace/all-target tests, locked release build;
readme-bench, hygiene, history-scan (review diagnostics privately), license-notices
`--check`, test-adoption, test-adoption-bench, test-distribution, test-external-pilot,
test-release, test-v100-release, validate-workflows, npm tests and diff whitespace.
Then run:

```sh
python3 scripts/package-rc.py
python3 scripts/record-platform-gate.py
python3 scripts/npm-archive-smoke.py
cargo run --locked -p verify-cli --example validate_release -- \
  release/release-manifest.schema.json release/release-manifest.json
python3 scripts/validate-archive.py release/artifacts
```

The full platform gate runs actual Docker and one fresh extracted offline install,
CLI/product/report/MCP smoke and the complete public 31-case benchmark. Do not use
`--without-docker` or `--skip-benchmark` to claim this full local gate. The separate
adoption regression validates its existing public measured data and fresh bounded
journeys; do not overwrite the measured record. Final private holdout is NOT run here.
Installed smoke executes inspect/init/prepare/interactive-approval refusal/readiness/
registered verification and CI bootstrap; PATH traps and a missing installed CLI
negative control reject fallback. Help/readiness checks do not establish product PASS.

The generated manifest must bind actual base/dirty state, target/runtime scope,
source inventory, binary/archive/notice/SBOM hashes, benchmark identity and supplied
CI metadata. Packaging alone grants no runtime or publication readiness. Keep
`publication_ready=false`, `owner_publication_authorized=false`. Embedded manifests
record pre-smoke state; only the external index records subsequent runtime scope.

Package from clean audited HEAD after committing any repairs, then validate its
commit and `working_tree_dirty=false` in all generated manifests. After validation,
restore the historical tracked `release/release-manifest.json` from HEAD. Keep candidate artifacts
and diagnostic logs ignored; never commit dirty-build hashes, target caches, raw
runtime/holdout stores, credentials or participant data. The historical manifest
is not expected to validate under the new product constraint; validate the generated
candidate before restoration. Compare the complete branch against main and preserve
all frozen semantics, schema versions, P8 inputs and historical tags/releases.

## Publication procedure — AFTER qualification, never during this task

1. Independent Astra/xhigh release audit of the local preparation commit.
2. Push `release/0.3.0`; open a PR into main and require cross-platform PR CI.
3. Merge only after review/gates. Run exact-main `workflow_dispatch` candidate CI;
   record its real commit, run/attempt, artifacts and per-platform scope.
4. Have the independent controller perform a fresh final isolated holdout against
   the exact main candidate artifact. Retain only its approved sanitized aggregate.
5. Review exact archive inventories, manifests, SBOMs, checksums and provenance;
   require final owner authorization separately. Candidate flags are not authority
   to publish. Resolve any missing evidence before proceeding.
6. Create the annotated `v0.3.0` tag at that exact qualified main commit. Preserve
   `v0.1.0` and `v0.2.0` without moving/replacing tags, assets or hashes.
7. Publish a GitHub Release using the qualified three native archives, source,
   external per-target manifests, SBOMs and a reviewed combined SHA256SUMS. Common
   cross-run assets may differ in nonsemantic timestamps: select one consistent
   set and revalidate every manifest/hash binding; do not overwrite collisions
   silently. Do not publish npm/Cargo, Windows/Linux-arm64 archives or old MCPB.
8. Download the real release assets, verify checksums/manifest/binary identities
   and exact-version install on supported native hosts. Verify tag/commit and links.
9. Close state/docs with only actual final CI/holdout/publication evidence. Keep
   external participant evidence DEFERRED until a separately reviewed genuine study.

## Candidate release-note draft — do not publish yet

**B2IGE Verify 0.3.0**

Main additions: easier existing-project adoption; registered identity verification;
safer CI bootstrap; secure exact-version online/offline installer; public adoption
workflow benchmark with 9 scenarios; external pilot framework.

Boundaries: macOS unsigned/unnotarized; npm/Cargo unpublished; Windows source/build
and bounded CLI smoke only, not VERIFIED_NATIVE; Linux arm64 deferred; actual external
adoption evidence deferred. Verification is bounded, not exhaustive proof. Trusted
operator inputs and existing isolation limits remain mandatory.

Final release commit, native CI, isolated holdout and public asset URLs: PENDING.
