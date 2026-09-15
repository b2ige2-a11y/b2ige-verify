//! Independent P1A review. Failing assertions record unmet contracts; do not
//! regenerate fixtures, change expectations, or repair production to green this review.
//! Existing golden files supply inputs only. No conformance/support.rs helpers are used.
use serde_json::{json, Value};
use verify_core::*;
use verify_evidence::*;
use verify_replay::*;
use verify_runner::*;

fn fixture(name: &str) -> ConformanceFixture {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}

// Bind changed experiment inputs, never alter baseline/invariant authorization pins.
fn bind(f: &mut ConformanceFixture) {
    f.run.context.plan_hash = canonical_hash(&f.policy.plan).unwrap();
    f.run.context.experiment_hash = f.run.context.plan_hash.clone();
    f.run.context.config_hash = canonical_hash(&f.policy.config).unwrap();
    f.run.replay_inputs.experiment_plan = Some(serde_json::to_value(&f.policy.plan).unwrap());
    f.run.replay_inputs.config = Some(f.policy.config.clone());
    f.run.replay_inputs.config_hash = Some(f.run.context.config_hash.clone());
}

fn observe(e: &mut Evidence, observation: Observation) {
    e.observation = observation;
    e.integrity_hash = canonical_hash(&e.observation).unwrap();
}

fn verdict(f: &ConformanceFixture) -> Verdict {
    evaluate(&f.policy, &f.run).verdict
}

fn schema(name: &str) -> jsonschema::Validator {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas")
        .join(name);
    jsonschema::validator_for(
        &serde_json::from_str::<Value>(&std::fs::read_to_string(path).unwrap()).unwrap(),
    )
    .unwrap()
}

#[test]
fn ig_01_mixed_domain_record_must_not_hide_a_proven_duplicate() {
    let mut f = fixture("sideeffect-pass.json");
    let mut second = f.run.evidence[0].clone();
    second.evidence_id = "second-snapshot".into();
    second.order = 2;
    let Observation::CommittedEffects { effects } = &mut second.observation else {
        panic!("fixture")
    };
    effects[0].external_effect_id = "payment-2".into();
    second.integrity_hash = canonical_hash(&second.observation).unwrap();
    f.run.evidence.push(second);
    assert_eq!(verdict(&f), Verdict::Fail, "two in-domain commits control");

    let Observation::CommittedEffects { effects } = &mut f.run.evidence[1].observation else {
        panic!("fixture")
    };
    let mut unrelated = effects[0].clone();
    unrelated.idempotency_identity = "different-purchase".into();
    unrelated.external_effect_id = "other-payment".into();
    effects.push(unrelated);
    f.run.evidence[1].integrity_hash = canonical_hash(&f.run.evidence[1].observation).unwrap();
    assert!(schema("conformance-fixture.schema.json").is_valid(&serde_json::to_value(&f).unwrap()));
    let result = evaluate(&f.policy, &f.run);
    assert_ne!(
        result.verdict,
        Verdict::Pass,
        "IG-01: known duplicate was discarded: {result:?}"
    );
}

#[test]
fn ig_02_unsupported_correlation_requirement_must_not_pass() {
    let mut f = fixture("sideeffect-pass.json");
    let ProductContract::Sideeffect { effect } = &mut f.policy.plan.product_contract else {
        panic!("fixture")
    };
    effect.identity.correlation_fields = vec!["customer_id".into()];
    bind(&mut f);
    assert_eq!(
        verdict(&f),
        Verdict::Error,
        "IG-02: no customer_id exists in the evidence model"
    );
}

