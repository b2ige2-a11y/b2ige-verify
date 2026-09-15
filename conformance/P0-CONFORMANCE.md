# P0 Contract Conformance Cases

These cases must become executable tests before P1 is considered production-capable.

## V-001 Missing critical observer
Given HTTP complete, Postgres unavailable, and the contract requires Postgres state,
PASS is forbidden. Expected verdict: INCONCLUSIVE.

## V-002 Observer initialization failure
Given a verdict-critical observer cannot initialize and the experiment cannot be executed as declared,
expected verdict: ERROR.

## V-003 Proven violation despite optional observer loss
Given sufficient authoritative evidence proves a violation and only an optional observer is unavailable,
expected verdict: FAIL.

## V-004 No failure in bounded space
Given every critical observer complete and every checker completes with no violation,
expected verdict: PASS with scope/budget attached.

## B-001 Unapproved Behavior baseline
An unapproved baseline may be explored but must not produce an authoritative regression PASS gate.
Expected: INCONCLUSIVE or policy rejection before execution.

## B-002 Baseline instability
If baseline repeats vary in a locked field beyond the noise contract, candidate comparison must not silently normalize it away.
Expected: INCONCLUSIVE until the stability model is sufficient.

## S-001 Two HTTP attempts, one committed payment
Exactly-once payment contract with authoritative provider evidence showing one commit.
Expected: PASS for the payment effect (assuming all other requirements satisfied).

## S-002 One HTTP success, provider commit invisible
Exactly-once payment contract but provider-side commit confirmation is required and unavailable.
Expected: INCONCLUSIVE.

## S-003 Duplicate provider commits
Provider evidence proves two commits with same effect identity.
Expected: FAIL.

## BT-001 LLM candidate invariant not approved
A deterministic check built from a candidate-only invariant cannot authoritatively fail the target.
Expected: policy rejection / non-authoritative advisory result.

## BT-002 Hidden grader mounted in agent workspace
Any mode claiming container/hardened isolation must fail preflight.
Expected: ERROR; PASS forbidden.

## BT-003 Hardened isolation unavailable
If policy requires hardened_linux and runner is macOS without Linux backend,
Expected: ERROR before verification.
