# b2ige — public CLI 0.1.0

[Install](../../docs/INSTALL.md) · [Quickstart](../../docs/QUICKSTART.md)

```sh
b2ige --help
b2ige --version
b2ige init --dry-run
b2ige init
b2ige doctor
b2ige behavior verify CONFIG --authorization AUTH --store STORE
b2ige sideeffect verify CONTRACT --store STORE
b2ige blindtest doctor
b2ige blindtest verify CONFIG --output agent
b2ige blindtest validate-suite CONFIG
b2ige report RESULT --store STORE --output human
b2ige report RESULT --store STORE --output json
b2ige report RESULT --store STORE --output agent
b2ige bench
b2ige-mcp --registry PROJECT
```

Uppercase operands are operator-supplied paths/identities, not literal commands to copy.
BlindTest requires `B2IGE_BLINDTEST_SEALED_ROOT` in the trusted controller environment.
Default report store `.b2ige/runs`; Behavior authorization `.b2ige/authorization.json`.
An ID, artifact directory, or full original `result.json` path is accepted. Exported JSON cannot
replace its source evidence. All report loads recheck required evidence and product contracts.
Human/Agent exports use schema/projection **v3**; `--output agent --protocol 1` uses independent
Agent Protocol **v1**. None is an authoritative evidence-import format.

Exit 0 PASS, 1 FAIL, 2 INCONCLUSIVE, 3 ERROR; argument misuse 64. Doctor is readiness only.
`--open` serves a read-only capability-URL loopback viewer until interrupted. Its human view is
trusted and may expose hidden evidence; `--output agent --open` is refused. Use noninteractive
commands when consuming a verdict exit code. No source/baseline/approval is updated by reports.

[Behavior](../../examples/behavior/README.md) · [SideEffect](../../examples/sideeffect/README.md) ·
[BlindTest](../../examples/blindtest/README.md) · [Benchmark reference](../../docs/BENCHMARKS.md).
