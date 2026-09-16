# Local distribution setup

The Cursor Agent Plugin and Codex plugin share this skill. Neither assumes `b2ige`
or `b2ige-mcp` is installed. If no reviewed CLI/MCP setup exists, report that setup
is required; do not claim verification or invent a successful result.

Use the trusted operator's already configured B2IGE MCP tools, or the unchanged
[v0.1.0 native release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0).
The portable MCPB packages that release's three native MCP binaries with a local
launcher. Its URL is reserved until the owner uploads the additional asset.

See the [distribution guide](https://github.com/b2ige2-a11y/b2ige-verify/blob/main/distribution/README.md)
for bundle integrity, supported platforms, and explicit Cursor/Codex stdio setup.
Use the original CLI archive for CLI commands; the MCPB contains only `b2ige-mcp`.

Trusted operators must register reviewed absolute config paths and applicable
Behavior authorization. BlindTest needs Docker and protected suites/sealed storage
outside the coding workspace. Installing the plugin or running doctor does not
create hidden-test isolation, approve suites, or prove a task complete.
