# Safety and Operational Guardrails

- Default to local targets and disposable test environments.
- Destructive fault schedules require explicit opt-in.
- Never target production by default.
- Side-effect adapters should prefer provider test/sandbox modes.
- Secrets must be redacted from evidence bundles by default.
- Raw artifacts containing credentials/PII must be excluded or encrypted according to configuration.
- Network egress in sealed BlindTest modes is deny-by-default except explicit policy grants.
- A verifier must fail safe: uncertainty becomes INCONCLUSIVE/ERROR, never PASS.
