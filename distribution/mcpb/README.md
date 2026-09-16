# B2IGE Verify — local MCPB 0.1.0

Don't trust done. Prove it.

Independent deterministic verification for AI-written software. This bundle contains
the unchanged v0.1.0 `b2ige-mcp` for macOS arm64, macOS x86_64, and GNU/Linux x86_64
(release-tested on Ubuntu 22.04). Linux arm64, musl/Alpine, and Windows are unsupported.
The OS/architecture launcher fails closed outside the three supported targets.
`compatibility.platforms` is OS-only because MCPB does not have an architecture filter.

Import into an MCPB-capable local client. Select a trusted operator's reviewed registry
file using an absolute path. All config, authorization, target, and store paths in that
registry must be absolute. The bundle never creates/approves configs or downloads code.
For manual stdio setup after verifying/extracting this bundle:

```sh
/bin/sh /absolute/trusted/b2ige-verify/launch.sh --registry /absolute/trusted/project.json
```

BlindTest additionally requires a local Docker Linux engine, approved suite and image,
and an absolute protected sealed root outside the coding workspace. Configure the
optional sealed-root field (or `B2IGE_BLINDTEST_SEALED_ROOT` for manual stdio setup).
Keep controller files, registry, and executable inaccessible to the coding agent when
claiming secrecy; same-user unrestricted host access is not an isolation boundary.

No API key, telemetry, cloud endpoint, or hosted service is required. Configured target
programs execute locally with the controller's authority. The host client has its own
data policy. Doctor checks readiness only. Missing evidence and transport failure never
mean PASS; bounded checks are not exhaustive proof.

Apache-2.0; see LICENSE, THIRD-PARTY-NOTICES.txt, SECURITY.md and TRADEMARKS.md.
This MCPB is unsigned; existing macOS binaries remain unsigned/unnotarized. Do not
bypass OS protections. CLI tools are available separately in the original release.

[Source and setup](https://github.com/b2ige2-a11y/b2ige-verify/blob/main/distribution/README.md)
· [Original release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0)
