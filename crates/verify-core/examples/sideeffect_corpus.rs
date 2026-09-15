//! Reproducible, paid-API-free corpus. Writes inputs, immutable evidence and summary.
//! cargo run -p verify-core --example sideeffect_corpus -- /absolute/output /absolute/p5-effect-fixture
use serde_json::json;
use std::{fs, path::PathBuf};
use verify_core::{
    behavior::{executable_identity, snapshot_identity},
    sideeffect::{execute, SideEffectContract},
    Verdict,
};
use verify_evidence::store::EvidenceStore;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("expected absolute output directory and p5-effect-fixture executable".into());
    }
    let root = PathBuf::from(&args[1]);
    let executable = PathBuf::from(&args[2]);
    if !root.is_absolute() || !executable.is_absolute() {
        return Err("paths must be absolute".into());
    }
    fs::create_dir(&root)?;
    let fixture = root.join("fixture");
    fs::create_dir(&fixture)?;
    rusqlite::Connection::open(fixture.join("ledger.db"))?.execute_batch("CREATE TABLE ledger(seq INTEGER PRIMARY KEY AUTOINCREMENT,external_id TEXT NOT NULL UNIQUE,idem TEXT NOT NULL,correlation TEXT NOT NULL,operation TEXT NOT NULL);")?;
    let corrupt = root.join("corrupt");
    fs::create_dir(&corrupt)?;
    fs::write(corrupt.join("ledger.db"), b"not SQLite")?;
    let target_hash = executable_identity(&executable)?;
    let store = EvidenceStore::new(root.join("runs"));
    let mut results = vec![];
    for (id, mode, schedule, expected) in [
        ("S1", "safe", vec!["NONE"], Verdict::Pass),
        ("S2", "safe", vec!["NONE", "RETRY"], Verdict::Pass),
        (
            "S3",
            "safe",
            vec!["KILL_AFTER_COMMIT", "RETRY"],
            Verdict::Pass,
        ),
        ("U1", "unsafe", vec!["NONE", "RETRY"], Verdict::Fail),
        (
            "U2",
            "unsafe",
            vec!["NONE", "DUPLICATE_DELIVERY"],
            Verdict::Fail,
        ),
        (
            "U3",
            "unsafe",
            vec!["KILL_AFTER_COMMIT", "RETRY"],
            Verdict::Fail,
        ),
        ("N1", "safe", vec!["NONE", "RETRY"], Verdict::Inconclusive),
        (
            "N2",
            "safe",
            vec!["KILL_AFTER_COMMIT", "RETRY"],
            Verdict::Inconclusive,
        ),
        ("N3", "safe", vec!["NONE"], Verdict::Error),
    ] {
        let source = if id == "N3" { &corrupt } else { &fixture };
        let contract: SideEffectContract = serde_json::from_value(json!({
            "contract_id":"checkout","schema_version":"1",
            "operation":{"operation_id":"checkout","idempotency_identity":"checkout_1","correlation_identity":"checkout_1"},
            "trigger":{"executable":executable,"executable_hash":target_hash,"args":[],"environment":{"MODE":mode,"KEY":"checkout_1","CORRELATION":"checkout_1","HOLD":if schedule[0]=="KILL_AFTER_COMMIT" && id!="N2" {"1"}else{"0"},"OPERATIONS":if id=="N2"{""}else{"payment"}},"fixture":{"source":source,"snapshot_identity":snapshot_identity(Some(source))?},"timeout_ms":3000},
            "effects":[{"effect_id":"payment","provider":"local_provider","adapter":"sqlite","operation":"payment","identity":{"external_identity":"provider_operation_external_id","idempotency_identity":"checkout_1","correlation_identity":"checkout_1"},"expectation":"exactly_once","authoritative_observer":"ledger"}],
            "relations":[],"fault_schedules":[{"schedule_id":id,"primitives":schedule}],"exploration_budget":{"max_schedules":1,"max_attempts":8,"reduction_executions":8},
            "required_observers":[{"observer_id":"ledger","db_path":if id=="N1"{"missing.db"}else{"ledger.db"},"table":"ledger","external_effect_id_column":"external_id","idempotency_column":"idem","correlation_column":"correlation","operation_column":"operation","commit_order_column":"seq","authoritative_source":"durable_append_only_committed_state"}]
        }))?;
        fs::write(
            root.join(format!("{id}.contract.json")),
            serde_json::to_vec_pretty(&contract)?,
        )?;
        let r = execute(&contract, &store, id)?;
        if r.verdict != expected {
            return Err(format!("{id}: expected {expected:?}, observed {:?}", r.verdict).into());
        }
        results.push(json!({"id":id,"expected":expected,"verdict":r.verdict,"attempts":r.schedules[0].attempts.len(),"committed":if id=="N1"||id=="N3"{None}else{Some(r.schedules[0].committed.len())},"schedule_executed":r.schedules[0].complete,"reduction_status":r.counterexample.as_ref().map(|x|x.status)}));
    }
    let summary = json!({"corpus":results,"known_unsafe_false_pass":0,"missing_committed_evidence_pass":0,"unexecuted_fault_pass":0,"safe_false_fail":0});
    fs::write(
        root.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
