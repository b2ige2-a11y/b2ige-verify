# BlindTest MVP — P6

**Does it actually work?** · **Tests your coding agent can't see.**

P6 is one phase: a trusted host controller executes a sealed, approved suite against
an immutable Linux Docker image. Only deterministic CLI predicates assign verdicts.
No LLM, mutation generator, remote grader, MCP, CI, or P7 integration is involved.

## Trust boundary and files

```text
Trusted controller (host)                  Target container / coding-agent workspace
  private-root/ (outside workspace)          public target source / build identity
    suite.json                              immutable target image
    runs/<id>/                              current case arguments / explicit env
      evidence/*.json                       optional current bounded fixture
      result.json                           /tmp tmpfs; no host mounts
  invariant checker / report loader
```

The repository is the **trusted verifier implementation**, not the workspace supplied
to a coding agent under test. Corpus construction creates separate public target
workspaces and a private sibling sealed root outside this repository. Each target
image contains only that target's implementation. The controller's suite constructor,
reference target and mutation variants are never mounted or copied into another target.
Concrete hidden case IDs, logical-time inputs, oracle artifacts and a random canary
are generated into the external sealed root. Do not copy raw artifacts into an agent
workspace. A same-user host process can still read host files; this is explicitly
outside the P6 security boundary.

`B2IGE_BLINDTEST_SEALED_ROOT` is a **trusted controller environment setting**. The
public JSON config contains no sealed path: only suite hash, approved invariant refs,
target identity/policies, requested isolation, case budget and an optional opaque
quality receipt ID. Default BlindTest evidence storage is `<sealed-root>/runs`.
Explicit `--store` must also be outside the agent workspace. Canonical path checks
reject overlap in either direction, including symlink aliases. Suite files must be
bounded regular files without symlink or hard-link aliases. Suite material is read
once into controller memory and pinned; target execution cannot change its oracle.

## Requirement and executable invariant

- `RequirementArtifact v1`: requirement ID, free text, source, version, content hash.
  Text is documentation, never a predicate. Its hash excludes the hash field itself.
- `InvariantArtifact v2`: linked requirement ID/hash, public summary and expected
  semantic, provenance, and a nonempty AND of at most 16 executable predicates.
- Candidate/reviewed provenance cannot support authoritative PASS **or** FAIL.
  Approved/authoritative status requires an approving actor and source.
- Every approved reference pins the entire invariant artifact hash, requirement hash
  and checker binding. The binding includes the fixed executable checker version
  `b2ige.blindtest.cli-bytes-and.v1`. An approved A cannot run with checker B.
- The suite manifest has its own required approval/provenance. Approval is trusted
  controller input, not an agent assertion or a cryptographic signature service.

| Predicate | Exact meaning |
|---|---|
| `exit_equals` | Actual container exit code equals the declared integer (0–255) |
| `exit_not_equals` | Actual container exit code differs from the declared integer |
| `stdout_equals` | Complete captured stdout bytes equal the declared bytes |
| `stderr_equals` | Complete captured stderr bytes equal the declared bytes |
| `stdout_not_contains` | Complete stdout does not contain the nonempty byte sequence |
| `stderr_not_contains` | Complete stderr does not contain the nonempty byte sequence |

No trimming, regex, natural-language interpretation or wall-clock comparison occurs.
Empty equality is valid; empty `not_contains` is invalid. Observations are direct
process evidence, not a claim about unobserved durable application state.

Each hidden case links an approved invariant, bounded args and allowed env keys,
optional fixture, host-only oracle, public failure label and public reproduction
steps. Oracle predicates must exactly match the approved invariant. Case hashes and
complete manifest inventory are checked before execution. Expected raw bytes are
never passed to the target. Optional fixtures are at most 16 KiB and delivered only
for the current case using `B2IGE_FIXTURE_NAME` and `B2IGE_FIXTURE_BYTES_JSON` env vars;
no suite file or fixture inventory is copied. The target is allowed to see its input.

## Docker runtime

