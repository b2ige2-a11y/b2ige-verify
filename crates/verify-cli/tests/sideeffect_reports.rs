#![cfg(unix)]
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use verify_core::{
    behavior::{executable_identity, snapshot_identity, BehaviorAuthorization, LocalFixture},
    sideeffect::*,
    Verdict,
};
use verify_evidence::store::EvidenceStore;
struct Case {
    dir: PathBuf,
    contract: SideEffectContract,
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
impl Case {
    fn new(mode: &str) -> Self {
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
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.dir.join("runs"))
    }
    fn report(&self) -> verify_cli::VerifiedReport {
        execute(&self.contract, &self.store(), "result").unwrap();
        verify_cli::load(&self.store(), "result", &auth()).unwrap()
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_b2ige"))
            .args(args)
            .arg("--store")
            .arg(self.dir.join("runs"))
            .output()
            .unwrap()
    }
}
fn auth() -> BehaviorAuthorization {
    BehaviorAuthorization {
        approved_baselines: Default::default(),
        approved_checker_bindings: Default::default(),
        baseline_stable: false,
    }
}
#[test]
fn sideeffect_human_agent_ui_projection_and_schema() {
    let c = Case::new("unsafe");
    let report = c.report();
    assert_eq!(report.document().verdict, Verdict::Fail);
    let h = report.human();
    for text in [
        "Duplicate committed effect proven",
        "Attempts      2",
        "Committed     2",
        "Locally minimized reproduction",
        "Expected",
        "Observed",
        "Timeline",
    ] {
        assert!(h.contains(text), "{text}: {h}");
    }
    let html = verify_cli::viewer::render(&report, "/token", "result");
    for text in [
        "Duplicate committed effect proven",
        "Timeline ·",
        "Evidence ·",
        "Runs · 2",
        "Raw artifact",
        "Locally minimized reproduction",
    ] {
        assert!(html.contains(text));
    }
    for (schema, value) in [
        (
            verify_cli::report_schema(),
            serde_json::to_value(report.document()).unwrap(),
        ),
        (
            verify_cli::agent_schema(),
            serde_json::to_value(report.agent()).unwrap(),
        ),
    ] {
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
    }
}
#[test]
fn agent_allowlist_excludes_paths_environment_and_raw_history() {
    let c = Case::new("unsafe");
    let r = c.report();
    let json = serde_json::to_string(&r.agent()).unwrap();
    for secret in [
        "secret-do-not-export",
        "PRIVATE_TOKEN",
        "provider-private",
        "ledger.db",
        "/usr/bin",
        "environment",
        "database",
        "logical_order",
        "correlation_identity",
    ] {
        assert!(!json.contains(secret), "leaked {secret}");
    }
    assert!(!json.contains(c.dir.to_str().unwrap()));
    let v = serde_json::to_value(r.agent()).unwrap();
    assert_eq!(v["observed"][0]["committed"], 2);
    assert_eq!(v["expected"][0]["expectation"], "exactly_once");
    assert!(v["reproduction"].is_object());
}
#[test]
fn cli_verify_and_report_preserve_all_four_exit_codes_without_behavior_auth() {
    for (mode, kind, expected) in [
        ("safe", "valid", 0),
        ("unsafe", "valid", 1),
        ("safe", "missing", 2),
        ("safe", "schema", 3),
    ] {
        let mut c = Case::new(mode);
        if kind == "missing" {
            c.contract.required_observers[0].db_path = "missing.db".into();
        }
        if kind == "schema" {
            c.contract.required_observers[0].table = "missing_table".into();
        }
        fs::write(
            c.dir.join("contract.json"),
            serde_json::to_vec(&c.contract).unwrap(),
        )
        .unwrap();
        let output = c.cli(&[
            "sideeffect",
            "verify",
            c.dir.join("contract.json").to_str().unwrap(),
            "--output",
            "agent",
        ]);
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let id = v["source"]["artifact_id"].as_str().unwrap();
        let again = c.cli(&["report", id, "--output", "json"]);
        assert_eq!(again.status.code(), Some(expected));
    }
}
#[test]
fn cli_unsupported_semantics_is_error() {
    let c = Case::new("safe");
    let mut v = serde_json::to_value(&c.contract).unwrap();
    v["effects"][0]["expectation"] = "eventually_within".into();
    fs::write(c.dir.join("contract.json"), serde_json::to_vec(&v).unwrap()).unwrap();
    assert_eq!(
        c.cli(&[
            "sideeffect",
            "verify",
            c.dir.join("contract.json").to_str().unwrap()
        ])
        .status
        .code(),
        Some(3)
    );
}
#[test]
fn report_rejects_corrupted_child_and_viewer_bind_fails_closed() {
    let c = Case::new("safe");
    c.report();
    fs::write(
        c.dir.join("runs/result-s0-a1/evidence/committed.json"),
        b"{}",
    )
    .unwrap();
    assert_eq!(c.cli(&["report", "result"]).status.code(), Some(3));
    assert!(verify_cli::viewer::Viewer::bind(c.store(), auth(), "result").is_err());
}
#[test]
fn pass_and_inconclusive_show_known_and_unknown_counts() {
    for missing in [false, true] {
        let mut c = Case::new("safe");
        if missing {
            c.contract.required_observers[0].db_path = "unavailable.db".into();
        }
        let r = c.report();
        assert_eq!(
            r.document().verdict,
            if missing {
                Verdict::Inconclusive
            } else {
                Verdict::Pass
            }
        );
        assert!(r.human().contains(if missing {
            "Committed     unknown"
        } else {
            "Committed     1"
        }));
        let html = verify_cli::viewer::render(&r, "/token", "result");
        assert!(html.contains(if missing {
            "? INCONCLUSIVE"
        } else {
            "✓ VERIFIED"
        }));
    }
}
