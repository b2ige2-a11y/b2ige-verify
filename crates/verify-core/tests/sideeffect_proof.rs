#![cfg(unix)]
use rusqlite::Connection;
use std::{collections::BTreeMap, fs, path::Path};
use verify_core::{
    behavior::{executable_identity, snapshot_identity, LocalFixture},
    sideeffect::*,
    Verdict,
};
use verify_evidence::{canonical_bytes, canonical_hash, store::EvidenceStore};
struct Case {
    dir: tempfile::TempDir,
    contract: SideEffectContract,
}
impl Case {
    fn new(mode: &str, primitives: Vec<FaultPrimitive>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let fixture = dir.path().join("fixture");
        fs::create_dir(&fixture).unwrap();
        let db = Connection::open(fixture.join("ledger.db")).unwrap();
        db.execute_batch("CREATE TABLE ledger(seq INTEGER PRIMARY KEY AUTOINCREMENT, external_id TEXT NOT NULL UNIQUE, idem TEXT NOT NULL, correlation TEXT NOT NULL, operation TEXT NOT NULL);").unwrap();
        drop(db);
        let executable = Path::new(env!("CARGO_BIN_EXE_p5-effect-fixture")).to_path_buf();
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
                args: vec![],
                environment: BTreeMap::from([
                    ("MODE".into(), mode.into()),
                    ("KEY".into(), "key".into()),
                    ("CORRELATION".into(), "correlation".into()),
                    ("HOLD".into(), "1".into()),
                ]),
                fixture: LocalFixture {
                    source: Some(fixture.clone()),
                    snapshot_identity: snapshot_identity(Some(&fixture)).unwrap(),
                },
                timeout_ms: 3000,
            },
            effects: vec![EffectContract {
                effect_id: "payment".into(),
                provider: "local_provider".into(),
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
                schedule_id: "scenario".into(),
                primitives,
            }],
            exploration_budget: ExplorationBudget {
                max_schedules: 1,
                max_attempts: 16,
                reduction_executions: 0,
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
    fn run(&self) -> SideEffectExperimentResult {
        execute(&self.contract, &self.store(), "proof").unwrap()
    }
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.dir.path().join("runs"))
    }
    fn path(&self, id: &str) -> std::path::PathBuf {
        self.dir.path().join("runs").join(id).join("result.json")
    }
    fn refresh(&mut self) {
        self.contract.trigger.fixture.snapshot_identity =
            snapshot_identity(self.contract.trigger.fixture.source.as_deref()).unwrap();
    }
    fn operations(&mut self, value: &str) {
        self.contract
            .trigger
            .environment
            .insert("OPERATIONS".into(), value.into());
    }
    fn fast(&mut self) {
        self.contract
            .trigger
            .environment
            .insert("HOLD".into(), "0".into());
    }
    fn second(&mut self) {
        let mut e = self.contract.effects[0].clone();
        e.effect_id = "email".into();
        e.operation = "email".into();
        e.expectation = Expectation::AtMostOnce;
        self.contract.effects.push(e);
    }
}
fn retry() -> Vec<FaultPrimitive> {
    vec![FaultPrimitive::None, FaultPrimitive::Retry]
}
fn kill() -> Vec<FaultPrimitive> {
    vec![FaultPrimitive::KillAfterCommit, FaultPrimitive::Retry]
}
fn edit(path: &Path, f: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    f(&mut value["manifest"]["result"]);
    value["integrity_hash"] = canonical_hash(&value["manifest"]).unwrap().into();
    fs::write(path, canonical_bytes(&value).unwrap()).unwrap();
}
#[test]
fn s1_safe_normal() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Pass);
    assert_eq!(r.schedules[0].committed.len(), 1);
}
#[test]
fn s2_safe_retry_two_attempts_one_commit_and_repeat_observation() {
    let mut c = Case::new("safe", retry());
    c.fast();
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Pass);
    assert_eq!(r.schedules[0].attempts.len(), 2);
    assert_eq!(r.schedules[0].committed.len(), 1);
    assert_eq!(
        r.schedules[0]
            .history
            .events
            .iter()
            .filter(|e| e.kind == HistoryKind::EffectObservedCommitted)
            .count(),
        2
    );
}
#[test]
fn s3_safe_commit_kill_retry() {
    let c = Case::new("safe", kill());
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Pass);
    assert!(r.schedules[0].complete);
    assert_eq!(r.schedules[0].committed.len(), 1);
    let kinds = r.schedules[0]
        .history
        .events
        .iter()
        .map(|e| e.kind)
        .collect::<Vec<_>>();
    assert!(
        kinds
            .iter()
            .position(|e| *e == HistoryKind::EffectObservedCommitted)
            .unwrap()
            < kinds
                .iter()
                .position(|e| *e == HistoryKind::TargetKilled)
                .unwrap()
    );
}
#[test]
fn u1_unsafe_retry_distinct_committed_ids() {
    let mut c = Case::new("unsafe", retry());
    c.fast();
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Fail);
    assert_eq!(r.schedules[0].committed.len(), 2);
    assert_ne!(
        r.schedules[0].committed[0].identity.external_effect_id,
        r.schedules[0].committed[1].identity.external_effect_id
    );
}
#[test]
fn u2_unsafe_duplicate_delivery() {
    let mut c = Case::new(
        "unsafe",
        vec![FaultPrimitive::None, FaultPrimitive::DuplicateDelivery],
    );
    c.fast();
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Fail);
    assert!(r.schedules[0]
        .executed_faults
        .iter()
        .any(|e| e.kind == HistoryKind::DuplicateDeliveryStarted));
}
#[test]
fn u3_unsafe_commit_kill_retry() {
    let c = Case::new("unsafe", kill());
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Fail);
    assert_eq!(r.schedules[0].committed.len(), 2);
    assert!(r.schedules[0]
        .executed_faults
        .iter()
        .any(|e| e.kind == HistoryKind::TargetKilled));
}
#[test]
fn one_attempt_two_commits_is_fail() {
    let mut c = Case::new("double", vec![FaultPrimitive::None]);
    c.fast();
    assert_eq!(c.run().verdict, Verdict::Fail);
}
#[test]
fn n1_unknown_commits_with_two_attempts_is_inconclusive() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.contract.required_observers[0].db_path = "missing.db".into();
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert_eq!(r.schedules[0].attempts.len(), 2);
}
#[test]
fn n2_kill_without_commit_cannot_succeed() {
    let mut c = Case::new("safe", kill());
    c.fast();
    c.operations("");
    c.contract.effects[0].expectation = Expectation::AtMostOnce;
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert!(!r.schedules[0].complete);
    assert!(!r.schedules[0]
        .executed_faults
        .iter()
        .any(|e| e.kind == HistoryKind::TargetKilled));
}
#[test]
fn observer_schema_mismatch_is_error() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.contract.required_observers[0].external_effect_id_column = "missing".into();
    assert_eq!(c.run().verdict, Verdict::Error);
}
#[test]
fn n3_corrupt_database_is_error() {
    let mut c = Case::new("safe", retry());
    c.fast();
    fs::write(
        c.contract
            .trigger
            .fixture
            .source
            .as_ref()
            .unwrap()
            .join("ledger.db"),
        b"broken SQLite",
    )
    .unwrap();
    c.refresh();
    assert_eq!(c.run().verdict, Verdict::Error);
}
#[test]
fn missing_or_wrong_effect_identity_is_not_pass() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.contract
        .trigger
        .environment
        .insert("KEY".into(), "wrong".into());
    assert_eq!(c.run().verdict, Verdict::Error);
}
#[test]
fn retry_budget_does_not_become_success() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.contract.exploration_budget.max_attempts = 1;
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert_eq!(r.schedules[0].attempts.len(), 1);
    assert!(r.schedules[0].executed_faults.is_empty());
}
#[test]
fn schedule_budget_and_zero_budget_are_inconclusive() {
    for budget in [0, 1] {
        let mut c = Case::new("safe", retry());
        c.fast();
        let mut s = c.contract.fault_schedules[0].clone();
        s.schedule_id = "second".into();
        c.contract.fault_schedules.push(s);
        c.contract.exploration_budget.max_schedules = budget;
        assert_eq!(c.run().verdict, Verdict::Inconclusive);
    }
}
#[test]
fn ordered_after_uses_committed_sequence() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    c.second();
    c.operations("payment,email");
    c.contract.relations.push(Relation::OrderedAfter {
        effect: "payment".into(),
        after: "email".into(),
    });
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Fail);
    assert_eq!(r.violations[0].kind, ViolationKind::CommitOrder);
}
#[test]
fn ordered_after_safe_order_passes() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    c.second();
    c.operations("email,payment");
    c.contract.relations.push(Relation::OrderedAfter {
        effect: "payment".into(),
        after: "email".into(),
    });
    assert_eq!(c.run().verdict, Verdict::Pass);
}
#[test]
fn ordered_after_cannot_pass_same_commit_through_observer_aliases() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    let mut observer = c.contract.required_observers[0].clone();
    observer.observer_id = "ledger_alias".into();
    c.contract.required_observers.push(observer);
    let mut effect = c.contract.effects[0].clone();
    effect.effect_id = "payment_alias".into();
    effect.authoritative_observer = "ledger_alias".into();
    c.contract.effects.push(effect);
    c.contract.relations.push(Relation::OrderedAfter {
        effect: "payment_alias".into(),
        after: "payment".into(),
    });

    // A real process commits one SQLite row. execute returns the verified loader's
    // result; no history, runtime, snapshot, or hash is modified by this test.
    let r = c.run();
    let committed = &r.schedules[0].committed;
    assert_eq!(committed.len(), 2);
    assert_eq!(committed[0].identity, committed[1].identity);
    assert_eq!(committed[0].identity.commit_order, 1);
    assert_ne!(
        r.verdict,
        Verdict::Pass,
        "ordered_after requires a strictly later commit; an observer alias cannot make a commit follow itself"
    );
}
#[test]
fn atomic_with_half_commit_is_fail() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    c.second();
    c.contract.relations.push(Relation::AtomicWith {
        effect: "payment".into(),
        other: "email".into(),
    });
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Fail);
    assert_eq!(r.violations[0].kind, ViolationKind::HalfCommit);
}
#[test]
fn atomic_with_missing_observer_cannot_claim_half_commit() {
    let mut c = Case::new("safe", vec![FaultPrimitive::None]);
    c.fast();
    c.second();
    c.contract.required_observers[0].db_path = "missing.db".into();
    c.contract.relations.push(Relation::AtomicWith {
        effect: "payment".into(),
        other: "email".into(),
    });
    assert_eq!(c.run().verdict, Verdict::Inconclusive);
}
#[test]
fn effect_semantics_use_observed_zero_or_presence() {
    for (expectation, ops, want) in [
        (Expectation::ExactlyOnce, "", Verdict::Fail),
        (Expectation::AtMostOnce, "", Verdict::Pass),
        (Expectation::AtLeastOnce, "", Verdict::Fail),
        (Expectation::Never, "", Verdict::Pass),
        (Expectation::Never, "payment", Verdict::Fail),
        (Expectation::AtLeastOnce, "payment", Verdict::Pass),
    ] {
        let mut c = Case::new("safe", vec![FaultPrimitive::None]);
        c.fast();
        c.operations(ops);
        c.contract.effects[0].expectation = expectation;
        assert_eq!(c.run().verdict, want);
    }
}
#[test]
fn nonzero_target_exit_is_not_infrastructure_error() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.contract
        .trigger
        .environment
        .insert("EXIT".into(), "7".into());
    assert_eq!(c.run().verdict, Verdict::Pass);
}
#[test]
fn unsupported_semantics_and_fault_config_rejected() {
    let c = Case::new("safe", retry());
    let mut json = serde_json::to_value(c.contract).unwrap();
    json["effects"][0]["expectation"] = "eventually_within".into();
    assert!(serde_json::from_value::<SideEffectContract>(json).is_err());
    let c = Case::new("safe", vec![FaultPrimitive::Retry]);
    assert!(execute(&c.contract, &c.store(), "proof").is_err());
}
#[test]
fn output_verdict_rehashed_is_recomputed_for_pass_and_fail() {
    for mode in ["safe", "unsafe"] {
        let mut c = Case::new(mode, retry());
        c.fast();
        let r = c.run();
        edit(&c.path("proof"), |v| {
            v["verdict"] = if mode == "safe" { "FAIL" } else { "PASS" }.into();
            v["violations"] = serde_json::json!([]);
        });
        assert_eq!(load(&c.store(), "proof").unwrap().verdict, r.verdict);
    }
}
#[test]
fn history_and_child_corruption_are_rejected() {
    for target in ["history", "child", "evidence"] {
        let mut c = Case::new("safe", retry());
        c.fast();
        c.run();
        match target {
            "history" => edit(&c.path("proof"), |v| {
                v["schedules"][0]["history"]["events"]
                    .as_array_mut()
                    .unwrap()
                    .remove(2);
            }),
            "child" => edit(&c.path("proof-s0-a0"), |v| {
                v["runtime"]["observation"]["exit_code"] = serde_json::json!(8)
            }),
            _ => fs::write(
                c.dir
                    .path()
                    .join("runs/proof-s0-a0/evidence/committed.json"),
                b"{}",
            )
            .unwrap(),
        };
        assert!(load(&c.store(), "proof").is_err());
    }
}
#[test]
fn cross_run_attempt_evidence_is_rejected() {
    let mut c = Case::new("safe", retry());
    c.fast();
    let r = c.run();
    let other = execute(&c.contract, &c.store(), "other").unwrap();
    edit(&c.path("proof"), |v| {
        v["schedules"][0]["attempts"][0] =
            serde_json::to_value(&other.schedules[0].attempts[0]).unwrap()
    });
    assert!(load(&c.store(), "proof").is_err());
    assert_eq!(r.verdict, Verdict::Pass);
}
#[test]
fn fabricated_precommit_kill_not_verified() {
    let mut c = Case::new("safe", kill());
    c.fast();
    c.operations("");
    c.run();
    edit(&c.path("proof-s0-a0"), |v| {
        v["runtime"]["termination_requested"] = true.into();
        v["runtime"]["observation"]["signal"] = 9.into();
    });
    assert!(load(&c.store(), "proof").is_err());
}
#[test]
fn fail_reproduction_actually_reruns_and_preserves_signature() {
    let mut c = Case::new(
        "unsafe",
        vec![
            FaultPrimitive::None,
            FaultPrimitive::Retry,
            FaultPrimitive::Retry,
        ],
    );
    c.fast();
    c.contract.exploration_budget.reduction_executions = 10;
    let r = c.run();
    let x = r.counterexample.unwrap();
    assert_eq!(x.status, ReductionStatus::LocallyMinimized);
    assert_eq!(x.retained_schedule.primitives.len(), 2);
    let child = load(&c.store(), &x.reproduction.unwrap().artifact_id).unwrap();
    assert_eq!(child.verdict, Verdict::Fail);
    assert!(child
        .violations
        .iter()
        .any(|v| v.kind == x.signature.kind && v.effect_ids == x.signature.effect_ids));
}
#[test]
fn reduction_budget_exhaustion_cannot_claim_minimality() {
    let mut c = Case::new("unsafe", retry());
    c.fast();
    c.contract.exploration_budget.reduction_executions = 1;
    let x = c.run().counterexample.unwrap();
    assert_eq!(x.status, ReductionStatus::BudgetExhausted);
    assert!(x.reproduction.is_some());
}
#[test]
fn nonoverwrite_and_fixture_identity_are_enforced() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.run();
    assert!(execute(&c.contract, &c.store(), "proof").is_err());
    fs::write(
        c.contract
            .trigger
            .fixture
            .source
            .as_ref()
            .unwrap()
            .join("changed"),
        b"changed",
    )
    .unwrap();
    assert!(execute(&c.contract, &c.store(), "next").is_err());
}
#[test]
fn baseline_poisoning_is_error() {
    let mut c = Case::new("safe", retry());
    c.fast();
    Connection::open(
        c.contract
            .trigger
            .fixture
            .source
            .as_ref()
            .unwrap()
            .join("ledger.db"),
    )
    .unwrap()
    .execute(
        "INSERT INTO ledger VALUES(1,'old','key','correlation','payment')",
        [],
    )
    .unwrap();
    c.refresh();
    assert_eq!(c.run().verdict, Verdict::Error);
}
#[test]
fn schemas_cover_real_contract_result_and_history() {
    let mut c = Case::new("safe", retry());
    c.fast();
    let r = c.run();
    for (schema, v) in [
        (
            contract_schema(),
            serde_json::to_value(&r.contract).unwrap(),
        ),
        (result_schema(), serde_json::to_value(&r).unwrap()),
        (
            history_schema(),
            serde_json::to_value(&r.schedules[0].history).unwrap(),
        ),
    ] {
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&v));
    }
}

