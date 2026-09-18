# V110-D external adoption pilot framework

**EVIDENCE_PENDING. V110-D is not complete.** This framework prepares collection from
one genuine external operator; development tests are not external adoption evidence.

B2IGE Verify checks bounded program behavior against explicitly trusted requirements.
It has three products: Behavior compares observations with an approved reference;
SideEffect checks durable committed effects; BlindTest runs a target under Docker
against controller-owned reviewed material. PASS means no violation was found in the
recorded scope, not universal correctness. FAIL/INCONCLUSIVE/ERROR exit 1/2/3; only
PASS exits 0. Doctor is readiness, never verification. Models cannot assign verdicts.

## Participant eligibility and privacy

You must self-attest that you did not develop B2IGE, did not author V100/V110 code or
fixtures, did not see private hidden-suite/oracle material, use your own environment,
follow public instructions, disclose assistance and consent to anonymous sanitized
measurements. You must also attest that you personally performed the recorded actions
and did not supply synthetic, AI-authored, benchmark or reused evidence. Disclosed AI
assistance is allowed; AI-generated participant observations are not. Reading this
kit's **public** fixtures is allowed. Self-attestation is
not cryptographic identity proof. The validator cannot detect a person lying or an
entire plausible record forged without its declared synthetic markers; independent
review and actual external run inspection remain mandatory.

Generate a random UUIDv4 ID with `python3 scripts/external-pilot.py id`. Do not use a name,
handle, email, account ID or existing personal identifier. Reuse this ID for reruns so
one operator is not counted repeatedly. The kit does not collect IPs, device serials,
home directories, environment dumps or tokens. GitHub repository/PR/run references are
explicitly consented exceptions and may indirectly reveal their owners; the participant
ID need not identify an account. Decline publication if this is unacceptable.

Controller files and human terminal results remain private on your machine. Run outside
the source checkout, with private directory permissions. Do not commit the entire run
directory or terminal logs. Only the last validated `measurement-NNNN.json` per run may
be offered for publication after inspecting it. Snapshots of the **same run** are not
separate results. Preserve older snapshots privately to document interruption/history.
Every distinct rerun, including failed/abandoned runs, must be submitted for denominator
review. No recorder can detect deliberately withheld attempts. An optional short note is sanitized through an exact phrase allowlist: “instructions
clear”, “instructions unclear”, or “needed help” become fixed categories; all other
text is discarded as `withheld_for_privacy`. Raw text is never persisted or committed.
Optional question explanations are bounded categories rather than free prose. The tool never commits or uploads anything.

## Installation and exact pin

The organizer supplies the reviewed repository URL and an exact **full commit SHA that
contains this kit**, not the pre-framework starting commit. Obtain public source through
your usual Git tooling, then run from that checkout (replace placeholders):

```sh
git checkout --detach FULL_PILOT_COMMIT_SHA
git rev-parse HEAD
git status --porcelain
python3 scripts/external-pilot.py id
python3 scripts/external-pilot.py run --commit FULL_PILOT_COMMIT_SHA --out ../pilot-run-1
```

Git status must be empty. Use a new output directory with an existing real parent;
symlinks and output inside the verifier checkout are refused. The recorder builds the
locked workspace and existing fixture example from the checked-out source. Prerequisites:
Git, Python 3.12+, stable Rust/C toolchain and local Linux Docker engine. macOS runs the
Docker Desktop Linux engine; Linux x86_64 is the minimum external CI runner. These Unix
fixtures do not expand V110-C's Windows or Linux arm64 support claims. See [installation](INSTALL.md).
The fixed public Docker base must be preloaded. Read its exact identity with:

```sh
head -n 1 benchmarks/adoption-v1/projects/docker-credentials/Dockerfile
```

The FROM line is an immutable local image ID, not necessarily a pullable registry
manifest digest. Provision the documented `node:24.18.1-bookworm-slim` image, then check
`docker image inspect` for the exact FROM identity. If that identity is unavailable,
ask the organizer for an integrity-checked `docker save` archive of the reviewed image
and load it normally. Do not edit the Dockerfile, substitute a digest or retag a different
image to hide a mismatch. This pre-existing corpus portability limitation remains
installation friction and can block a journey. Setup builds the candidate
with no network and no pull. Missing Docker/image is recorded as an incomplete path,
never skipped into success. Build/download time is setup, not a correctness criterion.

