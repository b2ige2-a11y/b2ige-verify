# B2IGE Verify — 100-Point Program

## Objective

Turn B2IGE Verify into a category-defining verification layer for AI-written software:

1. The verification target is defined before or independently of candidate implementation.
2. Coding agents cannot inspect or rewrite claimed-hidden verifier material.
3. Actual execution and evidence determine verdicts.
4. A PASS must never be manufactured from missing evidence.
5. Verification must be reproducible and externally inspectable.
6. First-use UX must remain simple despite strong internal guarantees.

---

## P4 — Adoption

Goal: make B2IGE easy enough to use continuously in real coding-agent workflows.

Required outcomes:

- one-command or near-one-command project setup
- reliable Codex integration
- GitHub Check / CI integration
- Windows x64 distribution
- diff-aware targeted verification
- actionable sanitized agent feedback
- installation and recovery paths tested

Exit gate:

- a fresh supported project can install, configure and obtain its first real verification result with minimal manual setup
- automation can determine affected verification scope without silently skipping required contracts
- existing verdict/evidence contracts remain intact

---

## P5 — Task Seal

Goal: bind verification intent before candidate implementation.

Required outcomes:

- task/contract seal manifest
- verifier identity commitment
- immutable linkage between task, verifier and candidate
- detection of post-seal verifier mutation
- sealed/private verifier storage boundary
- receipt-ready seal identity
- explicit schema/version semantics

Exit gate:

- candidate code cannot silently replace the committed verifier
- changed verifier identity invalidates or explicitly re-seals the task
- missing seal-critical evidence never becomes PASS

---

## P6 — Verifier Forge

Goal: create and qualify strong private verifiers from declared contracts.

Required outcomes:

- verifier blueprint
- boundary-case generation
- property-based verification support where appropriate
- metamorphic verification support where appropriate
- mutation generation / mutation adequacy
- verifier qualification result
- no LLM-generated verifier becomes authoritative without executable qualification

Exit gate:

- known bad mutants are measured against the verifier
- verifier inadequacy is represented honestly
- verifier-generation failure cannot become candidate PASS

---

## P7 — Adversarial Trust

Goal: resist agents attempting to learn, alter or game verification.

Required outcomes:

- anti-oracle feedback policy
- agent/human evidence projections
- retry/query budget semantics
- private seed or case-family rotation where justified
- verifier tampering detection
- evidence tampering detection
- leakage probes
- adversarial corpus

Exit gate:

- common verifier-reading, result-forging and leakage attempts are covered by reproducible tests
- security claims precisely match the documented threat boundary

---

## P8 — Verification Receipt

Goal: make verification independently inspectable.

Required outcomes:

- receipt schema
- candidate identity
- task/seal identity
- verifier identity
- environment identity
- evidence digest
- verdict binding
- offline receipt verification
- signing/attestation integration where appropriate

Exit gate:

- receipt mutation is detectable
- receipt never claims stronger guarantees than underlying evidence
- a third party can verify receipt integrity without an LLM

---

## P9 — Proof

Goal: establish public evidence that B2IGE adds value.

Required outcomes:

- RealBench expansion
- real historical bugs where practical
- false-PASS tracking
- false-FAIL tracking
- reproduction tracking
- adversarial RedBench
- published bounded methodology
- reproducible benchmark snapshots

Exit gate:

- benchmark claims are reproducible
- denominators and limitations are explicit
- no benchmark result is represented as exhaustive correctness

---

## P10 — Verification Protocol

Goal: separate the B2IGE trust model from one implementation.

Required outcomes:

- B2IGE Verification Protocol v1
- normative terminology
- task commitment semantics
- candidate identity semantics
- verifier identity semantics
- isolation semantics
- evidence contract
- verdict semantics
- agent/human projection rules
- receipt specification
- reference conformance suite

Exit gate:

- another implementation could implement the protocol from the specification
- conformance does not depend on subjective LLM judgment

---

## P11 — Managed / Team Layer

Goal: create a commercial path without weakening the OSS core.

Possible outcomes:

- remote sealed verifier vault
- ephemeral isolated runners
- organization verification policy
- approval workflow
- evidence retention
- enterprise identity / signing
- managed CI controls

Exit gate:

- OSS local verification remains useful independently
- commercial features solve operational/team trust problems rather than artificially crippling the core

---

# Automation Principles

Work in coherent work packages rather than one-file microtasks.

For each work package:

1. Read authoritative contracts first.
2. Identify risk level.
3. Select model according to MODEL_POLICY.yaml.
4. Implement only the bounded package.
5. Run targeted validation.
6. Check global invariants.
7. Retry from evidence at most three times.
8. If still failing, mark BLOCKED and stop.
9. If successful, commit.
10. Update STATE.json and DECISIONS.md.
11. Continue only when the package exit conditions are satisfied.

Never weaken a gate merely to make automation progress.
