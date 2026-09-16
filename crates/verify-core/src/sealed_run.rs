//! V100 authoritative local boundary. Only verified product loaders assign verdicts.
//! Trusted controller pins are external inputs, never learned from stored receipts.
pub mod query;
use crate::{behavior, blindtest, sideeffect, task_seal::TaskSeal, Verdict};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io, path::Path};
use verify_evidence::{
    canonical_hash, store::EvidenceStore, valid_hash, Evidence, Observation, TrustClass,
};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn hash(domain: &str, value: impl Serialize) -> io::Result<String> {
    Ok(canonical_hash(&json!({"domain": domain, "value": value}))?)
}

/// Independent v1 execution description. Candidate-independent intent remains in TaskSeal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "product", rename_all = "snake_case", deny_unknown_fields)]
pub enum Execution {
    Behavior {
        experiment: Box<behavior::BehaviorExperiment>,
        authorization: behavior::BehaviorAuthorization,
    },
    Sideeffect {
        contract: Box<sideeffect::SideEffectContract>,
    },
    Blindtest {
        config: Box<blindtest::BlindTestConfig>,
    },
}
impl Execution {
    pub fn identity(&self) -> io::Result<String> {
        hash("b2ige.verify.execution.v1", self)
    }
    /// Content identity, not a revision label. BlindTest requires an immutable image ID.
    pub fn candidate_identity(&self) -> io::Result<String> {
        let (product, identity) = match self {
            Self::Behavior { experiment, .. } => (
                "behavior",
                json!({"executable": experiment.after.identity, "input": experiment.after.input_identity}),
            ),
            Self::Sideeffect { contract } => (
                "sideeffect",
                json!({"executable": contract.trigger.executable_hash}),
            ),
            Self::Blindtest { config } => {
                if !valid_hash(&config.target.image) {
                    return Err(invalid(
                        "sealed BlindTest requires an immutable sha256 image ID",
                    ));
                }
                (
                    "blindtest",
                    json!({"image": config.target.image, "workspace": config.target.workspace_hash, "build": config.target.build_identity}),
                )
            }
        };
        hash("b2ige.verify.candidate.v1", (product, identity))
    }
}