Use these instructions, normal public project docs and OS/tool docs first. If AI,
another human or a B2IGE developer helps, say so at the first affected stage; assistance
does not disqualify your experience. Installation help is recorded before the source builds and inherited by each journey
setup, so it cannot become unassisted success. A blocked installation preserves the
installation attempt and initial three planned rows;
submit that snapshot as an installation drop-off and retain a new run for recovery.

## Three measured journeys

The recorder guides exactly this matrix using unchanged V110-B/P8 reviewed material:

| Product | Public fixture | Authority and boundary |
|---|---|---|
| Behavior | `rust-preserving` / `rust-cli` | Existing approved reference, baseline stability and checker binding; candidate execution never updates baseline |
| SideEffect | `sqlite-safe` / `sqlite-ledger` | Disposable durable SQLite committed state; attempts/stdout/logs never substitute |
| BlindTest | `docker-correct` / `docker-credentials` | Actual local Docker, immutable image, separate project/controller/store; public suite is not real hidden secrecy |

Each follows `inspect → init → prepare → manual trust approve → doctor → verify bench`,
with bounded setup/materialization of reviewed inputs before approval. The existing
fixture example relocates pre-reviewed inputs; it does not choose your trust decision.
The recorder never calls the benchmark PTY approval helper. At `trust approve`, read
REVIEW.md, the terminal's identity summary and trust statement. Type the exact requested
confirmation yourself only if you approve. Cancellation/failure remains in the record.
No approval flag, piped answer or simulated terminal is offered.

Human verification runs first on your private terminal. Then the recorder shows and
executes `b2ige verify bench --registry REGISTRY --output agent --protocol 1`, validates
the full response with existing `b2ige ci-check`, and reloads that stored source through
real `b2ige report`. BlindTest's public opaque alias is resolved privately by the loader.
These are distinct human/Agent executions, not byte-equality claims across executions.
The Agent run and its stored reload must agree exactly. A human/Agent exit disagreement
blocks completion and retains both outcomes; no retry is inferred. The published verdict is this
verified Agent result, never an answer inferred from human stdout. A source-backed
FAIL/INCONCLUSIVE is an evidence-backed first result, but is never product PASS.
ERROR without sufficient evidence cannot complete a journey.

The timer starts at journey setup and stops after the first **validated Agent result and
stored reload** (including the preceding human run); it is conservative observed time,
not a claim about the earliest instant the human saw a result. Command counts include
actual setup subprocesses, verification, protocol checks and reload probes; filesystem
copies are not shell commands. Stage epoch/duration data is observational only.

A failure stops that journey with no silent retry. Continue other planned products;
missing products remain `not_started`, attempted failures remain `incomplete`. Interruptions
preserve the snapshot written **before** invoking the command. To retry choose a new run
directory/ID; do not repair or overwrite old evidence. Report manual recovery attempts and
undocumented edits categorically. No partial record passes `validate --complete`.

## Agent and MCP exercises

The Agent exercise above runs the exact existing Protocol 1 surface. Only product,
operation, protocol version, verdict/exit, validation/source-backed booleans and a
SHA-256 identity of the source reference are
exported; raw Agent transports and human-only fields are not participant measurements.
The source identity detects copied observations within and across submitted runs, even
if a run ID or product label is edited. It is not a signature: replacement of every
observation and hash still requires independent review, not trust in arbitrary JSON.
Transport validation is not source authentication; product executors and verified loaders
remain authoritative.

After Behavior, the recorder offers a small stdio MCP client—no AI client required.
It displays the binary command `b2ige-mcp --registry REGISTRY` and exact JSON-RPC messages:
initialize, initialized notification, tools/list and `b2ige_behavior_verify` with registered
identity `bench`. This surface lists tools, not registry entries; the already approved
registry establishes the identity. The client checks initialization, the five existing
tools, response envelope/structured-content agreement and the existing Agent transport.
No arbitrary shell/file/approval tool is added. Answer whether receiving this result
gives MCP approval authority; your answer is retained even if mistaken. MCP results cannot
approve baselines, suite material, registry identities or a candidate.

## External GitHub CI exercise

This is the part requiring a real participant-owned public repository/fork and GitHub
Actions. Local tests, fabricated URLs, developer runs and a setup-only red job do not
qualify. The reviewed generated controller admits **only Docker BlindTest**, not host
Behavior/SideEffect. Do not widen the support matrix.

