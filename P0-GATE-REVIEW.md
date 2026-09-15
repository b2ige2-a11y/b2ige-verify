# P0 Gate Review

Date: 2026-09-14
Decision: CONDITIONAL PASS — DESIGN CONTRACTS ACCEPTED, P1 IMPLEMENTATION MAY BEGIN ONLY AS A SCAFFOLD WITH CONFORMANCE TESTS FIRST

## Questions

### What exactly counts as PASS?
All declared experiment steps and authoritative checkers complete; every verdict-critical observer meets minimum coverage; no contract violation is observed within the recorded scope/budget.

### When must the tool return INCONCLUSIVE?
Execution occurred but evidence is insufficient to distinguish pass/fail, including inaccessible verdict-critical state, insufficient baseline stability, or required provider confirmation that cannot be observed.

### What evidence is authoritative?
Only evidence classes explicitly permitted by the checker contract. Advisory/model output cannot be authoritative.

### What does observation coverage mean?
A claim-relative statement about whether the relevant state/event domain was observed completely, partially, not at all, or observer execution failed.

### How is replay guaranteed?
By recording canonical experiment inputs, target identity, seed, fault schedule, config, and evidence hashes. External-world identity is not guaranteed; replay reports reproduction success.

### How can a false PASS occur?
Major paths: missing critical evidence, poisoned baseline, over-broad noise suppression, incorrect generated invariant, provider attempt mistaken for commit, hidden observer data loss, or isolation failure being ignored. The P0 contracts explicitly block these paths from PASS.

### How is hidden grader isolation enforced?
By declared isolation modes. Strong claims require Linux hardened mode; preflight must reject configurations that expose protected artifacts.

### What is explicitly out of scope?
Exhaustive formal proof, universal hostile-code sandboxing, correctness of an untrusted baseline, and guaranteed observation of unsupported external providers.

## Gate result
P0 conceptual design is coherent enough to proceed, with one sequencing rule:

> P1's first executable code must be contract/conformance machinery, not feature expansion.

Before any public PASS claim, V-001 through BT-003 must exist as automated tests.
