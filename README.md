# B2IGE Verify

## Don't trust "done". Prove it.

**Hidden tests your coding agent can't see.**

An AI coding agent can say a task is done while a hidden edge case still breaks the contract.
BlindTest runs sealed hidden tests against the produced program in an attested Docker boundary,
independently of the agent's self-assessment. B2IGE Verify is deterministic verification
infrastructure for AI-written software: actual execution and recorded evidence—not an LLM—produce
the verdict.

```text
Correct implementation → PASS
Buggy implementation   → FAIL
```

Start with the [30-second BlindTest demo](#30-second-blindtest-demo).

## 30-second BlindTest demo

BlindTest is the fastest way to see the coding-agent use case. It runs the same visible
valid-credential check against both implementations, then sends them through sealed hidden tests:
the correct implementation passes, while the buggy implementation fails on an expired-credential
case. The run is real—it executes Docker targets, records evidence, and checks the sanitized Agent
view for private-value leakage.

With the native binary and Docker image already available, expect about 30–60 seconds. The first
Docker pull or image build can take longer. Docker is required; missing Docker or image
prerequisites stop the demo and do not turn into PASS.

### Download the v0.1.0 native binary

Choose the archive for your machine from the official [v0.1.0 GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0):

| Platform | Native archive |
|---|---|
| macOS Apple Silicon | [aarch64-apple-darwin](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/b2ige-0.1.0-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [x86_64-apple-darwin](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/b2ige-0.1.0-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 | [x86_64-unknown-linux-gnu](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/b2ige-0.1.0-x86_64-unknown-linux-gnu.tar.gz) |

Also download [SHA256SUMS](https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/SHA256SUMS).
Verify the matching archive line before extracting. The full [installation and platform guide](docs/INSTALL.md)
has the checksum and prerequisite details.

Windows x64 is currently supported for reviewed source/build CI (`x86_64-pc-windows-msvc`),
but no Windows runtime or native archive evidence is claimed by this Mac checkout.

### Download → extract → doctor → demo

```sh
# Choose the matching archive name from the table above.
B2IGE_RELEASE_URL=https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0
B2IGE_ARCHIVE=b2ige-0.1.0-aarch64-apple-darwin.tar.gz
curl -fL -O "$B2IGE_RELEASE_URL/$B2IGE_ARCHIVE"
curl -fL -O "$B2IGE_RELEASE_URL/SHA256SUMS"

# Compare this digest with the matching line in SHA256SUMS.
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$B2IGE_ARCHIVE"
else
  sha256sum "$B2IGE_ARCHIVE"
fi

tar -xzf "$B2IGE_ARCHIVE"
cd "${B2IGE_ARCHIVE%.tar.gz}"
export PATH="$PWD/bin:$PATH"
export B2IGE_BIN_DIR="$PWD/bin"

# A fresh archive has no registered project, so doctor reports readiness=false.
# It is readiness only, not a verification verdict.
b2ige doctor
docker pull node:24.18.1-bookworm-slim
b2ige blindtest doctor
scripts/demo-blindtest.sh
```

On a fresh archive, `b2ige doctor` exits 3 until a project is registered; that is expected for an
empty registry. `b2ige blindtest doctor` is the Docker readiness check for this demo. For a source
checkout, build once with `cargo build --workspace --release --locked`, pull the same Node image,
and run `scripts/demo-blindtest.sh` from the repository root.

The expected signals are:

```text
Coding-agent-visible valid-credential tests: PASS (correct and buggy)
blindtest correct: PASS
blindtest buggy: FAIL
blindtest probe: PASS
Agent private-value leakage: 0
```

These are produced by the actual demo runner, not canned terminal output. From a current repository
checkout, capture the same real terminal session for review outside the repository (the fixed
v0.1.0 release assets are unchanged):

```sh
scripts/record-demo-blindtest.sh /tmp/b2ige-blindtest.typescript
```

The recording helper uses the platform `script` utility and does not generate a fake GIF. No GIF is
checked into this release documentation; convert the actual typescript recording with a trusted
local renderer if a visual asset is needed.

## Three ways to verify

BlindTest is the first entry point for AI-written changes; the other products cover different
failure questions without being removed or reduced:

| Product | Question | What it does | Start |
|---|---|---|---|
| **BlindTest** | Does it actually work? | Independently runs sealed hidden tests against an agent-produced target inside the declared Docker boundary. | [BlindTest example](examples/blindtest/README.md) |
| **BehaviorSeal** | Did it change? | Compares trusted baseline and candidate behavior under equivalent deterministic experiments. The compatibility CLI remains `b2ige behavior ...`. | [BehaviorSeal example](examples/behavior/README.md) |
| **SideEffect Proof** | Did it actually happen? | Checks committed local SQLite effects under retries and fault schedules, rather than inferring success from attempts. | [SideEffect Proof example](examples/sideeffect/README.md) |

The [5-minute quickstart](docs/QUICKSTART.md) covers the three product demos. The [threat boundary](docs/THREAT-MODEL.md)
explains what BlindTest does and does not claim.

## Measured benchmark

The numbers below are the committed B2IGE Verify Bench v1 baseline on its bounded corpus. They are
measurements, not a claim of exhaustive correctness or a replacement for product evidence.

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

[Measured snapshot](benchmarks/baseline-v1/README.md) · [Methodology and denominators](docs/BENCHMARKS.md).
`python3 scripts/readme-bench.py` checks this table directly against the baseline.

## What the result means

- **PASS / 0:** no violation in the declared executed scope, with required evidence complete.
- **FAIL / 1:** a violation supported by evidence.
- **INCONCLUSIVE / 2:** evidence or execution coverage is incomplete.
- **ERROR / 3:** verifier, configuration, isolation setup or stored-evidence failure.

Missing evidence cannot become PASS. A model never assigns these verdicts. Bounded testing
cannot establish complete correctness. SideEffect currently observes local SQLite ledgers;
BehaviorSeal executes trusted Unix programs; BlindTest isolates Linux targets in Docker.
Hashes provide integrity checks, not authentication. Same-user host access is outside the secrecy boundary.

## Found a false PASS?

Found a case B2IGE incorrectly passed? [Open a False PASS report.](https://github.com/b2ige2-a11y/b2ige-verify/issues/new?template=false_pass.yml)
Use synthetic or sanitized details only; never upload a sealed suite, oracle, private canary, raw
Human evidence or credentials. The [security policy](SECURITY.md) explains when to use private
vulnerability reporting instead.

## Install and run

Native archives are the authoritative first distribution. macOS 0.1.0 binaries are intentionally
unsigned and unnotarized; Gatekeeper may show a warning, and no Developer ID signing or notarization
claim is made. See [installation and platforms](docs/INSTALL.md) for the complete policy.

For a source checkout, `python3 scripts/setup.py` performs the locked release build when needed
and safely creates the empty project registry. It never invents contracts or approves baselines;
registration and the [Codex/GitHub workflow](docs/CI.md) remain explicit trusted-operator steps.

The final npm package name is `@b2ige/verify`, but npm publication is intentionally deferred until
the `@b2ige` scope is actually controlled; npm is not required for the GitHub 0.1.0 release.

## Shared core and development

B2IGE Verify Core is shared OSS technology, maintained in one Rust workspace.
Products keep separate CLI/config/docs surfaces without splitting repositories.
[Contracts](docs/CONTRACTS.md) · [Architecture](docs/ARCHITECTURE.md) ·
[CLI reference](crates/verify-cli/README.md) · [MCP server](docs/MCP.md) ·
[Agent Skill](skills/b2ige-verify/SKILL.md) · [Contributing](CONTRIBUTING.md).

## License

Code is licensed under [Apache-2.0](LICENSE). Trademarks and product names are
handled separately; see [TRADEMARKS.md](TRADEMARKS.md). The code license does
not grant trademark or other brand-use rights. The current Core, CLI, BlindTest,
BehaviorSeal, SideEffect Proof, MCP, Skill, and Bench are OSS scope; future team,
enterprise, or managed features may be separate commercial offerings.

No telemetry, hosted service, or paid API is required.
