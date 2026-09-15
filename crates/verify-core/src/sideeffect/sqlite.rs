//! SQLite online backups capture committed pages, including WAL. Table order is
//! authoritative only under the explicit append-only ledger contract. No clock ordering.
use super::*;
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use std::{
    fs,
    time::{Duration, Instant},
};
use verify_evidence::{ObservationCoverage, MAX_SAFE_INTEGER};

const MAX_DATABASE: u64 = 8 * 1024 * 1024;
fn db_error(error: impl std::fmt::Display) -> io::Error {
    invalid(&format!("SQLite observer: {error}"))
}
fn open(path: &Path) -> io::Result<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(db_error)?;
    db.busy_timeout(Duration::from_millis(20))
        .map_err(db_error)?;
    db.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON;")
        .map_err(db_error)?;
    Ok(db)
}
fn query(
    db: &Connection,
    observer: &SQLiteEffectObserver,
    contract: &SideEffectContract,
) -> io::Result<Vec<CommittedRow>> {
    let integrity: String = db
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(db_error)?;
    if integrity != "ok" {
        return Err(invalid("SQLite integrity check failed"));
    }
    let kind: String = db
        .query_row(
            "SELECT type FROM sqlite_schema WHERE name=?1",
            [&observer.table],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    if kind != "table" {
        return Err(invalid("committed observer must read a real table"));
    }
    let sql = format!(
        "SELECT \"{}\", \"{}\", \"{}\", \"{}\", \"{}\" FROM \"{}\" ORDER BY \"{}\"",
        observer.external_effect_id_column,
        observer.idempotency_column,
        observer.correlation_column,
        observer.operation_column,
        observer.commit_order_column,
        observer.table,
        observer.commit_order_column
    );
    // Require columns explicitly; SQLite's double-quoted string fallback cannot hide a mismatch.
    let columns: BTreeSet<String> = db
        .prepare(&format!("PRAGMA table_info(\"{}\")", observer.table))
        .map_err(db_error)?
        .query_map([], |r| r.get(1))
        .map_err(db_error)?
        .collect::<rusqlite::Result<_>>()
        .map_err(db_error)?;
    if [
        &observer.external_effect_id_column,
        &observer.idempotency_column,
        &observer.correlation_column,
        &observer.operation_column,
        &observer.commit_order_column,
    ]
    .iter()
    .any(|c| !columns.contains(*c))
    {
        return Err(invalid("SQLite observer schema mismatch"));
    }
    let mut statement = db.prepare(&sql).map_err(db_error)?;
    let mut cursor = statement.query([]).map_err(db_error)?;
    let mut result = vec![];
    let mut identities = BTreeSet::new();
    let mut last_order = 0;
    while let Some(row) = cursor.next().map_err(db_error)? {
        let external: String = row.get(0).map_err(db_error)?;
        let idempotency: String = row.get(1).map_err(db_error)?;
        let correlation: String = row.get(2).map_err(db_error)?;
        let operation: String = row.get(3).map_err(db_error)?;
        let order: i64 = row.get(4).map_err(db_error)?;
        if [&external, &idempotency, &correlation, &operation]
            .iter()
            .any(|v| v.trim().is_empty())
            || order <= 0
            || order as u64 > MAX_SAFE_INTEGER
            || order as u64 <= last_order
            || !identities.insert((operation.clone(), external.clone()))
        {
            return Err(invalid(
                "SQLite ledger identity/order is missing, conflicting or nonunique",
            ));
        }
        last_order = order as u64;
        if correlation != contract.operation.correlation_identity {
            continue;
        }
        for effect in contract.effects.iter().filter(|e| {
            e.authoritative_observer == observer.observer_id && e.operation == operation
        }) {
            if idempotency != effect.identity.idempotency_identity {
                return Err(invalid(
                    "committed identity does not match contract correlation",
                ));
            }
            result.push(CommittedRow {
                effect_id: effect.effect_id.clone(),
                identity: CommittedEffectIdentity {
                    provider: effect.provider.clone(),
                    operation: operation.clone(),
                    external_effect_id: external.clone(),
                    idempotency_identity: idempotency.clone(),
                    correlation_identity: correlation.clone(),
                    commit_order: order as u64,
                },
            });
        }
        if result.len() > 4096 {
            return Err(invalid("SQLite committed row budget exceeded"));
        }
    }
    Ok(result)
}
fn capture(path: &Path) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("SQLite source is not a regular file"));
    }
    let db = open(path)?;
    let pages: u32 = db
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .map_err(db_error)?;
    let page_size: u32 = db
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .map_err(db_error)?;
    if u64::from(pages).saturating_mul(u64::from(page_size)) > MAX_DATABASE {
        return Err(invalid("SQLite snapshot exceeds 8 MiB"));
    }
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("snapshot.db");
    let mut target = Connection::open(&file).map_err(db_error)?;
    let backup = Backup::new(&db, &mut target).map_err(db_error)?;
    let start = Instant::now();
    loop {
        if matches!(backup.step(128).map_err(db_error)?, StepResult::Done) {
            break;
        }
        if start.elapsed() > Duration::from_millis(100) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "SQLite snapshot busy/incomplete",
            ));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(backup);
    drop(target);
    let bytes = fs::read(file)?;
    if bytes.len() as u64 > MAX_DATABASE {
        return Err(invalid("SQLite snapshot exceeds 8 MiB"));
    }
    Ok(bytes)
}
pub(super) fn rows_from_bytes(
    bytes: &[u8],
    observer: &SQLiteEffectObserver,
    contract: &SideEffectContract,
) -> io::Result<Vec<CommittedRow>> {
    if bytes.len() as u64 > MAX_DATABASE {
        return Err(invalid("SQLite snapshot exceeds 8 MiB"));
    }
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("snapshot.db");
    fs::write(&file, bytes)?;
    query(&open(&file)?, observer, contract)
}
pub(super) fn observe_all(workspace: &Path, contract: &SideEffectContract) -> Vec<SQLiteSnapshot> {
    contract
        .required_observers
        .iter()
        .map(|observer| {
            let result = capture(&workspace.join(&observer.db_path));
            let (database, rows, coverage, reason) = match result {
                Ok(bytes) => match rows_from_bytes(&bytes, observer, contract) {
                    Ok(rows) => (
                        Some(bytes),
                        rows,
                        ObservationCoverage::Complete,
                        "Committed SQLite backup complete".into(),
                    ),
                    Err(e) => (
                        Some(bytes),
                        vec![],
                        ObservationCoverage::Failed,
                        e.to_string(),
                    ),
                },
                Err(e) => (
                    None,
                    vec![],
                    if matches!(
                        e.kind(),
                        io::ErrorKind::NotFound
                            | io::ErrorKind::PermissionDenied
                            | io::ErrorKind::WouldBlock
                    ) {
                        ObservationCoverage::Unavailable
                    } else {
                        ObservationCoverage::Failed
                    },
                    e.to_string(),
                ),
            };
            SQLiteSnapshot {
                observer_id: observer.observer_id.clone(),
                database,
                rows,
                coverage,
                reason,
            }
        })
        .collect()
}
pub(super) fn verify_snapshots(
    snapshots: &[SQLiteSnapshot],
    contract: &SideEffectContract,
) -> io::Result<()> {
    if snapshots.len() != contract.required_observers.len() {
        return Err(invalid("missing required SQLite snapshot"));
    }
    for (s, observer) in snapshots.iter().zip(&contract.required_observers) {
        if s.observer_id != observer.observer_id || s.reason.is_empty() {
            return Err(invalid("invalid SQLite observer binding"));
        }
        match (&s.database, s.coverage) {
            (Some(bytes), ObservationCoverage::Complete) => {
                if rows_from_bytes(bytes, observer, contract)? != s.rows {
                    return Err(invalid("committed rows contradict SQLite backup"));
                }
            }
            (Some(bytes), ObservationCoverage::Failed) => {
                if !s.rows.is_empty() || rows_from_bytes(bytes, observer, contract).is_ok() {
                    return Err(invalid("invalid failed SQLite snapshot"));
                }
            }
            (None, ObservationCoverage::Failed | ObservationCoverage::Unavailable)
                if s.rows.is_empty() => {}
            _ => return Err(invalid("inconsistent SQLite coverage evidence")),
        }
    }
    Ok(())
}
