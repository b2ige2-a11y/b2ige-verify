# V100-0 — Trust Lock

## Purpose

Freeze B2IGE's existing trust semantics into executable regression protection before adding new trust features.

This phase MUST NOT redesign existing verdict semantics.

## Required outcomes

Create an executable trust-invariant regression layer covering at minimum:

- missing verdict-critical evidence cannot yield PASS
- required checker/verifier failure cannot yield PASS
- required observer/evidence failure cannot yield PASS
- advisory or LLM-originated material cannot authoritatively assign PASS/FAIL
- integrity/identity mismatch cannot silently yield PASS
- hidden-evidence absence or corruption cannot yield PASS
- bounded verification output cannot claim exhaustive correctness
- existing authoritative loaders continue to fail closed

Prefer testing shared authoritative boundaries rather than duplicating every product-specific test.

Reuse existing tests/helpers where possible.

## Constraints

Read first:
- AGENTS.md
- v100/INVARIANTS.md
- docs/CONTRACTS.md
- docs/VERDICTS.md
- docs/EVIDENCE.md

Read THREAT-MODEL.md only if the implementation touches isolation or hidden-material semantics.

Do not:
- redefine existing P0-P9 phase names
- create new product features
- implement Task Seal yet
- implement Receipt yet
- implement new cloud/network services
- weaken existing tests or contracts
- change a public schema unless unavoidable and explicitly justified
- commit, push, merge, rebase, or rewrite Git history

## Token discipline

Inspect only relevant verifier/loader/test code after reading the required contract files.

Do not perform repeated full-repository reviews.

Use targeted tests while implementing.

One coherent implementation package is preferred over many micro-edits.

## Completion evidence

Before declaring complete:

1. `git diff --check`
2. `cargo fmt --check`
3. relevant targeted tests pass
4. `cargo test --workspace --locked` passes
5. new regression tests demonstrate that representative trust-semantic weakening would be caught
6. summarize exactly which invariant is protected by which executable test/boundary

A test failure is evidence to fix the implementation, never a reason to weaken the test.
