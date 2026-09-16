# Directory metadata — copyable listing

Publication status: prepared locally, **not yet submitted**. The bundle URL is reserved
until the owner uploads the additional MCPB asset; do not advertise it as downloadable yet.

**Name:** B2IGE Verify

**One-line description:** Independent deterministic verification for AI-written software.

**Tagline:** Don't trust done. Prove it.

**Long description:** B2IGE Verify independently checks AI-written software in local
repositories using deterministic, evidence-backed verifiers. BlindTest checks approved
hidden CLI invariants under Docker isolation. BehaviorSeal compares approved before/after
executable behavior without automatically updating the baseline. SideEffect checks
durable commits in a local SQLite ledger. Use it to check whether a coding agent actually
finished a task, detect behavior regressions, and avoid false PASS. Missing critical
evidence never counts as success; bounded tests are not exhaustive proof. Hidden suites
require a trusted controller and protected storage outside the coding-agent workspace.
The MCP server exposes only five registered-config tools over local stdio. It is not a
remote hosted verification service or arbitrary shell gateway.

**Repository URL:** https://github.com/b2ige2-a11y/b2ige-verify

**Release URL:** https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0

**Bundle URL (pending upload):** https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/b2ige-verify-0.1.0.mcpb

**Bundle SHA-256:** `f205d467a639747ce684bb32c4a560e6f63ade73cddeddbb822aafd00a48c884`

**License:** Apache-2.0. Existing SECURITY.md, TRADEMARKS.md and third-party notices apply.

**Supported platforms:** macOS/darwin arm64; macOS/darwin x86_64; GNU/Linux x86_64
(release-tested Ubuntu 22.04). No Windows, Linux arm64, or musl/Alpine support.

**MCP transport:** local stdio; native `b2ige-mcp` 0.1.0. No remote HTTP endpoint.

**Registry name:** `io.github.b2ige2-a11y/b2ige-verify`

**Install:** Import the SHA-256-verified MCPB into a compatible local client and configure
the trusted registry path. Alternatively extract the verified bundle and configure this
stdio command, replacing paths with the trusted operator's actual absolute paths:

```sh
/bin/sh /absolute/trusted/b2ige-verify-0.1.0/launch.sh --registry /absolute/trusted/project.json
```

For CLI use, install the matching original v0.1.0 native archive. No npm install/publish
or product recompilation is required. BlindTest additionally requires a local Docker
Linux engine, approved suite/image, and `B2IGE_BLINDTEST_SEALED_ROOT` pointing to protected
absolute storage. See [setup](README.md). The shared skill plugin alone does not install
or configure a verifier.

**Keywords:** verification, deterministic, AI-generated code, coding agent, task completion,
hidden tests, BlindTest, behavior regression, BehaviorSeal, side effects, SQLite,
committed effects, false PASS, local-first, MCP, stdio, Cursor, Codex, developer tools.

**Privacy/security note:** B2IGE does not require telemetry, an API key, or a hosted service.
Verification executes reviewed local targets and writes local evidence under operator
authority; a local process is not automatically sandboxed. Host AI clients and configured
target programs have their own data/network behavior. MCP returns sanitized Agent output;
protect hidden suites, registry/configuration, binaries and sealed reports from the coding
agent. Unrestricted same-user host access cannot provide suite secrecy. Missing evidence,
observer failures and transport failures must not be presented as PASS. macOS binaries
and this MCPB are unsigned/unnotarized. Report vulnerabilities via
[SECURITY.md](https://github.com/b2ige2-a11y/b2ige-verify/blob/main/SECURITY.md);
branding follows [TRADEMARKS.md](https://github.com/b2ige2-a11y/b2ige-verify/blob/main/TRADEMARKS.md).

**Platform mapping:** Cursor Marketplace uses the repository's root Agent Plugin manifest;
Codex uses `.codex-plugin/plugin.json`. cursor.directory and other MCP directories can
reuse this listing. Smithery should use its local MCPB publication path with the identical
bundle, not a URL-hosted server. Actual names/category selection and publication approval
remain the platform owner's/reviewer's decision.
OpenAI public-directory submission for this local-execution workflow requires confirming
the supported path with OpenAI; local Codex manifest validation is not directory approval.
