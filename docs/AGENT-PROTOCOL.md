# Agent Protocol v1 — P7

Agent/MCP/Skill/CI are invocation and presentation layers. They never assign product
verdicts. Execution uses the existing product API; results are reloaded through the
P4/P5/P6 verified loaders before projection. No evidence is inferred or repaired.

## Version decision and compatibility

Independent `Agent Protocol v1`, request v1 and project registry v1 are added.
Existing authoritative artifacts and ReportDocument/AgentReport v3 stay unchanged.
Existing CLI `--output agent` retains AgentReport v3 for backward compatibility.
Use `--output agent --protocol 1` for the stable response, or MCP (always v1).
JSON/human projections remain complete trusted local views, including BlindTest
human-only evidence. They must not be supplied to the coding agent or auto-uploaded.

Schemas: [request](../schemas/agent-request.v1.schema.json),
[response](../schemas/agent-protocol.v1.schema.json). Generate with
`cargo run -p verify-cli --example schemas`. Protocol output is deterministic for the
same verified artifact and operation; separate executions have distinct source IDs,
evidence timestamps/hashes and may differ. Determinism is not cross-run byte equality.

## Request

```json
{"protocol_version":"1","product":"behavior","operation":"verify",
 "identity":"login","output":"agent","execution_budget":null}
```

Products: `behavior`, `sideeffect`, `blindtest`. Operations: `verify`, `report`,
`doctor`. Identity names a registered typed config; report identity is an opaque source
alias returned earlier in the same MCP session. Unknown fields, unknown versions,
human output requests and command/path substitutions are refused. Optional execution
budget is reserved: any non-null override is ERROR. Product configs already define
bounded budgets; overriding an approved experiment silently would break its identity.

## Response

Required concepts: protocol version, product, operation, verdict, kind, summary,
expected/observed, reproduction, evidence references, limitations, next action.
`scope` preserves bounded product scope, numeric coverage and execution-budget metadata
from the verified result. Only known public numeric fields are copied. Nullable fields
explicitly represent unavailable information. `source` is a public
artifact reference; it is not an authoritative import. At most eight evidence refs
are projected; `other_failure_count` counts other projected findings. BlindTest uses
the existing AgentReport sanitation, only the primary public reproduction, and no
human-only fields. No raw logs/environment/filesystem locators are added.

FAIL carries recorded reproduction reference or explicit replay unavailability.
BlindTest exact args/env/oracle stay in trusted storage, with public steps for agents.
The P6 projector bounds its distinct public findings to 16; count is not a disclosure
of the full hidden failure inventory. General automatic replay remains unavailable.

| Verdict | Exit | Agent action |
|---|---:|---|
| PASS | 0 | Completion only within tested scope; consider remaining changed surfaces |
| FAIL | 1 | Repair from reproduction/evidence; rerun |
| INCONCLUSIVE | 2 | Obtain missing evidence; no completion claim |
| ERROR | 3 | Fix verifier infrastructure/configuration; no product defect inference |

Argument misuse remains 64. Missing config/evidence, failed execution/load and malformed
CI output cannot be green. Machine execution errors are fixed structured ERROR messages;
private serde/path/Docker diagnostics are not emitted. For a missing untyped report
artifact, the error response uses the default Behavior routing label because no verified
product can be discovered. Do not treat that label as evidence about the artifact.

Doctor has `operation: doctor`, `kind: readiness`, and no verified source. Its PASS
means prerequisites checked, not verification success. General project doctor also
emits `verification_performed: false`. Empty/unconfigured projects are not ready.

## Configs and trust

Behavior config is the existing `BehaviorExperiment` v1 plus a separate trusted
`BehaviorAuthorization`; SideEffect uses `SideEffectContract` v1; BlindTest uses
`BlindTestConfig` v1 and trusted sealed-root environment. Baselines are never generated
or approved automatically. Hashes and schema checks do not authenticate a malicious
host/controller. Same-user access remains outside the secrecy boundary.
