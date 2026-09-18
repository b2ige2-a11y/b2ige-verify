# Quickstart

After [installation](INSTALL.md), the fastest first result is the BlindTest demo. With native
binaries and Docker's base image already available, it typically takes 30–60 seconds; initial
archive extraction, image downloads and source builds can take longer. Docker is required for
BlindTest, and missing prerequisites do not become PASS.

## 30-second BlindTest path

From the source workspace:

```sh
docker pull node:24.18.1-bookworm-slim
python3 scripts/setup.py
scripts/demo-blindtest.sh
```

`setup.py` is a safe bootstrap: it builds the locked workspace if needed and creates only
the empty registry. It does not approve a baseline or discover a hidden suite. Windows x64
can use `py -3 scripts/setup.py` for source/build setup, but this Mac run makes no Windows
runtime or Docker claim; Windows CI-only setup does not require the Docker/demo lines above.

From an extracted native release archive, run from its top-level directory:

```sh
export PATH="$PWD/bin:$PATH"
export B2IGE_BIN_DIR="$PWD/bin"
b2ige doctor                 # fresh archive: ready=false until a project is registered
b2ige blindtest doctor      # Docker readiness; not a verification verdict
scripts/demo-blindtest.sh
```

On a fresh archive, `b2ige doctor` prints `ready=false` and exits 3 because no project is
registered yet. That readiness result is expected and is not a verification verdict; continue with
`b2ige blindtest doctor` for the Docker preflight.

The demo executes both implementations against the same visible valid-credential check, then
performs the actual sealed hidden verification: correct `PASS`, buggy `FAIL`, and probe `PASS`.
It also checks that private hidden values do not appear in the Agent projection. There is no
fabricated terminal output. See the [README demo](../README.md#30-second-blindtest-demo) for
release archive links and the [Docker prerequisites](INSTALL.md).

## Run all three demos

From the source workspace use `scripts/demo-behavior.sh`, `scripts/demo-sideeffect.sh`, or
`scripts/demo-blindtest.sh`. From an extracted archive:

```sh
export B2IGE_BIN_DIR="$PWD/bin"
python3 scripts/demo.py all
```

Run that command from the archive's top-level directory. Each invocation creates a new temporary
controller directory, prepares reviewed example configs and checks actual CLI exits and reports.
The printed evidence path is trusted local data; do not upload it as a public issue or CI artifact.

- [BehaviorSeal: hello / changed output](../examples/behavior/README.md)
- [SideEffect: safe and unsafe SQLite retries](../examples/sideeffect/README.md)
- [BlindTest: visible tests pass, hidden contract fails](../examples/blindtest/README.md)

## Use your own project

For the new adoption commands, build the current source checkout (the published
v0.2.0 archives are unchanged). Follow [Easy Adoption](ADOPTION.md):

```text
b2ige inspect → b2ige init → b2ige prepare --product PRODUCT
→ b2ige trust approve (independent trusted operator)
→ b2ige doctor → b2ige verify REGISTERED_ID
```

Inspection and preparation are non-authoritative. Init is safe to repeat. Approval
requires an interactive controller terminal and existing independent trust inputs;
it cannot promote a candidate into the baseline or create a hidden suite. Doctor is
readiness only. The guide includes all three products, exact preparation options,
controller setup, path rules and recovery. Existing expert commands below and
[MCP setup](MCP.md) remain supported. A directory outside the project is not an
OS-level boundary against an unrestricted process with the same host user.

Use `--output agent --protocol 1` for integrations. Use `b2ige report` to reload an evidence-backed
result, with `--output human|json|agent`. Human and JSON BlindTest views are trusted private views.
The raw store must remain available; exported report JSON is not authoritative input.

## Current-source installation and CI bootstrap

For safe version-pinned/offline installation and reviewed `b2ige ci init` preview,
see [V110-C distribution and CI](V110-DISTRIBUTION.md). V110 commands require the
current reviewed source build; published v0.2.0 does not include them. Generated CI
fails closed until candidate build provisioning and controller approval are configured.
