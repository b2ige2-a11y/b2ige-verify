# Verdict Semantics

## PASS
All required execution and evidence conditions completed and no violation was observed within declared scope.

PASS must carry:
- scope
- coverage summary
- exploration budget
- limitations

## FAIL
A deterministic checker found a contract violation supported by sufficient evidence.

A FAIL should include a counterexample and replay path. If reduction fails, the unreduced reproduction is retained.

## INCONCLUSIVE
The experiment ran sufficiently to attempt judgment, but evidence cannot establish PASS or FAIL.

Examples:
- provider commit state cannot be observed
- critical iframe/runtime state is inaccessible
- baseline instability exceeds configured confidence/stability threshold

## ERROR
The verifier failed to execute its own declared experiment correctly.

Examples:
- invalid configuration
- runner crash
- observer initialization failure when execution cannot proceed
- corrupted evidence store

## Mandatory invariant
`missing verdict-critical evidence != PASS`
