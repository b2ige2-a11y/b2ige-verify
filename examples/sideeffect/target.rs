//! Public SQLite example: the safe transaction checks idempotency before committing.
use rusqlite::{params, Connection};
fn main() {
    let mut db = Connection::open("ledger.db").unwrap();
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM ledger WHERE idem=?1)",
            ["checkout_1"],
            |r| r.get(0),
        )
        .unwrap();
    if std::env::var("MODE").as_deref() != Ok("safe") || !exists {
        tx.execute("INSERT INTO ledger(external_id,idem,correlation,operation) VALUES(lower(hex(randomblob(16))),?1,?2,?3)",params!["checkout_1","checkout_1","payment"]).unwrap();
    }
    tx.commit().unwrap();
}