#[test]
fn ig_03_approved_invariant_must_bind_executable_predicate() {
    let mut f = fixture("blindtest-pass.json");
    observe(
        &mut f.run.evidence[0],
        Observation::Value {
            value: json!(false),
        },
    );
    assert_eq!(verdict(&f), Verdict::Fail);
    let pins = f.policy.approved_invariants.clone();
    // Approved statement still says 'state must equal true'.
    f.policy.plan.claims[0].predicate = Predicate::Equals {
        expected: json!(false),
    };
    bind(&mut f);
    assert_eq!(f.policy.approved_invariants, pins);
    assert_ne!(
        verdict(&f),
        Verdict::Pass,
        "IG-03: changed checker reused the original invariant pin"
    );
}

#[test]
fn ig_04_baseline_observation_contract_must_bind_checker() {
    let mut f = fixture("pass.json");
    observe(
        &mut f.run.evidence[0],
        Observation::Value {
            value: json!(false),
        },
    );
    assert_eq!(verdict(&f), Verdict::Fail);
    let pins = f.policy.approved_baselines.clone();
    f.policy.plan.claims[0].predicate = Predicate::Equals {
        expected: json!(false),
    };
    bind(&mut f);
    assert_eq!(f.policy.approved_baselines, pins);
    assert_ne!(
        verdict(&f),
        Verdict::Pass,
        "IG-04: changed observation contract retained baseline approval"
    );
}

#[test]
fn ig_05_mutable_target_must_not_be_replay_available() {
    let mut f = fixture("fail.json");
    f.policy.plan.target_revision = "main".into();
    f.run.context.target_revision = "main".into();
    f.run.replay_inputs.target_revision = Some("main".into());
    bind(&mut f);
    assert_eq!(verdict(&f), Verdict::Fail);
    assert!(
        matches!(
            evaluate(&f.policy, &f.run).replayability,
            Replayability::Unavailable { .. }
        ),
        "IG-05: main does not pin the target"
    );
}

#[test]
fn ig_06_invalid_run_metadata_must_not_claim_replay_available() {
    let mut f = fixture("fail.json");
    f.run.context.observer_versions.clear();
    assert_eq!(verdict(&f), Verdict::Error);
    assert!(
        matches!(
            evaluate(&f.policy, &f.run).replayability,
            Replayability::Unavailable { .. }
        ),
        "IG-06: missing observer versions yet replay available"
    );
}

#[test]
fn ig_07_jcs_uses_unescaped_utf16_property_order() {
    // RFC 8785 section 3.2.3 order; independent SHA-256 of exact UTF-8 bytes:
    // {"\r":1,"1":2,"\u0080":3,"ö":4,"€":5,"😀":6,"דּ":7}
    let value = json!({"\r":1,"1":2,"\u{0080}":3,"ö":4,"€":5,"😀":6,"דּ":7});
    assert_eq!(
        canonical_hash(&value).unwrap(),
        "sha256:4bd52d82f332c2e5c7206abd57c74b45dd4f0fef63ab7af87b8bde4481e450e7",
        "IG-07: RFC 8785 key order"
    );
}

#[test]
fn ig_08_jcs_rejects_nonfinite_numbers() {
    assert!(
        canonical_hash(&f64::NAN).is_err(),
        "IG-08: NaN was hashed as null"
    );
}

#[test]
fn ig_09_same_canonical_plan_and_evidence_must_have_same_verdict() {
    let mut f = fixture("pass.json");
    f.policy.plan.claims[0].predicate = Predicate::Equals { expected: json!(1) };
    observe(
        &mut f.run.evidence[0],
        Observation::Value { value: json!(1) },
    );
    bind(&mut f);
    assert_eq!(verdict(&f), Verdict::Pass);
    let original_hash = f.run.context.plan_hash.clone();
    // Same JSON number after JCS; no rehash or rebind of the run.
    f.policy.plan.claims[0].predicate = Predicate::Equals {
        expected: json!(1.0),
    };
    assert_eq!(canonical_hash(&f.policy.plan).unwrap(), original_hash);
    assert_eq!(
        verdict(&f),
        Verdict::Pass,
        "IG-09: same plan hash now gives a false divergence"
    );
}

