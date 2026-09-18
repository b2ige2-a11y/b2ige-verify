# V110-C platform and distribution (source / future candidate)

This is source-checkout tooling for a future release decision. Published **v0.2.0**
and its tag/artifacts remain immutable. v0.2.0 does not contain V110-A adoption,
V110-B adoption bench, or V110-C `ci init`. Package versions remain 0.2.0 during
candidate testing; that version string alone does not identify V110 functionality.
No public release, npm publication or Cargo publication is part of this package.
V110-C completion and external CI qualification remain separate decisions.

## CI preview and explicit create-new write

From a reviewed V110 source build:

```sh
b2ige ci init --identity login --provider github-actions \
  --registry /trusted/project.json \
  --verifier-repo b2ige2-a11y/b2ige-verify \
  --verifier-ref FULL_REVIEWED_40_HEX_COMMIT
# Repeat with --write only after reviewing the preview.
```

The commit must contain the reviewed V110-C tooling; neither `main`, `latest`, a
short SHA nor a mutable tag is accepted. The example workflow deliberately has an
invalid placeholder until an operator selects that commit. Repository owner/name
accept only letters, digits, underscores and hyphens in this initial interface.
Identity accepts the V110 adoption alphabet (letters, digits, underscores, hyphens,
1–64 characters). All registry entries are validated, including duplicate identities.
No config is executed or approved by bootstrap.

`--root DIRECTORY` selects an existing project directory independently of CWD;
otherwise CWD is the project root. `--registry FILE` is resolved from the caller's
CWD and is used for local registered-identity inspection. The generated controller
requires that reviewed inventory at **`.b2ige/project.json` in the trusted base**.
It conservatively verifies every registered contract, including the selected identity;
bootstrap does not invent a diff map or omit other required contracts.
`--workflow .github/workflows/NAME.yml` changes the destination within that exact
area. Preview prints the deterministic workflow and readiness information without
writing. `--write` creates only that workflow, refusing existing files, symlinks,
unsafe parents, traversal and unsupported paths. Compare against a preview made in a
disposable root when a workflow already exists; replacement is a manual review action.
The same-user hostile filesystem-race limitation remains explicit.

### Two separate trust sources

1. GitHub `pull_request_target` loads the workflow from the trusted base. The
   project checkout uses the exact event base SHA with credentials persistence off.
   Project registry/config/authorization inputs belong to that reviewed controller.
2. A separate verifier checkout uses the explicitly selected full commit. Only that
   checkout builds Cargo code and supplies `b2ige`, `diff-verify.py`, `ci-verify.py`
   and `ci-check`. `--project-root` lets the pinned adapter inspect the project Git
   objects without importing project scripts. Candidate objects are fetched only
   for diff/identity inspection; the controller never checks out candidate workflows
   or builds candidate Cargo hooks/package scripts.

Candidate build provisioning remains **REQUIRED** on a separate unprivileged,
isolated runner. The generated workflow intentionally fails until controller-owned
candidate approval, exact approved runtime targets/fixtures and product prerequisites
are provisioned. An artifact, its self-reported hash, a successful build or previous
PASS is not approval. Configure this protected mechanism by reviewing the workflow;
bootstrap has no automatic approval flag, Agent approval or MCP approval tool.
The generated controller additionally uses `--trusted-controller` and accepts only
an all-BlindTest inventory whose configs require `DOCKER_ISOLATION`. It refuses
Behavior and SideEffect before target execution because their existing host process
model cannot isolate a hostile PR from the controller. Separate builds and hash
approvals do not fix this runtime boundary. Those products require a separately
reviewed isolated execution route before enabling hosted CI; existing trusted local
product commands and approval semantics are unchanged.

The candidate approval format and admission checks remain those in [CI.md](CI.md):
exact head SHA, complete contract inventory, config/authorization hashes and target
identity. The registry must equal the independently retained base inventory.
Behavior baseline/checker authorization, SideEffect committed-state authority and
BlindTest sealed inputs remain controller-owned. No authority is created by CI init.

Only PASS/0 makes the verification step green. FAIL/1, INCONCLUSIVE/2, ERROR/3,
malformed/missing output and readiness remain non-green. No `continue-on-error`,
`|| true`, doctor-only gate or plan-only gate is generated. `ci-check` retains
existing Agent Protocol sanitization. Only validated responses are serialized into a
new `$RUNNER_TEMP/b2ige-agent-artifacts` directory. The adapter enables upload only
after this completes; only that directory's `*.json` files are retained for **14 days**.
Unrelated files and symlinks are not copied. Preexisting report or artifact directories
are refused. Raw stores, `.b2ige`,
authorizations, sealed suites, oracle/canary, human reports, logs, secrets and Docker
internals are never upload inputs. Read-only GitHub permissions and pinned action
commits are retained. These are bounded controls, not exhaustive secrecy proof.

## Version-pinned and offline installation

Use the reviewed source copy of the Python stdlib helper (Python 3.12+). It is also
included in future candidate native archives, not retroactively in public v0.2.0.

```sh
python3 scripts/install-release.py --version 0.2.0 --destination /existing/new-install
python3 scripts/install-release.py --offline \
  --archive /trusted/b2ige-0.2.0-aarch64-apple-darwin.tar.gz \
  --checksums /trusted/SHA256SUMS --destination /existing/new-install --smoke
```