The target spec includes image reference, absolute in-container command, fixed args,
explicit env, case arg/env policy, public workspace snapshot hash, public build identity,
and resource bounds. The workspace hash is checked before and after execution. It
identifies the public source snapshot; it is not a reproducible-build attestation.
The actual image content identity is the executable identity that decides the run.

A supplied tag must resolve locally through `docker image inspect` to a full
`sha256:` image ID. Every container is created using that immutable identity; the
original mutable tag is never used to execute. No implicit image pull/build occurs
in verification. Image volumes/OnBuild hooks are rejected. Corpus-only build support
uses a pinned, already available Node base image. An isolated empty Docker config
avoids unnecessary credential-helper access for these public local fixture builds.

The P1 bounded process observer launches Docker CLI operations with a cleared,
explicit controller environment and captures byte-exact stdout/stderr. It never
forwards the controller environment to the target. The controller uses a local Unix
Docker endpoint and Linux engine. Docker setup calls have a 30-second bound; each
attached target has the declared 1–60,000 ms deadline and 1 MiB capture per stream.
Timeout/capture failure kills the target container and forces incomplete coverage.
Containers are removed after execution, including error cleanup.

Before **and** after execution, actual `docker container inspect` is checked for:

- correct image, command, args, explicit/inherited image env and non-TTY capture;
- network `none`, no published ports/extra hosts/links;
- no mounts/binds/volumes, including Docker socket, sealed suite, oracle or grader;
- read-only root, numeric non-root UID/GID `65532:65532`;
- all capabilities dropped, no capability additions, `no-new-privileges`;
- private IPC/cgroup namespaces, no host PID/UTS/user namespace, no devices;
- bounded pids, memory plus swap, CPU; bounded noexec/nosuid/nodev `/tmp` tmpfs;
- bounded 8 MiB shared memory, default masked/read-only proc paths;
- disabled restart and persistent target logging; actual created → exited lifecycle.

Docker's optional `OomKillDisable` field may be false or explicitly null after start
on this engine; true or a missing field is rejected. Actual OOM termination is
incomplete evidence, never a successful product observation. The observed memory,
CPU and pid limits remain mandatory.

The v1 P6 runtime supports **DOCKER_ISOLATION only**, including macOS Docker Desktop.
The four-level enum also preserves NONE, WORKSPACE_SEPARATION and HARDENED_LINUX,
but unsupported requested levels are rejected. A doctor's success is not a run
attestation. P6 never claims HARDENED_LINUX, even on a Linux host.

## Verdict and verified loading

| Verdict | Required evidence |
|---|---|
| PASS | Approved/bound suite and invariants; correct attested image/isolation; every required case executed; complete captures; all predicates hold; no canary leak |
| FAIL | A complete actual case proves a predicate violation, or actual target output contains the private canary |
| INCONCLUSIVE | Case budget/completion/capture/cleanup/OOM evidence is incomplete without a proven violation |
| ERROR | Invalid configuration/suite/oracle, Docker/image/init/isolation setup failure, or corrupt/unavailable storage |

A proven violation may remain FAIL even if another case is incomplete; missing evidence
alone cannot create FAIL. Controller infrastructure error takes precedence over product
findings. Zero cases and any partial suite cannot PASS. Invalid configuration or an
unreadable suite is refused before acquisition (CLI exit 3, no fabricated run artifact).

The store retains independent plan/suite, image-inspect, per-case runtime/attestation
and controller evidence. `load` verifies store hashes, exact evidence linkage, complete
inventory and image/case order; revalidates isolation; and runs the host checker again.
Cached verdict, violation list, coverage, leakage flag and quality summary do not
assign authority. Changing just verdict and rehashing returns the original evidence's
verdict. Changing observations without corresponding evidence is rejected. Rehashing
unsafe isolation settings in both result and evidence is still rejected. Wholesale
host-side evidence fabrication is outside this unauthenticated local store model.

