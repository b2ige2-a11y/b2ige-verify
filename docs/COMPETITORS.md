# Competitor Review — 2026-09-14

## TestSprite
Current strength: agent-first CLI, `setup`, `doctor`, multiple coding-agent skill installs, stable JSON/exit-code orientation, self-contained failure bundles, live cloud test execution.
Adopt: onboarding, doctor, agent installation/status, failure bundle UX.
Do not copy: core dependency on hosted account/API key; black-box cloud as mandatory authority.
B2IGE opening: trustworthy local-first deterministic evidence engine.

## Reticle
Current strength: runtime observation of DOM/network/console/routes/storage/app state and explicit `partial` coverage + `unknown` semantics.
Adopt: truthful observability limits and no silent pass.
Opening: broader backend/CLI/DB common observation model.

## Proctor
Current strength: OS-level hidden-evaluator isolation and signed/tamper-evident integrity artifacts on Linux.
Adopt: real access control, explicit honest claim scope, signed integrity direction.
Opening: everyday developer verification UX rather than benchmark-only framing.

## RepoTrials
Current strength: private historical tasks, hidden verifier, content-addressed vault, versioned schemas, local-first artifacts; explicitly distinguishes unsafe local from Docker and does not overclaim Docker hardening.
Adopt: verifier self-validation, artifact integrity, honest backend claims.

## Schemathesis
Current strength: OpenAPI/GraphQL generation, deterministic coverage phase, fuzzing, stateful workflows, shrinking.
Adopt or integrate: API case generation/shrinking ideas.
Do not rebuild commodity schema fuzzing unless necessary.

## RESTler
Strength: producer/consumer inference and deeper stateful REST exploration.
Adopt: dependency-aware sequence generation concepts.

## equiv
Current strength: deterministic same-input old/new comparison, exact counterexamples, bounded-claim honesty, signed receipts, static Rust binary.
Adopt: reproducibility discipline and explicit bounded scope.
Opening: application-wide multi-observable behavior and intent-aware locks.

## Diffy
Historical strength: candidate vs primary plus secondary-known-good to estimate nondeterministic noise.
Important status: original Twitter repository is archived; treat it as a design reference, not active competitive velocity.
Adopt: three-way noise-learning concept, not its old HTTP-only surface.

## Keploy
Current strength: network/eBPF-based stack-independent record/replay, offline tests, time freezing, dependency mocks/sandboxes.
Do not compete on protocol breadth.
Opening: generated differential exploration, evidence semantics, and minimal counterexamples.

## EffectFence
Current strength: deterministic crash/retry schedule verification focused on side-effect safety and MCP conformance.
Important naming signal: `EffectFence` is already colliding with another runtime-fence package/repo; B2IGE should avoid this name entirely.
Adopt: schedule exploration and safe/unsafe corpora.

## Toxiproxy
Mature deterministic network-fault primitive. Prefer adapter/integration over reimplementation.

## Jepsen
Current strength: fault injection + operation histories + checkers for distributed-system verification.
Adopt: history/checker philosophy.
Do not copy: expert-only complexity for ordinary SaaS users.

## Strategic conclusion
B2IGE should not win by breadth. Its defensible combination is:
local-first + deterministic orchestration + explicit evidence trust + honest verdicts + claim-relative observation coverage + reduction/replay + agent-native UX.
