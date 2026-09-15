# Integration v1

`b2ige init --dry-run` detects Rust/Node/Docker/existing config. `b2ige init` creates
an empty registry without approving any verifier. The operator registers typed configs
and separate approved Behavior authorization. `b2ige doctor` checks readiness.

MCP tools: `b2ige_doctor`, `b2ige_behavior_verify`, `b2ige_sideeffect_verify`,
`b2ige_blindtest_verify`, `b2ige_report`. Arguments:
```json
{"protocol_version":"1","product":"behavior","operation":"verify","identity":"reviewed-config-key","output":"agent","execution_budget":null}
```
Report uses the returned `source.artifact_id` as identity in the same server session;
operation is `report`. Doctor uses a config identity and `doctor`. Budgets reside in
reviewed product configs; overrides fail closed. Tools only return sanitized v1.

Portable bundle: copy this folder into the active Codex skills directory. This host
uses `~/.codex/skills`; an explicit `$CODEX_HOME/skills` overrides that location. Do
not invent repository-local config paths for other clients. No target-specific
installer is included; unsupported client installation must be performed according
to that client's documented setup. MCP setup and trust boundaries: `docs/MCP.md`
in the verifier repository. JSON schema: [response](../schemas/agent-protocol.v1.schema.json).
