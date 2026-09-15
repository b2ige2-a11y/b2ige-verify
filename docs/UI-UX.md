# UI / UX Contract

## Human information hierarchy
1. Verdict
2. Most important failure
3. Expected vs observed
4. Minimal reproduction
5. Evidence
6. Timeline/state changes
7. Raw artifacts

## PASS
Quiet. Must still expose scope/coverage/limitations under details.

## FAIL
Use proof language only when supported by evidence. Prefer “Duplicate payment proven” only when committed effects are authoritatively observed.

## INCONCLUSIVE
Must be visually first-class, not a gray PASS. Show exactly which evidence was missing.

## Agent output
Compact JSON containing only actionable fields and sanitized evidence. Hidden-test inventory/oracle source must not leak.

## Progressive disclosure
Use labels such as `Evidence · 6`, `Timeline · 8`, `Raw trace`, never ambiguous `More details`.

## P6 BlindTest

PASS: `✓ VERIFIED`, `Hidden verification passed`, and `No failures detected within
the executed hidden suite`; display hidden check count and attested Docker isolation.
FAIL: `✕ NOT READY`, contract violation, expected/observed and **Reproduction**.
INCONCLUSIVE/ERROR remain first-class outcomes. Evidence, Isolation, Runs, Hidden
details (trusted human only), quality and Raw artifact use progressive disclosure.
Agent output has no Hidden details section and cannot open the full human viewer.
