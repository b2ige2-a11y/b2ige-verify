# Signing decision — 0.1.0

No Apple Developer ID identity is available in the current environment. No
certificate was created, purchased or imported, and no notarization was performed.

The owner must choose one of these before public binary distribution:

1. **Unsigned 0.1.0** — publish only with an explicit unsigned-build warning,
   authenticated delivery and checksum instructions. This is an owner decision,
   not an automatic default.
2. **Developer ID signing + notarization** — sign every macOS executable, notarize
   the distribution container, retain the notarization evidence, then recompute
   hashes and re-run the release checks after signing.

Current status: **OWNER DECISION REQUIRED**. Ad-hoc linker signatures, if present,
do not authenticate a publisher and do not select either option.
