# V110 — Productization & Adoption Plan

**Status:** V110-A design review, re-anchored to the canonical post-release
repository. This document contains no V110 implementation or phase-gate record.

**Design input:** The earlier V110 review from the superseded checkout was used as
design input only. The adoption model, trust boundaries and four-package roadmap below
are retained; current-state claims were rechecked against canonical `main`.

**Review boundary:** A first-time external developer using B2IGE Verify on an existing
project, from installation through the first evidence-backed result and safe CI/agent
adoption.

**Protected baseline:** V100 verification semantics, evidence semantics, isolation
claims, schemas, contracts, workflows, release records and existing product paths remain
frozen. V110 adds an adoption layer around the existing execution and verified-loading
paths; it does not replace them.

## Canonical baseline and release state

The review is anchored to `/Users/kim/Projects/b2ige-verify`:

| Item | Canonical fact |
|---|---|
| Branch | `main` |
| HEAD | `9649fc0c062373aecb60c6f167f96227c638573e` |
| HEAD subject | `docs: close V100 after v0.2.0 release` |
| V100 state | `COMPLETE`; `v100/STATE.md` records `v0.2.0 released` |
| v0.2.0 release target | `1fc1b8cda1e005a8d90ce5b7388cc2a62538b8d8`, tag `v0.2.0` |
| Release state | Public GitHub/native release; public archives and `SHA256SUMS` are available |
| V100 external qualification | Final external CI and the final fresh isolated holdout are complete and recorded in the V100 release state |
| Working tree | Clean at review time; no unrelated changes are part of this plan |

The superseded checkout's release-state notes are not applicable to this baseline. V110-D
may collect post-release external adoption evidence, but it is not a missing V100 release
gate.

## Current installation and platform facts

The current distribution is a real v0.2.0 release, while distribution breadth remains
deliberately bounded:

| Platform/target | Current status |
|---|---|
| macOS Apple Silicon / `aarch64-apple-darwin` | Public native archive; verified native support with the documented no-Docker partial scope |
| macOS Intel / `x86_64-apple-darwin` | Public native archive; verified native support with the documented no-Docker partial scope |
| Linux x86_64 / `x86_64-unknown-linux-gnu` | Public native archive; verified native support including actual Docker tests and the full fixed/reverse benchmark gate |
| Linux arm64 / `aarch64-unknown-linux-gnu` | Deferred until a native runner gate is verified |
| Windows x64 / `x86_64-pc-windows-msvc` | Reviewed source/build CI support only; no Windows runtime or native archive evidence is claimed |

Additional current facts:

- BlindTest needs a local Docker engine. macOS uses Docker Desktop's Linux engine;
  Linux uses a local Unix-socket engine. No remote Docker TCP support is claimed.
- The native archives are the authoritative first distribution. A source checkout can
  use `python3 scripts/setup.py`, or the locked Cargo workspace build/install path.
- The npm package name is `@b2ige/verify`, but npm publication remains intentionally
  deferred/private. Individual Cargo crate registry publication is also intentionally
  disabled because the workspace depends on workspace-external examples, fixtures and
  tests.
- The v0.2.0 release makes no Developer ID signing or notarization claim for macOS.
  Checksums provide integrity checking, not publisher authentication.
- These are distribution facts, not new verifier semantics and not reasons to reopen the
  completed V100 release.

## Audit of current command reality

The canonical CLI was checked from source with `b2ige --version` and the actual
`--help` path. The version output is `verify-cli 0.2.0`. The current help surface is:

```text
b2ige bench [behavior|sideeffect|blindtest] [--output human|json] [--save DIRECTORY] [--reverse] [--case ID]
b2ige init [--dry-run]
b2ige setup [--dry-run]
b2ige doctor [--config project.json]
b2ige behavior verify <config.json> [--authorization FILE] [--store PATH] [--output human|json|agent] [--protocol 1]
b2ige blindtest doctor
b2ige blindtest verify <config.json> [--output human|json|agent] [--protocol 1] [--open]
b2ige blindtest validate-suite <validation-config.json>
b2ige sideeffect verify <contract.json> [--store PATH] [--output human|json|agent] [--protocol 1] [--open]
b2ige report <artifact-id|store/id/result.json> [--store PATH] [--authorization FILE] [--output human|json|agent] [--open]
```

