# Evidence Model

## Evidence principles
Evidence supports claims. A log line is not automatically authoritative evidence.

## Trust classes
1. `authoritative_external` — provider-side committed-state confirmation or canonical external source.
2. `authoritative_target_state` — durable target state such as committed database rows.
3. `direct_runtime` — directly observed process/network/runtime event.
4. `derived` — inference computed from other evidence.
5. `advisory` — model explanation or heuristic; never verdict-authoritative by itself.

## Evidence object
Required fields:
- evidence id
- type
- source
- trust class
- timestamp/order
- raw artifact reference or canonical value
- integrity hash
- related claim ids

## Claims
A checker must declare which evidence classes are sufficient for each claim.

Example: `payment_committed_exactly_once` may require provider-side commit evidence or an explicitly configured authoritative substitute. Two outbound HTTP attempts are not sufficient.

## EvidenceBundle
Contains observations, claims, counterexamples, coverage, replay instructions, and hashes.

## P5 SQLite committed state

A configured append-only durable SQLite ledger is `authoritative_target_state`.
An online SQLite backup, including committed WAL pages, is recorded as canonical Evidence
and re-queried on load. Rows bind external effect ID, operation, idempotency/correlation
identity and integer commit order. Repeated observation of one committed row is one effect.
Process attempt evidence is `direct_runtime` and cannot substitute for committed state.
History is reconstructed against child process/snapshot evidence before its claims are used.

## P6 hidden process evidence

The private store binds approved suite/plan, immutable image inspect, actual Docker
create/exit attestation and complete P1 byte captures as `direct_runtime` evidence.
Only a host-side deterministic, approved checker evaluates those observations.
Target stdout does not prove unobserved durable state. Human projections may expose
full private evidence; Agent projections carry only public semantics and safe refs.
P6 loaders revalidate required evidence and recompute verdict/coverage/leakage/quality.
