# V100-5 local release foundation

V100 is the 0.2.0 local release candidate, separate from the historical 0.1.0
release described in RELEASE.md. Historical public-release flags, CI runs,
manifests and benchmark snapshots do not qualify the current source. Package
version is 0.2.0 for this candidate;
no tag, commit, push, publication or deployment is part of this phase.

## Local deterministic gate

From the repository root, with installed locked dependencies and a native Unix
Rust toolchain, run:

```sh
python3 scripts/v100-release-gate.py /absolute/new-controller-gate-directory
```

The script runs targeted protocol/trust/receipt/benchmark/integration conformance,
packaging and adoption safeguards, public-file hygiene, workflow validation, fmt, scoped clippy and a
workspace release build. Cargo is offline; tests use four threads. It never runs
the full workspace test suite, fetches dependencies, updates baselines or reads
private holdout material. Failed/timeout/spawn-error checks and zero Rust tests
block; the gate stops at the first failure. Each command is bounded to 900 seconds.

The new output directory must be outside the repository and starts mode 0700.
It retains ordered commands, exit codes and log hashes in `result.json`, with a
SHA-256 inventory of public source bytes before/after (including uncommitted files).
Output is initially BLOCKED_LOCAL, so interruption cannot leave a success record.
Source changes during execution block readiness. Do not edit source during a run.
Logs are controller diagnostics and must not be packaged or uploaded automatically.

Exit 0 means only `WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT`; exit 1 is BLOCKED_LOCAL.
The report always has `publication_ready: false`. It is independent tooling schema
v1, not a product verdict, signed attestation or substitute for independent audit.
It cannot authenticate a hostile controller replacing source, script and output.

## Outer runner and release packaging

The outer runner owns the authoritative full workspace fmt/clippy/test/build gate.
It must also run actual Docker integration and a fresh complete public benchmark:

```sh
python3 benchmarks/run-local.py /absolute/new-full-public-benchmark
```

This requires both complete 31-case fixed/reverse gates and recomputed semantic
equality. Scoped product runs cannot satisfy it. A baseline JSON or controller
self-report cannot substitute for fresh execution and verified product loading.
See BENCHMARKS.md for denominators, source retention and bounded claims.

Packaging uses the existing `scripts/package-rc.py`, license notices, offline SBOM
validation, `validate-archive.py`, `record-platform-gate.py` and npm archive smoke
workflow in RELEASE.md. Native packages include protocol/release docs and the
conformance map; source packages include gate scripts and executable tests.
Packaging alone grants no runtime or publication readiness. Source inventory,
binary/archive hashes, source-change checks, clean public inventory, private-data
exclusion and owner publication safeguards remain mandatory. No old manifest is
rewritten to claim current qualification.

External native CI, independent audit and private holdout are separate outstanding
requirements. The development agent never accesses or executes private holdout
material. Even after every locally achievable check passes, final local state is
`WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT`, never independent holdout PASS.

## Final v0.2.0 release record

The historical local gate above retained its bounded meaning: its
`WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT` output did not prove external completion, holdout
completion, tagging or publication. Those later release steps completed separately before the
public v0.2.0 release:

- Release commit/tag target: `1fc1b8cda1e005a8d90ce5b7388cc2a62538b8d8`.
- Final exact-main GitHub Actions run: `35159158561`. Windows source/build, macOS Apple Silicon,
  macOS Intel and Linux x86_64 actual-Docker/full benchmark checks were PASS.
- Final fresh isolated holdout: PASS. Sanitized aggregate: RealBench 14/14, RedBench/trust 35/35,
  leakage 6/6, regression 6/6, total 61/61.
- Holdout authoritative checking was deterministic; an LLM did not directly assign product verdicts.
  It used a dedicated workspace and isolated Docker-in-Docker candidate execution under the same
  macOS user account. This record does not claim OS-account-level separation or stronger
  independence than that setup established.
- The `v0.2.0` tag and [GitHub Release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.2.0)
  are published. Npm publication remains deferred/private; macOS binaries remain unsigned and
  unnotarized, and Windows remains source/build CI only without a Windows runtime/native archive
  release.
