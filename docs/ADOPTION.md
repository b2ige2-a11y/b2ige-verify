# Adopt an existing project (V110-A source checkout)

This path is available in the 0.3.0 native candidate or when building this checkout. The published v0.2.0 archives
are unchanged and do not contain these new adoption commands. Build with
`cargo build --workspace --release --locked`, then use `target/release/b2ige` or put
that directory on PATH. Existing product commands and release demos remain supported.

```text
inspect → init → prepare → trusted approve → doctor → verify ID
```

The project/candidate lane discovers facts and prepares public inputs. The trusted
controller lane independently reviews identities, retains approvals and evidence,
and runs verification. A controller directory outside the project prevents accidental
exposure; an unrestricted process running as the same host user can still read it.
Neither a local registry nor a terminal confirmation establishes OS-level separation.
Do not ask a coding agent to approve its own candidate.

## 1. Discover and initialize (project lane)

```sh
b2ige inspect /absolute/project
b2ige init /absolute/project --dry-run
b2ige init /absolute/project
b2ige init /absolute/project   # safe to repeat; existing registry is preserved
```

`inspect` checks a fixed list of metadata filenames and the bounded registry. It
labels detected, absent, ambiguous, unsupported and configured findings, and leaves
product selection to the operator. Docker executable presence is discovery only;
inspect does not execute it or probe the daemon. No project scripts, builds, image
pulls, production databases or hidden artifacts are opened or executed.

`init` creates only an empty `.b2ige/project.json`. A valid existing v1 registry is
preserved byte-for-byte. Malformed/ambiguous JSON, incompatible versions and symlink
paths fail closed. Correct the unsafe state explicitly; there is no overwrite flag.
`setup` remains an idempotent compatibility alias. No product configs, approvals,
evidence, or hidden suites are created.

## 2. Prepare explicit public inputs (project lane)

Run `b2ige prepare --help` for all options. Product selection is always explicit,
including non-interactive use. There is no `auto` selection or standalone scaffold.
An output directory must be new and its parent must already exist. It must not
overlap the fixture/workspace being hashed. Preparation never executes a target.

Paths in supplied configs resolve from the invocation working directory. Prepared
paths become absolute; hashes bind actual executable bytes, fixture/workspace
contents and existing input identities. Named inputs must be public. Linux's standard
`/bin`/`/sbin` aliases and direct executable aliases within `/usr/bin` or `/usr/sbin`
are resolved only when root owns the link, every parent and the destination, and
the parents/destination are not group- or world-writable. The resolved path is
persisted and its actual bytes are hashed. Other symlinks, including project or
controller links to a system executable, remain rejected.
Do not pass a production database or a directory containing private artifacts.

### Behavior: reference and candidate have different roles

```sh
b2ige prepare --product behavior --out /absolute/project/behavior-draft \
  --reference /absolute/reviewed-reference --candidate /absolute/current-candidate \
  --case-id greeting --timeout-ms 3000 --seed 42 \
  --args-json '["hello"]' --env-json '{}'
```

This writes an existing v1 `BehaviorExperiment` to `behavior.json` and a bounded
`REVIEW.md`. Without `--fixture DIRECTORY`, the existing explicit empty-fixture
semantics apply. The observer is `cli_process`; the comparison is byte-exact
exit/signal/stdout/stderr under one explicit case, not general equivalence.

The generated baseline is **unapproved**, pinned to the reference, never the
candidate. No stability assertion, checker authorization or approval set is written.
A structurally valid draft is not ready for approval or PASS.

A trusted operator must independently supply an approved baseline and the existing
`BehaviorAuthorization` containing its baseline hash, checker binding and explicit
`baseline_stable` assertion. See the [Behavior expert path](../examples/behavior/README.md)
and [existing authorization format](MCP.md). Preparation does not author these trust
inputs. With an independently approved baseline, prepare a **new** draft using
`--baseline /controller/approved-baseline.json` with the same explicit inputs.
Alternatively, use `--config EXISTING_BEHAVIOR_JSON [--baseline APPROVED_BASELINE]`.
An approved baseline that does not match the reference/observation contract is rejected;
changing the candidate cannot rebind the baseline. Never approve a reference change
merely to erase a candidate regression. Baseline stability is not inferred from a run.