#[test]
fn ig_10_raw_duplicate_coverage_key_must_be_rejected() {
    let f = fixture("pass.json");
    let mut value = serde_json::to_value(&f.run).unwrap();
    value.as_object_mut().unwrap().remove("coverage");
    let prefix = serde_json::to_string(&value).unwrap();
    let complete = serde_json::to_string(&f.run.coverage["state"]).unwrap();
    let raw = format!(
        r#"{},"coverage":{{"state":{{"status":"unavailable","verdict_critical":true,"reason":"missing"}},"state":{complete}}}}}"#,
        &prefix[..prefix.len() - 1]
    );
    let parsed = serde_json::from_str::<Run>(&raw);
    if let Ok(run) = &parsed {
        assert_eq!(evaluate(&f.policy, run).verdict, Verdict::Pass);
    }
    assert!(
        parsed.is_err(),
        "IG-10: duplicate coverage key erased UNAVAILABLE before checking"
    );
}

#[test]
fn ig_11_legacy_schema_must_not_be_bypassed_by_null_roundtrip() {
    let f = fixture("pass.json");
    let ProductContract::Behavior { baseline, .. } = f.policy.plan.product_contract else {
        panic!("fixture")
    };
    let mut raw = serde_json::to_value(baseline).unwrap();
    raw["notes"] = Value::Null;
    assert!(!schema("behavior-baseline.schema.json").is_valid(&raw));
    let parsed = serde_json::from_value::<Baseline>(raw);
    if let Ok(baseline) = &parsed {
        assert!(schema("behavior-baseline.schema.json")
            .is_valid(&serde_json::to_value(baseline).unwrap()));
    }
    assert!(
        parsed.is_err(),
        "IG-11: P0-invalid null disappears before legacy schema check"
    );
}

#[test]
fn ig_12_harness_schema_and_rust_u64_ranges_must_agree() {
    let f = fixture("pass.json");
    let mut value = serde_json::to_value(f).unwrap();
    value["run"]["context"]["seed"] = serde_json::from_str("18446744073709551616").unwrap();
    assert!(serde_json::from_value::<ConformanceFixture>(value.clone()).is_err());
    assert!(
        !schema("conformance-fixture.schema.json").is_valid(&value),
        "IG-12: uint64 format lacks an enforced maximum"
    );
}

#[test]
fn ig_13_empty_requirement_and_zero_budget_controls() {
    for mutation in 0..7 {
        let mut f = fixture("pass.json");
        match mutation {
            0 => f.policy.plan.claims.clear(),
            1 => f.policy.plan.required_observers.clear(),
            2 => f.policy.plan.scope.contract_ids.clear(),
            3 => f.policy.plan.scope.exploration_budget.clear(),
            4 => {
                f.policy
                    .plan
                    .scope
                    .exploration_budget
                    .insert("cases".into(), 0);
            }
            5 => f.policy.plan.claims[0].accepted_trust.clear(),
            _ => f.policy.plan.claims[0].observer.clear(),
        }
        bind(&mut f);
        assert_eq!(verdict(&f), Verdict::Error, "mutation {mutation}");
    }
}

#[test]
fn ig_14_authority_identity_and_integrity_controls() {
    for mutation in 0..8 {
        let mut f = fixture("pass.json");
        match mutation {
            0 => f.run.evidence[0].run_id = "old-run".into(),
            1 => f.run.evidence[0].source = "other-observer".into(),
            2 => f.run.evidence[0].related_claim_ids = vec!["other-claim".into()],
            3 => f.run.evidence[0].trust_class = TrustClass::Derived,
            4 => f.run.evidence[0].trust_class = TrustClass::Advisory,
            5 => f.run.evidence.push(f.run.evidence[0].clone()),
            6 => f.run.evidence[0].integrity_hash = "sha256:bad".into(),
            _ => f.run.evidence.clear(),
        }
        assert_ne!(verdict(&f), Verdict::Pass, "mutation {mutation}");
    }
}

