# BlindTest

Does it actually work? Tests your coding agent can't see.

Install [native binaries](../../docs/INSTALL.md). From the source workspace run
`scripts/demo-blindtest.sh` for setup, real verification, expected exit checks and sanitized output.
With installed binaries, create a new example config:

```sh
demo="$(mktemp -d)/blindtest"
b2ige-demo blindtest "$demo"
export B2IGE_BLINDTEST_SEALED_ROOT="$demo/sealed"
b2ige blindtest doctor
b2ige blindtest verify "$demo/correct.json" --output agent
```

Expected: Correct PASS; buggy FAIL; probing correct target PASS with no private canary observed.

For FAIL, use `mutant_a.json` (exit 1). `scripts/demo-blindtest.sh` actually runs a visible valid
credential test on both implementations, then hidden verification and Agent leakage checks.
`correct.json`, `mutant_a.json`, and `probe.json` are generated example configs with actual image
and workspace hashes. Setup needs the locally available digest-bearing Node image in INSTALL.

[Controller source](controller.rs) is public educational fixture source. It generates sealed
runtime inputs, oracle, canary and evidence under `$demo/sealed`, outside each `public-build-*`
target workspace. Only the target implementation/Dockerfile enters its image; no host mounts.
Give an agent only its selected target workspace and sanitized output. A public tutorial's
known source is not a secret production suite; use privately authored approved suites for that.
Same-host-user access is not isolated. No perfect secrecy or exhaustive proof is claimed.
Public API: `verify_core::blindtest`. [Exact threat boundary](../../docs/THREAT-MODEL.md).

Config identities vary per installation; generated configs avoid invalid placeholder hashes.
Do not copy private runtime reports into the repository. Product version 0.3.0; schema versions
remain those defined by the existing contracts.
