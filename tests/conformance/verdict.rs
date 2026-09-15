use super::support::*;
use verify_core::*;
use verify_evidence::*;
use verify_runner::*;

#[test]
fn v_001_missing_critical_observer() {
    let (p, mut r) = case();
    r.coverage.get_mut("state").unwrap().status = ObservationCoverage::Unavailable;
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn v_002_observer_initialization_failure() {
    let (p, mut r) = case();
    r.execution = ExecutionStatus::ObserverInitializationFailure;
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
#[test]
fn v_003_violation_despite_optional_loss() {
    let (p, mut r) = case();
    set_value(&mut r, serde_json::json!(false));
    r.coverage.insert(
        "optional".into(),
        Coverage {
            status: ObservationCoverage::Unavailable,
            verdict_critical: false,
            reason: "unsupported".into(),
        },
    );
    let result = evaluate(&p, &r);
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(!result.counterexamples.is_empty());
    assert!(!result.counterexamples[0].evidence_refs.is_empty());
}
#[test]
fn v_004_bounded_pass_has_scope() {
    let (p, r) = case();
    let result = evaluate(&p, &r);
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.scope, p.plan.scope);
    assert!(!result.limitations.is_empty());
}
#[test]
fn v_005_observer_ran_without_sufficient_results() {
    let (p, mut r) = case();
    r.evidence.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn v_006_runner_errors_dominate_even_proven_violation() {
    let (p, mut r) = case();
    set_value(&mut r, serde_json::json!(false));
    for error in [
        ExecutionStatus::RunnerCrash,
        ExecutionStatus::InvalidAuthoritativeConfig,
        ExecutionStatus::CorruptedEvidenceStore,
        ExecutionStatus::ObserverInitializationFailure,
    ] {
        r.execution = error;
        assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
    }
}
#[test]
fn v_007_missing_checker_never_passes() {
    let (p, mut r) = case();
    r.completed_checkers.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn v_008_empty_scope_or_contract_is_invalid() {
    let (mut p, r) = case();
    p.plan.claims.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
#[test]
fn cov_001_all_incomplete_states_block_pass() {
    let (p, mut r) = case();
    for status in [
        ObservationCoverage::Partial,
        ObservationCoverage::Unavailable,
        ObservationCoverage::Failed,
    ] {
        r.coverage.get_mut("state").unwrap().status = status;
        assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
    }
    r.coverage.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn cov_002_cannot_downgrade_required_observer_to_optional() {
    let (p, mut r) = case();
    r.coverage.get_mut("state").unwrap().verdict_critical = false;
    assert_ne!(evaluate(&p, &r).verdict, Verdict::Pass);
}
#[test]
fn cov_003_coverage_is_claim_relative() {
    let (mut p, mut r) = case();
    let mut claim = p.plan.claims[0].clone();
    claim.id = "session-absent".into();
    claim.observer = "database".into();
    p.plan.claims.push(claim);
    p.plan.scope.contract_ids.push("session-absent".into());
    p.plan.required_observers.push("database".into());
    r.context
        .observer_versions
        .insert("database".into(), "1".into());
    r.completed_checkers.insert("session-absent".into());
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn e_001_derived_and_advisory_cannot_decide() {
    let (p, mut r) = case();
    for class in [TrustClass::Derived, TrustClass::Advisory] {
        r.evidence[0].trust_class = class;
        for value in [true, false] {
            set_value(&mut r, serde_json::json!(value));
            assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
        }
    }
}
#[test]
fn e_002_wrong_source_run_claim_or_trust_is_not_evidence() {
    let (p, r) = case();
    for mutation in 0..4 {
        let mut changed = r.clone();
        match mutation {
            0 => changed.evidence[0].source = "other".into(),
            1 => changed.evidence[0].run_id = "older-run".into(),
            2 => changed.evidence[0].related_claim_ids = vec!["other".into()],
            _ => changed.evidence[0].trust_class = TrustClass::DirectRuntime,
        }
        assert_ne!(evaluate(&p, &changed).verdict, Verdict::Pass);
    }
}
#[test]
fn e_003_corrupt_payload_and_duplicate_identity_are_errors() {
    let (p, mut r) = case();
    r.evidence[0].integrity_hash = "sha256:invalid".into();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
    let (_, mut r) = case();
    r.evidence.push(r.evidence[0].clone());
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
#[test]
fn e_004_advisory_policy_is_rejected() {
    let (mut p, mut r) = case();
    p.plan.claims[0].accepted_trust = vec![TrustClass::Advisory];
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
#[test]
fn e_005_evidence_order_does_not_hide_violation() {
    let (p, mut r) = case();
    let mut bad = r.evidence[0].clone();
    bad.evidence_id = "bad".into();
    bad.order = 2;
    bad.observation = Observation::Value {
        value: serde_json::json!(false),
    };
    bad.integrity_hash = canonical_hash(&bad.observation).unwrap();
    r.evidence.push(bad);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
    r.evidence.reverse();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
}
#[test]
fn e_006_jcs_hash_is_key_order_independent_and_payload_sensitive() {
    assert_eq!(
        canonical_hash(&serde_json::json!({"b":2,"a":1})).unwrap(),
        canonical_hash(&serde_json::json!({"a":1,"b":2})).unwrap()
    );
    assert_ne!(
        canonical_hash(&true).unwrap(),
        canonical_hash(&false).unwrap()
    );
}
#[test]
fn v_009_exit_codes_are_exact() {
    for (v, code) in [
        (Verdict::Pass, 0),
        (Verdict::Fail, 1),
        (Verdict::Inconclusive, 2),
        (Verdict::Error, 3),
    ] {
        assert_eq!(v.exit_code(), code);
    }
}
#[test]
fn v_010_proven_violation_survives_unrelated_missing_evidence() {
    let (mut p, mut r) = case();
    set_value(&mut r, serde_json::json!(false));
    let mut second = p.plan.claims[0].clone();
    second.id = "other".into();
    p.plan.claims.push(second);
    p.plan.scope.contract_ids.push("other".into());
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
}
