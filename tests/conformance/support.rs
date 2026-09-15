use std::collections::{BTreeMap, BTreeSet};
use verify_core::*;
use verify_evidence::*;
use verify_replay::*;
use verify_runner::*;

pub fn case() -> (TrustedPolicy, Run) {
    let baseline = Baseline {
        baseline_id: "baseline-1".into(),
        target_revision: "baseline-revision".into(),
        created_by: BaselineCreator::Human,
        approval: BaselineApproval {
            status: ApprovalStatus::Approved,
            actor: Some("trusted-human".into()),
            reason: Some("reviewed locked observation contract".into()),
        },
        observation_contract_hash: canonical_hash(&"observation-v1").unwrap(),
        stability_runs: Some(3),
        notes: None,
    };
    let mut p = TrustedPolicy {
        plan: ExperimentPlan {
            plan_id: "bounded-plan".into(),
            version: "1".into(),
            target_revision: "candidate-revision".into(),
            seed: 42,
            required_observers: vec!["state".into()],
            optional_observers: vec!["optional".into()],
            reset_strategy: "synthetic fresh state".into(),
            time_budget_ms: 100,
            scope: Scope {
                contract_ids: vec!["contract-1".into()],
                exploration_budget: BTreeMap::from([("cases".into(), 1)]),
            },
            claims: vec![Claim {
                id: "contract-1".into(),
                observer: "state".into(),
                accepted_trust: vec![TrustClass::AuthoritativeTargetState],
                predicate: Predicate::Equals {
                    expected: serde_json::json!(true),
                },
            }],
            isolation: IsolationLevel::None,
            product_contract: ProductContract::Behavior {
                baseline: baseline.clone(),
                stable: true,
            },
        },
        config: serde_json::json!({"fault_schedule":[], "mode":"synthetic"}),
        approved_baselines: BTreeSet::from([canonical_hash(&baseline).unwrap()]),
        approved_invariants: BTreeSet::new(),
        approved_checker_bindings: BTreeSet::new(),
    };
    // Two explicitly approved synthetic checks: boolean safety and numeric JCS
    // equivalence. The independent numeric control changes true -> 1 -> 1.0
    // without changing approvals. False, 2, and other predicates are NOT approved.
    // Keep this fixture setup out of rebind: mutations never acquire approvals.
    for expected in [serde_json::json!(true), serde_json::json!(1)] {
        let mut claim = p.plan.claims[0].clone();
        claim.predicate = Predicate::Equals { expected };
        p.approved_checker_bindings
            .insert(checker_binding_hash(&baseline, &claim).unwrap());
    }
    let observation = Observation::Value {
        value: serde_json::json!(true),
    };
    let mut r = Run {
        context: RunContext {
            run_id: "run-1".into(),
            schema_version: "1".into(),
            tool_version: "p1a-0.1.0".into(),
            seed: 42,
            target_revision: "candidate-revision".into(),
            config_hash: String::new(),
            plan_hash: String::new(),
            experiment_hash: String::new(),
            platform_runtime: "synthetic/rust".into(),
            observer_versions: BTreeMap::from([("state".into(), "1".into())]),
            started_at: "2026-09-14T00:00:00Z".into(),
            ended_at: "2026-09-14T00:00:01Z".into(),
        },
        execution: ExecutionStatus::Completed,
        isolation: IsolationAssessment {
            available: IsolationLevel::None,
            linux_backend: false,
            protected_artifacts_visible: false,
        },
        coverage: BTreeMap::from([(
            "state".into(),
            Coverage {
                status: ObservationCoverage::Complete,
                verdict_critical: true,
                reason: "entire declared synthetic observation domain".into(),
            },
        )]),
        completed_checkers: BTreeSet::from(["contract-1".into()]),
        evidence: vec![Evidence {
            evidence_id: "e-1".into(),
            run_id: "run-1".into(),
            source: "state".into(),
            trust_class: TrustClass::AuthoritativeTargetState,
            order: 1,
            integrity_hash: canonical_hash(&observation).unwrap(),
            observation,
            related_claim_ids: vec!["contract-1".into()],
        }],
        replay_inputs: ReplayInputs {
            seed: Some(42),
            experiment_plan: None,
            config: None,
            config_hash: None,
            fault_schedule: Some(vec![]),
            target_revision: Some("candidate-revision".into()),
        },
    };
    // Content-addressed synthetic target, not an unresolved branch/ref.
    let target = canonical_hash(&"synthetic candidate artifact v1").unwrap();
    p.plan.target_revision = target.clone();
    r.context.target_revision = target.clone();
    r.replay_inputs.target_revision = Some(target);
    rebind(&mut p, &mut r);
    (p, r)
}
pub fn rebind(p: &mut TrustedPolicy, r: &mut Run) {
    r.context.plan_hash = canonical_hash(&p.plan).unwrap();
    r.context.experiment_hash = r.context.plan_hash.clone();
    r.context.config_hash = canonical_hash(&p.config).unwrap();
    r.replay_inputs.experiment_plan = Some(serde_json::to_value(&p.plan).unwrap());
    r.replay_inputs.config = Some(p.config.clone());
    r.replay_inputs.config_hash = Some(r.context.config_hash.clone());
}
pub fn set_value(r: &mut Run, value: serde_json::Value) {
    r.evidence[0].observation = Observation::Value { value };
    r.evidence[0].integrity_hash = canonical_hash(&r.evidence[0].observation).unwrap();
}
pub fn sideeffect() -> (TrustedPolicy, Run) {
    let (mut p, mut r) = case();
    p.plan.product_contract = ProductContract::Sideeffect {
        effect: EffectContract {
            effect_id: "contract-1".into(),
            semantics: EffectSemantics::ExactlyOnce,
            identity: IdentityRule {
                correlation_fields: vec!["idempotency_identity".into()],
                dedupe_domain: "provider".into(),
            },
            commit_observation: CommitObservation {
                source: "state".into(),
                minimum_trust_class: TrustClass::AuthoritativeExternal,
                eventual_window_ms: Some(1000),
            },
            related_effect: None,
        },
    };
    p.plan.claims[0].accepted_trust = vec![TrustClass::AuthoritativeExternal];
    p.plan.claims[0].predicate = Predicate::CommittedCount {
        provider: "provider".into(),
        operation: "payment".into(),
        idempotency_identity: "purchase-1".into(),
        expected: 1,
    };
    r.evidence[0].trust_class = TrustClass::AuthoritativeExternal;
    r.evidence[0].observation = Observation::CommittedEffects {
        effects: vec![EffectIdentity {
            provider: "provider".into(),
            operation: "payment".into(),
            external_effect_id: "payment-1".into(),
            idempotency_identity: "purchase-1".into(),
        }],
    };
    refresh(&mut r);
    rebind(&mut p, &mut r);
    (p, r)
}
pub fn refresh(r: &mut Run) {
    r.evidence[0].integrity_hash = canonical_hash(&r.evidence[0].observation).unwrap();
}
pub fn blindtest() -> (TrustedPolicy, Run) {
    let (mut p, mut r) = case();
    let invariant = Invariant {
        invariant_id: "contract-1".into(),
        statement: "state must equal true".into(),
        origin: InvariantOrigin::LlmCandidate,
        authority: InvariantAuthority {
            status: InvariantStatus::Authoritative,
            approved_by: Some("trusted-human".into()),
            source_ref: Some("review:1".into()),
        },
        required_observations: vec!["state".into()],
        hidden: Some(true),
    };
    p.approved_invariants
        .insert(canonical_hash(&invariant).unwrap());
    p.approved_checker_bindings
        .insert(checker_binding_hash(&invariant, &p.plan.claims[0]).unwrap());
    p.plan.product_contract = ProductContract::Blindtest {
        invariants: vec![invariant],
    };
    rebind(&mut p, &mut r);
    (p, r)
}
pub fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
