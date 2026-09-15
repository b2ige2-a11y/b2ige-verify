# Automatic gates

- PASS — P1–P8 verifier, evidence schemas, Cargo.lock and baselines unchanged.
- PASS — fmt, clippy, 376 Rust tests, release build; 28 npm wrapper tests.
- PASS — Native CycloneDX 1.5: six SBOMs, all 143 locked packages, schemas and hashes validated.
- PASS — Clean public inventory/hygiene, release manifest v3, archives/checksums, README/quickstart and fresh installed Docker smoke.
- PASS — CI preparation, guarded version/platform download, checksum/version refusal, private publish guards.
- PASS — Naming separation, security policy and signing options documented.
- FAIL (external gates pending) — Native Linux/Intel macOS, actual hosted delivery, namespace control and enabled private reporting.

# Remaining owner decisions

1. Approve or reject BehaviorSeal as the final Behavior brand (no trademark clearance claim).
2. Select the GitHub organization/repository.
3. Confirm ownership/create the `@b2ige` npm scope for `@b2ige/verify`.
4. Choose unsigned 0.1.0 with documented warning, or Developer ID signing + notarization.
5. Enable GitHub Private Vulnerability Reporting after repository creation.
6. Authorize and review native Linux x86_64 / macOS Intel CI.
7. Review the initial snapshot and give final public approval before any publication.

# Remaining external execution

- macOS arm64: VERIFIED_NATIVE; Docker is the actual local Linux arm64 VM, not Linux x86_64 evidence.
- macOS x86_64 / Linux x86_64: NOT_RUN — REMOTE EXECUTION REQUIRED; no Git remote. Cross-build/Rosetta/emulation do not close these gates.
- After account/host selection and authorization: run CI, supply version-matched assets and trusted archive/binary pins, verify real GitHub delivery, and confirm private-report submission. Apply the chosen Apple policy. Rebuild from the reviewed initial commit before a public release.

# Initial commit command

Run only after reviewing this local snapshot. This command was **not executed**.

```sh
cd ~/Documents/B2IGE-Verify-Public
git add -A
git commit -m "Initial public release candidate: B2IGE Verify 0.1.0"
```

Generated archives/SBOMs are intentionally ignored in `release/artifacts/` and must be
regenerated from the committed source. This command grants no permission to push or publish.

# Publication status

CLEAN_PUBLIC_REPO_READY = true
TECHNICAL_PUBLICATION_READY = false
PUBLICATION_READY = false

No commit, push, repository creation, release, registry publication, deployment,
certificate/account change, history rewrite or private-repository modification occurred.
