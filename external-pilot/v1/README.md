# External operator pilot v1

**EVIDENCE_PENDING.** This kit contains instructions and tools, not external evidence.
One person who did not develop B2IGE can perform all three journeys. No AI account,
Codex, ChatGPT, source architecture knowledge or JSON editing is needed.

Read the [participant guide](../../docs/V110-EXTERNAL-PILOT.md). The recorder displays
commands, asks bounded questions and preserves unsuccessful attempts. It never types
approval for you. Start only as a genuine external operator, in your own environment.
Developers must use `python3 scripts/test-external-pilot.py`, never the pilot `run` command.

The source commit containing this kit is the pilot pin. The organizer supplies its full
40-character SHA after review; check it out detached and pass the same SHA to `run`.
The pin cannot be embedded as its own commit hash. The tool checks clean HEAD and binds
both protocol bytes and the existing adoption-v1 corpus. No mutable branch/tag is accepted.
The historical public v0.2.0 archive does not contain this kit.

- Protocol: [protocol.json](protocol.json), `external-pilot-v1`.
- Tooling schema: string version `2`, `kind: external-adoption-pilot`, `authoritative: false`.
  Version 2 adds source identities, personal/non-synthetic attestation and integration
  attempt/assistance records. Version 1 is refused, never silently upgraded.
- Normative validator: [external-pilot.py](../../scripts/external-pilot.py), `validate`.
- CI capture: [external_pilot_ci.py](../../scripts/external_pilot_ci.py).
- All public measurements are categorical, numeric, hashes or explicitly consented GitHub references.
- Optional notes are sanitized to exact allowlisted categories or withheld; raw text is never persisted.
- No participant results, completed report or synthetic substitute is shipped here.

```sh
python3 scripts/external-pilot.py report
```

With no inputs this deterministically reports `EVIDENCE_PENDING`, zero operators and
zero journeys, never completion. Source schemas, V100, P8 and state files are unchanged.
