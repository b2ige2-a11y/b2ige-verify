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
