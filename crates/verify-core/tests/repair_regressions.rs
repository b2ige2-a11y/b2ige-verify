//! Additional repair controls; independent_gate.rs and its assertions are unchanged.
use serde::Serialize;
use serde_json::{json, Value};
use verify_core::*;
use verify_evidence::*;
use verify_replay::Replayability;

fn fixture(name: &str) -> ConformanceFixture {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn bind(f: &mut ConformanceFixture) {
    f.run.context.plan_hash = canonical_hash(&f.policy.plan).unwrap();
    f.run.context.experiment_hash = f.run.context.plan_hash.clone();
    f.run.replay_inputs.experiment_plan = Some(serde_json::to_value(&f.policy.plan).unwrap());
}

#[test]
fn mixed_domains_preserve_duplicates_and_do_not_prove_completeness() {
    for field in ["provider", "operation", "idempotency_identity"] {
        let mut f = fixture("sideeffect-pass.json");
        let mut effects = match &f.run.evidence[0].observation {
            Observation::CommittedEffects { effects } => effects.clone(),
            _ => panic!("fixture"),
        };
        let mut duplicate = effects[0].clone();
        duplicate.external_effect_id = "second-commit".into();
        let mut other = effects[0].clone();
        match field {
            "provider" => other.provider = "other".into(),
            "operation" => other.operation = "other".into(),
            _ => other.idempotency_identity = "other".into(),
        }
        effects.push(other);
        f.run.evidence[0].observation = Observation::CommittedEffects {
            effects: effects.clone(),
        };
        f.run.evidence[0].integrity_hash = canonical_hash(&f.run.evidence[0].observation).unwrap();
        assert_eq!(evaluate(&f.policy, &f.run).verdict, Verdict::Inconclusive);
        effects.push(duplicate);
        f.run.evidence[0].observation = Observation::CommittedEffects { effects };
        f.run.evidence[0].integrity_hash = canonical_hash(&f.run.evidence[0].observation).unwrap();
        f.run.coverage.get_mut("state").unwrap().status = ObservationCoverage::Partial;
        let result = evaluate(&f.policy, &f.run);
        assert_eq!(result.verdict, Verdict::Fail);
        assert_eq!(result.counterexamples[0].observed, json!(2));
        assert_eq!(result.counterexamples[0].evidence_refs, vec!["e-1"]);
    }
}

#[test]
fn unsupported_correlation_lists_are_errors() {
    for fields in [
        vec![],
        vec!["provider"],
        vec!["idempotency_identity", "customer_id"],
        vec!["idempotency_identity", "idempotency_identity"],
    ] {
        let mut f = fixture("sideeffect-pass.json");
        let ProductContract::Sideeffect { effect } = &mut f.policy.plan.product_contract else {
            panic!("fixture")
        };
        effect.identity.correlation_fields = fields.into_iter().map(str::to_owned).collect();
        bind(&mut f);
        assert_eq!(evaluate(&f.policy, &f.run).verdict, Verdict::Error);
    }
}

#[test]
fn fixed_approvals_do_not_auto_authorize_new_values() {
    for name in ["pass.json", "blindtest-pass.json"] {
        for value in [json!(false), json!(2), json!(null), json!({"state":true})] {
            let mut f = fixture(name);
            let before = f.policy.approved_checker_bindings.clone();
            f.policy.plan.claims[0].predicate = Predicate::Equals {
                expected: value.clone(),
            };
            f.run.evidence[0].observation = Observation::Value { value };
            f.run.evidence[0].integrity_hash =
                canonical_hash(&f.run.evidence[0].observation).unwrap();
            bind(&mut f);
            let result = evaluate(&f.policy, &f.run);
            assert_eq!(result.verdict, Verdict::Inconclusive);
            assert!(result
                .reasons
                .iter()
                .any(|s| s.contains("artifact-bound approval")));
            assert_eq!(f.policy.approved_checker_bindings, before);
        }
        let mut f = fixture(name);
        f.policy.approved_checker_bindings.clear();
        assert_eq!(evaluate(&f.policy, &f.run).verdict, Verdict::Inconclusive);
    }
}

#[test]
fn binding_cannot_be_reused_for_a_new_artifact_even_if_artifact_is_approved() {
    for name in ["pass.json", "blindtest-pass.json"] {
        let mut f = fixture(name);
        match &mut f.policy.plan.product_contract {
            ProductContract::Behavior { baseline, .. } => {
                baseline.target_revision = "different-baseline".into();
                f.policy
                    .approved_baselines
                    .insert(canonical_hash(baseline).unwrap());
            }
            ProductContract::Blindtest { invariants } => {
                invariants[0].statement = "different contract".into();
                f.policy
                    .approved_invariants
                    .insert(canonical_hash(&invariants[0]).unwrap());
            }
            _ => panic!("fixture"),
        }
        bind(&mut f);
        let result = evaluate(&f.policy, &f.run);
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert!(result
            .reasons
            .iter()
            .any(|s| s.contains("artifact-bound approval")));
    }
}

#[test]
fn jcs_primitive_vectors_and_reparse_are_stable() {
    // RFC 8785 section 3.2.2 and Appendix B, including signed zero and subnormals.
    for (number, expected) in [
        (-0.0, "0"),
        (f64::from_bits(1), "5e-324"),
        (f64::MAX, "1.7976931348623157e+308"),
        (1e30, "1e+30"),
        (4.5, "4.5"),
        (0.002, "0.002"),
        (1e-27, "1e-27"),
        (2_f64.powi(60), "1152921504606847000"),
    ] {
        let bytes = canonical_bytes(&number).unwrap();
        assert_eq!(bytes, expected.as_bytes());
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical_bytes(&parsed).unwrap(), bytes);
    }
    let left = json!({"nested":[1,-0.0,{"a":2.0}]});
    let right = json!({"nested":[1.0,0,{"a":2}]});
    assert!(canonical_equal(&left, &right).unwrap());
    assert_eq!(
        canonical_hash(&left).unwrap(),
        canonical_hash(&right).unwrap()
    );
    assert!(!canonical_equal(&json!(true), &json!(1)).unwrap());
}