Use a disposable external repository containing only the public Docker fixture and a
single `bench` identity. The source verifier pin must be the same exact pilot commit.
For the small public fixture exercise, the kit can generate a workflow for a disposable,
ephemeral operator-controlled Linux x86_64 self-hosted runner with label `b2ige-pilot`.
Only enable it on your isolated pilot repository with no untrusted contributors, no
secrets and no other jobs. Destroy the runner after the pair; this is not a general
self-hosted runner security claim. Do not run candidate build scripts on the controller.
A separately reviewed hosted protected-provisioning route is also possible but is not
implemented by the minimum helper.

The preparation sequence is deliberately separated from Actions and manual approval:

1. In the external repository, copy the existing `docker-credentials` public fixture,
   including `correct.js` as `target.js`, and commit it. Keep controller material outside
   that checkout. Create a candidate PR that changes a public file; record its exact head.
   Later use `mutant_a.js` as `target.js` in a separate candidate commit/PR for the non-PASS
   control. These predicates already exist in P8; no new correctness labels are created.
2. On a **separate unprivileged build host**, check out that exact candidate head, build
   using the reviewed Dockerfile with `--pull=false --network=none`, and record the immutable
   image identity. Transfer the image and public workspace snapshot to the isolated
   controller through your normal reviewed build transfer procedure. Independently check
   the head-to-image association. Never take an approval file from a PR artifact.
3. On the controller, provision neutral paths `/opt/b2ige-pilot/project`,
   `/opt/b2ige-pilot/controller`, `/opt/b2ige-pilot/draft` as separate fresh directories
   owned by the operator. The first contains the built public workspace. With the pinned
   verifier checkout as current directory, execute (substitute IMAGE_ID):

   ```sh
   target/release/examples/adoption_fixture blindtest correct /opt/b2ige-pilot/project /opt/b2ige-pilot/controller IMAGE_ID
   export B2IGE_BLINDTEST_SEALED_ROOT=/opt/b2ige-pilot/controller/material/sealed
   target/release/b2ige prepare --product blindtest --config /opt/b2ige-pilot/controller/input.json --workspace /opt/b2ige-pilot/project --out /opt/b2ige-pilot/draft
   target/release/b2ige trust approve /opt/b2ige-pilot/draft --product blindtest --identity bench --project-root /opt/b2ige-pilot/project --controller /opt/b2ige-pilot/controller --registry /opt/b2ige-pilot/controller/registry.json --store /opt/b2ige-pilot/controller/store
   python3 scripts/external-pilot.py ci-approval --registry /opt/b2ige-pilot/controller/registry.json --head FULL_EXTERNAL_CANDIDATE_SHA --out /opt/b2ige-pilot/controller/candidate-approval.json
   ```

   `correct` here selects the existing full suite, not the candidate's expected verdict;
   the same suite applies to the mutant image. Both commands require actual human review;
   the second requests `APPROVE-CI` plus the exact head. It emits the existing V110-C
   admission format, never a new product approval format. No JSON editing is needed.
4. Copy only the reviewed registry (neutral paths, no secrets) into `.b2ige/project.json`
   of the external repository's trusted base. Generate the external workflow:

   ```sh
   python3 scripts/external-pilot.py ci-workflow --root EXTERNAL_REPO --registry /opt/b2ige-pilot/controller/registry.json --verifier-repo VERIFIER_OWNER/VERIFIER_REPO --out EXTERNAL_REPO/.github/workflows/b2ige-verify.yml
   ```

   This invokes real `b2ige ci init`; only runner selection and a fixed pre-verification
   copy of **already independently approved** controller inputs are added. Gate body,
   pinned actions, verifier SHA, PASS-only-green behavior and sanitized upload stay intact.
   Review the output and recorded workflow SHA-256 before committing it to the external
   trusted base. The helper never alters this B2IGE repository's workflows. The generated
   file must be available in the trusted base before the candidate PR event. If the
   candidate head changed while setting up the base, rebuild/reapprove its new exact head.
5. Launch a fresh ephemeral runner with those preprovisioned files/image and trigger the
   corresponding PR event. Wait for the **verification** step/check: PASS must be green.
   For the mutant, repeat on a new fresh controller/runner with the same neutral paths,
   independently approve its new image/head and use the unchanged base registry references
   and workflow. The verifier must run and produce a non-PASS artifact/check, not merely
   fail setup. Do not alter approvals while a job is executing. Do not disable required
   safeguards to obtain a result.
