//! Executable local provider: commits SQLite ledger rows, then optionally waits
//! before returning success. SAFE idempotency is enforced in the same transaction.
use rusqlite::{params, Connection};
fn main() {
    let mut db = Connection::open("ledger.db").expect("ledger");
    db.busy_timeout(std::time::Duration::from_secs(2)).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS ledger(seq INTEGER PRIMARY KEY AUTOINCREMENT, external_id TEXT NOT NULL UNIQUE, idem TEXT NOT NULL, correlation TEXT NOT NULL, operation TEXT NOT NULL);").unwrap();
    let mode = std::env::var("MODE").unwrap();
    let key = std::env::var("KEY").unwrap();
    let correlation = std::env::var("CORRELATION").unwrap();
    let operations = std::env::var("OPERATIONS").unwrap_or_else(|_| "payment".into());
    for operation in operations.split(',').filter(|s| !s.is_empty()) {
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ledger WHERE idem=?1 AND operation=?2)",
                params![key, operation],
                |r| r.get(0),
            )
            .unwrap();
        if mode != "safe" || !exists {
            let count = if mode == "double" { 2 } else { 1 };
            for _ in 0..count {
                tx.execute("INSERT INTO ledger(external_id,idem,correlation,operation) VALUES(lower(hex(randomblob(16))),?1,?2,?3)",params![key,correlation,operation]).unwrap();
            }
        }
        tx.commit().unwrap();
    }
    // The response marker is durable target-local state so retry can return.
    // The commit-before-response window is guaranteed on the first attempt.
    if std::env::var("HOLD").as_deref() == Ok("1") && !std::path::Path::new("attempted").exists() {
        std::fs::write("attempted", b"first attempt").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    std::process::exit(
        std::env::var("EXIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
}
