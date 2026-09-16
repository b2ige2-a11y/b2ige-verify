//! P1A synthetic contract harness, not a production verifier.
//!
//! `TrustedPolicy`, run observations and isolation assessment must come from a trusted
//! verifier boundary. JSON and hashes provide structure/integrity, not authentication.
//! No target/LLM may supply this policy or label its own evidence as authoritative.
pub mod acquisition;
pub mod behavior;
pub mod blindtest;
mod contracts;
pub mod sealed_run;
pub mod sideeffect;
pub mod task_seal;
pub use contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use verify_evidence::*;
use verify_replay::{ReplayInputs, Replayability};
use verify_runner::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
    Error,
}
impl Verdict {
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
            Self::Inconclusive => 2,
            Self::Error => 3,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub contract_ids: Vec<String>,
    #[serde(deserialize_with = "unique_map")]
    pub exploration_budget: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Equals {
        expected: serde_json::Value,
    },
    CommittedCount {
        provider: String,
        operation: String,
        idempotency_identity: String,
        expected: u64,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,
    pub observer: String,
    pub accepted_trust: Vec<TrustClass>,
    pub predicate: Predicate,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "product", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductContract {
    Behavior { baseline: Baseline, stable: bool },
    Sideeffect { effect: EffectContract },
    Blindtest { invariants: Vec<Invariant> },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentPlan {
    pub plan_id: String,
    pub version: String,
    pub target_revision: String,
    pub seed: u64,
    pub required_observers: Vec<String>,
    pub optional_observers: Vec<String>,
    pub reset_strategy: String,
    pub time_budget_ms: u64,
    pub scope: Scope,
    pub claims: Vec<Claim>,
    pub isolation: IsolationLevel,
    pub product_contract: ProductContract,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrustedPolicy {
    pub plan: ExperimentPlan,
    pub config: serde_json::Value,
    /// Canonical hashes pinned by human/policy outside the coding-agent workspace.
    /// No API automatically adds candidate artifacts to these authorization sets.
    pub approved_baselines: BTreeSet<String>,
    pub approved_invariants: BTreeSet<String>,
    /// Independently approved bindings between an artifact and its executable claim.
    /// Missing bindings fail closed; evaluation never creates or updates approvals.
    #[serde(default)]
    pub approved_checker_bindings: BTreeSet<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub context: RunContext,
    pub execution: ExecutionStatus,
    pub isolation: IsolationAssessment,
    #[serde(deserialize_with = "unique_map")]
    pub coverage: BTreeMap<String, Coverage>,
    pub completed_checkers: BTreeSet<String>,
    pub evidence: Vec<Evidence>,
    pub replay_inputs: ReplayInputs,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Counterexample {
    pub contract_id: String,
    pub expected: serde_json::Value,
    pub observed: serde_json::Value,
    pub evidence_refs: Vec<String>,
    pub replay_reference: String,
    pub replayability: Replayability,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub verdict: Verdict,
    pub scope: Scope,
    pub coverage: BTreeMap<String, Coverage>,
    pub counterexamples: Vec<Counterexample>,
    pub replayability: Replayability,
    pub isolation_claim: IsolationLevel,
    pub limitations: Vec<String>,
    pub reasons: Vec<String>,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty()
}
fn present(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(nonempty)
}
fn pinned<T: Serialize>(artifact: &T, hashes: &BTreeSet<String>) -> bool {
    canonical_hash(artifact).is_ok_and(|h| hashes.contains(&h))
}

/// Identity to be approved outside evaluation, in addition to artifact approval.
/// Binds predicate, observer, trust requirements and claim ID to the entire artifact.
pub fn checker_binding_hash<T: Serialize>(
    artifact: &T,
    claim: &Claim,
) -> Result<String, serde_json::Error> {
    canonical_hash(&serde_json::json!({
        "binding_version": "1",
        "domain": "b2ige.verify.approved-checker",
        "artifact_hash": canonical_hash(artifact)?,
        "claim_hash": canonical_hash(claim)?,
    }))
}

impl TrustedPolicy {
    fn valid(&self) -> bool {
        let p = &self.plan;
        let ids: BTreeSet<_> = p.claims.iter().map(|c| c.id.clone()).collect();
        let required: BTreeSet<_> = p.required_observers.iter().collect();
        nonempty(&p.plan_id)
            && nonempty(&p.version)
            && nonempty(&p.target_revision)
            && p.seed <= MAX_SAFE_INTEGER
            && nonempty(&p.reset_strategy)
            && p.time_budget_ms > 0
            && p.time_budget_ms <= MAX_SAFE_INTEGER
            && !p.scope.exploration_budget.is_empty()
            && p.scope
                .exploration_budget
                .values()
                .all(|b| *b > 0 && *b <= MAX_SAFE_INTEGER)
            && !ids.is_empty()
            && ids.len() == p.claims.len()
            && ids == p.scope.contract_ids.iter().cloned().collect()
            && ids.len() == p.scope.contract_ids.len()
            && required.len() == p.required_observers.len()
            && p.optional_observers
                .iter()
                .all(|o| nonempty(o) && !required.contains(o))
            && required
                .iter()
                .all(|o| p.claims.iter().any(|c| &c.observer == *o))
            && p.claims.iter().all(|c| {
                nonempty(&c.id)
                    && nonempty(&c.observer)
                    && required.contains(&c.observer)
                    && !c.accepted_trust.is_empty()
                    && c.accepted_trust.iter().all(|t| t.may_decide())
                    && match &c.predicate {
                        Predicate::Equals { .. } => true,
                        Predicate::CommittedCount {
                            provider,
                            operation,
                            idempotency_identity,
                            expected,
                        } => {
                            [provider, operation, idempotency_identity]
                                .iter()
                                .all(|s| nonempty(s))
                                && *expected <= MAX_SAFE_INTEGER
                                && c.accepted_trust.iter().all(|t| {
                                    matches!(
                                        t,
                                        TrustClass::AuthoritativeExternal
                                            | TrustClass::AuthoritativeTargetState
                                    )
                                })
                        }
                    }
            })
            && match &p.product_contract {
                ProductContract::Sideeffect { effect } => {
                    nonempty(&effect.effect_id) && nonempty(&effect.identity.dedupe_domain)
                    && !effect.identity.correlation_fields.is_empty()
                    // P1A implements this one correlation rule, not arbitrary fields.
                    && effect.identity.correlation_fields == ["idempotency_identity"]
                    && matches!(effect.semantics, EffectSemantics::ExactlyOnce)
                    && effect.commit_observation.eventual_window_ms.is_none_or(|n| n <= MAX_SAFE_INTEGER)
                    && p.claims.iter().any(|c| c.id == effect.effect_id
                        && c.observer == effect.commit_observation.source
                        && c.accepted_trust == vec![effect.commit_observation.minimum_trust_class]
                        && matches!(&c.predicate, Predicate::CommittedCount { expected: 1, provider, .. } if provider == &effect.identity.dedupe_domain))
                }
                ProductContract::Behavior { baseline, .. } => {
                    nonempty(&baseline.baseline_id)
                        && nonempty(&baseline.target_revision)
                        && valid_hash(&baseline.observation_contract_hash)
                        && baseline
                            .stability_runs
                            .is_none_or(|n| n > 0 && n <= MAX_SAFE_INTEGER)
                }
                ProductContract::Blindtest { invariants } => {
                    !invariants.is_empty()
                        && invariants.len() == p.claims.len()
                        && invariants
                            .iter()
                            .map(|i| &i.invariant_id)
                            .collect::<BTreeSet<_>>()
                            .len()
                            == invariants.len()
                        && invariants.iter().all(|i| {
                            nonempty(&i.statement)
                                && p.claims.iter().any(|c| {
                                    c.id == i.invariant_id
                                        && i.required_observations == vec![c.observer.clone()]
                                })
                        })
                }
            }
    }
    fn authorized(&self) -> bool {
        match &self.plan.product_contract {
            ProductContract::Behavior { baseline, stable } => {
                *stable
                    && baseline.approval.status == ApprovalStatus::Approved
                    && present(&baseline.approval.actor)
                    && present(&baseline.approval.reason)
                    && pinned(baseline, &self.approved_baselines)
            }
            ProductContract::Blindtest { invariants } => invariants.iter().all(|i| {
                i.authority.status == InvariantStatus::Authoritative
                    && present(&i.authority.source_ref)
                    && (i.origin == InvariantOrigin::MachineContract
                        || present(&i.authority.approved_by))
                    && pinned(i, &self.approved_invariants)
            }),
            ProductContract::Sideeffect { .. } => true,
        }
    }
    fn checker_authorized(&self, claim: &Claim) -> bool {
        let binding = match &self.plan.product_contract {
            ProductContract::Behavior { baseline, .. } => checker_binding_hash(baseline, claim),
            ProductContract::Blindtest { invariants } => {
                let Some(invariant) = invariants.iter().find(|i| i.invariant_id == claim.id) else {
                    return false;
                };
                checker_binding_hash(invariant, claim)
            }
            ProductContract::Sideeffect { .. } => return true,
        };
        binding.is_ok_and(|hash| self.approved_checker_bindings.contains(&hash))
    }
}

/// Deterministic, bounded evaluation of synthetic observations. Error wins; a
/// supported violation wins over unrelated missing evidence; otherwise uncertainty wins.
pub fn evaluate(policy: &TrustedPolicy, run: &Run) -> Evaluation {
    let mut out = Evaluation {
        verdict: Verdict::Error, scope: policy.plan.scope.clone(), coverage: run.coverage.clone(),
        counterexamples: vec![], replayability: Replayability::Unavailable { reason: "invalid run or experiment metadata".into() },
        isolation_claim: IsolationLevel::None,
        limitations: vec!["Bounded synthetic contract conformance; not exhaustive correctness proof.".into(),
            "Replay describes controllable inputs only; external world and outcome reproduction are not guaranteed.".into(),
            "No runner, observer, approval authentication or isolation enforcement is implemented in P1A.".into()],
        reasons: vec![],
    };
    let mut evidence_ids = BTreeSet::new();
    if !policy.valid()
        || !run.context.valid()
        || run.execution != ExecutionStatus::Completed
        || run.context.target_revision != policy.plan.target_revision
        || run.context.seed != policy.plan.seed
        || canonical_hash(&policy.plan).ok().as_ref() != Some(&run.context.plan_hash)
        || canonical_hash(&policy.config).ok().as_ref() != Some(&run.context.config_hash)
        || policy
            .plan
            .required_observers
            .iter()
            .any(|o| !run.context.observer_versions.contains_key(o))
        || run.evidence.iter().any(|e| {
            !e.valid() || e.run_id != run.context.run_id || !evidence_ids.insert(&e.evidence_id)
        })
    {
        out.reasons.push(
            "invalid authoritative configuration, run metadata, execution, or evidence integrity"
                .into(),
        );
        return out;
    }
    out.replayability = run
        .replay_inputs
        .assess(&run.context, &policy.plan.required_observers);
    match run.isolation.validate(policy.plan.isolation) {
        Ok(level) => out.isolation_claim = level,
        Err(reason) => {
            out.reasons.push(reason.into());
            return out;
        }
    }
    if !policy.authorized() {
        out.verdict = Verdict::Inconclusive;
        out.reasons
            .push("baseline/invariant lacks trusted authorization or baseline stability".into());
        return out;
    }
    for claim in &policy.plan.claims {
        if !policy.checker_authorized(claim) {
            out.reasons.push(format!(
                "{}: executable checker lacks artifact-bound approval",
                claim.id
            ));
            continue;
        }
        let covered = run.coverage.get(&claim.observer).is_some_and(|c| {
            c.verdict_critical && c.status == ObservationCoverage::Complete && nonempty(&c.reason)
        });
        if !covered {
            out.reasons
                .push(format!("{}: incomplete critical coverage", claim.id));
        }
        if !run.completed_checkers.contains(&claim.id) {
            out.reasons
                .push(format!("{}: checker incomplete", claim.id));
            continue;
        }
        let eligible: Vec<_> = run
            .evidence
            .iter()
            .filter(|e| {
                e.source == claim.observer
                    && e.related_claim_ids.contains(&claim.id)
                    && e.trust_class.may_decide()
                    && claim.accepted_trust.contains(&e.trust_class)
            })
            .collect();
        let mut comparisons = Vec::new();
        match &claim.predicate {
            Predicate::Equals { expected } => {
                for e in &eligible {
                    if let Observation::Value { value } = &e.observation {
                        comparisons.push((
                            expected.clone(),
                            value.clone(),
                            vec![e.evidence_id.clone()],
                        ));
                    }
                }
            }
            Predicate::CommittedCount {
                provider,
                operation,
                idempotency_identity,
                expected,
            } => {
                let mut identities = BTreeSet::new();
                let mut refs = Vec::new();
                for e in &eligible {
                    if let Observation::CommittedEffects { effects } = &e.observation {
                        let matching: Vec<_> = effects
                            .iter()
                            .filter(|effect| {
                                &effect.provider == provider
                                    && &effect.operation == operation
                                    && &effect.idempotency_identity == idempotency_identity
                            })
                            .collect();
                        if !effects.is_empty() && matching.is_empty() {
                            continue;
                        }
                        if matching.len() != effects.len() {
                            out.reasons
                                .push(format!("{}: mixed observation domains", claim.id));
                        }
                        identities.extend(matching.iter().map(|effect| &effect.external_effect_id));
                        refs.push(e.evidence_id.clone());
                    }
                }
                let count = identities.len() as u64;
                // Partial observation can prove duplicates, but cannot prove absence.
                if !refs.is_empty() && (covered || count > *expected) {
                    comparisons.push((serde_json::json!(expected), serde_json::json!(count), refs));
                }
            }
        }
        let usable = !comparisons.is_empty();
        for (expected, observed, evidence_refs) in comparisons {
            let equal = match canonical_equal(&expected, &observed) {
                Ok(equal) => equal,
                Err(_) => {
                    out.verdict = Verdict::Error;
                    out.reasons
                        .push("invalid canonical comparison input".into());
                    return out;
                }
            };
            if !equal {
                out.counterexamples.push(Counterexample {
                    contract_id: claim.id.clone(),
                    expected,
                    observed,
                    evidence_refs,
                    replay_reference: format!("run:{}:replay_inputs", run.context.run_id),
                    replayability: out.replayability.clone(),
                });
            }
        }
        if !usable {
            out.reasons
                .push(format!("{}: missing sufficient evidence", claim.id));
        }
    }
    out.verdict = if !out.counterexamples.is_empty() {
        Verdict::Fail
    } else if !out.reasons.is_empty() {
        Verdict::Inconclusive
    } else {
        Verdict::Pass
    };
    out
}

/// Maps evaluated output to the unchanged P0 run-result v1 wire schema.
pub fn run_result(policy: &TrustedPolicy, run: &Run) -> Result<serde_json::Value, &'static str> {
    if !run.context.valid() {
        return Err("cannot serialize invalid run identity");
    }
    let result = evaluate(policy, run);
    let product = match policy.plan.product_contract {
        ProductContract::Behavior { .. } => "behavior",
        ProductContract::Sideeffect { .. } => "sideeffect",
        ProductContract::Blindtest { .. } => "blindtest",
    };
    Ok(serde_json::json!({
        "schema_version": run.context.schema_version, "run_id": run.context.run_id,
        "product": product, "verdict": result.verdict, "scope": result.scope,
        "coverage": result.coverage, "seed": run.context.seed, "plan_hash": run.context.plan_hash,
        "config_hash": run.context.config_hash, "limitations": result.limitations,
        "failure": if result.verdict == Verdict::Fail { Some(serde_json::json!({"counterexamples": result.counterexamples})) } else { None }
    }))
}

/// Fixtures are test-owned. This envelope must never be used as an untrusted policy import API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConformanceFixture {
    pub harness_schema_version: String,
    pub policy: TrustedPolicy,
    pub run: Run,
    pub expected: Evaluation,
}
pub fn validate_fixture(fixture: &ConformanceFixture) -> Result<(), &'static str> {
    if fixture.harness_schema_version != "1" {
        return Err("unsupported harness schema version");
    }
    if evaluate(&fixture.policy, &fixture.run) != fixture.expected {
        return Err("result contradicts deterministic contract checker");
    }
    Ok(())
}
pub fn conformance_schema() -> serde_json::Value {
    let mut schema =
        serde_json::to_value(schemars::schema_for!(ConformanceFixture)).expect("schema is JSON");
    schema["$id"] =
        serde_json::json!("https://b2ige.dev/schemas/verify/conformance-fixture.v1.json");
    schema["properties"]["harness_schema_version"] =
        serde_json::json!({"type":"string", "const":"1"});
    fn constrain_integers(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("format").is_some_and(|f| f == "uint64") {
                    object.insert("maximum".into(), serde_json::json!(MAX_SAFE_INTEGER));
                }
                for value in object.values_mut() {
                    constrain_integers(value);
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(constrain_integers),
            _ => {}
        }
    }
    constrain_integers(&mut schema);
    schema
}
