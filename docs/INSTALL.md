# Installation — 0.1.0 candidate

No GitHub Release, crates.io or npm publication has occurred. Download instructions below
apply to a locally supplied candidate archive. Verify its SHA-256 against the accompanying
SHA256SUMS from a trusted channel; a checksum alone does not authenticate the publisher.

| Platform | Target | Candidate support |
|---|---|---|
| macOS Apple Silicon | aarch64-apple-darwin | Local native build and full smoke gate |
| macOS Intel | x86_64-apple-darwin | NOT_RUN; native CI prepared; prior cross-build/Rosetta evidence is supplemental |
| Linux x86_64 | x86_64-unknown-linux-gnu | NOT_RUN; Ubuntu 22.04+ native CI and Docker gate configured |
| Linux arm64 | aarch64-unknown-linux-gnu | Deferred until native runner gate is verified |
| Windows | — | Unsupported: current runner requires Unix capabilities |

macOS requires Docker Desktop running a Linux engine for BlindTest. Linux requires a local
Unix-socket Docker engine and permission to access it. No remote Docker TCP support is claimed.
Behavior needs `/bin/sh` for these examples. The SQLite library is bundled; no database service
is required. Demo scripts need Python 3.12+; the CLI/MCP themselves do not require Python or Node.
BlindTest's example image contains Node. Prepare it explicitly:

```sh
docker pull node:24.18.1-bookworm-slim
```

The demo resolves that local image to its RepoDigest before building with networking disabled.
You may choose another available digest-bearing Node image through `B2IGE_P6_BASE_IMAGE`;
record that choice with the run. Missing Docker/image prerequisites fail, never skip to PASS.

## Native archive

```sh
shasum -a 256 -c SHA256SUMS
# Substitute the archive's actual version/target file name.
tar -xzf b2ige-0.1.0-aarch64-apple-darwin.tar.gz
export PATH="$PWD/b2ige-0.1.0-aarch64-apple-darwin/bin:$PATH"
b2ige --version
b2ige-mcp --help
```

Keep all binaries together. `p5-effect-fixture` is a synthetic benchmark provider required
by `b2ige bench`; `b2ige-demo` and `b2ige-demo-effect` prepare public examples only.
Do not register these helpers as production verification targets accidentally.
On macOS this candidate is unsigned/unnotarized; publisher signing/distribution decision is pending; see [provenance](RELEASE-PROVENANCE.md).
No script removes quarantine or bypasses OS controls automatically.

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
See [release packaging](RELEASE.md). [npm wrapper](../npm/b2ige/README.md) has guarded GitHub delivery; no host or pins are configured yet.
