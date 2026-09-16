# B2IGE Verify V100 — Harness Preflight

This is a read-only preflight. Do not modify, create, delete, stage, commit, or push project files.

Goal:
Confirm that LongHorizon-Harness can safely operate this B2IGE Verify repository before the autonomous V100 program begins.

Required checks:

1. Confirm the current branch is `automation/v100-longhorizon`.
2. Read `AGENTS.md`.
3. Confirm these authoritative documents exist:
   - `docs/CONTRACTS.md`
   - `docs/VERDICTS.md`
   - `docs/EVIDENCE.md`
   - `docs/THREAT-MODEL.md`
4. Run `git status --short` and confirm the workspace is clean apart from harness-owned ignored runtime files.
5. Run `cargo metadata --no-deps --format-version 1` successfully.
6. Run `docker info` successfully.
7. Do not run full tests, builds, benchmarks, or package validation in this preflight.
8. Do not inspect or search for any private/hidden holdout repository or external secret material.

Completion requires actual command evidence for the checks above.

This preflight must consume as little work as reasonably possible.
