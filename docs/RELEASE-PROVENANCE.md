# Release provenance — 0.1.0 candidate

The current binaries have no publisher signature and are not Apple notarized.
macOS arm64 linkers may create an ad-hoc Mach-O signature; that does not authenticate
a publisher. The local keychain reports **0 valid code-signing identities**.
No certificate, identity, account setting, public tag or release was created.

| Mechanism | First 0.1.0 policy | Current evidence |
|---|---|---|
| SHA256SUMS for every distributed archive, SBOM and external manifest | REQUIRED | Generated and verified locally |
| Reviewable Git commit/tag, clean checkout, locked dependencies | REQUIRED for public release | Unborn public HEAD: git_commit=null, dirty=true; public tag absent |
| Source inventory, binary hashes, release manifest and benchmark corpus hash | REQUIRED | Manifest v3 and extracted-archive checks |
| Authenticated delivery and owner approval | REQUIRED | Hosting and authorization unresolved |
| GitHub Actions artifact attestation | RECOMMENDED; owner chooses before publication | Strategy only; no attestation emitted |
| Apple Developer ID signing and notarization | RECOMMENDED for macOS public downloads; OWNER_DECISION | No valid identity; no notarization |
| Signed checksum file / offline release key | DEFERRED unless chosen as initial authentication mechanism | No key generated or signing claim |
| Bit-for-bit reproducible builds | DEFERRED | Not established by matching source inventories |

## Binding and checksum layout

Manifest schema **v3** replaces release-manifest v2 only. It permits `git_commit: null`
for an unborn public repository, adds explicit clean/technical readiness and platform
scope, and uses VERIFIED_NATIVE / CROSS_BUILD_ONLY / EMULATED / NOT_RUN. A null commit
is never a clean-tag claim. The source inventory identifies this uncommitted snapshot.
The npm distribution manifest independently advances to v2 for repository and archive
pins. The private history-scan diagnostic report advances to v2 for nullable unborn
HEAD. Product, evidence, Agent, benchmark and verifier schemas remain unchanged.

After owner approval of the snapshot, the owner creates the initial commit. Public
release artifacts must be rebuilt/retested from the reviewed committed revision and
bound to the authorized version tag. This task creates neither commit nor tag.

`source_inventory_sha256` hashes sorted relative paths, NUL, and each file's SHA-256.
Only the mutable `release/release-manifest.json` is excluded to avoid recursion.
Native/source archives contain an **embedded pre-smoke manifest**. The external
**release-index manifest** adds archive/SBOM hashes and subsequently recorded runtime
scope. Neither manifest hashes itself. SHA256SUMS hashes the external manifest as well
as the archives and SBOM. Re-check all layers before use. This is integrity evidence,
not authentication of the person or service distributing it.

The packager grants no runtime-ready state. `record-platform-gate.py` checks source
identity and archives, executes fresh installed smoke, and only then records its
native target and explicit Docker/non-Docker scope in the external manifest.
Rosetta and cross builds are supplemental evidence and never populate native target
verification from this script. Platform scope records whether Docker/benchmarks ran.
Finalization may use `--skip-benchmark` only after checking that verifier and baseline
files are unchanged; this preserves the prior benchmark evidence without claiming a rerun.

## Hosted provenance plan

The CI has full-history checkout without persisted credentials, pinned action commits,
a pinned compiler, locked dependencies, bounded platform tests and fresh archive checks.
The manifest records repository/ref/SHA/run ID/attempt/workflow when GitHub supplies them.
These fields are metadata, not a cryptographic attestation.

After explicit publication approval, a dedicated trusted release job may use GitHub
artifact attestations over the final archive digests. That future job would need scoped
`id-token: write` and `attestations: write`; current candidate workflows grant neither.
Verify attestation repository, workflow identity, commit/ref and artifact digest against
trusted policy. Do not enable signing on untrusted PR inputs or upload private verifier
stores. [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations).

## macOS plan

An owner with a real Apple Developer ID may sign every executable, submit an appropriate
distribution container using `notarytool`, retain the result, staple where the container
format supports it, and verify on a clean macOS machine with Gatekeeper enabled. Hash
and package **after signing**; signing changes bytes. Notarization is Apple's malware
checking/distribution mechanism and is not proof of verifier correctness.
[Apple notarization guidance](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

If the owner instead elects an unsigned first release, document that decision explicitly
and provide authenticated delivery and integrity instructions. This sweep does not grant
that exception. No installation script removes quarantine or bypasses platform checks.
