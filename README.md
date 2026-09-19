# B2IGE Verify

## Don't trust "done". Prove it.

B2IGE Verify is deterministic verification infrastructure for AI-written software. It runs
real programs under declared contracts, records evidence, and produces a bounded
PASS/FAIL/INCONCLUSIVE/ERROR result without asking the coding agent to grade its own work.
BlindTest makes the boundary concrete: hidden tests exercise the produced program without
exposing the suite or oracle to the agent.

[![Latest GitHub Release](https://img.shields.io/github/v/release/b2ige2-a11y/b2ige-verify?display_name=tag&sort=semver)](https://github.com/b2ige2-a11y/b2ige-verify/releases/latest)
[![Candidate checks](https://github.com/b2ige2-a11y/b2ige-verify/actions/workflows/check.yml/badge.svg)](https://github.com/b2ige2-a11y/b2ige-verify/actions/workflows/check.yml)
[![Apache-2.0](https://img.shields.io/github/license/b2ige2-a11y/b2ige-verify)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org/)

## Latest release

[B2IGE Verify v0.3.0](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.3.0)
is the current public release. The [final release record](docs/RELEASE-0.3.0-FINAL.md)
documents its exact-main qualification, platform boundaries, and public asset set.

| Platform | Native archive |
|---|---|
| macOS Apple Silicon | [b2ige-0.3.0-aarch64-apple-darwin.tar.gz](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.3.0/b2ige-0.3.0-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [b2ige-0.3.0-x86_64-apple-darwin.tar.gz](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.3.0/b2ige-0.3.0-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 | [b2ige-0.3.0-x86_64-unknown-linux-gnu.tar.gz](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.3.0/b2ige-0.3.0-x86_64-unknown-linux-gnu.tar.gz) |

Also download [SHA256SUMS](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.3.0/SHA256SUMS)
and verify the matching archive before extracting. See [installation details](docs/INSTALL.md)
for prerequisites and offline validation.

## Quick install

Choose the archive for the host, then verify, extract, and put `bin` on `PATH`:

```sh
B2IGE_RELEASE_URL=https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.3.0
B2IGE_ARCHIVE=b2ige-0.3.0-aarch64-apple-darwin.tar.gz
curl -fL -O "$B2IGE_RELEASE_URL/$B2IGE_ARCHIVE"
curl -fL -O "$B2IGE_RELEASE_URL/SHA256SUMS"

if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$B2IGE_ARCHIVE"     # compare with SHA256SUMS
else
  sha256sum "$B2IGE_ARCHIVE"         # compare with SHA256SUMS
fi
tar -xzf "$B2IGE_ARCHIVE"
cd "${B2IGE_ARCHIVE%.tar.gz}"
export PATH="$PWD/bin:$PATH"
b2ige --version
```

The archive is ready for the adoption flow or the product-specific commands. Docker is
required for BlindTest; missing prerequisites remain a readiness or non-PASS result.

## 30-second BlindTest demo

BlindTest asks: does the produced program actually work? It runs visible checks against a
correct and a buggy implementation, then executes sealed hidden cases in an attested Docker
boundary. The correct implementation passes, the buggy implementation fails, and the
sanitized Agent view is checked for private-value leakage.

From a source checkout or the source archive, with Docker available:

```sh
docker pull node:24.18.1-bookworm-slim
scripts/demo-blindtest.sh
```

The demo performs real execution and records evidence; its output is not canned terminal
text. See the [BlindTest example](examples/blindtest/README.md) for the bounded secrecy
boundary and the [threat model](docs/THREAT-MODEL.md) for its limits.

## Three verification products

| Product | Question | What it checks |
|---|---|---|
| [BlindTest](examples/blindtest/README.md) | Does it actually work? | Sealed hidden tests against an agent-produced target inside the declared Docker boundary. |
| [BehaviorSeal](examples/behavior/README.md) | Did it change? | Trusted reference/candidate behavior under equivalent deterministic experiments. |
| [SideEffect Proof](examples/sideeffect/README.md) | Did it actually happen? | Committed local SQLite effects under retries and fault schedules, not inferred attempts. |

## Easy Adoption

```text
inspect → init → prepare → trust approve → doctor → verify ID
```

`prepare` is non-authoritative: it discovers inputs and writes a reviewable draft. Trust
approval remains an interactive human/trusted-controller decision. `doctor` reports readiness,
not verification. `verify ID` resolves the approved identity and produces the real,
evidence-backed result. Read the [adoption guide](docs/ADOPTION.md) for the full boundary.

## Measured benchmark

These are committed measurements on the bounded B2IGE Verify Bench v1 corpus, not a claim of
exhaustive correctness or a replacement for product evidence.

<!-- benchmark:start -->
### B2IGE Verify Bench v1 / 31 explicit cases

| Measurement (on the benchmark corpus) | Observed |
|---|---:|
| Known bugs detected | 16 / 16 |
| False PASS | 0 |
| False FAIL | 0 |
| Verified reproduction | 16 / 16 |
| Hidden leakage observed | 0 |
| Agent private-value leakage | 0 |
| Mutation adequacy: known benchmark mutants | 2 / 2 |

All numbers above are on the benchmark corpus only. Bounded testing cannot establish complete correctness.
<!-- benchmark:end -->

See the [measured snapshot](benchmarks/baseline-v1/README.md) and [benchmark methodology](docs/BENCHMARKS.md).

## Platform support

| Platform | Support boundary |
|---|---|
| macOS Apple Silicon | Native release |
| macOS Intel | Native release |
| Linux x86_64 | Native release; Docker and full benchmark CI |
| Windows x64 | Source/build plus bounded CLI smoke only; not `VERIFIED_NATIVE` |
| Linux arm64 | Deferred |

macOS binaries are unsigned and unnotarized. See [installation and platform details](docs/INSTALL.md)
for Docker, source-build, and archive boundaries.

## What the result means

- **PASS / 0:** required evidence is complete within the declared executed scope, with no violation observed.
- **FAIL / 1:** a violation is supported by evidence.
- **INCONCLUSIVE / 2:** required evidence or execution coverage is incomplete.
- **ERROR / 3:** the verifier, configuration, isolation setup, or stored evidence failed.

Missing evidence cannot become PASS. Models do not assign verdicts. A result is bounded by the
declared experiment, evidence, and platform scope.

## Trust boundary and limitations

- Hashes provide integrity, not publisher authentication.
- Same-user host access is outside the BlindTest secrecy boundary.
- Bounded verification is not exhaustive proof.
- The internal v0.3.0 holdout was AI-operated, not independent human or external validation.
- Genuine external-user V110-D evidence is deferred; internal or synthetic evidence is not presented as external adoption.

Read the [threat model](docs/THREAT-MODEL.md), [contracts](docs/CONTRACTS.md), and
[evidence model](docs/EVIDENCE.md) before relying on a result.

## Documentation

Use the [documentation index](docs/README.md) for getting started, product guides, trust and
architecture, benchmarks, release history, and developer resources. The [false PASS report](https://github.com/b2ige2-a11y/b2ige-verify/issues/new?template=false_pass.yml)
accepts only synthetic or sanitized details; never upload hidden suites, private canaries, raw
human evidence, or credentials.

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture](docs/ARCHITECTURE.md), and
the [security policy](SECURITY.md). Changes to verifier authority, evidence, contracts, or schemas
require deliberate review; missing evidence must never be converted into success.

## License

B2IGE Verify is licensed under [Apache-2.0](LICENSE). No telemetry, hosted service, or paid API
is required. Product names and trademarks are covered separately by [TRADEMARKS.md](TRADEMARKS.md).
