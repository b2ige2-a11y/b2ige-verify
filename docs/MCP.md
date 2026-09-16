# Local MCP integration — P7

`cargo build --workspace --release` produces `b2ige` and `b2ige-mcp`.
No cloud, telemetry, paid API, LLM, remote transport or shell gateway is involved.

## Trusted setup

1. `b2ige init --dry-run` inspects Cargo.toml/package.json, Docker availability and
   existing `.b2ige/project.json`. Detection is bounded; it does not prove stack support.
2. `b2ige init` exclusively creates an empty v1 registry. Existing files and symlinks
   are never overwritten; application source and AGENTS.md are unchanged.
3. A trusted operator registers reviewed configs with absolute paths:

```json
{
  "schema_version": "1",
  "entries": {
    "login": {
      "product": "behavior",
      "config": "/trusted/login.experiment.json",
      "store": "/trusted/public-runs",
      "authorization": "/trusted/authorization.json"
    }
  }
}
```

SideEffect/BlindTest entries use their existing typed config and `authorization: null`.
BlindTest store must be outside the coding workspace, preferably
`/trusted/sealed/runs`. Set `B2IGE_BLINDTEST_SEALED_ROOT=/trusted/sealed` in the trusted
controller environment. Prepare its approved suite and locally built immutable target
image according to [BLINDTEST](BLINDTEST.md). Do not give a hostile coding agent write
access to the registry/approval policy, sealed suite or server executable.

4. `b2ige doctor --config /trusted/project.json` checks config shape/semantics, target
   hashes, fixtures, approved Behavior binding, writable storage and applicable
   BlindTest suite/path/image/Docker prerequisites. It does not execute verification
   or attest a container. SideEffect also checks a complete, empty initial ledger
   snapshot; doctor does not prove coverage of future execution.
5. Launch `b2ige-mcp --registry /trusted/project.json` as a stdio child process.

Relative config/storage paths are relative to the server's working directory, not the
registry file. Absolute paths avoid client working-directory ambiguity. Registry/config
contents and Behavior authorization are loaded at startup. Invalid entries stay disabled
and return fixed ERROR. Restart after reviewed config changes. Executable/fixture changes
are still checked by the verifier, so stale identity pins cannot PASS.

## Codex

From a source checkout, run `python3 scripts/setup.py` once to build the locked binaries and
create the empty project registry. It does not register unreviewed configs or alter baseline
authorizations. Then launch the trusted MCP binary with absolute paths:

```sh
codex mcp add b2ige -- /absolute/verifier/target/release/b2ige-mcp --registry /trusted/project.json
```

For BlindTest supply the controller environment using the supported `--env` option:

```sh
codex mcp add b2ige --env B2IGE_BLINDTEST_SEALED_ROOT=/trusted/sealed -- /absolute/verifier/target/release/b2ige-mcp --registry /trusted/project.json
```

Run these commands only when installing into the user's chosen client. P7 does not
change this host's Codex settings. Portable [Skill v1](../skills/b2ige-verify/SKILL.md)
can be copied into the configured Codex skills directory (`$CODEX_HOME/skills`, normally
`~/.codex/skills`), without overwriting an existing skill. No guessed repository-local
Codex config or installer is provided. For other clients use their documented stdio
setup; unsupported target-specific automatic installation is not offered.
If the registry or installed binary is damaged, rerun setup only after preserving the original
and restoring a reviewed copy. A setup/doctor result is readiness, not a verification verdict.

## Tools and transport

Five tools: `b2ige_doctor`, `b2ige_behavior_verify`, `b2ige_sideeffect_verify`,
`b2ige_blindtest_verify`, `b2ige_report`. All take the
[Agent Request](AGENT-PROTOCOL.md#request). Tool/product/operation must agree.
Verify/doctor identity is a registry key. Report identity is the returned
`source.artifact_id`; the alias is resolved server-side and reloaded with the product
loader. Report aliases last for the process lifetime. No arbitrary path is accepted.
For historical reports after restarting, use the trusted CLI `report` command.

Implementation pins the [MCP 2025-11-25 lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
and [tool response format](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
Newline-delimited JSON-RPC 2.0 over stdio; initialize, initialized notification, ping,
tools/list, tools/call. Responses include structuredContent and matching text JSON.
Non-PASS tool outcomes use `isError: true`; clients must inspect verdict and operation.
Unknown methods/tools, malformed envelopes/arguments and pre-initialization calls fail
closed. Input is bounded to 1 MiB per line. stdout contains protocol messages only.

Calls are synchronous/serial. Cancellation, progress, pagination, remote HTTP and
interactive approval are outside this bounded v1 server. Product-configured time,
capture, case and attempt budgets still apply. Forcibly stopping a server is not a
successful tool result; rerun/reload rather than assuming completion.

## Security boundary

The exposed arbitrary shell/file/SQL/Docker-exec/raw-report tool count is zero.
Tools select registered typed configs; they cannot inject args/env/commands/storage.
Behavior and SideEffect inherently execute trusted local configured programs; registration
is execution authorization and is not a sandbox. BlindTest targets keep the existing
Docker isolation and sanitation. Config registration does not create stronger host-user
isolation or hide controller files from another unrestricted same-user process.
All result paths go through verified loading. Stored projections are never imported as
source evidence. Raw hidden reports are not exposed as resources, tools or CI artifacts.
