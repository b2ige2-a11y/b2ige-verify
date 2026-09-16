# B2IGE V100 Invariants

This is a compact working summary. Authoritative documents remain authoritative.

1. Missing verdict-critical evidence must never become PASS.
2. Verifier, checker, observer, loader, or required-evidence failure must never be converted into PASS.
3. An LLM must never directly assign an authoritative verification verdict.
4. Candidate-controlled data must not silently replace verifier, oracle, seal, or authoritative evidence.
5. Hidden verifier/oracle material must not enter an agent-visible workspace when secrecy is claimed.
6. Identity mismatch or integrity failure must fail closed according to the authoritative contract.
7. A bounded experiment must never be described as exhaustive correctness proof.
8. FAIL must be replayable when possible; otherwise replay unavailability and reason must be explicit.
9. Behavior baselines must not be silently accepted or rewritten after candidate changes.
10. Schema-visible trust-semantic changes require an explicit schema/version decision.
11. Advisory/model output is never sufficient verdict evidence by itself.
12. Security and isolation claims must not exceed the implemented and evidenced threat boundary.
13. Tests or contracts must never be weakened merely to make automation pass.
14. Existing P0-P9 terminology in authoritative docs must not be redefined by the V100 program.
15. Private holdout material is outside the development-agent trust boundary.

Authority order remains:
docs/CONTRACTS.md
docs/VERDICTS.md
docs/EVIDENCE.md
docs/THREAT-MODEL.md
docs/ARCHITECTURE.md
implementation
