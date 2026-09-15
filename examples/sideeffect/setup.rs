use super::*;
use verify_core::{
    behavior::{executable_identity, snapshot_identity},
    sideeffect::SideEffectContract,
};
pub fn setup(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(root)?;
    let fixture = root.join("fixture");
    fs::create_dir(&fixture)?;
    rusqlite::Connection::open(fixture.join("ledger.db"))?.execute_batch("CREATE TABLE ledger(seq INTEGER PRIMARY KEY AUTOINCREMENT,external_id TEXT NOT NULL UNIQUE,idem TEXT NOT NULL,correlation TEXT NOT NULL,operation TEXT NOT NULL);")?;
    let executable = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("b2ige-demo-effect");
    let target_hash = executable_identity(&executable)?;
    for (id, mode) in [("safe", "safe"), ("unsafe", "unsafe")] {
        let schedule = vec!["NONE", "RETRY"];
        let contract: SideEffectContract = serde_json::from_value(json!({
            "contract_id":"checkout","schema_version":"1",
            "operation":{"operation_id":"checkout","idempotency_identity":"checkout_1","correlation_identity":"checkout_1"},
            "trigger":{"executable":executable,"executable_hash":target_hash,"args":[],"environment":{"MODE":mode,"KEY":"checkout_1","CORRELATION":"checkout_1","HOLD":"0","OPERATIONS":"payment"},"fixture":{"source":fixture,"snapshot_identity":snapshot_identity(Some(&fixture))?},"timeout_ms":3000},
            "effects":[{"effect_id":"payment","provider":"local_provider","adapter":"sqlite","operation":"payment","identity":{"external_identity":"provider_operation_external_id","idempotency_identity":"checkout_1","correlation_identity":"checkout_1"},"expectation":"exactly_once","authoritative_observer":"ledger"}],
            "relations":[],"fault_schedules":[{"schedule_id":id,"primitives":schedule}],"exploration_budget":{"max_schedules":1,"max_attempts":8,"reduction_executions":8},
            "required_observers":[{"observer_id":"ledger","db_path":"ledger.db","table":"ledger","external_effect_id_column":"external_id","idempotency_column":"idem","correlation_column":"correlation","operation_column":"operation","commit_order_column":"seq","authoritative_source":"durable_append_only_committed_state"}]
        }))?;
        fs::write(
            root.join(format!("{id}.contract.json")),
            serde_json::to_vec_pretty(&contract)?,
        )?;
    }
    Ok(())
}
