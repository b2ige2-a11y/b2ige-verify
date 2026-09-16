# Protocol v1 conformance and bounded qualification

These are executable public regression checks, not independent holdout results.
Existing P0/P1 conformance IDs and all P0–P9 terminology are unchanged.

| Requirement | Executable coverage |
| --- | --- |
| Required evidence, checker completion, advisory exclusion, bounded PASS | `verify-core --test conformance`; `--test trust_lock` |
| Strict TaskSeal wire, hash boundaries, domain separation, content changes | `verify-core --test task_seal` |
| Receipt/authorization/execution/identity missing, duplicate, unknown fields; canonical key-order/whitespace invariance | `behavior_differential::sealed_protocol_wire_matrix_and_canonical_receipt_metamorphism` |
| Independent authorization checked before any target execution | `behavior_differential::sealed_authorization_rejects_missing_and_changed_bindings_before_execution` |
| Every identity mutation, rehashed receipt, missing transitive evidence | `behavior_differential::sealed_behavior_verified_reload_and_every_identity_tamper_fails_closed` |
| Original FAIL/ERROR/INCONCLUSIVE preserved, historical reload without live target, no legacy fallback | remaining `behavior_differential::sealed_` tests |
| SQLite runtime/attempt binding and missing source rejection | `sideeffect_proof::sealed_sideeffect_binds_verified_attempts_and_rejects_missing_source` |
| Crash/concurrency/query limits, strict policy, ERROR receipt without refund | `verify-core --test adversarial_query` |
| Actual hidden execution, missing evidence, immutable image and inspect protections | `verify-core --test blindtest` (Docker required; outer gate) |
| Public measurement inventory, planned denominators, source verdict reload | `verify-cli --lib bench::tests`; `--test benchmark` |
| CLI integration projection/exit behavior | `verify-cli --test agent_protocol` |
| Missing/failed/empty/reordered local gate checks and source mutation block readiness | `python3 scripts/test-v100-release.py` |
| Packaging membership, unsafe links, private-value exclusion | `python3 scripts/test-release.py` |

## Qualification justified by Proof

V100-3 measured Behavior 9/9 and SideEffect 13/13, with BlindTest 0/9 locally.
Missing measurement cannot establish zero leakage or adequate hidden predicates.
Existing public Docker controls cover two named mutants; their historical 2/2
score is not general mutation adequacy and is not a current Docker measurement.

The added measurement-controller qualification starts with the unchanged 31-row
public historical snapshot as a **test vector**, recomputes its positive control,
and checks row-order invariance. It independently removes each row, removes its
verdict, removes verified reload, and injects a late harness failure: 31 × 4 = 124
negative variants. Each must block while preserving planned denominators 24/7/31.
This qualifies the measurement controller against measured availability weaknesses;
it does not count these variants as product bugs, executions, mutants or holdouts.

The receipt wire matrix extends prior identity mutation coverage across every
top-level field of four actual execution artifacts and checks canonical invariance.
Existing trust tests already exercise hash-length/case/version and query-budget
boundaries. No new random property-testing dependency, generalized mutation
engine, automatic Verifier Forge or protocol product breadth is justified here.

The local gate rejects zero executed Rust tests, including platform-gated suites.
Successful local qualification must still stop at
`WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT`.
