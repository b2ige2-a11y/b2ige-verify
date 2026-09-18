# Local-first GitHub Actions

For current V110 source, use reviewed `b2ige ci init` preview and explicit `--write`;
see [V110-C distribution and CI](V110-DISTRIBUTION.md) for exact commands, protections
and required candidate provisioning. Public v0.2.0 does not include this command.
The [example workflow](../.github/workflows/b2ige-verify.yml.example) intentionally
requires replacement of its verifier-commit placeholder before it can run.
`pull_request_target` loads the controller from the trusted project base, while a
separate full-commit-pinned checkout supplies all verifier tooling. Candidate Git
objects are fetched only for diff inspection. Candidate workflows/build hooks and
project-supplied verifier scripts are never run by this controller. Project inputs
stay in the reviewed base; the pinned `diff-verify.py --project-root DIRECTORY`
resolves Git/config paths there and retains its own trusted `ci-verify.py` adapter.
No hosted B2IGE service or automatic candidate approval is added.
The setup status, diff map/selection plan and step summary are non-authoritative tooling
messages (`schema_version: "1"` where emitted); product contracts and verified loaders
remain the only verdict authority.

Build candidates on a separate unprivileged isolated runner, then have the trusted
controller approve their actual identities and provision product inputs.
Do not execute PR build scripts in this `pull_request_target` job or expose its
controller inputs to candidate jobs. The generated workflow uses `--trusted-controller`
and admits only BlindTest contracts requiring `DOCKER_ISOLATION`, with an approved
sealed suite outside the checkout and a locally available immutable target image.
It rejects the entire inventory if any Behavior or SideEffect contract is present:
their existing host process execution is unsafe for an untrusted PR on this controller.
A reviewed build or candidate hash approval does not provide runtime isolation.
Those products remain blocked in generated CI pending a separately reviewed isolated
execution route; their existing trusted local CLI/adapter paths are unchanged.
Missing Docker, image, suite or approval is an error, never a demo PASS.

```sh
python3 scripts/setup.py                 # build once and create an empty registry safely
python3 scripts/diff-verify.py \
  --base "$GITHUB_BASE_SHA" \
  --head "$CANDIDATE_SHA" \
  --trusted-revision "$CONTROLLER_APPROVED_SHA" \
  --required-registry .b2ige/project.json \
  --registry .b2ige/project.json \
  --candidate-approval "$CONTROLLER_CANDIDATE_APPROVAL" \
  --trusted-controller \
  --b2ige target/release/b2ige \
  --artifact-dir "$RUNNER_TEMP/b2ige-agent-artifacts" \
  --report-dir b2ige-agent-reports \
  --summary "$GITHUB_STEP_SUMMARY"
```

`setup.py` never invents a contract, approves a baseline, creates a hidden suite, or
overwrites an existing registry. A malformed or symlinked registry is a recovery error;
the original bytes remain untouched. Re-running setup on a valid registry preserves it
and reports readiness separately from verification.

## Current candidate admission

Execution requires `--candidate-approval`, a controller-retained tooling v1 JSON
file supplied independently of candidate control. The workflow expects this at
`$RUNNER_TEMP/b2ige-candidate-approval.json`; until a trusted provisioning step
supplies it, the template fails closed. Do not download this approval from a PR
artifact or generate it by merely hashing whatever old build happens to be present.
The controller must independently approve the association between the event head
and the prepared build. No adapter creates or refreshes that approval.

```json
{
  "schema_version": "1",
  "head": "<full current candidate commit SHA>",
  "contracts": {
    "login": {
      "config_sha256": "sha256:<SHA-256 of exact config file bytes>",
      "authorization_sha256": "sha256:<SHA-256 of exact Behavior authorization bytes>",
      "target_identity": "sha256:<actual candidate executable bytes or immutable Docker image ID>"
    }
  }
}
```

Every required registry contract must appear exactly once, including those omitted
by diff selection. For SideEffect and BlindTest, `authorization_sha256` is null.
Behavior binds `after.identity`, SideEffect binds `trigger.executable_hash`, and
BlindTest requires `target.image` to be an immutable `sha256:` image ID. Full config
byte hashes also bind inputs, workspace/build identities and checker references.
Admission checks the resolved head, config/authorization bytes and actual local
executable bytes before execution and again around every child invocation. Product
loaders retain actual runtime/image validation and all verdict authority. Missing,
stale or mismatched binding is ERROR/3 even if old fixtures would PASS.
`--plan-only` remains selection-only and emits no gate success or candidate approval.