The implementation also has `b2ige ci-check RESPONSE EXIT_CODE`, the existing CI
transport guard, although it is not listed in the current usage block. The current
MCP binary is `b2ige-mcp --registry FILE`; it serves registered typed identities and
has no arbitrary shell/file tools or interactive approval.

The important current behaviors are:

| Surface | Current behavior on canonical `main` | V110 implication |
|---|---|---|
| `init --dry-run` | Boundedly checks Cargo/package markers, Docker availability and an existing `.b2ige/project.json`; it does not discover a proof contract | Keep discovery bounded and non-authoritative |
| `init` | Creates only an empty v1 registry. It never overwrites an existing file; repeating it fails closed with exit 3 | Make the adoption-facing `init` idempotent while preserving no-overwrite behavior; do not describe the current command as already idempotent |
| `setup` | Compatibility alias that preserves a valid existing registry; `scripts/setup.py` is the source/archive bootstrap and also preserves existing state | Reuse this safety behavior rather than adding another bootstrap concept |
| `doctor --config PATH` | Checks configured entries and prerequisites; emits `kind: readiness` and `verification_performed: false`; an empty registry is not ready | Keep doctor readiness-only, even when it exits 0 |
| Product verification | Uses explicit paths: `behavior verify`, `sideeffect verify`, and `blindtest verify` | Preserve expert/manual paths as escape hatches while adding one safer adoption route |
| `report` | Reloads a stored result through the verified loader; projections are not authoritative input | Reuse report/loader semantics |
| `bench` | Runs the bounded B2IGE Verify Bench; it is not an external adoption measure | V110-B must be a separately versioned adoption bench |
| `blindtest doctor` | Checks Docker/runtime readiness only | Do not present it as a product result |
| Planned adoption commands | No `inspect`, `prepare`, `trust approve`, unified identity-based `verify`, or `ci init` exists today; there is no standalone `scaffold` command | These are V110 design targets, not current command claims |

The current project registry is v1 and contains typed entries with product, config,
store and optional Behavior authorization paths. MCP and the trusted local integration
load registered entries; a caller cannot substitute an arbitrary config at tool time.
Relative paths remain relative to the process working directory, not the registry file.

## Design-review conclusion

The verifier is substantially safer than it is adoptable. The first-use path still
requires an external developer to understand product-specific contract files, compute
identities, author a registry, manage Behavior authorization or SideEffect observers,
and keep BlindTest private material outside the project workspace. The safe product
response is not to infer a contract or hide the trust decision. It is to make the
decision visible, prepare bounded non-authoritative drafts, and leave a small number of
explicit trust checkpoints.

The retained adoption lane is:

```text
inspect (read-only)
    -> init (idempotent local project setup)
    -> prepare (explicit product, safe draft and hashes)
    -> trust approve (trusted operator only)
    -> doctor (readiness, never verification)
    -> verify <registered-identity> (existing executor + verified loader)
    -> CI/MCP wiring using the existing registered-identity boundary
```

`inspect` may be run internally by `init`, but remains available as an explicitly
read-only command. `scaffold` remains absorbed into `prepare`: a standalone scaffold
command would imply that generated configuration is authoritative and would invite
unsafe copying of a demo or hidden-suite setup.

## Frozen V100 and trust decisions

The following decisions from the earlier review remain binding for V110:

- Project/candidate preparation and trusted-controller approval are separate lanes. This
  is an operational separation, not a new cryptographic or same-user isolation claim.
- `inspect` is read-only. It does not execute the target, build or pull an image, open a
  hidden suite, inspect production state, or write the registry.
- The V110 adoption-facing `init` is idempotent and preserves all existing files. It may
  create only the documented local convenience setup; it never creates authority,
  evidence or approval.
- `prepare` creates only non-authoritative drafts/review material and computed identity
  information. A draft is not an approved source and cannot be used by identity-based
  `verify` until a trusted operator approves/registers it.
- `trust approve` is the explicit authority transition. It is a trusted-controller
  action, not a convenience flag, result approval, MCP tool or agent capability.
- MCP, Agent Protocol and LLM output cannot self-approve a candidate, a registry entry,
  a Behavior baseline, a checker binding, a SideEffect observer or a BlindTest suite.
- Unified `verify` accepts a registered identity only. It does not infer a product from
  a path, accept an arbitrary candidate config, fall back to Behavior, or import a
  report projection as source evidence.
- `doctor` remains readiness-only. A ready doctor is not PASS and never replaces a
  verification run.
