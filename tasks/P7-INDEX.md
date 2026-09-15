# P7 — Agent / MCP / CI Integration

Single phase; no sub-phases. P6 prerequisite gate is recorded in P6-RESULT.md.
P8 Benchmark is excluded and must not start before the recorded P7 gate.

## Scope

- Independent Agent Protocol v1 request/response/schema and deterministic projection.
- Short portable versioned skill, confirmed Codex stdio MCP setup.
- Local Rust MCP with five registered-config tools, startup-pinned configuration,
  opaque report aliases, no arbitrary command/file/raw report surface.
- Init no-overwrite/dry-run, readiness doctor, Behavior CLI and existing CLI compatibility.
- Human/json/agent output plus explicit protocol v1 and fixed machine errors.
- Local-first GitHub Actions template, sanitized CI artifacts, strict exit agreement.
- Real process CLI/MCP/SQLite/Docker workflows, hidden/canary checks and negative controls.

## Gate

Required: fmt, clippy -D warnings, all workspace tests (original 339 retained), release
build, P7 MCP/CLI/CI tests, actionlint, existing BlindTest Docker gate. Record actual
counts, metrics and limitations in P7-RESULT.md and state/CURRENT.md. No commit/push/deploy.
