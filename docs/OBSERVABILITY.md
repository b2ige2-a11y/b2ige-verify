# Observability

## Initial surfaces
- HTTP
- CLI
- Filesystem
- SQLite
- PostgreSQL

## Coverage is claim-relative
A run does not have one magical coverage percentage. Each contract claim maps to required observers.

Example:
`auth_session_created == false` may require HTTP + database/session-store observation. HTTP 401 alone is insufficient if the contract concerns absence of persistent session creation.

## Coverage states
- complete: required event/state domain was observed under the declared adapter contract.
- partial: some relevant domain is invisible or dropped.
- unavailable: observer cannot operate in this environment.
- failed: observer malfunctioned.

## Observer quality requirements
Observers must document:
- what they can see
- what they cannot see
- ordering guarantees
- buffering/drop behavior
- durability semantics
- clock source