- No LLM assigns PASS, FAIL, INCONCLUSIVE or ERROR. Only the existing product execution
  and verified loaders determine product verdicts.
- Behavior baselines and checker bindings are never generated or automatically approved.
  A candidate must never become the approved baseline merely because it differs.
- `prepare`, `inspect`, `doctor` and Agent output never generate, enumerate, copy,
  validate for approval, or expose BlindTest hidden cases, oracles or private canaries.
- The same-user host limitation remains explicit: a process with unrestricted access to
  the same host user can read controller files. Workspace separation reduces accidental
  exposure; it is not an OS-account security boundary.
- V100 sealed execution, identity binding, evidence completeness, verified reload,
  replayability claims, product scopes, exit codes and the invariant
  `missing verdict-critical evidence != PASS` remain frozen.

## Two explicit operating lanes

| Lane | Allowed location | Allowed actions | Cannot do |
|---|---|---|---|
| Project/candidate lane | Existing project workspace or disposable preparation directory | Bounded discovery, explicit input validation, hashing named files, creating drafts and printing next actions | Approve baselines/invariants/checker bindings, create or expose a BlindTest suite, write trusted registry/authorization, assign a verdict, or turn a draft into an approved source |
| Trusted-controller lane | Operator terminal, protected controller directory or protected CI inputs | Review exact identities and scope, approve existing authority artifacts, own registry/authorization/sealed root/store, invoke registered verification and retain raw evidence | Delegate approval to the coding agent, expose hidden suite/oracle/raw human evidence, or claim stronger isolation from a same-user host process |

## Proposed smallest safe CLI surface

The adoption surface should be kept narrow and should route into existing product APIs:

| Command | Role | Write behavior | Trust rule |
|---|---|---|---|
| `b2ige inspect [ROOT]` | Bounded project discovery and product-fit explanation | None; human by default, stable machine output only when requested | Detection is labelled as detected, absent, ambiguous, unsupported or configured; it never selects a contract or approval |
| `b2ige init [ROOT] [--dry-run]` | Idempotent project-local setup | Creates/reuses only the empty convenience registry; preserves existing files | No product config, baseline, invariant, authorization, registry entry or evidence is created automatically |
| `b2ige prepare [ROOT]` | Explicit product preparation | Writes only a draft directory/review pack and identity summaries | Requires an explicit product in non-interactive mode; no approval, verdict, baseline promotion or hidden-suite access |
| `b2ige trust approve DRAFT` | Trusted review and authority transition | Writes only explicitly selected controller-owned existing registry/config/authorization destinations | Interactive review is required in V110-A; MCP/Agent Protocol has no approval path |
| `b2ige doctor [--registry PATH]` | Actionable readiness inspection | None | Emits readiness, never a product verdict; retain current `--config` compatibility where practical |
| `b2ige verify ID [--registry PATH]` | One obvious first verification route | Evidence is written by the existing product executor to the configured store | Resolves only a registered identity, then calls existing execution and verified loading; unknown or substituted identities are ERROR |
| `b2ige ci init --identity ID --provider github-actions` | CI preview/bootstrap | V110-C surface; preview by default, explicit write required | Uses registered identity and sanitized Agent Protocol output; only exit 0/PASS is green |

The last row is retained from the earlier design but belongs primarily to V110-C. It is
not required for the smallest V110-A implementation. Existing direct commands, `report`,
`bench`, `b2ige blindtest doctor`, `b2ige blindtest validate-suite`, MCP and protocol
surfaces remain available and documented; the adoption lane is an entry point, not a
second verifier.

### `inspect`

`inspect` may:

- identify the requested project root and bounded markers such as `Cargo.toml`,
  `package.json`, conventional build metadata, Docker availability, Dockerfile presence,
  and explicitly named executable/fixture paths;
- compute hashes for explicitly named files/directories, labelling them as integrity
  identities rather than publisher authentication;
- inspect a supplied disposable SQLite fixture read-only and report that committed-ledger
  authority still requires operator review;
- report product prerequisites that are present, absent, ambiguous or outside scope; and
- print the exact inputs still required for a draft.

It must not recursively execute project scripts, build or pull an image, open a hidden
suite, read an oracle/canary, inspect production state, infer a contract from tests, or
ask an LLM to select authoritative semantics.

### `init`

The current `init` is create-only; V110 changes the adoption contract so that rerunning
it is harmless and preserves the existing registry, symlink protections and source
files. A valid existing registry is reported rather than replaced. A malformed or unsafe
registry remains a visible recovery error. `setup` stays as a compatibility alias and
source/archive bootstrap path.

