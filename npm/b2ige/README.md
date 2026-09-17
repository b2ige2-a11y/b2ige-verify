# @b2ige/verify — thin launcher (npm unpublished/private)

The wrapper contains distribution and process-launch code only. All product verification
runs in the version-matched native executable. No npm dependencies or postinstall hook.
The final package name is `@b2ige/verify`. The public [GitHub/native v0.2.0 release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.2.0)
is available, while npm publication remains intentionally deferred/private and is not part of
that release. The package is private and `prepublishOnly` always fails until the `@b2ige` scope is
actually controlled and publication is separately authorized.
Unscoped npm `blindtest` is occupied and is not used by this project.

## Native delivery

When configured, the flow is:

GitHub Release → exact version/platform archive → pinned archive SHA-256 → safe
USTAR extraction → pinned binary SHA-256 → exact `verify-cli 0.2.0` version probe → execution.

`native-manifest.json` schema v2 carries `release_repository` and, for each platform,
`sha256` (binary) and `archive_sha256`. The repository is currently **null** and platform
pins are empty. No real host, asset or registry ownership is claimed. An authorized
release preparer must fill these fields from the selected repository and the checked
external native manifests after all platform runs. Pins travel inside the reviewed npm
package; they are never fetched as trust inputs from the archive response.

The URL is constructed from the configured repository, wrapper version and fixed Rust
target mapping. Only GitHub HTTPS and its two explicitly allowed release-asset redirect
hosts are accepted. Downloads have a 60-second total deadline, three redirects and a
64 MiB compressed limit. Extraction accepts USTAR regular files/directories only, with
512 MiB decompressed limit; escaping paths, links, duplicate entries, malformed headers
and truncated archives fail closed. All contents stay in a private staging directory
until an atomic cache installation; partial downloads are never executed.

Cache: `~/.cache/b2ige-verify/<version>/<platform>/<archive-sha256>/`.
The executable's SHA-256 and version are rechecked before every launch, including offline
cache use. A mismatch never retries another binary or falls back to PATH.
Hashes provide integrity, not publisher authentication or same-user host protection.
The version probe executes only after the binary digest matches; a version mismatch
prevents all product-command execution.

## Trusted local execution

Set `B2IGE_BINARY` to a trusted native executable and `B2IGE_BINARY_SHA256` to its expected
binary digest. Relative paths resolve to an absolute file before checking or launching.
An owner-supplied bundled `native/<platform>/b2ige` also requires a manifest pin.
Explicit overrides never trigger download fallback when invalid.

Mappings: darwin-arm64 → aarch64-apple-darwin; darwin-x64 → x86_64-apple-darwin;
linux-x64 → x86_64-unknown-linux-gnu. Unsupported platforms, unavailable hosts/assets,
download errors, missing pins, checksum or version failures produce **ERROR / exit 3**.
`b2ige` forwards arguments and `blindtest` prefixes the subcommand. stdio, exit codes and
process signals are preserved. No arbitrary binary is searched for on PATH.

## Validation and remaining external work

`npm test` exercises local launch and synthetic download-to-execution, offline cache,
archive/binary tampering, version refusal, unsafe extraction, redirects and download
failure. The transport fixture is process-local test code, not a production host override.
Real GitHub-hosted delivery still requires the actual repository, native CI artifacts
and an authorized release. No live-host end-to-end result is claimed.

From the repository root, `python3 scripts/package-rc.py` stages LICENSE/TRADEMARKS and
runs `npm pack --ignore-scripts`; a bare development-directory pack is not the reviewed
release path. Publication guards remain active even after a host is configured.
