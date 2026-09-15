# B2IGE Verify — P0 Design Review

Date: 2026-09-14
Status: CONDITIONAL PASS TO AUTHOR P0 CONTRACTS; IMPLEMENTATION NOT AUTHORIZED

## Executive verdict

The product thesis is strong, but the handoff document contains several places where a verifier could accidentally overclaim certainty. The project should proceed only after the contracts below are treated as authoritative.

## Major risks found

### 1. PASS semantics are still too broad
A PASS must never mean “the software is correct.” It means only:

> All required checks for a declared verification scope executed successfully, every verdict-critical observer met its required coverage level, and no contract violation was observed in the explored space.

A PASS therefore MUST include scope identity, observation requirements, and exploration budget.

### 2. Baseline poisoning in Behavior
The baseline may already contain a bug. Behavior can prove divergence from baseline, not correctness. Therefore:
- `NO_DIVERGENCE_FOUND` is not correctness.
- A baseline contract needs provenance and approval metadata.
- Baseline updates require human/policy authorization.

### 3. Missing observer trust model
“Postgres observed” and “Stripe committed effect confirmed” are not equally strong claims. Evidence sources need trust classes:
- authoritative provider/state source
- target-local durable state
- network attempt
- derived/inferred evidence

Verdict-critical claims must state which class is sufficient.

### 4. Replay cannot mean byte-identical environment replay
External services, clocks, kernels, randomized IDs, and target dependencies can make exact world-state replay impossible. The contract should promise deterministic **experiment replay** where controllable inputs/fault schedules/seeds are fixed, and report whether the observed outcome reproduced.

### 5. Isolation claim needs explicit modes
BlindTest must not claim hostile-agent resistance on every OS. Define modes:
- `none`
- `workspace_separation`
- `container_isolation`
- `hardened_linux`

Only the last mode may carry the strongest hidden-artifact claim.

### 6. Generated invariants are not automatically authoritative
LLM-generated invariants are candidates until accepted by a policy/human or derived from a machine-readable contract. A bad invariant can produce a perfectly deterministic wrong verdict.

### 7. SideEffect “exactly_once” needs effect identity
A committed effect must have an identity rule. Counting requests is insufficient. Every effect adapter must define:
- correlation key
- deduplication domain
- commit event
- observation window
- eventual-consistency timeout

### 8. `ERROR` vs `INCONCLUSIVE` requires hard boundary
Use:
- `ERROR`: verifier could not execute its declared experiment correctly.
- `INCONCLUSIVE`: experiment executed, but evidence is insufficient to establish PASS/FAIL.

### 9. Coverage must be requirement-aware
Coverage is not one global percentage. It is a map from verdict-critical claims to required observers and actual coverage.

### 10. Mutation score must not become a vanity metric
Mutation adequacy is useful only for mutation families that represent plausible defects for the target contract. Raw kill-rate alone is not a release gate.

## Product direction retained

The following core direction survives review:
- local-first core
- deterministic orchestration
- evidence-first output
- 4-way verdict model
- minimal counterexamples
- agent-native machine output
- progressive human UI
- Behavior first for implementation, BlindTest first for launch
- Toxiproxy/Schemathesis-style primitives reused where practical instead of rewritten

## P0 gate conditions

P0 passes only when all of these are defined and testable:
1. Exact verdict state machine.
2. Required-vs-optional observer semantics.
3. Evidence trust classes.
4. Replay guarantees and non-guarantees.
5. Run identity and canonical hashing.
6. Isolation modes and claims.
7. Baseline provenance rules.
8. No-evidence-no-PASS tests.
9. Out-of-scope claims.
10. Benchmark methodology for false PASS and replay reliability.