The smallest V110-A layout is intentionally small:

```text
.b2ige/
  project.json        # local convenience registry; empty until trusted registration
  drafts/              # created only when prepare persists a draft
```

Do not pre-create an `adoption/` directory or add generic onboarding metadata unless a
concrete user-facing artifact needs it. A project-local registry is a convenience
integration surface, not a secrecy boundary; trusted MCP/CI/BlindTest inputs should be
able to live in a controller-owned location outside the coding workspace.

### `prepare` and the persistence boundary

`prepare` should accept explicit product inputs, validate them against the existing typed
product schemas, compute the identities already required by those schemas, and produce a
reviewable draft directory. The first implementation should not introduce a generic
V110 draft schema:

- use the existing `BehaviorExperiment`, `SideEffectContract` and `BlindTestConfig`
  shapes for any product-specific draft files;
- include a human-readable review summary with role-labelled identities and next action;
- keep the draft outside the registered identity map and mark it as non-authoritative by
  location and command behavior;
- let `trust approve` re-read and revalidate the existing typed files instead of trusting
  a draft's self-reported status; and
- add an independently versioned V110 draft schema only if durable resume, draft IDs,
  approval audit or machine-to-machine transport genuinely requires persistence.

This avoids smuggling onboarding state into V100/product schemas. A typed draft may still
be passed to an existing expert command by a human who deliberately chooses that escape
hatch; it is not accepted by the new registered-identity route and is not an approval.

Preparation must distinguish absent, unavailable, partial and complete inputs. It must
never use a friendly default to turn missing evidence into a PASS-capable source.

### Preparation by product

| Product | Safe preparation | Decision that remains explicit |
|---|---|---|
| Behavior / BehaviorSeal | Accept explicit reference and candidate executable paths, case args/env, fixture, timeout and comparison policy; compute target/input/fixture identities; show reference and candidate roles separately; validate the existing typed shape | Which reference is the approved baseline, whether it is stable, why the byte-exact observation contract is appropriate, and the checker binding. Candidate identity never enters `approved_baselines`. |
| SideEffect Proof | Accept an explicit disposable SQLite fixture and trigger; read supplied schema read-only; suggest fields; compute executable/fixture identities; validate limits and relation shape | Whether the table is truly durable append-only committed state, which external/effect/idempotency/correlation identities are authoritative, whether the fixture is safe, and which schedules matter. Attempts, stdout and request logs cannot be substituted for commits. |
| BlindTest | Inspect public target workspace, explicit command/image inputs, Docker capability and path separation; prepare a public checklist and validate references to already approved artifacts | Requirements, invariants, oracle predicates, hidden cases, suite approval, checker binding, sealed-root ownership, image/build identity and Docker isolation. Preparation must not generate, copy, enumerate, validate for approval or expose hidden content. |

If more than one product appears plausible, interactive preparation presents the choices
and asks the operator to choose. Non-interactive preparation requires an explicit product.
`auto` may be a discovery label only; it is never verifier routing or authority.

### `trust approve`

Approval is a review transaction, not a convenience flag. Before any write, the summary
must show:

- product and identity name;
- exact draft/config identity and existing schema version;
- reference/baseline and candidate roles with actual file identities;
- fixture/workspace identity and the fact that it will be rechecked at run time;
- required observers, trust classes, coverage needed for PASS and bounded budgets;
- the Behavior baseline/checker binding, or the corresponding SideEffect/BlindTest
  authority inputs;
- storage location and whether it is outside the project workspace;
- for BlindTest, required Docker isolation and sealed-root separation without showing
  hidden values or inventory; and
- an explicit statement that the coding agent did not supply the trust decision.

V110-A may write existing v1 registry/config/authorization artifacts into an explicit
controller destination. It must not add approval fields to existing schemas or invent a
new approval receipt without a separate schema decision. The first version refuses
non-interactive approval. A future organization policy can be a separate authority
surface; it is not a reason for `prepare` or an LLM to self-approve.

Approval refuses missing evidence requirements, unapproved Behavior baseline/checker
bindings, unapproved BlindTest suite/invariant inputs, unsafe/overlapping sealed paths,
or an ambiguous SideEffect observer. It also refuses to turn the candidate into the
Behavior baseline merely because the candidate differs.

### `doctor` and `verify`

