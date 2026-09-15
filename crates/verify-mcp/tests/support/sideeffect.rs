#![allow(dead_code)]
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use verify_core::{
    behavior::{executable_identity, snapshot_identity, LocalFixture},
    sideeffect::*,
};
pub struct Case {
    pub dir: PathBuf,
    pub contract: SideEffectContract,
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
impl Case {
    pub fn new(mode: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "p5-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let fixture = dir.join("fixture");
        fs::create_dir(&fixture).unwrap();
        assert!(Command::new("/usr/bin/python3").args(["-c","import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('CREATE TABLE ledger(seq INTEGER PRIMARY KEY, external_id TEXT, idem TEXT, correlation TEXT, operation TEXT)'); c.commit()",fixture.join("ledger.db").to_str().unwrap()]).status().unwrap().success());
        // This fixture runs an actual process and durable SQLite transaction in every CLI test.
        let executable = Path::new("/usr/bin/python3").to_path_buf();
        let script="import sqlite3,os,sys; c=sqlite3.connect('ledger.db'); n=c.execute('SELECT count(*) FROM ledger').fetchone()[0]; mode=os.environ['MODE']; c.execute('INSERT INTO ledger VALUES(?,?,?,?,?)',(n+1,str(n+1),'key','correlation','payment')) if mode!='safe' or n==0 else None; c.commit()";
        let contract = SideEffectContract {
            contract_id: "checkout".into(),
            schema_version: "1".into(),
            operation: OperationIdentity {
                operation_id: "checkout".into(),
                idempotency_identity: "key".into(),
                correlation_identity: "correlation".into(),
            },
            trigger: Trigger {
                executable_hash: executable_identity(&executable).unwrap(),
                executable,
                args: vec!["-c".into(), script.into()],
                environment: BTreeMap::from([
                    ("MODE".into(), mode.into()),
                    ("PRIVATE_TOKEN".into(), "secret-do-not-export".into()),
                ]),
                fixture: LocalFixture {
                    source: Some(fixture.clone()),
                    snapshot_identity: snapshot_identity(Some(&fixture)).unwrap(),
                },
                timeout_ms: 3000,
            },
            effects: vec![EffectContract {
                effect_id: "payment".into(),
                provider: "provider-private".into(),
                adapter: Adapter::Sqlite,
                operation: "payment".into(),
                identity: IdentityContract {
                    external_identity: ExternalIdentity::ProviderOperationExternalId,
                    idempotency_identity: "key".into(),
                    correlation_identity: "correlation".into(),
                },
                expectation: Expectation::ExactlyOnce,
                authoritative_observer: "ledger".into(),
            }],
            relations: vec![],
            fault_schedules: vec![FaultSchedule {
                schedule_id: "retry".into(),
                primitives: vec![FaultPrimitive::None, FaultPrimitive::Retry],
            }],
            exploration_budget: ExplorationBudget {
                max_schedules: 1,
                max_attempts: 2,
                reduction_executions: 3,
            },
            required_observers: vec![SQLiteEffectObserver {
                observer_id: "ledger".into(),
                db_path: "ledger.db".into(),
                table: "ledger".into(),
                external_effect_id_column: "external_id".into(),
                idempotency_column: "idem".into(),
                correlation_column: "correlation".into(),
                operation_column: "operation".into(),
                commit_order_column: "seq".into(),
                authoritative_source: LedgerAuthority::DurableAppendOnlyCommittedState,
            }],
        };
        Self { dir, contract }
    }
}
