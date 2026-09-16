#!/bin/sh
# No downloads, PATH binary fallback, config generation, or stdout diagnostics.
set -eu
fail() { printf '%s\n' "B2IGE Verify: $*" >&2; exit 64; }
os=$(uname -s) || fail 'cannot detect operating system'
arch=$(uname -m) || fail 'cannot detect architecture'
case "$os/$arch" in
  Darwin/arm64) target=aarch64-apple-darwin ;;
  Darwin/x86_64) target=x86_64-apple-darwin ;;
  Linux/x86_64) target=x86_64-unknown-linux-gnu ;;
  *) fail "unsupported platform $os/$arch; supported: darwin arm64, darwin x86_64, linux x86_64 (GNU libc)" ;;
esac
[ "$#" -eq 2 ] || fail 'expected --registry /absolute/trusted/project.json'
[ "$1" = '--registry' ] || fail 'expected --registry /absolute/trusted/project.json'
case "$2" in /*) ;; *) fail 'registry must be an absolute path' ;; esac
[ -f "$2" ] && [ -r "$2" ] || fail 'registry is not a readable file; trusted operator setup is required'
# Empty optional MCPB configuration means no BlindTest sealed root was supplied.
if [ -z "${B2IGE_BLINDTEST_SEALED_ROOT:-}" ]; then
  unset B2IGE_BLINDTEST_SEALED_ROOT
else
  case "$B2IGE_BLINDTEST_SEALED_ROOT" in
    /*) ;;
    *) fail 'BlindTest sealed root must be an absolute path' ;;
  esac
  [ -d "$B2IGE_BLINDTEST_SEALED_ROOT" ] || fail 'BlindTest sealed root is not a directory'
fi
bundle_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
binary="$bundle_dir/bin/$target/b2ige-mcp"
[ -x "$binary" ] || fail "bundled executable is missing or not executable: $target"
exec "$binary" "$@"