Doctor should show independent readiness blockers, for example:

```text
Behavior login
  config shape                 READY
  reference identity           READY
  candidate identity           READY
  approved baseline            BLOCKED — operator approval required
  checker binding              BLOCKED — operator approval required
  fixture/input identity       READY
  storage                      READY

Result: NOT READY FOR VERIFICATION
Next: b2ige trust approve <draft> --registry <trusted-path>
```

The exact output can evolve, but it must retain readiness semantics and
`verification_performed: false`. It must not print private BlindTest values or call
readiness PASS a product result.

`verify ID` should be a thin registry router. It resolves the registered product/config,
invokes the existing executor, reloads through the existing verified loader, and projects
the existing human or Agent Protocol result. It does not duplicate checker logic, infer
verdicts, accept path/config overrides from an agent, import exported JSON, or alter
exit codes:

```text
PASS          0  required evidence complete within declared scope
FAIL          1  deterministic violation with evidence
INCONCLUSIVE  2  execution attempted but required evidence is incomplete
ERROR         3  verifier/configuration/infrastructure could not execute the experiment
```

Missing verdict-critical evidence remains non-PASS. A FAIL retains a replay path where
available or an explicit `replayability: unavailable` reason.

## Proposed first-use journey

1. Install the supported, versioned v0.2.0 archive or use the documented locked source
   bootstrap, then run `b2ige --version`.
2. Enter an existing project and run `b2ige inspect`. It explains bounded detections,
   product meanings, unsupported surfaces and missing explicit inputs.
3. Run `b2ige init`. It creates or reuses the safe local convenience setup and is safe to
   rerun. It does not create a contract or approval.
4. Run `b2ige prepare --product behavior|sideeffect|blindtest` with explicit inputs. It
   writes only a non-authoritative draft/review pack.
5. A trusted operator reviews the pack and runs `b2ige trust approve <draft>` against a
   controller-owned registry/config destination. The command prints the exact path and
   scope before confirmation.
6. Run `b2ige doctor --registry /trusted/project.json`. A ready result is still only
   readiness.
7. Run `b2ige verify <registered-identity> --registry /trusted/project.json`. Experts
   may continue to use the product-specific commands directly.
8. Read the human result locally, or use `--output agent --protocol 1` for CI/agents.
   Human/JSON BlindTest views and raw stores remain trusted-controller artifacts.
9. In V110-C, preview and explicitly write a reviewed CI workflow. It invokes registered
   verification, preserves exit codes and uploads only sanitized output.

## V110-A implementation plan

This is an implementation sequence for the Easy Adoption package, not a new roadmap
package.

1. **Freeze the small adoption contract.** Record command grammar, status vocabulary,
   lane separation, idempotent/no-overwrite behavior, path rules, compatibility aliases
   and the rule that a draft is never authoritative. Decide that the first slice has no
   generic draft schema.
2. **Add bounded discovery and setup around existing APIs.** Implement read-only
   `inspect`, make `init` reuse rather than overwrite a project, retain `setup`, and
   reuse existing identity/path/validation helpers. Never execute application code during
   discovery.
3. **Build one preparation command with product adapters.** Convert explicit inputs into
   existing typed draft files plus a review summary and computed identities. Keep
   Behavior, SideEffect and BlindTest questions separate. For BlindTest, accept only
   public inputs/references to already approved artifacts.
4. **Build the trusted approval transition.** Require interactive operator review,
   role-labelled identity confirmation and an explicit controller destination. Reuse the
   existing v1 registry and Behavior authorization. Do not add an MCP/Agent approval
   tool, and do not claim that a same-user prompt is a security boundary.
5. **Add identity-only verification and improve readiness output.** `verify ID` routes to
   existing `execute` plus verified `load`; doctor exposes blockers without executing the
   target. Reuse the existing Agent Protocol and exit contracts.
6. **Run negative controls before broader rollout.** Cover fresh setup, rerun/idempotence,
   malformed/symlink paths, stale identities, unapproved baseline/checker, observer
   failure, missing sealed evidence, Docker failure, all four verdicts, sanitized output
   and CI transport rejection of doctor output.

V110-A should not implement `ci init`, platform packaging or an adoption benchmark. Those
belong to V110-C and V110-B respectively, with only interface tests or documentation
coordination as needed.

## V110-A acceptance checklist

- A fresh supported project reaches a clearly explained draft without undocumented JSON
  edits on supported paths.
