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

## V100 sealed boundary

V100 sealed execution/reload returns a verdict only after its mandatory seal,
authorization, candidate, runtime and complete source-identity checks pass.
Missing/mismatched trust bindings are verifier ERROR-boundary load/execution
errors (`io::Error`), not evidence of a product FAIL. The original product loader
still determines PASS/FAIL/INCONCLUSIVE/ERROR, scope, replayability and limitations;
identity binding cannot upgrade an incomplete or failed observation. A receipt
contains no authoritative cached verdict. Legacy results do not imply V100 sealing.

V100-2 query-admission exhaustion, missing policy evidence or policy/scope mismatch
is an ERROR-boundary `io::Error` before a new product experiment, not a product
violation. Admission records assign no verdict. An admitted experiment still uses
the original sealed/product loader, including ERROR for unavailable Docker/image
evidence; errors never refund the attempt or upgrade its observation to PASS.