6. Before GitHub's 14-day artifact expiry, privately inspect **all** uploaded artifacts
   in each run. There must be only `b2ige-sanitized-agent-reports`, containing one validated
   Agent Protocol v1 JSON file. No raw store, sealed material, oracle/canary, human report,
   credentials, Docker internals or arbitrary logs may be present. Record file count,
   bounded kind and names; do not publish hidden values to demonstrate absence.
7. With GitHub CLI `gh` authenticated using your normal tool procedure (never paste tokens
   into the recorder), run:

   ```sh
   python3 scripts/external-pilot.py ci ../pilot-run-1/measurement-NNNN.json --out ../pilot-run-1/with-ci.json
   python3 scripts/external-pilot.py validate ../pilot-run-1/with-ci.json --complete
   python3 scripts/external-pilot.py report ../pilot-run-1/with-ci.json
   ```

The collector makes read-only GitHub API calls; it creates no repository, PR, comment,
workflow run or upload. It fetches the run's PR-linked base/head, exact workflow bytes,
verification job and step conclusion, all artifact inventory and ZIP content. It checks
strict names/types/counts, traversal/link/size controls and existing Protocol 1 validation
without extraction. It rejects missing/expired artifacts, pagination, unlinked runs,
wrong workflow pin/hash, skipped verification and non-PASS green. The operator additionally
attests private-artifact inspection: protocol shape and pattern scans alone cannot prove
absence of arbitrary encoded secrets. Only fixed measurements, consented URLs and hashes
survive. Workflow hash is an integrity identifier, not a signature or independent audit.
CI capture writes create-new `.attempt-NNNN.json` snapshots beside the requested output
before collection and after each observed run. A missing second run, declined inspection
or interruption retains the first observation and the incomplete CI attempt. Submit the
latest snapshot if collection stops. A failed integration is not retried within that run.
Only the fixed generated `.github/workflows/b2ige-verify.yml` and one `bench.json`
artifact qualify. Explicit test/simulated repository names are refused locally.
Run, verification-job and artifact run/base identities must agree. GitHub reruns of
an existing run ID are refused to avoid mixing artifacts across attempts; create a
fresh run and retain the earlier failure for review.
The non-PASS completion control must be an evidence-backed FAIL or INCONCLUSIVE;
ERROR remains reportable infrastructure friction and cannot complete this control.

GitHub does not always retain populated `pull_requests` linkage (for example some fork
runs). Such a run is conservatively refused rather than guessing head/base from a mutable
PR. Use an operator-owned same-repository PR whose API run has linkage; retain failed
integration as friction. Do not rewrite or simulate the association.

## Questions, assistance and interpretation

All six exact verifier-logic questions from AGENTS.md are asked with yes/no/unsure,
consulted-documentation boolean and optional bounded explanation. Answers are never
rewritten. Expected conceptual answers and qualifications are stored separately in
[protocol.json](../external-pilot/v1/protocol.json). In particular, same-user host access
**can** read controller material, nondeterminism/baseline poisoning are risks, and replay
is scoped or explicitly unavailable. Comparison measures comprehension, not verifier
truth or operator qualification; an incorrect answer remains evidence of confusion.

Assistance categories: `none`, `public_docs`, `normal_tool_docs`, `ai_assistant`,
`external_human`, `b2ige_developer`. Each journey event stores its first affected stage;
MCP and CI separately record their strongest assistance category before starting,
including incomplete attempts. Later disclosure can increase but never erase recorded
help. Journey stages ask again after execution so help received during a command is
retained. Public reports distinguish
unassisted/doc-only completion, assisted completion and incomplete/drop-off.
Friction/terminology categories: installation, product_selection, terminology,
path_or_identity, trusted_controller, approval, readiness, docker, evidence, ci, agent,
mcp, recovery, other. Counts/first blocking stage, confidence, undocumented edits and
recovery attempts remain visible. No failure or assistance is normalized into success.

## Schema, admission and report