/// Retain outside candidate control, alongside the pre-candidate seal commitment.
/// Approval explicitly associates the later execution/candidate with the earlier seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authorization {
    pub schema_version: String,
    pub seal_commitment: String,
    pub execution_identity: String,
    pub candidate_identity: String,
}
impl Authorization {
    fn check(&self, seal: &TaskSeal, execution: &Execution) -> io::Result<()> {
        if self.schema_version != "1"
            || ![
                &self.seal_commitment,
                &self.execution_identity,
                &self.candidate_identity,
            ]
            .iter()
            .all(|s| valid_hash(s))
            || seal.commitment().map_err(invalid)? != self.seal_commitment
            || execution.identity()? != self.execution_identity
            || execution.candidate_identity()? != self.candidate_identity
        {
            return Err(invalid(
                "missing or mismatched sealed execution authorization",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityBundle {
    pub schema_version: String,
    pub seal_commitment: String,
    pub task_identity: String,
    pub verifier_identity: String,
    pub environment_contract_identity: String,
    pub candidate_identity: String,
    pub execution_identity: String,
    pub evidence_identity: String,
    /// Recorded runtime content only; None on runs with no runtime evidence.
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub runtime_identity: Option<String>,
    #[serde(deserialize_with = "verify_evidence::unique_map")]
    pub source_identities: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub source_run_id: String,
    pub seal: TaskSeal,
    pub execution: Execution,
    pub identities: IdentityBundle,
}
impl Receipt {
    pub fn commitment(&self) -> io::Result<String> {
        hash("b2ige.verify.receipt.v1", self)
    }
}
/// Cannot be deserialized or constructed from model output or cached verdicts.
pub struct VerifiedReceipt {
    receipt: Receipt,
    verdict: Verdict,
}
impl VerifiedReceipt {
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }
    pub fn commitment(&self) -> io::Result<String> {
        self.receipt.commitment()
    }
}

fn context(store: &EvidenceStore, id: &str) -> io::Result<Value> {
    let (raw, _) = store.load(id)?;
    let context = raw
        .get("context")
        .ok_or_else(|| invalid("missing runtime context"))?;
    let parsed: verify_runner::RunContext = serde_json::from_value(context.clone())?;
    if !parsed.valid() {
        return Err(invalid("invalid runtime context"));
    }
    Ok(context.clone())
}

fn verify_sources(
    store: &EvidenceStore,
    id: &str,
    seal: &TaskSeal,
    execution: &Execution,
) -> io::Result<(Verdict, IdentityBundle)> {
    let ((verdict, runtime), sources) = store.record_reads(|store| {
        let mut runtime = Vec::new();
        let verdict = match execution {
            Execution::Behavior { experiment, authorization } => {
                let r = behavior::load(store, id, authorization)?;
                if r.experiment != **experiment { return Err(invalid("sealed Behavior execution mismatch")); }
                for link in r.before.iter().chain(r.after.iter()) { runtime.push(context(store, &link.run_id)?); }
                r.verdict
            }
            Execution::Sideeffect { contract } => {
                let r = sideeffect::load(store, id)?;
                if r.contract != **contract { return Err(invalid("sealed SideEffect execution mismatch")); }
                for attempt in r.schedules.iter().flat_map(|s| &s.attempts) {
                    let (raw, _) = store.load(&attempt.artifact_id)?;
                    let run: sideeffect::AttemptRecord = serde_json::from_value(raw)?;
                    runtime.push(json!({"identity": run.identity, "process": run.process, "runtime": run.runtime}));
                }
                r.verdict
            }
            Execution::Blindtest { config } => {
                let r = blindtest::load(store, id)?;
                if r.config != **config { return Err(invalid("sealed BlindTest execution mismatch")); }
                if let Some(image) = &r.image {
                    if image.image_id != config.target.image { return Err(invalid("sealed runtime image mismatch")); }
                    runtime.push(json!({"image": image, "executions": r.executions}));
                }
                r.verdict
            }
        };
        if verdict == Verdict::Pass && runtime.is_empty() { return Err(invalid("missing verdict-critical runtime identity")); }
        Ok((verdict, runtime))
    })?;
    if sources.is_empty() {
        return Err(invalid("missing verdict-critical source identity"));
    }
    let bundle = IdentityBundle {
        schema_version: "1".into(),
        seal_commitment: seal.commitment().map_err(invalid)?,
        task_identity: seal.task_identity.clone(),
        verifier_identity: seal.verifier_identity.clone(),
        environment_contract_identity: seal.environment_contract_identity.clone(),
        candidate_identity: execution.candidate_identity()?,
        execution_identity: execution.identity()?,
        evidence_identity: hash("b2ige.verify.evidence-inventory.v1", &sources)?,
        runtime_identity: if runtime.is_empty() {
            None
        } else {
            Some(hash("b2ige.verify.runtime.v1", runtime)?)
        },
        source_identities: sources,
    };
    Ok((verdict, bundle))
}
fn evidence(receipt: &Receipt) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(receipt)?,
    };
    Ok(Evidence {
        evidence_id: "identity".into(),
        run_id: receipt.receipt_id.clone(),
        source: "sealed_controller".into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec!["v100.sealed_identity.v1".into()],
    })
}

/// Validates trusted bindings before reserving or executing. `sealed_root` is used
/// only for BlindTest and must remain on the trusted controller's private surface.
/// Store failures return ERROR-boundary io::Errors, never product FAIL or PASS.
pub fn execute(
    store: &EvidenceStore,
    receipt_id: &str,
    seal: &TaskSeal,
    authorization: &Authorization,
    execution: &Execution,
    workspace: &Path,
    sealed_root: Option<&Path>,
) -> io::Result<VerifiedReceipt> {
    authorization.check(seal, execution)?;
    if receipt_id.len() > 114 {
        return Err(invalid("sealed receipt ID exceeds derived run limit"));
    }
    // Enforce the existing P6 secrecy boundary before writing even a partial receipt.
    if let Execution::Blindtest { config } = execution {
        let sealed = sealed_root.ok_or_else(|| invalid("sealed root required"))?;
        blindtest::check_paths(&config.target.workspace, sealed, store.root())?;
    }
    let reserved = store.reserve(receipt_id)?;
    let source_run_id = format!("{receipt_id}-source");
    match execution {
        Execution::Behavior {
            experiment,
            authorization,
        } => {
            behavior::execute(store, workspace, &source_run_id, experiment, authorization)?;
        }
        Execution::Sideeffect { contract } => {
            sideeffect::execute(contract, store, &source_run_id)?;
        }
        Execution::Blindtest { config } => {
            blindtest::execute(
                config,
                sealed_root.ok_or_else(|| invalid("sealed root required"))?,
                store,
                &source_run_id,
            )?;
        }
    }
    let (verdict, identities) = verify_sources(store, &source_run_id, seal, execution)?;
    let receipt = Receipt {
        schema_version: "1".into(),
        receipt_id: receipt_id.into(),
        source_run_id,
        seal: seal.clone(),
        execution: execution.clone(),
        identities,
    };
    let item = evidence(&receipt)?;
    reserved.write_evidence(&item)?;
    reserved.complete(&receipt, &[item.evidence_id])?;
    // Read the published receipt and all original evidence before exposing a verdict.
    let loaded = load(store, receipt_id, authorization, &receipt.commitment()?)?;
    if loaded.verdict != verdict {
        return Err(invalid("verdict changed during receipt publication"));
    }
    Ok(loaded)
}

/// The caller must retain both authorization and receipt commitment independently.
/// Never fall back to legacy loaders when this boundary rejects a receipt.
pub fn load(
    store: &EvidenceStore,
    id: &str,
    authorization: &Authorization,
    expected_commitment: &str,
) -> io::Result<VerifiedReceipt> {
    if !valid_hash(expected_commitment) {
        return Err(invalid("missing retained receipt commitment"));
    }
    let (raw, items) = store.load(id)?;
    let receipt: Receipt = serde_json::from_value(raw)?;
    authorization.check(&receipt.seal, &receipt.execution)?;
    if receipt.schema_version != "1"
        || receipt.receipt_id != id
        || receipt.source_run_id != format!("{id}-source")
        || receipt.commitment()? != expected_commitment
        || items != [evidence(&receipt)?]
    {
        return Err(invalid("invalid sealed receipt identity/integrity"));
    }
    let (verdict, identities) = verify_sources(
        store,
        &receipt.source_run_id,
        &receipt.seal,
        &receipt.execution,
    )?;
    if receipt.identities != identities {
        return Err(invalid("sealed source/runtime/evidence identity mismatch"));
    }
    Ok(VerifiedReceipt { receipt, verdict })
}
