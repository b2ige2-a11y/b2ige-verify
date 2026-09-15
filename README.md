# B2IGE Verify

**Don't warn. Prove.**

Deterministic verification infrastructure for AI-written software.

| Product | Question | Start |
|---|---|---|
| **BlindTest** | Does it actually work? Hidden tests within the declared Docker boundary. | [Hidden verification](examples/blindtest/README.md) |
| **Behavior** (working name; BehaviorSeal candidate) | Did it change? | [Before / after](examples/behavior/README.md) |
| **SideEffect Proof** | Did it actually happen? | [SQLite retries](examples/sideeffect/README.md) |

Try a real before/after comparison from a local candidate build:

```sh
cargo build --workspace --release
scripts/demo-behavior.sh
```

Expected: `behavior pass: PASS`, then `behavior fail: FAIL`.
The demo executes both programs and reloads their evidence. It does not print a fabricated verdict.

For the coding-agent story, run `scripts/demo-blindtest.sh` after the
[Docker prerequisites](docs/INSTALL.md): both implementations pass a visible valid-credential
check; hidden verification finds the buggy implementation's expired-credential violation.
The Agent view excludes the hidden case itself. [Threat boundary](docs/THREAT-MODEL.md).

## Install and run

**0.1.0 local release candidate. No public release or registry package exists yet.**
Native archives are the authoritative distribution. See [installation and platforms](docs/INSTALL.md),
[5-minute quickstarts](docs/QUICKSTART.md), and [release blockers](release/checklist.md).

```sh
b2ige --help
b2ige init --dry-run
b2ige init
b2ige doctor
```

`init` creates an empty registry; doctor reports readiness only. Register a reviewed example
before expecting readiness. Product commands: `b2ige behavior verify`, `b2ige sideeffect verify`,
`b2ige blindtest verify`. Evidence reports: `b2ige report`. Bounded benchmark: `b2ige bench`.
[CLI reference](crates/verify-cli/README.md) · [MCP server](docs/MCP.md) · [Agent Skill](skills/b2ige-verify/SKILL.md).

## Measured benchmark

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
Behavior executes trusted Unix programs; BlindTest isolates Linux targets in Docker.
Hashes provide integrity checks, not authentication. Same-user host access is outside the secrecy boundary.

## Shared core and development

B2IGE Verify Core is shared OSS technology, maintained in one Rust workspace.
Products keep separate CLI/config/docs surfaces without splitting repositories.
[Contracts](docs/CONTRACTS.md) · [Architecture](docs/ARCHITECTURE.md) ·
[Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md).

## License

Code is licensed under [Apache-2.0](LICENSE). Trademarks and product names are
handled separately; see [TRADEMARKS.md](TRADEMARKS.md). The code license does
not grant trademark or other brand-use rights. The current Core, CLI, BlindTest,
Behavior, SideEffect Proof, MCP, Skill, and Bench are OSS scope; future team,
enterprise, or managed features may be separate commercial offerings.

No telemetry, hosted service, or paid API is required.
