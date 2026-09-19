# Installation — B2IGE Verify 0.3.0

The public [v0.3.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.3.0)
is the current native distribution. Download a matching archive and `SHA256SUMS`, then verify
the archive before extraction. For a trusted local copy:

```sh
python3 scripts/install-release.py --offline --version 0.3.0 \
  --archive /trusted/b2ige-0.3.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Choose the actual host target. The [final release record](RELEASE-0.3.0-FINAL.md) documents
the release assets and qualification boundary.

## Platform support

The current v0.3.0 platform boundary is:

| Platform | Target | Release support |
|---|---|---|
| macOS Apple Silicon | aarch64-apple-darwin | Native release; exact-main CI and internal holdout qualified |
| macOS Intel | x86_64-apple-darwin | Native release; documented no-Docker CI scope qualified |
| Linux x86_64 | x86_64-unknown-linux-gnu | Native release; actual Docker tests and full benchmark CI qualified |
| Linux arm64 | aarch64-unknown-linux-gnu | Deferred until native runner gate is verified |
| Windows x64 | x86_64-pc-windows-msvc | Source/build plus bounded CLI smoke only; not `VERIFIED_NATIVE` |

macOS requires Docker Desktop running a Linux engine for BlindTest. Linux requires a local
Unix-socket Docker engine and permission to access it. No remote Docker TCP support is claimed.
BehaviorSeal examples need `/bin/sh`. The SQLite library is bundled; no database service
is required. Demo scripts need Python 3.12+; the CLI/MCP themselves do not require Python or Node.
BlindTest's example image contains Node. Prepare it explicitly:

```sh
docker pull node:24.18.1-bookworm-slim
```

The demo resolves that local image to its RepoDigest before building with networking disabled.
You may choose another available digest-bearing Node image through `B2IGE_P6_BASE_IMAGE`;
record that choice with the run. Missing Docker/image prerequisites fail, never skip to PASS.

Windows source builds use the MSVC Rust target and are checked by the dedicated
`windows-latest` workflow job. The Unix process-cleanup backend and the P6 Docker endpoint
are not asserted on Windows. Do not turn a successful Windows compile into a product
verification result; the workflow is the platform evidence path. A Windows native
archive/MCPB is not listed until a Windows host build and installation smoke have been
independently run.

## Native archive

The reviewed [installation helper](../scripts/install-release.py) installs the current
v0.3.0 archive without modifying its artifacts:

```sh
python3 scripts/install-release.py --version 0.3.0 --destination /existing/new-install
# With manually acquired trusted assets; no network:
python3 scripts/install-release.py --offline \
  --archive /trusted/b2ige-0.3.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Use an existing parent and a new destination. The helper validates integrity and
archive structure before extraction or execution. It is current-source tooling,
not a file retroactively added to the v0.3.0 native archive.
See [V110-C distribution](V110-DISTRIBUTION.md) for exact CLI options, offline recovery,
archive safety and platform boundaries.
Add the extracted package's `bin` directory to PATH manually.

Keep all binaries together. `p5-effect-fixture` is a synthetic benchmark provider required
by `b2ige bench`; `b2ige-demo` and `b2ige-demo-effect` prepare public examples only.
Do not register these helpers as production verification targets accidentally.
macOS binaries in the v0.3.0 release are intentionally unsigned and unnotarized. Gatekeeper
may show a warning; signing is a future release improvement.
See [provenance](RELEASE-PROVENANCE.md). No script removes quarantine or bypasses OS controls automatically.

For a source checkout, the near-one-command bootstrap is:

```sh
python3 scripts/setup.py
```

On Windows use `py -3 scripts/setup.py` when the Python launcher is installed. The helper
builds with `cargo build --workspace --release --locked` when needed, creates only an empty
`.b2ige/project.json`, and never approves or rewrites a contract. For an extracted native
archive, pass the already trusted binary and skip the source build:

```sh
python3 scripts/setup.py --binary /absolute/trusted/bin/b2ige --skip-build
```

On Windows, the equivalent is `py -3 scripts/setup.py --binary C:\trusted\bin\b2ige.exe --skip-build`.

If setup reports `recovery_required`, preserve the reported registry and recover it from a
reviewed backup before retrying. Setup deliberately has no overwrite or automatic baseline
repair mode.

## Source workspace / Cargo

From the full reviewed source workspace (or the supplied source archive):

```sh
cargo build --workspace --release --locked
cargo install --path crates/verify-cli --locked
cargo install --path crates/verify-mcp --locked
cargo install --path crates/verify-core --bin p5-effect-fixture --locked
```

The CLI install includes `b2ige-demo` and `b2ige-demo-effect`. Keep the benchmark provider
beside `b2ige`. Individual `.crate` registry publication is intentionally disabled:
workspace-external benchmark/example/test sources require the whole source distribution.
No `cargo install verify-cli` registry command is claimed. Source builds require Rust stable,
a C toolchain, and Cargo dependencies (download on first build; Cargo.lock pins versions).
See the [final release record](RELEASE-0.3.0-FINAL.md). The [npm wrapper](../npm/b2ige/README.md)
keeps its publication guard active; npm remains deferred and is not required for this release.

## Current source versus published release

V110-A adoption commands and V110-C `ci init` are in the public 0.3.0 native release.
The source archive also includes the V110-B benchmark runner and V110-D pilot tooling;
they require a reviewed Git checkout and source build as documented. Genuine V110-D
external-user evidence remains deferred. See the
[V110 distribution boundary and platform table](V110-DISTRIBUTION.md).

## Historical v0.2.0 installation

The prior [v0.2.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.2.0)
remains available for historical reproduction. Its artifacts are unchanged and it is not the
current download path.
