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

`b2ige init --dry-run`, then `b2ige init` creates `.b2ige/project.json` without overwriting files.
It does not invent contracts, approve baselines, or discover hidden suites. Review a product
example config, pin your target/fixture identities and register its absolute path as described in
[MCP setup](MCP.md). Run `b2ige doctor`, then the product's `verify` command. A ready doctor is
not a product PASS. Never approve a changed baseline just to make a failing candidate pass.

Use `--output agent --protocol 1` for integrations. Use `b2ige report` to reload an evidence-backed
result, with `--output human|json|agent`. Human and JSON BlindTest views are trusted private views.
The raw store must remain available; exported report JSON is not authoritative input.
