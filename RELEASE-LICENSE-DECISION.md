# Public License Decision

The 0.1.0 repository code is licensed under the Apache License 2.0; the
authoritative text is [LICENSE](LICENSE). The current Core, CLI, BlindTest,
Behavior, SideEffect Proof, MCP, Skill, and Bench are in the OSS scope under
this code license. The Open Core model permits future team, enterprise, or
managed offerings to be commercial offerings.

This decision covers code only. B2IGE, B2IGE Verify, BlindTest, BehaviorSeal
(tentative name), SideEffect Proof, and related branding are handled separately
in [TRADEMARKS.md](TRADEMARKS.md). The code license does not automatically grant
brand-use rights.

Cargo packages remain `publish = false`; the npm wrapper uses the provisional
scoped metadata name `@b2ige/verify`, remains private and has a failing
prepublish guard. Scope ownership and the final package name are unverified.
No repository URL is asserted. Public
publication remains blocked by the unresolved items in the
[release checklist](release/checklist.md), including the dependency license
review marked `REVIEW REQUIRED`.
