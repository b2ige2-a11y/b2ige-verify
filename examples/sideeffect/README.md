# SideEffect Proof

Did it actually happen?

Install [native binaries](../../docs/INSTALL.md). From the source workspace run
`scripts/demo-sideeffect.sh` for setup, real verification, expected exit checks and sanitized output.
With installed binaries, create a new example config:

```sh
demo="$(mktemp -d)/sideeffect"
b2ige-demo sideeffect "$demo"
b2ige sideeffect verify "$demo/safe.contract.json" --output agent
```

Expected: PASS: safe idempotent SQLite retry; FAIL: unsafe retry commits twice.

For FAIL, use `unsafe.contract.json` (exit 1). [Target](target.rs) checks idempotency in the safe
transaction; unsafe mode inserts another row. [Config generator](setup.rs) creates one contract
per mode, an empty ledger and `[NONE, RETRY]` schedules. These generated `.contract.json` files
are the executable example configurations. Counts come from committed SQLite backup evidence,
not printed output or attempt counts. Public API: `verify_core::sideeffect`.
[Contract limits](../../docs/SIDEEFFECT.md).

Config identities vary per installation; generated configs avoid invalid placeholder hashes.
Do not copy private runtime reports into the repository. Product version 0.1.0; schema versions
remain those defined by the existing contracts.