Online inventory is deliberately limited to the reviewed v0.2.0 target matrix.
Both downloads use exact `/releases/download/v0.2.0/` assets; unknown versions fail
instead of falling back to latest. HTTPS delivery is restricted to the exact selected
GitHub release URL and approved GitHub asset CDN hosts; redirects to another release,
latest, an arbitrary host, credentials or a custom port are refused.
Offline mode never calls the downloader. Optional `--version` and `--target` constrain
local filenames; absent target means the supported native host target. Offline
candidate archives can be checked with their exact version and known target, without
claiming a public release exists. Explicit cross-target extraction is allowed;
`--smoke` requires the native host target. No Windows or Linux arm64 archive is accepted.

The helper verifies the complete checksum document (including duplicate/malformed
entries), the selected archive digest, all member paths/types, known package root,
manifest v3 version/target, all five installed binaries and their digests, executable
bits, and required license/notices before creating the destination. Validation and
extraction use one immutable in-process archive snapshot. Absolute/traversal paths,
links (including internal links), hardlinks, devices/FIFOs, sparse files, extension
headers, special modes, duplicate or Unicode/case-colliding paths (including implicit
parents) and file/directory conflicts are refused. Complete gzip CRC/trailer and tar
end-marker validation precede all writes. Archive sizes, unpacked content, member
count, path lengths and nesting depth are bounded. Extraction uses exclusive regular-file creation and
never follows archive links. Only then can optional installed-binary smoke execute.

The destination must be new, with an existing real parent; existing state is never
replaced. On extraction failure, only the newly created partial destination is
removed. A later smoke failure leaves the validated installation for inspection.
The helper never edits profiles, elevates privileges, removes quarantine or disables
Gatekeeper. Keep all installed binaries together and add their `bin` to PATH manually.
Checksums prove integrity relative to the supplied digest, **not publisher identity**.
Obtain both assets and the helper from independently trusted channels.

| Failure | Recovery |
|---|---|
| Missing/duplicate/malformed checksum or mismatch | Preserve input/state; reacquire the exact archive and original checksum document through a trusted channel |
| Malformed archive or manifest target/version mismatch | Obtain the matching native asset; never relax extraction validation |
| Existing/symlink destination | Choose a new real directory; review old contents manually |
| Unsupported host/release target | Use reviewed source/build instructions within documented scope; no target substitution |
| Missing Docker/runtime prerequisite | Start the supported local engine, provision the approved image/fixtures, rerun `blindtest doctor`; readiness is not verification |
| Broken setup/registry | Preserve state and restore reviewed inputs; setup never repairs authority automatically |
| Installed smoke failure | Inspect the validated install and native prerequisites; do not bypass OS controls |

## Platform and publication boundary

| Host | Historical public archive evidence | V110-C evidence boundary |
|---|---|---|
| macOS Apple Silicon | VERIFIED_NATIVE, documented no-Docker partial scope | Local candidate install/CLI/product smoke can be measured here; external workflow evidence remains separate |
| macOS Intel | VERIFIED_NATIVE, documented no-Docker partial scope | Fresh installer/archive gate added to existing native CI; new external run required |
| Linux x86_64 | VERIFIED_NATIVE, actual Docker and full fixed/reverse bench | Fresh installer/archive gate added to existing native CI; new external run required |
| Windows x64 | SOURCE/BUILD/CI_ONLY; no public archive | Added runner gate for version/help/inspect/idempotent init/setup/CI preview; **pending a Windows runner run**, no new runtime evidence claimed locally |
| Linux arm64 | DEFERRED | No native Linux arm64 runner; Docker Linux/aarch64 on macOS is not native Linux host evidence |

Windows product verification, Unix process cleanup, P6 Docker runtime and Windows
archives remain unqualified. macOS has no Developer ID signing/notarization; ad-hoc
linker signatures do not authenticate a publisher. npm remains deferred/private with
the existing prepublish guard; Cargo registry publication remains disabled. No scope
ownership, signing credentials or publisher authentication is inferred.

## Validation and schema decision

`test-distribution.py` exercises synthetic safe and adversarial archives, no-network
offline installation, exact online URLs, deterministic preview/create-new writes,
unsafe inputs, YAML parsing and controller mutations. `validate-workflows.py` reuses
one reviewed embedded template for exact controller structure. Existing adoption and
Agent Protocol tests cover candidate binding, config substitution, sanitization and
verdict/exit-code agreement. `platform-cli-smoke.py` is tooling-only and portable.
`fresh-smoke.py` uses the reviewed installer with SHA256SUMS before invoking absolute
installed binaries from an empty project; it checks V110 help and bundled installer
presence, then existing product/MCP/benchmark smoke. No source fallback is accepted.
These local tests do not substitute for remote native runners or private holdouts.

**Schema decision:** no authoritative/product/evidence/Agent/approval/release schema
or version changes. CI bootstrap uses human tooling output only; the installer
consumes existing embedded manifest v3 without modifying it. No new verdict path,
benchmark input, baseline, P8 label, package version or completion-state update.
