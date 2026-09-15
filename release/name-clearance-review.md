# Name and package review — 2026-09-16

This is a bounded public collision review. It does not establish legal trademark
clearance, ownership, registration, reservation or future availability. The final
owner decisions below settle the public product/package names without making those
legal claims.
[Timestamped endpoint evidence](name-check-evidence.json) retains the previous 38
same-day checks; no new account or namespace claim was made during finalization.

## Product brands

| Brand candidate | Status | Meaning |
|---|---|---|
| B2IGE Verify | RC_USABLE | Usable for the local candidate based on the recorded search |
| BlindTest | RC_USABLE | Product brand in this candidate; adjacent software uses remain disclosed below |
| BehaviorSeal | CONFIRMED | Final public brand for the Behavior product; no legal clearance claim |
| SideEffect Proof | RC_USABLE | Usable for the local candidate based on the recorded search |

BehaviorSeal has **no final trademark clearance**. The owner's naming decision does
not itself establish legal clearance. The compatible `b2ige behavior ...` CLI and
internal module/schema/API names remain unchanged.

## Package namespaces (separate from product brands)

| Surface | Identity | Status | Action |
|---|---|---|---|
| npm wrapper | `@b2ige/verify` | DEFERRED | Final package name settled; do not publish until the owner controls/creates `@b2ige` |
| Unscoped npm | `blindtest` | CONFLICT | Occupied; prohibited as this project's npm package or `npx blindtest` install instruction |
| Rust/native | `b2ige-*` family | RC_USABLE | Native executables/assets use this family; internal `verify-*` crates stay `publish=false` |
| GitHub | `b2ige2-a11y/b2ige-verify` | SELECTED | Official 0.1.0 repository; it remains private until the public release action |

The [existing npm blindtest record](https://registry.npmjs.org/blindtest) describes
adjacent AI/language-model tooling. Existing research software also uses the name:
[Keshina/BlindTest](https://github.com/Keshina/BlindTest). This collision is disclosed
without conflating the product brand with the unavailable unscoped package.
The local `blindtest` CLI alias does not claim registry ownership.

The recorded npm organization lookup returned 403; package 404s do not prove scope
control. Rust exact-name 404s likewise do not establish crates.io reservation.
No separate crate publication is part of 0.1.0. The npm package name is final, but
scope ownership remains **INCOMPLETE** until `@b2ige` is actually controlled. Npm
publication is intentionally deferred and is not a GitHub 0.1.0 blocker.