#[test]
fn ig_15_sideeffect_attempt_commit_and_run_controls() {
    for (attempts, commits, expected) in [
        (2, None, Verdict::Inconclusive),
        (2, Some(1), Verdict::Pass),
        (1, Some(2), Verdict::Fail),
    ] {
        let mut f = fixture("sideeffect-pass.json");
        let template = f.run.evidence[0].clone();
        f.run.evidence.clear();
        for n in 0..attempts {
            let mut e = template.clone();
            e.evidence_id = format!("attempt-{n}");
            e.trust_class = TrustClass::DirectRuntime;
            observe(
                &mut e,
                Observation::Attempt {
                    request_id: format!("request-{n}"),
                },
            );
            f.run.evidence.push(e);
        }
        if let Some(count) = commits {
            let Observation::CommittedEffects { effects } = &template.observation else {
                panic!("fixture")
            };
            let effects = (0..count)
                .map(|n| {
                    let mut e = effects[0].clone();
                    e.external_effect_id = format!("payment-{n}");
                    e
                })
                .collect();
            let mut e = template.clone();
            observe(&mut e, Observation::CommittedEffects { effects });
            f.run.evidence.push(e);
        }
        assert_eq!(verdict(&f), expected);
    }
    let mut f = fixture("sideeffect-pass.json");
    let mut e = f.run.evidence[0].clone();
    e.evidence_id = "other-run-evidence".into();
    e.run_id = "other-run".into();
    f.run.evidence.push(e);
    assert_eq!(verdict(&f), Verdict::Error);
    let mut f = fixture("sideeffect-pass.json");
    f.run.evidence[0].trust_class = TrustClass::DirectRuntime;
    assert_eq!(verdict(&f), Verdict::Inconclusive);
}

#[test]
fn ig_16_authorization_pin_mutation_controls() {
    for mutation in 0..4 {
        let mut f = fixture("pass.json");
        let ProductContract::Behavior { baseline, .. } = &mut f.policy.plan.product_contract else {
            panic!("fixture")
        };
        match mutation {
            0 => baseline.target_revision = "other-baseline".into(),
            1 => baseline.observation_contract_hash = canonical_hash(&"changed").unwrap(),
            2 => baseline.baseline_id.clear(),
            _ => f.policy.approved_baselines.clear(),
        }
        bind(&mut f);
        assert_ne!(verdict(&f), Verdict::Pass);
    }
    for mutation in 0..5 {
        let mut f = fixture("blindtest-pass.json");
        let ProductContract::Blindtest { invariants } = &mut f.policy.plan.product_contract else {
            panic!("fixture")
        };
        match mutation {
            0 => invariants[0].statement = "replacement".into(),
            1 => invariants[0].invariant_id = "different-invariant".into(),
            2 => invariants[0].statement.clear(),
            3 => invariants[0].authority.source_ref = None,
            _ => f.policy.approved_invariants.clear(),
        }
        bind(&mut f);
        assert_ne!(verdict(&f), Verdict::Pass);
    }
}

#[test]
fn ig_17_coverage_precedence_and_isolation_controls() {
    for status in [
        ObservationCoverage::Partial,
        ObservationCoverage::Unavailable,
        ObservationCoverage::Failed,
    ] {
        let mut f = fixture("pass.json");
        f.run.coverage.get_mut("state").unwrap().status = status;
        assert_eq!(verdict(&f), Verdict::Inconclusive);
    }
    let mut f = fixture("fail.json");
    f.run.execution = ExecutionStatus::RunnerCrash;
    assert_eq!(verdict(&f), Verdict::Error);
    let mut f = fixture("blindtest-pass.json");
    f.policy.plan.isolation = IsolationLevel::HardenedLinux;
    f.run.isolation.available = IsolationLevel::HardenedLinux;
    bind(&mut f);
    assert_eq!(verdict(&f), Verdict::Error);
    f.run.isolation.linux_backend = true;
    f.run.isolation.protected_artifacts_visible = true;
    assert_eq!(verdict(&f), Verdict::Error);
}