### SideEffect: only disposable SQLite committed-state inputs

For an existing explicit contract:

```sh
b2ige prepare --product sideeffect --out /absolute/project/effect-draft \
  --config /absolute/public-contract.json --fixture /absolute/disposable-fixture
```

For a single ledger/effect, no config editing is required:

```sh
b2ige prepare --product sideeffect --out /absolute/project/effect-draft \
  --fixture /absolute/disposable-fixture --trigger /absolute/trigger-program \
  --contract-id checkout --operation-id checkout --idempotency order-1 \
  --correlation request-1 --provider local --operation payment \
  --expectation exactly-once --db ledger.db --table ledger \
  --external-id-column external_id --idempotency-column idem \
  --correlation-column correlation --operation-column operation \
  --commit-order-column seq --timeout-ms 3000 \
  --schedules-json '[{"schedule_id":"retry","primitives":["NONE","RETRY"]}]' \
  --budget-json '{"max_schedules":1,"max_attempts":2,"reduction_executions":0}'
```

Use your actual reviewed table/column names and identities. The explicitly supplied
fixture must contain an empty compatible SQLite table. Readiness validates all
configured observers using existing helpers. Additional effects/relations can use
the existing typed contract with `--config`. `--args-json` and `--env-json` are optional.
The review shows observers, effect identities, relations, schedules and budgets.

Preparation records a proposed durable-state contract; it cannot establish that the
table is actually an authoritative append-only committed ledger. The trusted operator
must decide that separately. Attempts, stdout, outbound requests and logs are never
committed-effect evidence. Partial, unavailable or failed observers remain non-PASS.

### BlindTest: public config, private controller

```sh
b2ige prepare --product blindtest --out /absolute/project/blind-draft \
  --config /absolute/approved-public-config.json --workspace /absolute/public-target
```

The public config contains existing approved suite/invariant/checker references,
image, command, environment, build identity and resource/case bounds. A flags-only
adapter is also available: `--workspace`, `--image`, `--command`, `--build-identity`,
`--suite-hash`, `--approved-invariants-json`, `--bounds-json`, `--max-cases`, and
`--max-case-args`; optional `--allowed-case-env-json`, `--args-json`, `--env-json`.
These must be supplied from **already independently approved public references**.
No suite, case, oracle, canary, invariant approval or image is generated.

Preparation only hashes the public workspace. It does not read the sealed root.
If `B2IGE_BLINDTEST_SEALED_ROOT` is set, overlapping public workspace/fixture paths
are rejected before snapshot acquisition. An absent approved public config/reference
is a trusted-operator blocker, not an invitation to copy a hidden suite into the project.

## 3. Approve exact inputs (trusted-controller lane only)

The operator creates a controller directory outside the project and draft. Registry,
approved copies and store must be inside that directory. Existing approved Behavior
authorization must already be inside the controller; it is copied unchanged, never
synthesized. For example, with independently reviewed Behavior inputs:

```sh
b2ige trust approve /absolute/project/reviewed-behavior-draft \
  --product behavior --identity greeting \
  --project-root /absolute/project --controller /absolute/controller \
  --registry /absolute/controller/project.json --store /absolute/controller/runs \
  --authorization /absolute/controller/reviewed-authorization.json
```

For SideEffect or BlindTest, select that product and omit `--authorization`.
BlindTest additionally requires the trusted terminal's `B2IGE_BLINDTEST_SEALED_ROOT`
to reference the existing private suite, outside the candidate workspace. The local
image must be an immutable `sha256:` identity and supported Docker must be available.

