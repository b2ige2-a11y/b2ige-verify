# Dependency license final review — 0.1.0

**DONE for the reviewed locked graph and local artifact notice assembly.** This is an
engineering redistribution review, not legal advice or a vulnerability assessment.
Cross-platform linkage/runtime testing remains separately outstanding.

## Evidence and reproducibility

Cargo.lock has 143 records: six Apache-2.0 workspace crates and **137 registry packages**.
`cargo metadata --locked --offline` resolves a license expression for every registry
record. [Per-package decisions and notice hashes](dependency-licenses.json) bind each
name/version, Cargo registry checksum, declared expression and selected license.
`python3 scripts/license-notices.py --check` checks all 137 against the locked metadata
and collected license text. All 137 cached registry crate archives were independently SHA-256 checked against Cargo.lock. No dependency versions or verifier semantics changed.

[THIRD-PARTY-NOTICES.txt](../THIRD-PARTY-NOTICES.txt) retains upstream license/copyright
files, including nested notices. The full graph deliberately includes build/dev and
foreign-target packages; inclusion does not assert that every crate is in each binary.
The native package additionally ships the exact compiler installation's
`RUST-RUNTIME-NOTICES.html` (Rust's COPYRIGHT-library.html), hashed in manifest v2,
for statically linked Rust library components. Source archives retain the project
license and third-party notices; they do not vendor the external Cargo crate sources.

## Resolved license choices and obligations

| Issue | Decision / evidence |
|---|---|
| GPL / AGPL / strong copyleft | No mandatory GPL/AGPL expression in the locked Cargo graph |
| r-efi 5.3.0 LGPL alternative | Select MIT, retain its complete AUTHORS license/copyright file; LGPL is not selected |
| LGPL static/dynamic implications | No LGPL-selected component. No LGPL relinking/source obligation is inferred from an unused OR alternative |
| MPL | No MPL-declared package in the reviewed graph |
| MIT / Apache alternatives | Select MIT where offered; legacy slash expressions are reviewed as the documented alternatives |
| ryu-js 1.0.3 | Select Apache-2.0; include its Apache text and upstream notice files |
| unicode-ident | Select MIT AND Unicode-3.0; both notices retained |
| ICU/Unicode, MIT-0, Zlib | Exact upstream terms and attribution retained; no blanket MIT relabeling |
| BSD alternatives | Select MIT for zerocopy; preserve upstream BSD texts too. Bundled SQLCipher notice is retained as a conservative inventory entry, not a claim it is enabled |
| Apache NOTICE | Search included NOTICE/COPYRIGHT files recursively and retain them. No project-origin NOTICE is fabricated; third-party obligations remain independent |
| Native binary redistribution | Project LICENSE, TRADEMARKS, third-party notices and exact Rust library notice bundle included and checked |
| Source redistribution | Project source license and third-party notices included; external dependency sources fetched through pinned Cargo metadata, not bundled into source tar |
| npm wrapper | Apache-2.0; zero dependencies/optionalDependencies/devDependencies; root LICENSE and TRADEMARKS added by release staging |

`r-efi` puts the MIT text and copyright owners in AUTHORS, not a LICENSE-named file.
The crate archives for `uuid-simd 0.8.0`, `vsimd 0.8.0` and `rsqlite-vfs 0.1.1` omit
root license files. Their MIT files were retrieved from the **exact repository commits
recorded in .cargo_vcs_info.json**, stored in [license-supplements](license-supplements/sources.json)
and included in the notice bundle. No unversioned replacement license was guessed.

## SQLite

`libsqlite3-sys 0.38.2` / `rusqlite 0.40.2` use the bundled SQLite path. Their wrappers
are MIT; the inspected SQLite amalgamation identifies version 3.53.2 and contains the
copyright disclaimer/public-domain blessing, retained verbatim in the notice bundle.
SQLCipher and WASM alternatives are not selected by the local native feature graph.
SQLite documents its public-domain dedication and distinguishes external contributions
and tests from the core. [SQLite copyright](https://www.sqlite.org/copyright.html).

Redistributors must preserve applicable texts and attributions. Apache's redistribution
requirements include license copies and relevant upstream NOTICE attribution when
present. [Apache-2.0 terms](https://www.apache.org/licenses/LICENSE-2.0.html).
This review is specific to the recorded lockfile/features; changing those requires a new review.

## Standard SBOM coverage

**DONE: Rust/native Cargo dependency SBOMs generated.** Pinned official
[cargo-cyclonedx 0.5.9](https://github.com/CycloneDX/cyclonedx-rust-cargo) generates
six CycloneDX **1.5** documents, one per workspace crate, with `--target all --all`
and build dependencies retained. Their union is exactly **143 locked packages**:
six workspace crates and 137 registry packages, including transitive, build/dev and
foreign-target packages. Registry SHA-256 values match Cargo.lock. Local workspace
reference prefixes are replaced consistently with stable URNs; inventory and dependency
edges are preserved. `scripts/native-sbom.py` stages public source only.

`release/artifacts/b2ige-0.1.0-native-*.cdx.json` are validated against the official
locally pinned schemas, with dependency-reference and complete package-inventory checks.
The release index and SHA256SUMS bind all six. The wrapper-only npm CycloneDX 1.5
SBOM is separate. Compiler/OS components are not claimed as Cargo dependencies;
the exact Rust library notices and bundled SQLite license evidence remain attached.

**SBOM != vulnerability scan.** A complete locked-package inventory does not establish
vulnerability absence, exhaustive linkage analysis or verifier correctness.