#[test]
fn nonfinite_numbers_are_rejected_recursively_before_json_conversion() {
    #[derive(Serialize)]
    struct Nested {
        values: Vec<Option<f64>>,
    }
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(canonical_hash(&number).is_err());
        assert!(canonical_hash(&vec![number]).is_err());
        assert!(canonical_hash(&Nested {
            values: vec![Some(number)]
        })
        .is_err());
        assert!(canonical_hash(&std::collections::BTreeMap::from([("x", number)])).is_err());
    }
}

#[test]
fn replay_requires_valid_context_even_when_called_directly() {
    let f = fixture("pass.json");
    let assess = |context: &verify_runner::RunContext| {
        f.run
            .replay_inputs
            .assess(context, &f.policy.plan.required_observers)
    };
    assert_eq!(assess(&f.run.context), Replayability::Available);
    for mutation in 0..7 {
        let mut context = f.run.context.clone();
        match mutation {
            0 => context.observer_versions.clear(),
            1 => context.schema_version = "2".into(),
            2 => context.seed += 1,
            3 => context.experiment_hash = canonical_hash(&"other").unwrap(),
            4 => context.config_hash = canonical_hash(&"other").unwrap(),
            5 => context.target_revision = "HEAD".into(),
            _ => context
                .observer_versions
                .insert("state".into(), "".into())
                .map(|_| ())
                .unwrap(),
        }
        assert!(matches!(
            assess(&context),
            Replayability::Unavailable { .. }
        ));
    }
    for target in ["main", "HEAD", "v1.0.0", "git:abcdef", "sha256:bad"] {
        let mut f = fixture("pass.json");
        f.policy.plan.target_revision = target.into();
        f.run.context.target_revision = target.into();
        f.run.replay_inputs.target_revision = Some(target.into());
        bind(&mut f);
        assert!(matches!(
            evaluate(&f.policy, &f.run).replayability,
            Replayability::Unavailable { .. }
        ));
    }
}

#[test]
fn exact_experiment_metadata_cannot_alias_under_binary64_hashing() {
    assert_eq!(
        canonical_hash(&(1_u64 << 53)).unwrap(),
        canonical_hash(&((1_u64 << 53) + 1)).unwrap()
    );
    for seed in [1_u64 << 53, (1_u64 << 53) + 1, u64::MAX] {
        let mut f = fixture("pass.json");
        f.policy.plan.seed = seed;
        f.run.context.seed = seed;
        f.run.replay_inputs.seed = Some(seed);
        bind(&mut f);
        let result = evaluate(&f.policy, &f.run);
        assert_eq!(result.verdict, Verdict::Error);
        assert!(matches!(
            result.replayability,
            Replayability::Unavailable { .. }
        ));
    }
    let mut f = fixture("pass.json");
    f.run.evidence[0].order = MAX_SAFE_INTEGER + 1;
    assert_eq!(evaluate(&f.policy, &f.run).verdict, Verdict::Error);
}