- `inspect` never runs the target, writes a registry, changes approval, or reveals
  BlindTest hidden content.
- `init` is safe to rerun and never overwrites an existing registry, symlink or source
  file; `setup` remains compatible.
- `prepare` computes identities and validates typed shape but cannot create a
  PASS-capable Behavior authorization, approved baseline/checker binding or BlindTest
  approval.
- A candidate, coding agent or LLM cannot use preparation, doctor, MCP or Agent Protocol
  to self-approve identity or promote a Behavior baseline.
- The trusted approval summary separates reference/baseline from candidate and rejects
  missing approval/evidence rather than filling defaults.
- SideEffect preparation cannot label attempts, stdout or a non-authoritative database
  as committed state; incomplete observers remain non-PASS.
- BlindTest preparation and Agent output contain no hidden case inventory, oracle bytes,
  private canary, raw private paths or human-only report fields.
- Doctor reports readiness only; CI transport rejects readiness as a verification result.
- `verify ID` accepts only registered identities, preserves product routing, reuses the
  existing executor/loader and preserves exit codes 0/1/2/3.
- Human output is a trusted local view; Agent/CI output is the existing sanitized
  protocol. Report exports remain non-authoritative.
- Non-interactive mode never prompts, silently selects a product, creates approval or
  treats an absent input as a default.
- Existing product CLI/MCP paths, V100 contracts, schemas, evidence loaders, hidden-suite
  boundary, replayability semantics and tests remain compatible.

## Required verifier-logic review questions

Before V110-A implementation is accepted, the review record must answer:

1. Can missing evidence become PASS?
2. Can observer failure be mistaken for product failure?
3. Can nondeterminism create a false divergence?
4. Can baseline poisoning hide a regression?
5. Can the agent read verifier secrets?
6. Is the failure reproducible from recorded experiment inputs?

## Risks that could weaken independence

| Risk | Required mitigation |
|---|---|
| A draft is accepted as an authoritative config | Keep drafts outside the registered identity map; use existing typed schemas only; make `verify ID` reject unregistered/draft paths |
| Missing evidence gets a friendly default | Distinguish absent, unavailable, partial and complete; rely on the existing verified loader; add negative tests for each required observer |
| Discovery selects a contract from framework signals | Treat detection as a hint; require explicit product choice in non-interactive mode and human semantic confirmation |
| Candidate output becomes a Behavior baseline | Show immutable reference/candidate roles; never add candidate hashes to approved baseline/checker sets; require separate re-approval for baseline changes |
| Candidate or agent self-approves | No MCP/Agent approval tool; interactive trusted-controller destination; state the same-user limitation |
| Behavior stability is inferred from one run | Require the existing trusted assertion/profile; absence remains a visible blocker |
| SideEffect attempts are mistaken for commits | Keep durable append-only SQLite observation and reload checks authoritative; suggestions are never evidence |
| BlindTest suite/oracle is generated or exposed | No suite authoring/enumeration/copying in adoption; keep sealed root outside the workspace; sanitize Agent/CI output |
| A quality receipt is treated as approval | Keep suite validation as bounded quality evidence; suite/invariant provenance and checker bindings still require trust |
| Project registry is mistaken for an independence boundary | Label it convenience-only; use controller-owned registry/config/auth/store paths for trusted flows |
| Relative paths or symlinks resolve differently | Canonicalize and display resolved paths, reject unsafe overlap/symlink cases, and test changed working directories |
| Doctor becomes a green build | Preserve `kind: readiness` and `verification_performed: false`; reject readiness in CI transport |
| Onboarding adds a second verdict engine | Keep `verify` as routing/presentation around existing execute + verified load; do not synthesize evidence or verdicts |
| Persistence silently changes authority | No generic draft schema in the first slice; make an independent schema/version decision before durable machine metadata is introduced |

## Four-package V110 roadmap

V110 remains four packages. The package names and design intent are retained from the
earlier review; package completion does not rewrite V100 semantics or retroactively change
the v0.2.0 release.

### V110-A — Easy Adoption

**User problem**

An external developer cannot reach a first trustworthy result without expert JSON,
identity work, registry editing and a separate trusted controller. Product choice,
readiness and verification are too easy to confuse.

**Exact deliverables**

- bounded read-only `inspect`;
- idempotent/no-overwrite `init`, retaining `setup` compatibility;
- one explicit-product `prepare` command for Behavior, SideEffect and BlindTest public
  inputs, producing only existing typed drafts and a review pack;
