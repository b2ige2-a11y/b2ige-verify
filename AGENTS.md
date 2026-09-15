# AGENTS.md

## Mission
Build deterministic verification infrastructure. Do not replace evidence with model judgment.

## Authority order
1. `docs/CONTRACTS.md`
2. `docs/VERDICTS.md`
3. `docs/EVIDENCE.md`
4. `docs/THREAT-MODEL.md`
5. `docs/ARCHITECTURE.md`
6. current phase task index
7. implementation

If code conflicts with an authoritative document, the code is wrong until the contract is deliberately versioned.

## Non-negotiable rules
- Never emit PASS when a verdict-critical evidence requirement is missing.
- Never silently downgrade missing evidence into success.
- Never let an LLM directly set PASS/FAIL/INCONCLUSIVE/ERROR.
- Never claim exhaustive proof from bounded testing.
- Never update a Behavior baseline automatically after a candidate change.
- Never expose hidden grader/oracle artifacts to the coding-agent workspace in an isolation mode that claims secrecy.
- Every FAIL should be replayable or explicitly marked `replayability: unavailable` with a reason.
- Every schema-visible change requires a schema version decision.
- Do not start the next phase until the current phase gate is recorded in `state/CURRENT.md`.
- Do not expand protocol breadth unless the current product hypothesis requires it.

## Required review before merging verifier logic
Ask:
1. Can missing evidence become PASS?
2. Can observer failure be mistaken for product failure?
3. Can nondeterminism create a false divergence?
4. Can baseline poisoning hide a regression?
5. Can the agent read verifier secrets?
6. Is the failure reproducible from recorded experiment inputs?

## LLM permissions
Allowed: candidate invariants, mutation ideas, explanations, repair hints, source-location hints.
Forbidden: authoritative verdict assignment.
