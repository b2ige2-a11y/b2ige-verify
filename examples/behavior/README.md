# BehaviorSeal

Did it change?

BehaviorSeal is the public product name. The compatible CLI command remains
`b2ige behavior verify`, and the internal `behavior` module and schema names are unchanged.

Install [native binaries](../../docs/INSTALL.md). From the source workspace run
`scripts/demo-behavior.sh` for setup, real verification, expected exit checks and sanitized output.
With installed binaries, create a new example config:

```sh
demo="$(mktemp -d)/behavior"
b2ige-demo behavior "$demo"
b2ige behavior verify "$demo/pass/experiment.json" --authorization "$demo/pass/authorization.json" --output agent
```

Expected: PASS for unchanged hello; FAIL for goodbye.

For FAIL, use `fail/experiment.json` and its `fail/authorization.json` (exit 1).
[Config generator](setup.rs) pins explicit human-authored reference programs, exact executable
and input hashes, and checker approval. This is example-only approval, never a policy to derive
baselines from candidate output. `pass/experiment.json` is the executable example configuration.
Public API: `verify_core::behavior`; byte-exact stdout/stderr/exit observations are bounded.

Config identities vary per installation; generated configs avoid invalid placeholder hashes.
Do not copy private runtime reports into the repository. Product version 0.3.0; schema versions
remain those defined by the existing contracts.
