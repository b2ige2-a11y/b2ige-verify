use super::support::*;
use verify_core::*;
use verify_evidence::*;
use verify_runner::*;

#[test]
fn b_001_unapproved_baseline() {
    let (mut p, mut r) = case();
    if let ProductContract::Behavior { baseline, .. } = &mut p.plan.product_contract {
        baseline.approval.status = ApprovalStatus::Unapproved;
    }
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn b_002_unstable_baseline() {
    let (mut p, mut r) = case();
    if let ProductContract::Behavior { stable, .. } = &mut p.plan.product_contract {
        *stable = false;
    }
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn bh_003_agent_cannot_self_approve_or_replace_pinned_baseline() {
    let (mut p, mut r) = case();
    if let ProductContract::Behavior { baseline, .. } = &mut p.plan.product_contract {
        baseline.approval.actor = Some("coding-agent".into());
        baseline.target_revision = "candidate-revision".into();
    }
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn bh_004_approved_label_without_trusted_pin_is_insufficient() {
    let (mut p, r) = case();
    p.approved_baselines.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn s_001_two_attempts_one_commit() {
    let (p, mut r) = sideeffect();
    for n in 0..2 {
        let mut attempt = r.evidence[0].clone();
        attempt.evidence_id = format!("attempt-{n}");
        attempt.order = n + 2;
        attempt.trust_class = TrustClass::DirectRuntime;
        attempt.observation = Observation::Attempt {
            request_id: format!("request-{n}"),
        };
        attempt.integrity_hash = canonical_hash(&attempt.observation).unwrap();
        r.evidence.push(attempt);
    }
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Pass);
}
#[test]
fn s_002_http_success_without_provider_confirmation() {
    let (p, mut r) = sideeffect();
    r.evidence[0].observation = Observation::Attempt {
        request_id: "http-200".into(),
    };
    refresh(&mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn s_003_duplicate_provider_commits() {
    let (p, mut r) = sideeffect();
    if let Observation::CommittedEffects { effects } = &mut r.evidence[0].observation {
        let mut duplicate = effects[0].clone();
        duplicate.external_effect_id = "payment-2".into();
        effects.push(duplicate);
    }
    refresh(&mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
}
#[test]
fn se_004_repeated_observation_is_not_duplicate_commit() {
    let (p, mut r) = sideeffect();
    if let Observation::CommittedEffects { effects } = &mut r.evidence[0].observation {
        effects.push(effects[0].clone());
    }
    refresh(&mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Pass);
}
#[test]
fn se_005_wrong_effect_identity_does_not_establish_count() {
    let (p, mut r) = sideeffect();
    if let Observation::CommittedEffects { effects } = &mut r.evidence[0].observation {
        effects[0].idempotency_identity = "other-purchase".into();
    }
    refresh(&mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn se_006_empty_snapshot_and_missing_snapshot_are_different() {
    let (p, mut r) = sideeffect();
    r.evidence[0].observation = Observation::CommittedEffects { effects: vec![] };
    refresh(&mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
    r.coverage.get_mut("state").unwrap().status = ObservationCoverage::Partial;
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
    r.evidence.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn se_007_runtime_commit_label_cannot_substitute_provider_state() {
    let (p, mut r) = sideeffect();
    r.evidence[0].trust_class = TrustClass::DirectRuntime;
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn bt_001_unapproved_llm_invariant_cannot_pass_or_fail() {
    let (mut p, mut r) = blindtest();
    if let ProductContract::Blindtest { invariants } = &mut p.plan.product_contract {
        invariants[0].authority.status = InvariantStatus::Candidate;
    }
    rebind(&mut p, &mut r);
    for value in [true, false] {
        set_value(&mut r, serde_json::json!(value));
        assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
    }
}
#[test]
fn bt_002_hidden_grader_exposure_fails_preflight() {
    let (mut p, mut r) = blindtest();
    for mode in [
        IsolationLevel::WorkspaceSeparation,
        IsolationLevel::ContainerIsolation,
        IsolationLevel::HardenedLinux,
    ] {
        p.plan.isolation = mode;
        r.isolation = IsolationAssessment {
            available: mode,
            linux_backend: true,
            protected_artifacts_visible: true,
        };
        rebind(&mut p, &mut r);
        assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
    }
}
#[test]
fn bt_003_hardened_linux_unavailable() {
    let (mut p, mut r) = blindtest();
    p.plan.isolation = IsolationLevel::HardenedLinux;
    r.isolation.available = IsolationLevel::HardenedLinux;
    r.isolation.linux_backend = false;
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
#[test]
fn bt_004_no_isolation_overclaim() {
    let (mut p, mut r) = blindtest();
    p.plan.isolation = IsolationLevel::ContainerIsolation;
    r.isolation.available = IsolationLevel::WorkspaceSeparation;
    rebind(&mut p, &mut r);
    let result = evaluate(&p, &r);
    assert_eq!(result.verdict, Verdict::Error);
    assert_eq!(result.isolation_claim, IsolationLevel::None);
}
#[test]
fn bt_005_forged_authority_label_and_modified_invariant_rejected() {
    let (mut p, mut r) = blindtest();
    p.approved_invariants.clear();
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
    let (original, _) = blindtest();
    p.approved_invariants = original.approved_invariants;
    if let ProductContract::Blindtest { invariants } = &mut p.plan.product_contract {
        invariants[0].statement = "always true".into();
    }
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Inconclusive);
}
#[test]
fn se_008_duplicates_across_separate_evidence_records_are_fail() {
    let (p, mut r) = sideeffect();
    let mut second = r.evidence[0].clone();
    second.evidence_id = "e-2".into();
    second.order = 2;
    if let Observation::CommittedEffects { effects } = &mut second.observation {
        effects[0].external_effect_id = "payment-2".into();
    }
    second.integrity_hash = canonical_hash(&second.observation).unwrap();
    r.evidence.push(second);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Fail);
}
#[test]
fn se_009_unsupported_effect_semantics_rejected() {
    let (mut p, mut r) = sideeffect();
    if let ProductContract::Sideeffect { effect } = &mut p.plan.product_contract {
        effect.semantics = EffectSemantics::AtomicWith;
    }
    rebind(&mut p, &mut r);
    assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
}