Tooling schema **2**, pilot **external-pilot-v1**, `kind: external-adoption-pilot`,
`authoritative: false`. The stdlib Python validator is the normative schema; no JSON
Schema dependency or authoritative product schema changes. Every object has an exact
field allowlist, bounded enums/counts and finite durations. Unknown fields/versions,
duplicate JSON keys, NaN/Infinity, paths/raw prose, missing/duplicate products, stale code
or protocol/corpus pins, developer/synthetic declarations and inconsistent exit/stage
records are rejected. Environment is only OS, architecture and Docker availability.
Participant/run IDs are random 32-hex UUIDv4; commit pins are 40-hex, content IDs SHA-256.
Shared participant/pilot/environment fields apply to each of the three journey rows.
Schema decision: the unshipped tooling v1 is replaced by v2 because source identities,
personal/non-synthetic attestation, CI response/assistance and integration attempts are
required fields. Old results are rejected without migration. Pilot corpus, questions,
Agent Protocol v1 and every authoritative/product/approval schema remain unchanged.

All writes reserve new files/directories. Snapshots never replace previous measurements;
`ci` creates a new augmented snapshot with the **same run ID**, preserving its original
attempts. It does not rerecord a journey or mutate its outcomes. Public reports take
exactly one final snapshot per run and reject duplicates; repeat runs of one operator
increase journey/run denominators, not external operator count. No result ever writes
P8, V110-B, baselines, approvals, sealed material, registry authority or program state.
The separately invoked manual `ci-approval` command takes controller inputs, never a
pilot result, and requires the independent human decision described above.

`validate` accepts honest incomplete captures. `validate --complete` requires all three
completed evidence-backed paths, manual checkpoints, Agent validation and stored reload,
six answers, actual MCP and the CI PASS/non-PASS pair with private-artifact control.
The CLI `validate --complete` also re-fetches the actual GitHub runs/artifacts; unavailable
or expired evidence blocks this check. Offline deterministic structural tests do not
perform that network operation. Report generation is offline structural validation;
`READY_FOR_INDEPENDENT_REVIEW` does not assert that GitHub was independently inspected.
The collection gate requires a non-synthetic personal externality declaration, completed
installation, all three distinct evidence-backed journeys and manual checkpoints, six
answers, source-backed non-ERROR MCP, completed integration attempts, distinct CI
PASS-green and evidence-backed non-PASS-failure runs, both artifact inspections, exact
clean commit/protocol/corpus pins and sanitized fields. No checkbox replaces these checks.
It means **collection structurally ready for review**, not V110-D complete, proof of
external identity, trustworthy arbitrary JSON, or a new product verdict. Product loaders,
verify/report/registry/trust surfaces reject these tooling documents. Internal benchmark
results and synthetic TEST DATA cannot substitute. Hashes do not authenticate publishers.

## Future minimum gate and independent review

After collection, an independent final reviewer must:

- Confirm at least one genuinely external operator's self-attestation and disclosed help;
  audit all run IDs/attempts, including incomplete paths, without collecting identities.
- Validate the sanitized evidence in the exact clean pinned checkout; independently inspect
  actual public PR/run/workflow/artifacts and provisioning boundary while available.
- Verify Behavior, SideEffect and BlindTest attempted/completed with evidence-backed results,
  manual trust checkpoints, actual Agent/MCP operation and all six answers retained.
- Confirm PASS-green, non-PASS-non-green and no-private-artifact control; review claimed
  absence using actual artifact content, not a checkbox alone.
- Generate/review the deterministic bounded public report with exact operator/run sample
  size, platforms, 3-per-run journey denominator, assistance, drop-off, trust checkpoints,
  time-to-result, recovery, terminology, comprehension and integration outcomes.
- Commit consented evidence only in a later reviewed change, obtain independent final
  audit and external CI for that B2IGE PR, then make a separate explicit V110-D completion
  decision. None of these last decisions is performed by this kit.

No zero-operator report or synthetic/local/developer run qualifies. One external operator
covering all paths suffices for this minimum; three different people are not required.
More participants strengthen future evidence. The result is a **bounded external operator
sample**, not representative adoption, universal usability/compatibility, third-party
project compatibility, exhaustive correctness, independent correctness labels, same-user
host secrecy, publisher authentication or market demand. A consented real-project study
may be conducted separately later; it is not required or mixed into this minimum matrix.
V100/P8 remain unchanged. V110-D remains **EVIDENCE_PENDING** until genuine evidence and
all later review gates exist.
