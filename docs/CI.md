# Local-first GitHub Actions

Copy [.github/workflows/b2ige-verify.yml.example](../.github/workflows/b2ige-verify.yml.example)
into a chosen workflow after supplying the project-specific reviewed configs/fixtures.
The template is intentionally inactive in this repository until those inputs are wired.
It builds the workspace locally and has Behavior, SideEffect SQLite and BlindTest Docker
matrix jobs. No hosted B2IGE service is needed. Other repositories should pin a verifier
checkout and reference its built binary/script by absolute path.

Before verify, build target executables and record their actual hashes; provide an
approved Behavior baseline/authorization, resettable SideEffect SQLite fixture, or an
approved sealed BlindTest suite outside the checkout. For BlindTest use an actual Linux
Docker runner and locally available target image. The doctor prerequisite fails when
Docker is unavailable. Missing fixture/suite/approval is an error, never a demo PASS.

```sh
python3 scripts/ci-verify.py behavior .b2ige/behavior.json --authorization .b2ige/authorization.json
python3 scripts/ci-verify.py sideeffect .b2ige/sideeffect.json
B2IGE_BLINDTEST_SEALED_ROOT=/trusted/sealed python3 scripts/ci-verify.py blindtest .b2ige/blindtest.json
```

The wrapper runs fixed product CLI surfaces with `--output agent --protocol 1`, then
passes the child output and exit status to `b2ige ci-check`. That transport guard parses
the strict v1 response, requires verify (not doctor), requires a source for non-ERROR,
and preserves verdict/exit agreement. It does not establish evidence authenticity or
turn an arbitrary JSON file into a verified artifact. Use a trusted verifier binary.
A crash, missing output, malformed response, unknown version, mismatch or non-verification
response is ERROR/3. Only PASS/0 makes a job green. FAIL/1, INCONCLUSIVE/2 and ERROR/3 all
fail CI. Argument misuse/64 in the child becomes infrastructure ERROR/3 at the CI boundary.

Default retained file: `b2ige-agent-report.json`, the sanitized machine output only.
The workflow uploads exactly that file with `always()` so failures can be investigated.
If the executable cannot run, the wrapper removes stale output and exits 3. Unvalidated
child output is temporary and stderr is not uploaded. Never broaden artifact paths to
`.b2ige/**`, the sealed root, raw store, Docker internals or human reports. Hidden human
report uploading is deliberately unsupported. No pipeline `|| true`, forced zero exit
or `continue-on-error` is used.

Validation: actionlint checks workflow syntax; CLI/CI integration tests execute actual
local SQLite and Behavior fixtures, including all four exit classes. MCP tests also
execute actual Docker BlindTest PASS/FAIL/INCONCLUSIVE/ERROR cases. These are bounded
conformance checks, not exhaustive proof or a benchmark.
