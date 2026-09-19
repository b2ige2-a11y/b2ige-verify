# B2IGE Verify 0.3.0 final release record

Status: RELEASED

## Release identity

- Version: `0.3.0`
- Tag: `v0.3.0`
- Annotated tag object: `421b2351986a97476ce15a0aa1a29caee6da2c7a`
- Release commit: `fcda2949b793a5caf6744e968a642ee7e17a56fe`
- Exact-main CI run: `35404711862` — SUCCESS
- GitHub Release ID: `391890898`
- GitHub Release: https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.3.0

The tag points exactly to the qualified release commit.

## Final qualification

Fresh V4 internal isolated holdout:

- Run ID: `v030-final-v4-20260918T233824Z-70652b649600`
- Planned / completed / accepted: `70 / 70 / 70`
- Disagreements: `0`
- Blocked: `0`
- Invalid: `0`
- False PASS: `0`
- False FAIL: `0`
- Observed private-value leakage: `0`

The frozen deterministic checker computed agreement. The LLM did not directly assign product verdicts.

The holdout included actual committed-ledger SideEffect PASS/FAIL cases and actual Docker BlindTest PASS/FAIL cases with isolation attestation.

This was AI-operated internal release validation, not genuine external-user validation or statistically independent human validation.

Historical V1 and V2 BLOCKED runs remain historical harness-blocked attempts. V3 remains a historical PASS against the earlier pre-publication-fix candidate and does not qualify the final release commit.

## Exact-main platform evidence

- macOS Apple Silicon: qualified native archive; exact-main CI PASS; final V4 internal Docker holdout PASS
- macOS Intel: qualified native/no-Docker CI scope PASS
- Linux x86_64: actual-Docker/full-benchmark CI scope PASS
- Windows x64: source/build plus bounded CLI runtime smoke only; not VERIFIED_NATIVE
- Linux arm64: DEFERRED

## Public release assets

The GitHub Release contains exactly eight assets:

- `b2ige-0.3.0-aarch64-apple-darwin.tar.gz`
- `b2ige-0.3.0-aarch64-apple-darwin.manifest.json`
- `b2ige-0.3.0-x86_64-apple-darwin.tar.gz`
- `b2ige-0.3.0-x86_64-apple-darwin.manifest.json`
- `b2ige-0.3.0-x86_64-unknown-linux-gnu.tar.gz`
- `b2ige-0.3.0-x86_64-unknown-linux-gnu.manifest.json`
- `b2ige-0.3.0-source.tar.gz`
- `SHA256SUMS`

Native archive SHA256:

- Apple Silicon: `8b9d9a00a76a1c5f87ece1eb631bd6a2279e18581e9c47fdffde8e8358da98be`
- macOS Intel: `3044ddf5b6e4e1611418e6e8cb077f6227162b3659670f47f731741c34f62cfe`
- Linux x86_64: `297d5f79b0241a9116f48c15e094902d9ecba540c31f75c8e6a7fc7bf429e4be`
- Combined `SHA256SUMS`: `88b6b9638e7bad391f9a575b8f8b5f618ae97b3f1462d68bca167675628e129f`

After publication, all eight public assets were downloaded again. Public-set extraction, checksums, manifests, native binaries, embedded SBOMs and source validation passed.

A fresh Apple Silicon offline installation from the public release assets passed and reported `verify-cli 0.3.0`; `b2ige-mcp --help` also passed.

## Distribution boundaries

- Native SBOMs are embedded and validated inside each native archive.
- npm tgz and npm SBOM remain candidate-only.
- npm registry publication remains deferred/private.
- Cargo remains `publish=false` and unpublished.
- macOS binaries are unsigned and unnotarized.
- No Windows native archive is published.
- No Linux arm64 archive is published.
- SHA256 provides integrity, not publisher authentication.

## V110 state

- V110-A: implementation COMPLETE
- V110-B: implementation COMPLETE
- V110-C: implementation COMPLETE
- V110-D pilot framework: COMPLETE
- V110-D genuine external-user evidence: DEFERRED

External adoption remains a post-release validation objective and is not represented as completed product evidence.

## Claim boundary

B2IGE Verify 0.3.0 provides bounded verification evidence. The release does not claim exhaustive correctness, universal usability, statistically representative adoption, independent human validation, or secrecy against an unrestricted process running as the same host user.
