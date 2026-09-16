//! Optional controller-owned admission boundary for repeated hidden-suite queries.
//! Never expose the ledger or bypass execution APIs to the candidate.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: String,
    pub seal_commitment: String,
    pub suite_hash: String,
    /// Total attempts across candidates and run IDs; zero denies all queries.
    pub max_attempts: u32,
}
impl Policy {
    pub fn commitment(&self) -> io::Result<String> {
        if self.schema_version != "1"
            || !valid_hash(&self.seal_commitment)
            || !valid_hash(&self.suite_hash)
            || self.max_attempts > 1024
        {
            return Err(invalid("invalid hidden query policy"));
        }
        hash("b2ige.verify.hidden-query-policy.v1", self)
    }
}

/// One persistent ledger per controller-approved task/suite, shared across candidates.
/// The controller retains both this path and the policy commitment independently.
pub struct Ledger {
    store: EvidenceStore,
    policy: Policy,
}
fn item(id: &str, value: Value) -> io::Result<Evidence> {
    let observation = Observation::Value { value };
    Ok(Evidence {
        evidence_id: "admission".into(),
        run_id: id.into(),
        source: "query_controller".into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec!["v100.hidden_query.v1".into()],
    })
}
impl Ledger {
    /// Explicit controller setup only. Never initializes or repairs an existing ledger.
    pub fn initialize(
        root: &Path,
        workspace: &Path,
        sealed: &Path,
        policy: &Policy,
    ) -> io::Result<Self> {
        let pin = policy.commitment()?;
        blindtest::check_paths(workspace, sealed, root)?;
        // Exclusive root creation prevents treating a lost policy as a fresh budget.
        std::fs::create_dir(root)?;
        let store = EvidenceStore::new(root);
        let run = store.reserve("policy")?;
        let evidence = item("policy", serde_json::to_value(policy)?)?;
        run.write_evidence(&evidence)?;
        run.complete(policy, &[evidence.evidence_id])?;
        Self::open(root, &pin)
    }

    pub fn open(root: &Path, retained_policy_commitment: &str) -> io::Result<Self> {
        let store = EvidenceStore::new(root);
        let (raw, evidence) = store.load("policy")?;
        let policy: Policy = serde_json::from_value(raw.clone())?;
        if policy.commitment()? != retained_policy_commitment || evidence != [item("policy", raw)?]
        {
            return Err(invalid("hidden query policy integrity mismatch"));
        }
        Ok(Self { store, policy })
    }

    /// Reserves a durable slot before any target execution. Errors, crashes, partial
    /// executions and repeated receipt IDs consume slots; there are no refunds.
    /// Reloading an existing receipt does not execute or consume another query.
    pub fn execute(
        &self,
        store: &EvidenceStore,
        receipt_id: &str,
        seal: &TaskSeal,
        authorization: &Authorization,
        execution: &Execution,
        sealed: &Path,
    ) -> io::Result<VerifiedReceipt> {
        authorization.check(seal, execution)?;
        let Execution::Blindtest { config } = execution else {
            return Err(invalid("query admission requires BlindTest"));
        };
        if seal.commitment().map_err(invalid)? != self.policy.seal_commitment
            || config.suite_hash != self.policy.suite_hash
        {
            return Err(invalid("hidden query scope mismatch"));
        }
        blindtest::check_paths(&config.target.workspace, sealed, self.store.root())?;
        blindtest::check_paths(&config.target.workspace, sealed, store.root())?;
        // Recheck on every admission, including handles opened before tampering.
        Self::open(self.store.root(), &self.policy.commitment()?)?;
        for slot in 0..self.policy.max_attempts {
            let id = format!("attempt-{slot}");
            let run = match self.store.reserve(&id) {
                Ok(run) => run,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            };
            let value = json!({"schema_version":"1", "policy_commitment":self.policy.commitment()?,
                "slot":slot, "receipt_id":receipt_id, "authorization":authorization});
            let evidence = item(&id, value.clone())?;
            run.write_evidence(&evidence)?;
            run.complete(&value, &[evidence.evidence_id])?;
            return super::execute(
                store,
                receipt_id,
                seal,
                authorization,
                execution,
                &config.target.workspace,
                Some(sealed),
            );
        }
        Err(invalid("hidden query budget exhausted"))
    }
}
