# Signing decision — 0.1.0

## Selected policy: unsigned macOS release

macOS 0.1.0 is released as an unsigned, unnotarized binary. No Developer ID
signature or notarization claim is made. Gatekeeper may show a warning when a
user opens the binary; the installation guidance calls this out explicitly.

The current environment has no valid Apple Developer ID identity. No certificate
was created, purchased or imported, and no notarization was performed. Ad-hoc
linker signatures, if present, do not authenticate a publisher.

Delivery must retain authenticated release-channel and SHA-256 instructions.
No installation script removes quarantine or bypasses OS controls automatically.
Developer ID signing and notarization are deferred improvements for a future release.
