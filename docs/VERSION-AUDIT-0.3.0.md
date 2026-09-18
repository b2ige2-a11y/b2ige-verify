# Release version occurrence audit

Scope: every literal 0.1.0 / 0.2.0 / 0.3.0 occurrence in the public Git inventory,
including untracked release-prep files, before packaging. The rows group occurrences
by file/version/category; repeated line numbers mean multiple occurrences on that line.
This audit document itself is current release documentation and excluded from its own
enumeration. Ignored build caches, local diagnostics and candidate artifacts are generated
state, not release source. Generated candidate product versions are independently checked
by packaging, SBOM, manifest, archive and installed-binary gates. No stale product reference
is intentionally retained. Old plugin/MCPB pins are a separate historical distribution.
Dependency/license identifiers and frozen fixture producer versions are independent
historical values, not B2IGE product-version requirements. Negative 9.9.9 fixtures remain.

| File | Literal | Classification | Line occurrences |
|---|---|---|---|
| `.codex-plugin/plugin.json` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 3, 14 |
| `CHANGELOG.md` | 0.1.0 | historical record retained | 25, 44, 46 |
| `CHANGELOG.md` | 0.2.0 | historical record retained | 15 |
| `CHANGELOG.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3, 13 |
| `Cargo.lock` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 527 |
| `Cargo.lock` | 0.2.0 | historical: independent fixture/dependency/license/evidence version retained | 217 |
| `Cargo.lock` | 0.3.0 | current product version updated (including release procedure/tests) | 1072, 1087, 1104, 1116, 1129, 1140 |
| `Cargo.lock` | 0.3.0 | historical: independent fixture/dependency/license/evidence version retained | 171 |
| `Cargo.toml` | 0.3.0 | current product version updated (including release procedure/tests) | 6 |
| `README.md` | 0.1.0 | documentation referring specifically to old public release | 108, 177, 187 |
| `README.md` | 0.2.0 | documentation referring specifically to old public release | 23, 29, 44, 46, 47, 47, 51, 51, 52, 52, 53, 53, 55, 66, 67, 178, 186 |
| `README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 20, 21, 27 |
| `SECURITY.md` | 0.2.0 | documentation referring specifically to old public release | 10 |
| `SECURITY.md` | 0.3.0 | current product version updated (including release procedure/tests) | 10 |
| `THIRD-PARTY-NOTICES.txt` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 10227 |
| `THIRD-PARTY-NOTICES.txt` | 0.2.0 | historical: independent fixture/dependency/license/evidence version retained | 5107 |
| `THIRD-PARTY-NOTICES.txt` | 0.3.0 | current product version updated (including release procedure/tests) | 1 |
| `THIRD-PARTY-NOTICES.txt` | 0.3.0 | historical: independent fixture/dependency/license/evidence version retained | 4114 |
| `TRADEMARKS.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3 |
| `benchmarks/adoption-v1/README.md` | 0.2.0 | historical: independent fixture/dependency/license/evidence version retained | 13 |
| `benchmarks/baseline-v1/result.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 1 |
| `benchmarks/baseline-v1/reverse-result.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 1 |
| `crates/verify-cli/README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 1 |
| `distribution/README.md` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 1, 7, 7, 26, 33, 34, 50, 51, 51, 61, 61, 77, 90, 92, 154, 154, 180 |
| `distribution/build_mcpb.py` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 16 |
| `distribution/bundle.sha256` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 1 |
| `distribution/mcpb/README.md` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 1, 6, 36 |
| `distribution/mcpb/manifest.json` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 5, 7 |
| `distribution/metadata.md` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 25, 27, 27, 36, 45, 48 |
| `distribution/release-inputs.json` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 2, 3, 10, 12, 18, 20, 26, 28 |
| `distribution/test_distribution.py` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 140 |
| `docs/ADOPTION.md` | 0.2.0 | documentation referring specifically to old public release | 3 |
| `docs/ADOPTION.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3 |
| `docs/CI.md` | 0.2.0 | documentation referring specifically to old public release | 5 |
| `docs/CI.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3 |
| `docs/CONTRACTS.md` | 0.3.0 | current product version updated (including release procedure/tests) | 15, 159 |
| `docs/INSTALL.md` | 0.1.0 | documentation referring specifically to old public release | 76, 120 |
| `docs/INSTALL.md` | 0.2.0 | documentation referring specifically to old public release | 14, 16, 16, 16, 57, 60, 63, 69, 77, 119, 127 |
| `docs/INSTALL.md` | 0.3.0 | current product version updated (including release procedure/tests) | 1, 3, 6, 7, 11, 11, 12, 12, 118, 124, 127 |
| `docs/QUICKSTART.md` | 0.2.0 | documentation referring specifically to old public release | 64, 88 |
| `docs/QUICKSTART.md` | 0.3.0 | current product version updated (including release procedure/tests) | 63, 88 |
| `docs/RELEASE-0.3.0.md` | 0.1.0 | documentation referring specifically to old public release | 73, 140 |
| `docs/RELEASE-0.3.0.md` | 0.2.0 | documentation referring specifically to old public release | 85, 140 |
| `docs/RELEASE-0.3.0.md` | 0.3.0 | current product version updated (including release procedure/tests) | 1, 4, 8, 9, 30, 38, 58, 59, 60, 61, 62, 63, 73, 77, 78, 85, 86, 131, 139, 153 |
| `docs/RELEASE-PROVENANCE.md` | 0.1.0 | historical record retained | 1, 3, 7, 10, 13, 15, 17, 27, 33 |
| `docs/RELEASE.md` | 0.1.0 | historical record retained | 1, 3, 9, 17, 17, 128 |
| `docs/RELEASE.md` | 0.2.0 | historical record retained | 135 |
| `docs/V100-RELEASE.md` | 0.1.0 | historical record retained | 3 |
| `docs/V100-RELEASE.md` | 0.2.0 | historical record retained | 3, 6, 65, 70, 81, 81 |
| `docs/V110-ADOPTION-BENCH.md` | 0.2.0 | documentation referring specifically to old public release | 8 |
| `docs/V110-DISTRIBUTION.md` | 0.2.0 | documentation referring specifically to old public release | 3, 4, 90, 93, 99 |
| `docs/V110-DISTRIBUTION.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3, 5, 6, 8, 90, 95, 95, 100, 100, 148, 171 |
| `docs/V110-PLAN.md` | 0.2.0 | historical record retained | 27, 28, 29, 29, 40, 61, 69, 341, 457, 577, 581, 658 |
| `examples/behavior/README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 27 |
| `examples/blindtest/README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 33 |
| `examples/sideeffect/README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 25 |
| `external-pilot/v1/README.md` | 0.2.0 | documentation referring specifically to old public release | 16 |
| `npm/b2ige/README.md` | 0.2.0 | documentation referring specifically to old public release | 7, 7 |
| `npm/b2ige/README.md` | 0.3.0 | current product version updated (including release procedure/tests) | 3, 18 |
| `npm/b2ige/download.test.cjs` | 0.3.0 | current product version updated (including release procedure/tests) | 13, 28, 32, 32, 33, 34, 35, 36, 83, 99 |
| `npm/b2ige/native-manifest.json` | 0.3.0 | current product version updated (including release procedure/tests) | 3 |
| `npm/b2ige/package.json` | 0.3.0 | current product version updated (including release procedure/tests) | 3 |
| `npm/b2ige/test.cjs` | 0.3.0 | current product version updated (including release procedure/tests) | 9, 54 |
| `plugin.json` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 4 |
| `release/dependency-license-review.md` | 0.1.0 | documentation referring specifically to old public release | 1, 73 |
| `release/dependency-licenses.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 625 |
| `release/dependency-licenses.json` | 0.2.0 | historical: independent fixture/dependency/license/evidence version retained | 291 |
| `release/dependency-licenses.json` | 0.3.0 | historical: independent fixture/dependency/license/evidence version retained | 227 |
| `release/name-check-evidence.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 74 |
| `release/name-clearance-review.md` | 0.1.0 | documentation referring specifically to old public release | 30, 40, 42 |
| `release/release-manifest.json` | 0.1.0 | generated artifact / historical record retained | 4, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 60, 61, 62, 63, 64, 65, 70 |
| `release/release-manifest.schema.json` | 0.3.0 | current product version updated (including release procedure/tests) | 43 |
| `release/sbom-schema/spdx.schema.json` | 0.3.0 | historical: independent fixture/dependency/license/evidence version retained | 176 |
| `scripts/adoption_bench.py` | 0.2.0 | historical record retained | 838 |
| `scripts/install-release.py` | 0.2.0 | documentation/test/code retaining old exact public release support | 23 |
| `scripts/install-release.py` | 0.3.0 | current product version updated (including release procedure/tests) | 22, 23 |
| `scripts/license-notices.py` | 0.3.0 | current product version updated (including release procedure/tests) | 21 |
| `scripts/platform-cli-smoke.py` | 0.3.0 | current product version updated (including release procedure/tests) | 17 |
| `scripts/test-distribution.py` | 0.2.0 | documentation/test/code retaining old exact public release support | 109, 110, 110, 111, 305 |
| `scripts/test-distribution.py` | 0.3.0 | current product version updated (including release procedure/tests) | 42, 49, 52, 294, 295, 295, 296, 305, 310, 315, 320 |
| `scripts/test-distribution.py` | 0.3.0 | test negative fixture deliberately mismatched | 298, 298, 328, 329, 393 |
| `scripts/test-release.py` | 0.3.0 | current product version updated (including release procedure/tests) | 20, 33, 34, 35, 36, 53, 58, 73, 81 |
| `scripts/validate-archive.py` | 0.3.0 | current product version updated (including release procedure/tests) | 34, 45 |
| `server.json` | 0.1.0 | historical: independently pinned plugin/MCPB retained | 10, 14, 14 |
| `skills/b2ige-verify/references/distribution.md` | 0.1.0 | documentation referring specifically to old public release | 8, 8 |
| `tests/conformance/support.rs` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 71 |
| `tests/fixtures/blindtest-pass.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 77 |
| `tests/fixtures/error.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/fail.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/inconclusive.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/invalid-blindtest-unapproved-invariant.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 77 |
| `tests/fixtures/invalid-pass-incomplete-coverage.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/invalid-pass-missing-evidence.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/invalid-sideeffect-attempt-as-effect.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 75 |
| `tests/fixtures/pass.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 73 |
| `tests/fixtures/sideeffect-pass.json` | 0.1.0 | historical: independent fixture/dependency/license/evidence version retained | 75 |
| `v100/STATE.md` | 0.2.0 | historical record retained | 5, 7, 37 |
