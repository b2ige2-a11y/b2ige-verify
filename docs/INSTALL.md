# Installation — B2IGE Verify 0.3.0 candidate

The 0.3.0 candidate is local only. Use exact local artifacts before publication:

```sh
python3 scripts/install-release.py --offline --version 0.3.0 \
  --archive /trusted/b2ige-0.3.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Choose the actual host target. See [0.3.0 scope and pending gates](RELEASE-0.3.0.md).
No 0.3.0 public download is claimed; online `--version 0.3.0` is for after publication.

## Historical public 0.2.0 installation

0.2.0 is published. Its [v0.2.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.2.0)
is the installation source, with public archives and SHA256SUMS. The instructions below apply to
that release archive. Final external CI and the final fresh isolated holdout completed before
publication.
Verify its SHA-256 against the accompanying SHA256SUMS from a trusted channel; a checksum alone
does not authenticate the publisher.
The final npm package name is `@b2ige/verify`, but npm publication is intentionally deferred
until the `@b2ige` scope is actually controlled.

| Platform | Target | Release support |
|---|---|---|
| macOS Apple Silicon | aarch64-apple-darwin | VERIFIED_NATIVE; documented partial no-Docker scope PASS |
| macOS Intel | x86_64-apple-darwin | VERIFIED_NATIVE; documented partial no-Docker scope PASS |
| Linux x86_64 | x86_64-unknown-linux-gnu | VERIFIED_NATIVE; actual Docker tests and full fixed/reverse benchmark gate PASS |
| Linux arm64 | aarch64-unknown-linux-gnu | Deferred until native runner gate is verified |
| Windows x64 | x86_64-pc-windows-msvc | SOURCE/BUILD/CI_ONLY; Windows runtime and archive evidence are not recorded locally |

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

The reviewed current-source [installation helper](../scripts/install-release.py) can
install historical v0.2.0 without modifying its artifacts:

```sh
python3 scripts/install-release.py --version 0.2.0 --destination /existing/new-install
# With manually acquired trusted assets; no network:
python3 scripts/install-release.py --offline \
  --archive /trusted/b2ige-0.2.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Use an existing parent and a new destination. The helper validates integrity and
archive structure before extraction or execution. It is current-source tooling,
not a file retroactively added to v0.2.0. See [V110-C distribution](V110-DISTRIBUTION.md)
for exact CLI options, offline recovery, archive safety and pending platform gates.
Add the extracted package's `bin` directory to PATH manually.

Keep all binaries together. `p5-effect-fixture` is a synthetic benchmark provider required
by `b2ige bench`; `b2ige-demo` and `b2ige-demo-effect` prepare public examples only.
Do not register these helpers as production verification targets accidentally.
For the historical v0.1.0 release, macOS binaries were intentionally unsigned and unnotarized.
The 0.2.0 release makes no Developer ID signing or notarization claim. Gatekeeper may show a
warning; signing is a future release improvement.
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
See [current candidate packaging](RELEASE-0.3.0.md). The [npm wrapper](../npm/b2ige/README.md) keeps its
publication guard active; npm remains deferred and is not required for this 0.2.0 release.
The historical GitHub 0.1.0 release also did not require npm.

## Current source versus published release

V110-A adoption commands and V110-C `ci init` are in the 0.3.0 native candidate.
The source archive also includes the V110-B benchmark runner and V110-D pilot tooling;
they require a reviewed Git checkout and source build as documented. Historical
published v0.2.0 remains unchanged. No 0.3.0 publication is claimed. See the
[V110 distribution boundary and platform table](V110-DISTRIBUTION.md).
