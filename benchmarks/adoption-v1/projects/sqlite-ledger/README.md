# Disposable checkout ledger

The runner materializes the existing reviewed Python/SQLite transaction and schema
from `crates/verify-mcp/tests/support/sideeffect.rs`, without changing its semantics.
The unchanged transaction source is materialized as `src/checkout.py` and copied
into the reset fixture; `python3 checkout.py` is the run surface within that fixture.
The input contract records the exact Python command, fixture, retry schedule, and
committed-state observer. It runs against a fresh disposable `fixture/ledger.db`.
Safe mode deduplicates payment commits; unsafe mode commits twice on retry.
The corrupt scenario removes recorded evidence after an intact verified run,
exactly as P8 `sideeffect.corrupt` does. No production database is accessed.