#[test]
fn ig_18_unknown_observation_and_version_controls() {
    let mut value = serde_json::to_value(fixture("pass.json")).unwrap();
    value["run"]["evidence"][0]["observation"]["type"] = json!("unknown");
    assert!(serde_json::from_value::<ConformanceFixture>(value).is_err());
    let mut f = fixture("pass.json");
    f.run.context.schema_version = "2".into();
    assert_eq!(verdict(&f), Verdict::Error);
    let mut f = fixture("pass.json");
    f.harness_schema_version = "2".into();
    assert!(validate_fixture(&f).is_err());
    assert!(serde_json::from_str::<Value>("NaN").is_err());
}

#[test]
fn ig_19_jcs_exact_binary64_integer_has_one_representation() {
    // 2^60 is exactly representable in binary64; this is not a rounding collision.
    assert_eq!(
        canonical_hash(&(1_u64 << 60)).unwrap(),
        canonical_hash(&2_f64.powi(60)).unwrap(),
        "IG-19: Rust numeric storage type changes RFC 8785 serialization"
    );
}

#[test]
fn ig_20_documented_trusted_input_boundary_controls() {
    let mut f = fixture("pass.json");
    let hash = f.run.evidence[0].integrity_hash.clone();
    f.run.evidence[0].trust_class = TrustClass::Derived;
    assert_eq!(verdict(&f), Verdict::Inconclusive);
    // This is allowed ONLY because source/trust/run labels are trusted synthetic
    // inputs. It demonstrates the boundary, not an authenticated escalation exploit.
    f.run.evidence[0].trust_class = TrustClass::AuthoritativeTargetState;
    assert_eq!(f.run.evidence[0].integrity_hash, hash);
    assert_eq!(verdict(&f), Verdict::Pass);

    let mut f = fixture("blindtest-pass.json");
    f.policy.plan.isolation = IsolationLevel::HardenedLinux;
    f.run.isolation.available = IsolationLevel::HardenedLinux;
    f.run.isolation.linux_backend = true;
    bind(&mut f);
    let result = evaluate(&f.policy, &f.run);
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.isolation_claim, IsolationLevel::HardenedLinux);
    assert!(result.limitations.iter().any(|s| s.contains("No runner")));
}

#[test]
fn ig_21_conflicting_evidence_and_claim_relative_loss_controls() {
    let mut f = fixture("pass.json");
    let mut conflicting = f.run.evidence[0].clone();
    conflicting.evidence_id = "conflicting".into();
    // Identical sequence numbers cannot suppress a supported violation.
    observe(
        &mut conflicting,
        Observation::Value {
            value: json!(false),
        },
    );
    f.run.evidence.push(conflicting);
    assert_eq!(verdict(&f), Verdict::Fail);
    f.run.evidence.reverse();
    assert_eq!(verdict(&f), Verdict::Fail);

    let mut claim = f.policy.plan.claims[0].clone();
    claim.id = "database-claim".into();
    claim.observer = "database".into();
    f.policy.plan.claims.push(claim);
    f.policy
        .plan
        .scope
        .contract_ids
        .push("database-claim".into());
    f.policy.plan.required_observers.push("database".into());
    f.run
        .context
        .observer_versions
        .insert("database".into(), "1".into());
    f.run.completed_checkers.insert("database-claim".into());
    f.run.coverage.insert(
        "database".into(),
        Coverage {
            status: ObservationCoverage::Unavailable,
            verdict_critical: true,
            reason: "state inaccessible".into(),
        },
    );
    bind(&mut f);
    assert_eq!(verdict(&f), Verdict::Fail);
    f.run.evidence.remove(0);
    assert_eq!(verdict(&f), Verdict::Inconclusive);
}
