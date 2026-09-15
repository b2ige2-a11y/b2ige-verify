# Quickstart

After [installation](INSTALL.md), aim for a first result in five minutes once Docker's
base image and native binaries are available. Initial compilation/image downloads may take longer.

From the source workspace use `scripts/demo-behavior.sh`, `scripts/demo-sideeffect.sh`, or
`scripts/demo-blindtest.sh`. From an extracted archive:

```sh
export B2IGE_BIN_DIR="$PWD/bin"
python3 scripts/demo.py all
```

Run that command from the archive's top-level directory. Each invocation creates a new temporary
controller directory, prepares reviewed example configs and checks actual CLI exits and reports.
The printed evidence path is trusted local data; do not upload it as a public issue or CI artifact.

- [Behavior: hello / changed output](../examples/behavior/README.md)
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
