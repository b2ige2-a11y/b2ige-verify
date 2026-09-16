# P11 — Agent Distribution

Scope authorized 2026-09-16: package existing v0.1.0 release bytes for Official MCP
Registry, Cursor Marketplace, cursor.directory, Smithery, and local Codex plugins.
P10 commit `13539e165a5170003162ec0c2015af4537e61573` is excluded; no README discoverability
work. The prior P9 public-release gate is recorded in `state/CURRENT.md`.

- [x] One portable MCPB with three pinned release MCP binaries and fail-closed launcher.
- [x] MCPB pack/validate, archive/binary integrity, final SHA-256, repeatable packaging.
- [x] Registry server.json schema and mcp-publisher validate; no publish.
- [x] Portable Cursor Agent Plugin plus shared discovery skill; static schema preflight.
- [x] Codex manifest, Developer Tools category, installed validator; shared skill.
- [x] Common directory metadata and operator installation/submission instructions.
- [x] Distribution-only validation and unchanged product scope confirmed.
- [x] Local asset gate recorded; external submission/live-client checks explicitly pending.

No product compilation, full Rust tests/CI, npm publication, push, release asset upload,
registry publication, marketplace submission, or tag mutation is authorized in this task.

[Result and limitations](../P11-RESULT.md) · [Distribution guide](../distribution/README.md)