- interactive `trust approve` with controller-owned destination support and no MCP/agent
  approval path;
- registry-only identity-based `verify` that routes to existing products/loaders;
- clearer readiness-only `doctor` with field-level blockers;
- onboarding and compatibility documentation for current direct commands, with no change
  to V100 or existing MCP/protocol authority.

**Trust invariants**

- Missing verdict-critical evidence cannot produce PASS.
- No candidate, agent or LLM assigns a verdict or self-approves a candidate identity.
- Behavior baselines/checker bindings are never generated or automatically approved.
- BlindTest hidden suites, oracles, canaries and human-only evidence remain outside the
  project/agent/CI workspace.
- Existing product execution, verified loading, evidence, isolation and replayability
  semantics are reused unchanged.
- Registry/approval paths are trusted-controller inputs, not authenticated signatures or
  protection from same-user host access.

**Acceptance tests**

- Fresh Rust, Node and unsupported/ambiguous fixture projects pass through
  inspect/init/prepare with no undocumented manual JSON edits on supported paths.
- Init rerun, existing registry, malformed registry, symlink and changed-working-directory
  cases preserve user files and fail closed where required.
- Behavior tests prove candidate hashes never enter approved baseline sets and absent
  stability/checker approval cannot PASS.
- SideEffect tests prove missing/partial/failed observers remain non-PASS and request or
  attempt counts never become committed-effect evidence.
- BlindTest tests prove no hidden leakage through inspect/prepare/doctor/Agent output and
  no PASS with missing suite, image or isolation evidence.
- Unified verify covers PASS, FAIL, INCONCLUSIVE, ERROR, unknown identity and product/
  identity mismatch with exact exit agreement.
- Non-interactive tests prove no prompt, inferred product, defaulted trust decision or
  automatic approval.
- Relevant V100 regression, format, lint, build and fresh-install checks remain green;
  the diff contains no prohibited source/schema/workflow/Cargo/V100/release changes.

**Likely implementation scope**

- `crates/verify-cli/src/main.rs` and a small adoption adapter around
  `crates/verify-cli/src/integration.rs`;
- existing identity, path and typed validation helpers, only as non-authoritative or
  readiness adapters;
- CLI/agent/MCP integration tests and fresh-install fixtures;
- onboarding documentation in a later implementation change;
- no draft schema unless the persistence decision above proves it necessary.

**Must explicitly not change**

- `docs/CONTRACTS.md`, `docs/VERDICTS.md`, `docs/EVIDENCE.md`, `docs/THREAT-MODEL.md`,
  `docs/ARCHITECTURE.md` authority or meaning;
- V100/P1–P9 authoritative schemas, evidence loaders, checker logic, isolation policy,
  baseline semantics, report projections and exit codes;
- automatic baseline/invariant/suite creation or approval, LLM verdict authority,
  arbitrary shell/file/MCP surfaces, hidden-suite exposure or same-user isolation claims;
- existing advanced/manual CLI and MCP paths;
- release files, Cargo files, workflows or V100 records as part of this design review.

**Completion gate**

The fresh-project adoption suite passes with all four verdict classes and negative
controls; the six review questions have recorded answers; a source/schema/workflow/Cargo
diff audit shows no prohibited changes; and relevant full tests, lint/format/build and
fresh-install checks pass. Only then should a separate state update record a V110-A gate.
This design review does not record that gate.

### V110-B — Real-World Adoption Bench

**User problem**

The current 31-case B2IGE Verify Bench measures bounded verifier behavior on a reviewed
corpus. It does not measure whether an ordinary developer can install, configure and
operate the product on a real project.

**Exact deliverables**

- an independent, versioned adoption-bench corpus with representative public project
  archetypes for Behavior, a local SQLite SideEffect ledger and a Docker-targeted
  hidden-contract flow where safely publishable;
- fresh-environment scripts measuring installation, inspect, init, prepare, trusted
  approval, first verify, report handling and CI bootstrap separately from verdicts;
- a manual-step/drop-off taxonomy for missing inputs, terminology, path/identity errors,
  readiness confusion, controller setup and CI artifacts;
- bounded metrics for time-to-first-result, command count, config edits, recovery,
  successful first run and safe handling of PASS/FAIL/INCONCLUSIVE/ERROR;
- negative controls for false PASS, baseline poisoning, observer substitution, hidden
  leakage and doctor-as-verify confusion; and
