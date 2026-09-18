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

The publication-assembly repair retains manifest schema 3: its existing arbitrary
`artifact_sha256` map already permits the narrower target-owned inventory. No wire
field, product evidence schema or version changes. In an external `release-index`,
this map binds **public target-owned artifacts**, exactly
`b2ige-0.3.0-TARGET.tar.gz` for `built_target`. It excludes source, npm, SBOM
sidecars, other targets and the manifest itself. Embedded SBOM metadata/hashes
continue to describe the native archive's own dependency inventory.

The prior exact-main candidate at `71f058c6aeeda53c529b61268792f1e3f44d692d`
passed CI `35330052486` and the controller-reported V3 holdout (70/70/70;
false PASS/FAIL/leakage all zero). Its publication assembly was BLOCKED: per-target
indexes also bound runner-local common files, yielding 16 conflicting common-file
hashes and three candidate-only npm bindings (19 unsatisfied bindings). Those
records remain historical qualification of that exact artifact. After this repair
merges, **fresh exact-main CI and a fresh final V4 holdout are required**. V3 cannot
qualify artifacts from the changed source. v0.3.0 remains unpublished.

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

The future public GitHub Release contains exactly eight files:

- `b2ige-0.3.0-aarch64-apple-darwin.tar.gz`
- `b2ige-0.3.0-aarch64-apple-darwin.manifest.json`
- `b2ige-0.3.0-x86_64-apple-darwin.tar.gz`
- `b2ige-0.3.0-x86_64-apple-darwin.manifest.json`
- `b2ige-0.3.0-x86_64-unknown-linux-gnu.tar.gz`
- `b2ige-0.3.0-x86_64-unknown-linux-gnu.manifest.json`
- `b2ige-0.3.0-source.tar.gz`
- `SHA256SUMS`

Each CI candidate bundle contains only its host native archive and external index,
plus source, the private `b2ige-verify-0.3.0.tgz`, six native SBOM sidecars and the
npm SBOM. Its candidate `SHA256SUMS` covers **every** uploaded candidate file except
itself, including all sidecars. GitHub Actions candidate retention is qualification
evidence, not the public release asset list. npm tgz and npm SBOM are
**CANDIDATE_ONLY**; neither is a public GitHub Release asset or npm registry
publication. npm remains private/unpublished; Cargo remains `publish=false`/unpublished.

Six native CycloneDX SBOMs are shipped **inside each native archive**, with hashes
bound by that archive's embedded manifest and schema/dependency inventory validated.
Do not publicly stage SBOM sidecars for v0.3.0; no SBOM sidecar publication is required.
An SBOM is dependency inventory, not a vulnerability scan. Runner-specific truthful
timestamps, UUIDs and provenance may differ; no cross-run SBOM byte identity is claimed.

The source archive is release-global. Select one qualified exact-main source archive,
preferably Apple Silicon because its exact bundle is exercised by the final holdout.
Compare normal source member names/bytes across all runners and against exact main,
excluding only generated `release/release-manifest.json`; review each generated
manifest's truthful provenance separately. Do not require equal raw source archive
SHA256 values or normalize away provenance. The selected original bytes are bound
by final public `SHA256SUMS`, not by any target index.

Generate combined public `SHA256SUMS` only during final staging after cross-platform
CI. It covers the seven other public files and excludes itself. This release-level
integrity index does not authenticate a publisher or grant publication authority.

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

`test-release.py` also exercises a deterministic synthetic three-target public set
and rejects extra assets, incomplete checksums and inconsistent bindings. At future
final staging, run the structural validator with the independently qualified commit:

```sh
python3 scripts/validate-public-release.py /trusted/public-staging --commit FULL_MAIN_SHA
```

It requires the exact eight-file set, clean matching commit/CI identities, matching
native/index/source metadata, complete checksums, archive safety and all embedded
SBOM hashes/inventory. It executes no platform binaries and does not replace
cross-platform runtime CI, direct exact-main source comparison or final holdout.

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
7. Stage exactly the three qualified native archives, three matching external target
   manifests, one selected qualified source archive and one reviewed combined
   `SHA256SUMS` covering the seven other files. Compare normal source contents across
   runners and against exact main, then prefer the Apple Silicon source bytes.
   Preserve all original selected candidate bytes and truthful runner provenance.
   Run `validate-public-release.py` before publishing these eight files as the GitHub
   Release. Native CycloneDX SBOMs remain embedded and validated inside each native
   archive; do not stage sidecar SBOMs. npm tgz/npm SBOM remain candidate-only, with
   no npm registry publication. Cargo remains unpublished. Exclude Windows/Linux-arm64
   archives and old MCPB. No target manifest duplicates the release-global checksum role.
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
