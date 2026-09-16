# B2IGE Verify 0.1.0 release procedure

This procedure covers final candidate validation and post-release verification.
P1–P8 verifier semantics and baselines are unchanged. Package version remains 0.1.0.
The reviewed product/code commit is `28315fd`; the final release source commit is
`57c6977`. The release manifest alone advances from v1 to **v3**. See [schema and
provenance decisions](RELEASE-PROVENANCE.md).

Current state: `CLEAN_PUBLIC_REPO_READY=true`, `TECHNICAL_PUBLICATION_READY=true`,
`PUBLICATION_READY=true`. The official target is public:
[b2ige2-a11y/b2ige-verify](https://github.com/b2ige2-a11y/b2ige-verify). The
[v0.1.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0)
was created from `57c6977` and includes the public native archives, platform manifests
and SHA256SUMS. GitHub Private Vulnerability Reporting is enabled.

## Reproduce the local gate

Prerequisites: Rust 1.98.1 with rustfmt/clippy and rust-docs (library redistribution
notices), Python 3.12+, Node/npm with `npm sbom`, Ruby's standard YAML library, a C
toolchain, cargo-cyclonedx 0.5.9 and Python jsonschema 4.25.1 (isolated release tooling),
and the [actual Docker prerequisites](INSTALL.md) for the full gate.

```sh
cargo fetch --locked
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/platform-tests.py
cargo build --workspace --release --locked
python3 benchmarks/run-local.py "$(mktemp -d)/benchmark"
python3 scripts/readme-bench.py
python3 scripts/history-scan.py
python3 scripts/hygiene.py
python3 scripts/license-notices.py --check
python3 scripts/test-release.py
python3 scripts/validate-workflows.py
npm --prefix npm/b2ige test
python3 scripts/package-rc.py
python3 scripts/record-platform-gate.py
python3 scripts/npm-archive-smoke.py
cargo run --locked -p verify-cli --example validate_release -- release/release-manifest.schema.json release/release-manifest.json
python3 scripts/validate-archive.py release/artifacts
```

Heavy gates should run sequentially because some bounded timing tests are load-sensitive.
The history scanner reports candidates privately for human classification; exit zero means
the bounded scan finished, not that history is publishable. Its diagnostic output stays in
the local ignored `.b2ige/publication/` directory. No tool rewrites history.

Hosted macOS without Docker runs `platform-tests.py --without-docker` and
`record-platform-gate.py --without-docker`. These are explicit partial gates. They do not
stand in for the full 31-case benchmark or BlindTest runtime validation. Docker absence
must never turn a Docker verification requirement into success.

## Packaging and identity

The packager uses locked builds with host/source path remapping. Native/source/npm tarballs,
the npm-only CycloneDX SBOM, SHA256SUMS and the external target manifest are written to ignored
`release/artifacts/`. It selects the reviewed Git public inventory, not ignored local native
files or private runtime stores. Temporary npm staging supplies LICENSE/TRADEMARKS from the
root and runs `npm pack --ignore-scripts`. It never enables publishing.

Every archive is independently extracted; links, duplicate/unsafe paths, private files,
checksums, native binary digests, source inventory and required notices are checked. Public
synthetic examples/schemas remain public. Runtime hidden suites, canaries, raw Human evidence
and local screenshots/logs are excluded. Native binary bytes receive the same sensitive-pattern
scan. The npm archive smoke executes the extracted wrapper against the extracted native binary
and tests checksum refusal offline.

The packager requires pinned cargo-cyclonedx 0.5.9 and generates one CycloneDX 1.5
SBOM per workspace crate. Offline schema/reference/inventory checks cover all 143
locked packages. It fails if generation or validation fails. Install release-only tools:

```sh
cargo install cargo-cyclonedx --version 0.5.9 --locked
B2IGE_TOOL_DIR="$(mktemp -d)"
python3 -m venv "$B2IGE_TOOL_DIR/sbom-python"
"$B2IGE_TOOL_DIR/sbom-python/bin/pip" install jsonschema==4.25.1
export B2IGE_SBOM_PYTHON="$B2IGE_TOOL_DIR/sbom-python/bin/python"
```

`B2IGE_CYCLONEDX` may point to the pinned generator in an isolated tool directory.
An SBOM inventories dependencies; it is not a vulnerability scan. The dependency-free
npm wrapper receives its own separate CycloneDX SBOM.

Manifest v3 records base commit, dirty state, compiler, supported targets separately from
actually executed native targets, precise runtime scope, archive/binary/notice hashes,
B2IGE Verify Bench v1 (31 explicit cases) and scoped metrics, licensing, SBOM, unsigned
status, pending npm scope ownership and owner authorization=false. Embedded manifests describe
the pre-smoke inputs; the external index records successful later smoke. No claim of a
clean-tag or bit-for-bit reproducible build is made. See [provenance](RELEASE-PROVENANCE.md).

## Final candidate evidence

Candidate checks run `34974411043` and candidate validation and artifacts run `34991255871`
both completed with **SUCCESS**. Native verification covers Ubuntu 22.04 /
`x86_64-unknown-linux-gnu`, macOS arm64 and macOS Intel `x86_64`. Linux ran actual Docker
tests and the full fixed/reverse benchmark gate; macOS passed its documented partial
no-Docker scope. Release packaging, SBOM, manifest and npm archive checks passed. The
previous P3A signal fixture flake was a fixture cleanup race; the fix was included in the
final native Intel CI pass.

## CI and publication boundary

PR/manual `check.yml` reuses `release-candidate.yml`. The candidate workflow is callable and
manually dispatchable; each matrix target runs pinned setup actions, fmt, locked clippy,
appropriate tests, release build, package checks, CLI/product/archive smoke and npm checks.
Linux runs the actual Docker tests and fixed/reverse full benchmark. macOS runs the documented
non-Docker tests and archive/product/MCP smoke. Only selected candidate archives/SBOM/manifests
are retained for 14 days. Runner labels were checked against
[GitHub's runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

All workflows use `contents: read`; checkout does not retain credentials. There is no publish,
release creation, deployment, secret upload, signing credential or write permission. The
successful candidate runs above are validation evidence only; they did not make the repository
public or create a tag/release.

Use this procedure together with the checked-in release manifest, license notices and
provenance document to verify the public release state and any future release-specific actions.
The public Behavior brand is BehaviorSeal, while the compatible CLI remains `b2ige behavior ...`.
Current Cargo registry publication remains disabled because individual crate packages need a
deliberately reviewed full-workspace publication layout. The final npm package name is
`@b2ige/verify`, but npm publication is intentionally deferred until the `@b2ige` scope is
actually controlled. GitHub Private Vulnerability Reporting is enabled. macOS 0.1.0 is
unsigned and unnotarized; no Developer ID claim is made.