Every FAIL retains exact single-process case inputs, immutable target identity and
its isolation evidence in the trusted artifact. Human/Agent displays use **Reproduction**,
never an unearned minimality claim. Replayability is explicitly `unavailable` for
an automatic replay CLI, with the reason and retained exact inputs; an operator can
rerun the same private suite/config. No generalized reducer is introduced.

## Human and Agent projections

Both projections come from the same verified result and share its verdict.

Human `--output json` and local `--open` are **trusted full views**. They include
progressively disclosed Evidence, Isolation, Runs, Hidden details and Raw artifact.
Raw hidden metadata/canary can be inspected there. The viewer remains loopback-only,
read-only, capability-URL guarded and revalidated on navigation.

Agent `--output agent` is a separate allowlist: public invariant summary, failure
kind, safe expected semantic, category of observed behavior, public reproduction,
immutable target image, bounded coverage/quality and safe evidence references. Raw
stdout/stderr, hidden IDs/args/env/inventory/expected bytes, private metadata and host
Docker internals are never projected. Even public labels are checked against known
private values and paths, with fixed safe fallbacks. Caller-chosen run IDs become
opaque aliases. Evidence IDs point to controller-owned execution evidence; the
isolation reference identifies that same evidence's embedded attestation.

There is no `hidden_details` field/section in Agent output. `--output agent --open`
is rejected to prevent opening a full human view from an Agent-output request.
Agent errors use fixed messages so parser and filesystem errors cannot leak paths
or private content. Host access to the trusted human viewer remains a non-goal.

## Suite quality

`validate-suite` executes a declared finite corpus and persists
`BlindTestSuiteValidationReceipt v1`. Every receipt run pins and reloads an actual
child result. Its observed verdict and match flag are recomputed. All children use
the same sealed suite/checker; recursive receipt dependencies are refused.

The supplied corpus includes correct, removed-expiry-check mutant, inverted-expiry
mutant, no-op, probing target and unavailable-image verifier error. The quality
receipt is optional for ordinary suites; approval remains mandatory. Without it,
reports say **self-validation not supplied**. A receipt reports only its observed
bounded corpus; it does not prove comprehensive mutation adequacy.

## CLI and local reproduction

```sh
export B2IGE_DEMO_ROOT="$(mktemp -d)/b2ige-p6-demo"
cargo run -p verify-core --example blindtest_corpus -- "$B2IGE_DEMO_ROOT"
export B2IGE_BLINDTEST_SEALED_ROOT="$B2IGE_DEMO_ROOT/sealed"
b2ige blindtest doctor
b2ige blindtest verify "$B2IGE_DEMO_ROOT/correct.json"
b2ige blindtest verify "$B2IGE_DEMO_ROOT/mutant_a.json" --output agent
b2ige blindtest validate-suite "$B2IGE_DEMO_ROOT/validation.json"
b2ige report "$B2IGE_DEMO_ROOT/sealed/runs/quality-correct/result.json" --open
b2ige report "$B2IGE_DEMO_ROOT/sealed/runs/quality-correct/result.json" --output agent
```

Use a new demo root for every corpus invocation: committed runs are immutable and
never overwritten. Build prerequisite: local `node:24.18.1-bookworm-slim` (or set
`B2IGE_P6_BASE_IMAGE` to an available Node image with a RepoDigest). Verification
requires no paid API. Ordinary verdict exits remain 0/1/2/3; argument misuse is 64.
A validation receipt exits 0 only if every declared expected outcome matched; a
quality mismatch exits 3 and does not masquerade as target FAIL.

## Schema decision

P0 `blindtest-invariant.schema.json` v1 is **preserved byte-for-byte**. P6 adds
`blindtest-invariant.v2.schema.json` with explicit executable semantics. Independent
v1 schemas cover requirement, manifest, case, oracle, sealed suite, isolation
attestation, config, validation config, validation receipt and run result.

ReportDocument / AgentReport / projection move to **v3** for the additive BlindTest
fields and report kind. Prior v1/v2 schema files remain. Behavior and SideEffect
verdict/evidence semantics are unchanged; report exports remain non-authoritative.