The interactive summary shows the identity/product, config identity/schema version,
reference and candidate roles, actual file identities, fixture/workspace identity,
required observations, coverage/budgets, blockers and controller destinations.
Behavior also shows the independent authorization identity and explicit stability
assertion. BlindTest withholds controller destinations because they may locate the
sealed root; it confirms separation from the entire declared project instead.
Private BlindTest values, inventory and paths are never printed. The controller
validates existing private approvals internally; it does not author or copy them.
The operator confirms the product-specific trust statement by typing `APPROVE ID`.
Stdin and stdout must both be terminals. Piped input, `--yes`, environment bypasses,
MCP and Agent Protocol cannot perform approval.

The typed files and authorization are parsed from retained byte snapshots and reread
after confirmation; any byte change is refused, identities and prerequisites are
rechecked. Registry parsing and change detection use the same retained bytes.
Approved files are exclusively
created under `CONTROLLER/ID`; the v1 registry is published last, retaining other
identities. Existing identities/files are not overwritten. A failed filesystem write
can leave unregistered controller files or a staging file; inspect and recover those
explicitly before retrying. The registry's adjacent `.adoption-approval.lock` suffix
coordinates writers even when nested controller directories are selected. Do not
remove it while another operator is registering. Store paths must not overlap
registry/approval files, and controllers must not overlap the actual fixture/workspace.
This is local trusted-host coordination, not hostile-filesystem atomicity.

## 4. Readiness, then real verification

```sh
b2ige doctor --registry /absolute/controller/project.json --output human
b2ige verify greeting --registry /absolute/controller/project.json
b2ige verify greeting --registry /absolute/controller/project.json --output agent --protocol 1
```

Doctor is read-only readiness: field-level READY/BLOCKED messages and safe next
steps, always `verification_performed: false`. It creates no store/probe file. The
legacy default remains JSON and `--config FILE` remains an alias for `--registry`.
Storage checks are metadata-only; actual writability and runtime evidence are checked
when executing. Readiness exit 0 is **not verification PASS**. `ci-check` rejects
readiness as a verification result.

`verify ID` only resolves the trusted registry's typed product/config/store/authorization,
invokes its existing execution path, and reloads through the verified loader. No
config override, inferred product, report import, draft execution or second verdict
engine exists. Known draft files (`PRODUCT.json` beside `REVIEW.md`) are refused;
approval copies are separate controller files. Manually maintained trusted v1
registries remain supported. Legacy relative registry paths retain their existing
CWD semantics; new approvals store absolute paths and work from another directory.

Verification exits are unchanged: **0 PASS, 1 FAIL, 2 INCONCLUSIVE, 3 ERROR**. Unknown,
ambiguous or invalid identities exit 3. If no product can be resolved, machine mode
returns no fabricated product response (stderr + exit 3); the existing CI boundary
fails closed on absent output. Registered product errors use existing sanitized
Agent Protocol error responses. Human/JSON BlindTest verification views remain
trusted private views; only `--output agent --protocol 1` is for agents/CI.

`b2ige report` still reloads original evidence; exported reports are non-authoritative.
Behavior/SideEffect replay limitations remain those reported by the existing loader;
recorded inputs do not promise byte-identical external-world replay.

## Compatibility and version decision

No new draft/approval schema is introduced. Product draft files use unchanged
BehaviorExperiment, SideEffectContract and BlindTestConfig v1; registration and
Behavior authorization use their existing formats. `REVIEW.md` is human explanation,
never a status/approval source. Doctor's additional `fields` map is additive
readiness tooling output under its existing v1 marker; Agent Protocol and report
schemas are unchanged. Duplicate registry identity keys are rejected as ambiguous.
No V100 semantics, evidence loaders, isolation policy, package version, release
artifacts or workflow is changed. This implements V110-A only; `ci init`, V110-B/C/D
and a V110 completion-state update are not part of this package.
