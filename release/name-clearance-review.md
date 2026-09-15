# Name and package review — 2026-09-15

This is a bounded public collision review. It does not establish legal trademark
clearance, ownership, registration, reservation or future availability.
[Timestamped endpoint evidence](name-check-evidence.json) retains the previous 38
same-day checks; no new account or namespace claim was made during finalization.

## Product brands

| Brand candidate | Status | Meaning |
|---|---|---|
| B2IGE Verify | RC_USABLE | Usable for the local candidate based on the recorded search |
| BlindTest | RC_USABLE | Product brand in this candidate; adjacent software uses remain disclosed below |
| BehaviorSeal | OWNER_DECISION | Candidate only; Behavior remains the working name until owner approval |
| SideEffect Proof | RC_USABLE | Usable for the local candidate based on the recorded search |

BehaviorSeal has **no final trademark clearance**. The owner's naming decision does
not itself establish legal clearance. No final rename is performed here.

## Package namespaces (separate from product brands)

| Surface | Identity | Status | Action |
|---|---|---|---|
| npm wrapper | `@b2ige/verify` | INCOMPLETE | Metadata settled on this scoped name; owner must control/create `@b2ige` |
| Unscoped npm | `blindtest` | CONFLICT | Occupied; prohibited as this project's npm package or `npx blindtest` install instruction |
| Rust/native | `b2ige-*` family | RC_USABLE | Native executables/assets use this family; internal `verify-*` crates stay `publish=false` |
| GitHub | Owner/repository not selected | OWNER_DECISION | Select the actual organization/repository; no provisional URL is configured |

The [existing npm blindtest record](https://registry.npmjs.org/blindtest) describes
adjacent AI/language-model tooling. Existing research software also uses the name:
[Keshina/BlindTest](https://github.com/Keshina/BlindTest). This collision is disclosed
without conflating the product brand with the unavailable unscoped package.
The local `blindtest` CLI alias does not claim registry ownership.

The recorded npm organization lookup returned 403; package 404s do not prove scope
control. Rust exact-name 404s likewise do not establish crates.io reservation.
No separate crate publication is part of 0.1.0. Final package ownership remains
**INCOMPLETE**, with `final_package_names: null` in the release manifest.
