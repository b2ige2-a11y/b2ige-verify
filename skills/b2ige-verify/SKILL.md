---
name: b2ige-verify
description: Verify code changes with configured B2IGE Behavior, SideEffect, or BlindTest verifiers and interpret their evidence-backed outcomes.
metadata:
  version: "1"
---
# B2IGE Verify

After changes, assess the affected scope and select explicitly:
- **Does it work?** BlindTest: approved hidden CLI invariants in Docker.
- **Did behavior change?** Behavior: approved before/after executable comparison.
- **Did the effect actually happen?** SideEffect: durable local SQLite ledger.
Run multiple products if the changed scope requires them. Unsupported surfaces need
another verifier; do not infer support from Rust/Node detection.

Use an operator-registered MCP config identity, or a reviewed local config:
```sh
b2ige behavior verify config.json --authorization authorization.json --output agent --protocol 1
b2ige sideeffect verify contract.json --output agent --protocol 1
b2ige blindtest verify config.json --output agent --protocol 1
b2ige report RESULT --store STORE --output agent --protocol 1
```

Only **PASS** permits completion within the declared tested scope. Doctor success
means readiness, never verification. On **FAIL**, use public reproduction and evidence
references to repair the violated contract and rerun. Do not update the Behavior
baseline to accommodate the change. **INCONCLUSIVE** needs missing evidence; do not
claim completion. **ERROR** is verifier/configuration/infrastructure failure, not a
proven product defect. Missing/malformed output, transport errors and crashed verifier
processes never count as PASS. Check both machine verdict and exit code (0/1/2/3).

Never assign or override verdicts, synthesize evidence, read hidden suites/oracles,
request human raw reports, or copy sealed storage into the coding workspace. Use only
Agent output for BlindTest and CI artifacts. Exact hidden reproductions stay with the
trusted controller; use supplied public steps. Failure to reproduce is not success.
Do not run arbitrary shell/file tools through MCP. Source modification, suite approval,
and publication still require the user's existing authorization.

For setup and tool arguments, read [integration](references/integration.md).