// Corruption tests also repair every affected hash and evidence reference, so
// failures below exercise semantic verification rather than just checksum checks.
fn amend_evidence(c: &Case, id: &str, name: &str, change: impl FnOnce(&mut serde_json::Value)) {
    let file = c
        .dir
        .path()
        .join("runs")
        .join(id)
        .join("evidence")
        .join(format!("{name}.json"));
    let mut e: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    change(&mut e["observation"]["value"]);
    e["integrity_hash"] = canonical_hash(&e["observation"]).unwrap().into();
    fs::write(file, canonical_bytes(&e).unwrap()).unwrap();
    let path = c.path(id);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["manifest"]["evidence_hashes"][name] = canonical_hash(&e).unwrap().into();
    manifest["integrity_hash"] = canonical_hash(&manifest["manifest"]).unwrap().into();
    fs::write(path, canonical_bytes(&manifest).unwrap()).unwrap();
}
fn child_value(c: &Case) -> serde_json::Value {
    let v: serde_json::Value =
        serde_json::from_slice(&fs::read(c.path("proof-s0-a0")).unwrap()).unwrap();
    v["manifest"]["result"].clone()
}
fn bind_modified_child(c: &Case) {
    let value = child_value(c);
    let hash = canonical_hash(&value).unwrap();
    edit(&c.path("proof"), |v| {
        v["schedules"][0]["attempts"][0]["integrity_hash"] = hash.clone().into()
    });
    amend_evidence(c, "proof", "execution", |v| {
        v[0][0][0]["integrity_hash"] = hash.into()
    });
}
#[test]
fn rehashed_forged_committed_rows_cannot_replace_sqlite_evidence() {
    let mut c = Case::new("unsafe", retry());
    c.fast();
    c.run();
    edit(&c.path("proof-s0-a0"), |v| {
        v["after"][0]["rows"] = serde_json::json!([])
    });
    let after = child_value(&c)["after"].clone();
    amend_evidence(&c, "proof-s0-a0", "committed", |v| v[1] = after);
    bind_modified_child(&c);
    assert!(load(&c.store(), "proof")
        .unwrap_err()
        .to_string()
        .contains("SQLite backup"));
}
#[test]
fn rehashed_sqlite_corruption_is_detected_by_actual_database_read() {
    let mut c = Case::new("safe", retry());
    c.fast();
    c.run();
    edit(&c.path("proof-s0-a0"), |v| {
        v["after"][0]["database"] = serde_json::json!([0, 1, 2, 3])
    });
    let after = child_value(&c)["after"].clone();
    amend_evidence(&c, "proof-s0-a0", "committed", |v| v[1] = after);
    bind_modified_child(&c);
    assert!(load(&c.store(), "proof").is_err());
}
#[test]
fn rehashed_history_omission_and_unexecuted_retry_still_rejected() {
    for history in [false, true] {
        let mut c = Case::new("safe", retry());
        c.fast();
        c.run();
        if history {
            edit(&c.path("proof"), |v| {
                v["schedules"][0]["history"]["events"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            });
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(c.path("proof")).unwrap()).unwrap();
            amend_evidence(&c, "proof", "history-0", |v| {
                *v = value["manifest"]["result"]["schedules"][0]["history"].clone()
            });
        } else {
            edit(&c.path("proof"), |v| {
                v["schedules"][0]["attempts"].as_array_mut().unwrap().pop();
            });
            amend_evidence(&c, "proof", "execution", |v| {
                v[0][0].as_array_mut().unwrap().pop();
            });
        }
        assert!(load(&c.store(), "proof").is_err());
    }
}
#[test]
fn rehashed_kill_before_commit_has_no_schedule_authority() {
    let mut c = Case::new("safe", kill());
    c.fast();
    c.operations("");
    c.run();
    edit(&c.path("proof-s0-a0"), |v| {
        v["runtime"]["termination_requested"] = true.into();
        v["runtime"]["observation"]["signal"] = 9.into();
    });
    let runtime = child_value(&c)["runtime"].clone();
    amend_evidence(&c, "proof-s0-a0", "process", |v| v[6] = runtime);
    bind_modified_child(&c);
    assert!(load(&c.store(), "proof")
        .unwrap_err()
        .to_string()
        .contains("pre-kill"));
}
#[test]
fn rehashed_minimality_claim_requires_completed_reduction_trials() {
    let mut c = Case::new("unsafe", retry());
    c.fast();
    c.contract.exploration_budget.reduction_executions = 1;
    c.run();
    edit(&c.path("proof"), |v| {
        v["counterexample"]["status"] = "LOCALLY_MINIMIZED".into()
    });
    amend_evidence(&c, "proof", "execution", |v| {
        v[1]["status"] = "LOCALLY_MINIMIZED".into()
    });
    assert!(load(&c.store(), "proof").is_err());
}
#[test]
fn full_safe_exploration_has_unique_event_ids_and_fresh_ledgers() {
    let mut c = Case::new("safe", retry());
    c.fast();
    let mut s = c.contract.fault_schedules[0].clone();
    s.schedule_id = "delivery".into();
    s.primitives = vec![FaultPrimitive::None, FaultPrimitive::DuplicateDelivery];
    c.contract.fault_schedules.push(s);
    c.contract.exploration_budget.max_schedules = 2;
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Pass);
    assert!(r.schedules.iter().all(|s| s.committed.len() == 1));
    let events = r
        .schedules
        .iter()
        .flat_map(|s| &s.history.events)
        .collect::<Vec<_>>();
    let unique = events
        .iter()
        .map(|e| &e.event_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), events.len());
}

#[test]
fn controller_request_after_target_exits_is_not_delivered_fault() {
    use verify_runner::process::{observe_controlled, ProcessSpec};
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("released");
    let spec = ProcessSpec {
        executable: "/bin/sh".into(),
        args: vec![
            "-c".into(),
            "while [ ! -f released ]; do :; done; exit 0".into(),
        ],
        working_directory: dir.path().into(),
        environment: Default::default(),
        timeout_ms: 3000,
    };
    let observation = observe_controlled(&spec, &mut || {
        fs::write(&marker, b"go")?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        Ok(true)
    });
    assert!(observation.termination_requested);
    assert!(!observation.termination_delivered);
    assert_eq!(observation.observation.exit_code, Some(0));
}
#[test]
fn timeout_termination_is_in_history_but_not_an_executed_scheduled_kill() {
    let mut c = Case::new("safe", kill());
    c.operations("");
    c.contract.trigger.timeout_ms = 50;
    let r = c.run();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert!(r.schedules[0]
        .history
        .events
        .iter()
        .any(|e| e.kind == HistoryKind::TargetKilled));
    assert!(!r.schedules[0]
        .executed_faults
        .iter()
        .any(|e| e.kind == HistoryKind::TargetKilled));
}