- a methodology/results report labelling adoption metrics as UX evidence, not correctness
  proof or exhaustive coverage.

**Trust invariants and gate**

Expected classifications are independently authored, not inferred from candidate output.
Automation may scaffold and measure but cannot approve a baseline, suite or verdict.
Private suites/raw evidence stay in the trusted controller. Existing V100/P8 corpus
identities and labels are immutable inputs. The bench is independently reproducible on
its stated platforms and reports denominator, environment, platform and Docker scope
without claiming general user success from a small sample.

### V110-C — Platform & Distribution

**User problem**

v0.2.0 is already publicly released, but installation breadth is intentionally
asymmetric: three native archives are public, Linux arm64 is deferred, Windows is
source/build CI only, npm/Cargo registry publication is deferred, and CI setup still
requires manual controller assembly. V110-C improves adoption; it is not a prerequisite
for the completed v0.2.0 release.

**Exact deliverables**

- maintain one supported, documented installation path with version pinning,
  authenticated provenance where available and offline/manual recovery;
- add native platform/runtime gates only when each support claim has actual evidence;
  keep unsupported platforms explicitly unsupported;
- implement reviewed `b2ige ci init` with preview/write separation, pinned verifier
  source/version, protected registry/input instructions, sanitized retention and exact
  exit-code handling;
- keep fresh archive/package smoke, checksum/provenance validation and safe extraction;
  and
- improve installation, platform, Docker and CI diagnostics without changing verdict
  semantics.

**Trust invariants and gate**

No installation path executes an unverified binary or dependency. Checksums are not
called publisher authentication unless an authenticated mechanism exists. CI is green
only for verified PASS/exit 0; FAIL, INCONCLUSIVE, ERROR, malformed output and doctor
readiness fail closed. No raw evidence, sealed suite, oracle, canary, Docker socket or
human BlindTest view is shipped/uploaded by default. Completion requires actual support
matrix gates and owner-approved distribution decisions; it does not reopen completed V100
release records.

### V110-D — External Adoption Evidence

**User problem**

The public release and V100 qualification establish the recorded bounded verification
claims. A separate, bounded post-release study is still useful to learn whether an
external operator can understand the trust boundary, complete approval, obtain a first
result and wire safe CI without a false-PASS shortcut.

**Exact deliverables**

- a bounded external-pilot protocol using public or consented projects, with explicit
  project, trusted-controller and private BlindTest separation;
- at least one operator journey per supported adoption path, recording time-to-first-
  result, decisions, drop-off, terminology failures and recovery;
- independent review of the six verifier-logic questions;
- external CI and agent/MCP integration evidence, including only-PASS-green and
  no-private-artifact negative controls;
- a public bounded adoption report with sample/platform limitations and dispositions; and
- a feedback loop into reviewed issues or a future phase without silently changing
  contracts during the pilot.

**Trust invariants and gate**

Participants cannot approve hidden suites/baselines on behalf of the product, and their
candidate output cannot become expected labels. Raw credentials, private suites and human
evidence are not published. External success is not a product PASS unless the existing
verifier produced that PASS with complete evidence. The pilot is complete only for its
stated sample/platforms and never overrides V100 records or claims exhaustive adoption.

## Final V110-A implementation recommendation

Implement V110-A as the smallest safe slice:

1. Add bounded read-only `inspect`.
2. Make the existing adoption-facing `init` idempotent while retaining its no-overwrite
   and symlink protections; keep `setup` as the compatibility/bootstrap path.
3. Add one explicit-product `prepare` command that writes existing typed draft files and
   a human review pack. Do not add a generic persisted V110 schema in the first slice.
4. Add interactive `trust approve` that revalidates those drafts and writes only
   controller-owned existing registry/Behavior authorization artifacts. No agent/MCP
   approval and no automatic baseline/checker/suite approval.
5. Add registered-identity `verify` and clearer readiness-only `doctor` by reusing the
   current executor, verified loader, evidence store and Agent Protocol. Do not create a
   second verdict engine.
6. Prove the lane separation with negative controls before starting V110-B/C/D work.

If implementation later needs durable resumable drafts or an approval audit, stop for an
independent V110 draft-schema/version decision; do not extend a V100/product schema by
convention. Keep `ci init`, platform expansion, the adoption bench and external pilot in
their named packages. This recommendation preserves the earlier review's trust decisions
while matching the actual post-v0.2.0 canonical repository.