This is controller input admission, not a V100 sealed receipt, build provenance
proof or protection against a hostile same-user host. Configs/builds must remain
under trusted controller custody during verification. No raw approval is published.

## Diff selection

`diff-verify.py` requires a real Git base revision and an independently retained full
`--trusted-revision` SHA. It reads `--required-registry` from that Git commit, not from
the candidate working tree, and compares every entry and its product/config/store/
authorization references before selection or execution. Missing inventory or any
removed, added or replaced entry fails closed. The pin must come from the trusted
controller/event, never a PR parameter. A changed required inventory needs independent
review and adoption into the trusted revision; the adapter never updates it.

This comparison does not authenticate the calling script, binary, referenced file
contents or caller-supplied pin. The workflow enforces their provenance by executing
the independent pinned verifier checkout with project inputs from the trusted base.
Optional diff maps must also be controller-reviewed inputs from the project base. A local invocation using candidate-controlled tools or pins
is not a trusted GitHub gate. With no map it conservatively verifies every required entry.
An optional tooling-only `diff-map` v1 has this shape:

```json
{
  "schema_version": "1",
  "shared": ["crates/**", "Cargo.toml", ".github/**"],
  "contracts": {
    "login": ["src/login/**", ".b2ige/project.json"],
    "billing": ["src/billing/**", ".b2ige/project.json"]
  }
}
```

The map must name every registry entry exactly once. Shared paths select every contract;
an unmapped changed path is ERROR/3; an empty diff selects every contract. Thus an
optimization cannot silently omit a declared contract. `--plan-only` prints the selected
set without executing it. The map is an invocation hint, not authoritative evidence or
a replacement for product contracts.

## GitHub Check and feedback boundary

The workflow job itself is the GitHub Check. It has only `contents: read` permission; no
Checks API write token or third-party service is required. The adapter invokes fixed
product CLI surfaces with `--output agent --protocol 1`, then passes child output and
exit status to `b2ige ci-check`. That transport guard parses the strict v1 response,
requires verify (not doctor), requires a source for non-ERROR, and preserves
verdict/exit agreement. It does not establish evidence authenticity or turn arbitrary
JSON into a verified artifact. Use a trusted verifier binary.

A crash, missing output, malformed response, unknown version, mismatch or
non-verification response is ERROR/3. Only PASS/0 makes the job green. FAIL/1,
INCONCLUSIVE/2 and ERROR/3 all fail CI. Argument misuse/64 becomes infrastructure
ERROR/3 at the CI boundary. The step summary contains only fixed verdict text, bounded
numeric scope counters and fixed-shape public observables; raw paths, logs, environment,
sealed data and human reports are never uploaded.

Each selected contract gets one sanitized Agent Protocol JSON file in the private
working report directory. The adapter serializes only validated responses into a
separate new `$RUNNER_TEMP/b2ige-agent-artifacts` directory; unrelated files and
symlinks are not copied. Upload is enabled only by the adapter's `artifacts_ready`
output after this step completes. Only that directory's `*.json` files are retained
for 14 days. Missing or invalid reports are not accepted as a successful selection,
and stale report or artifact directories are refused. Never broaden artifact
paths to `.b2ige/**`, the sealed root, raw store, Docker internals or human reports. No
pipeline `|| true`, forced zero exit or `continue-on-error` is used.

## Windows x64

The repository workflow includes a `windows-latest` source/build job for
`x86_64-pc-windows-msvc`: locked Cargo fetch, fmt, clippy, workspace test compilation
and release build. V110-C adds bounded version/help/inspect/idempotent init/setup/CI
preview runtime smoke; its Windows execution is pending external CI. It is deliberately separate from the Unix/Docker release smoke. This Mac run does
not claim Windows runtime, product PASS, BlindTest Docker, or Windows release-archive
evidence; the job is the CI design and its eventual run is the platform evidence.

Validation: workflow structure checks, CLI/CI integration tests execute actual local
SQLite and Behavior fixtures, including all four exit classes. MCP tests execute actual
Docker BlindTest PASS/FAIL/INCONCLUSIVE/ERROR cases where Docker is available. These are
bounded conformance checks, not exhaustive proof or a benchmark.
