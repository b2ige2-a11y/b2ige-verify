# Threat Model

## BlindTest assets

Hidden suite source and case inventory; oracle source and expected raw values;
private grader configuration; reference solution; mutation inventory; hidden
artifact paths; private canary; full raw hidden run evidence.

## Adversary and trusted controller

P6 considers a target intentionally probing its **Docker container**, plus a coding
agent receiving its public workspace/config and sanitized AgentReport. The trusted
host controller loads approved sealed artifacts outside that workspace, executes the
checker outside the container, and stores raw evidence outside the workspace.
The verifier repository and Human local view are trusted controller surfaces.
Do not supply controller storage, Docker credentials, the Docker socket, or the
full human viewer to the coding agent as part of an isolation claim.

A coding agent or other process with the same unrestricted macOS host-user access
as the controller can read those user's files. P6 does **not** isolate that adversary.
Workspace separation by itself only prevents accidental disclosure.

## Isolation levels and version compatibility

- **NONE**: no secrecy claim.
- **WORKSPACE_SEPARATION**: hidden artifacts outside the project; no hostile-host
  probing protection.
- **DOCKER_ISOLATION**: P6's implemented boundary. Pinned Linux target image;
  network disabled; no host mounts, Docker socket or grader/oracle exposure;
  read-only root; explicit bounded tmpfs; non-root user; dropped capabilities;
  no-new-privileges; bounded memory/CPU/pids/time/output. Actual inspect data is
  verified before and after execution. macOS Docker Desktop can attest this level.
- **HARDENED_LINUX**: reserved for a separately implemented and verified Linux-host
  policy. P6 rejects this request and does not claim it on any platform.

P0/P1 wire isolation names (`none`, `workspace_separation`, `container_isolation`,
`hardened_linux`) remain unchanged. The P6 artifact family uses its explicit uppercase
Docker enum; this does not silently reinterpret old conformance artifacts.

## P6 protects against

- Direct opening/enumeration of sealed suite, grader/oracle and reference files from
  the target container; only current target input is delivered.
- Reading the complete or remaining hidden case/mutation inventory from target mounts.
- Ordinary network exfiltration from the isolated target.
- The obvious host-control path through a mounted Docker socket or privileged container.
- Hidden raw artifacts, values, paths, canaries and Docker host internals appearing
  in AgentReport. Raw output is never forwarded; public text is conservatively filtered.
- A report cache assigning a verdict without the required, bound hidden runtime evidence.

A unique private canary is part of sealed oracle metadata. The actual probing corpus
enumerates accessible filesystem paths, environment and mount data, cwd/root traversal,
and bounded `/proc` paths. Every target stdout/stderr stream is checked for the canary.
A detected marker is an isolation violation, never PASS. Leak testing is bounded;
absence of this marker is supporting evidence, not mathematically perfect secrecy.

## Explicit non-goals

- Malicious same-user host processes, host administrators/root, or users who expose
  controller artifacts or the full Human viewer to the coding agent.
- Compromise of Docker Desktop, its Linux VM, the daemon, or container runtime.
- Kernel/container escape vulnerabilities or side-channel attacks.
- Hiding the current args/env/fixture from the process that receives them.
- Proving equivalent hardened Linux isolation on macOS/Windows.
- Perfect secrecy, exhaustive proof, or comprehensive mutation adequacy.
- Reproducible-build provenance: the public workspace hash and build identity are
  recorded, while the resolved immutable image identifies the executed artifact.

## Integrity and evidence

Approval binds requirement identity/hash, invariant artifact hash and fixed checker.
Suite and case hashes pin complete inventory. Inspect binds target identity, actual
settings and lifecycle; captures remain direct runtime evidence. Verified loaders
recompute verdicts, coverage, leakage and quality from required source evidence.
Missing/corrupt observation is ERROR/INCONCLUSIVE, not inferred product success/failure.

Hashes detect corruption and result-only rewriting; they do not authenticate wholesale
rewrites by the trusted host user. Approval/provenance is trusted controller input,
not a signature service. Persistent Docker logs are disabled; full evidence is held
in the private host store. Human raw hidden evidence is intentionally visible only
on the trusted controller surface. See [BLINDTEST.md](BLINDTEST.md) for exact limits.

## V100 local seal/receipt boundary

The trusted controller independently retains pre-candidate seal commitments,
subsequent execution/candidate approval pins, and completed receipt commitments.
Candidate/LLM output must not supply or replace these trusted inputs. Replacing
both artifacts and every retained pin is outside this local hash boundary. Hashes
bind content, not chronology, human approval, signer identity or actual environment
conformance. Same-user host compromise remains outside the secrecy claim.

Sealed BlindTest retains the P6 private-store/path rules and immutable-image/actual
inspect requirements. Receipt storage is checked against the candidate workspace
before reservation. No private holdout access or new secret dissemination path is
required. Historical verification reuses recorded observations, not a claim that
the current filesystem or runtime still has the recorded contents. Scoped read
identity tracking rejects inconsistent repeated reads, but is not an adversarial
filesystem snapshot or defense against a hostile controller.

## V100-2 adversarial admission and boundary limits

A controller exposing repeated hidden-suite queries can choose
`sealed_run::query::Ledger::execute` with an independently pinned finite policy.
The counter spans candidate and receipt IDs under one retained task/suite ledger;
concurrent reservations, interrupted runs and retries cannot obtain free attempts.
The caller must not choose a fresh ledger, repin a task/suite, remove spent slots,
or invoke unmetered execution routes. All such authority remains with the trusted
controller. This is a persistent local admission limit, not a multi-tenant service,
authenticated quota, or defense against ledger rollback by the same host user.
Finite query limits reduce repeated feedback exposure; they do not prove that even
one response cannot reveal an oracle property. Current case inputs remain visible
to the target, and repeated access to already exported reports is not metered.

Docker targets cannot assign verdicts by printing PASS/JSON or writing a local
`result.json`: the host loader requires separately recorded runtime evidence and
recomputes approved predicates. Rehashing unsafe inspect evidence still fails its
policy checks. Image healthchecks are disabled so image metadata cannot schedule
an undeclared background command. Required proc masking/read-only paths are checked
as a set, not inferred from a single representative path. Fixed target arguments,
command and environment are checked for accidental private-canary overlap as well
as per-case inputs. Canary detection is literal and bounded, not arbitrary encoded
information-flow detection or protection against a controller deliberately leaking
secrets in approved image content or labels.

The public probe target attempts root/proc writes, Docker socket discovery, a bounded
TCP connection to the documentation-only address 192.0.2.1, and bounded filesystem,
environment and proc enumeration. Failure to connect alone is not network isolation
proof; actual before/after Docker settings remain mandatory. The probe corpus and
rehashed-loader attacks are regression evidence only, not kernel-escape testing,
exhaustive secrecy, or independent external/private-holdout qualification.

## V100-5 qualification and release boundary

Protocol v1 and its public conformance/mutation matrices stabilize existing local
checks; they add no isolation level or authenticated receipt. The local release
gate's source/log hashes are controller bookkeeping, not external attestation.
Public historical benchmark vectors qualify controller regression behavior only.
They cannot fill absent fresh Docker measurements or independent private holdout
evidence. Local completion stops at WAITING_EXTERNAL_CI_AND_PRIVATE_HOLDOUT;
packaging, model explanations and self-reported completion cannot advance it.
