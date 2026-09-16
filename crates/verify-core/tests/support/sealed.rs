use verify_core::{
    sealed_run::{Authorization, Execution},
    task_seal::TaskSeal,
};
use verify_evidence::canonical_hash;

// Trusted fixture intent is created separately from candidate/execution approval.
pub fn seal() -> TaskSeal {
    TaskSeal {
        schema_version: "1".into(),
        task_identity: canonical_hash(&"bounded public fixture task").unwrap(),
        verifier_identity: canonical_hash(&"deterministic fixture checker").unwrap(),
        environment_contract_identity: canonical_hash(&"local recorded runtime; no attestation")
            .unwrap(),
    }
}
pub fn approve(seal: &TaskSeal, execution: &Execution) -> Authorization {
    Authorization {
        schema_version: "1".into(),
        seal_commitment: seal.commitment().unwrap(),
        execution_identity: execution.identity().unwrap(),
        candidate_identity: execution.candidate_identity().unwrap(),
    }
}
