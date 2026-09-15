# Public License Decision

The 0.1.0 repository code is licensed under the Apache License 2.0; the
authoritative text is [LICENSE](LICENSE). The current Core, CLI, BlindTest,
Behavior, SideEffect Proof, MCP, Skill, and Bench are in the OSS scope under
this code license. The Open Core model permits future team, enterprise, or
managed offerings to be commercial offerings.

This decision covers code only. B2IGE, B2IGE Verify, BlindTest, BehaviorSeal,
SideEffect Proof, and related branding are handled separately
in [TRADEMARKS.md](TRADEMARKS.md). The code license does not automatically grant
brand-use rights.

Cargo packages remain `publish = false`; the npm wrapper uses the final scoped
metadata name `@b2ige/verify`, remains private and has a failing prepublish guard.
The scope must be actually controlled before npm publication. The official 0.1.0
GitHub repository is [b2ige2-a11y/b2ige-verify](https://github.com/b2ige2-a11y/b2ige-verify).
GitHub publication remains pending the public conversion, Private Vulnerability
Reporting activation, and `v0.1.0` tag/release creation; npm is intentionally deferred.
