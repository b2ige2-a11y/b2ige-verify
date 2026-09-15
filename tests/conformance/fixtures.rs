use super::support::*;
use verify_core::*;

fn schema(name: &str) -> jsonschema::Validator {
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root().join("schemas").join(name)).unwrap())
            .unwrap();
    jsonschema::validator_for(&value).unwrap()
}
fn validate(value: &serde_json::Value) -> Result<(), String> {
    schema("conformance-fixture.schema.json")
        .validate(value)
        .map_err(|e| e.to_string())?;
    let fixture: ConformanceFixture =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let (name, product) = match &fixture.policy.plan.product_contract {
        ProductContract::Behavior { baseline, .. } => (
            "behavior-baseline.schema.json",
            serde_json::to_value(baseline).unwrap(),
        ),
        ProductContract::Sideeffect { effect } => (
            "sideeffect-effect.schema.json",
            serde_json::to_value(effect).unwrap(),
        ),
        ProductContract::Blindtest { invariants } => {
            for invariant in invariants {
                schema("blindtest-invariant.schema.json")
                    .validate(&serde_json::to_value(invariant).unwrap())
                    .map_err(|e| e.to_string())?;
            }
            (
                "blindtest-invariant.schema.json",
                serde_json::to_value(&invariants[0]).unwrap(),
            )
        }
    };
    schema(name).validate(&product).map_err(|e| e.to_string())?;
    let result = run_result(&fixture.policy, &fixture.run).map_err(str::to_string)?;
    schema("run-result.schema.json")
        .validate(&result)
        .map_err(|e| e.to_string())?;
    validate_fixture(&fixture).map_err(str::to_string)
}
fn load(name: &str) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures").join(name)).unwrap(),
    )
    .unwrap()
}
#[test]
fn json_001_golden_verdicts_validate_against_schemas_and_checker() {
    for (name, verdict) in [
        ("pass.json", "PASS"),
        ("fail.json", "FAIL"),
        ("inconclusive.json", "INCONCLUSIVE"),
        ("error.json", "ERROR"),
        ("sideeffect-pass.json", "PASS"),
        ("blindtest-pass.json", "PASS"),
    ] {
        let value = load(name);
        assert_eq!(value["expected"]["verdict"], verdict);
        validate(&value).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}
#[test]
fn json_002_adversarial_golden_fixtures_are_rejected() {
    for name in [
        "invalid-pass-missing-evidence.json",
        "invalid-pass-incomplete-coverage.json",
        "invalid-sideeffect-attempt-as-effect.json",
        "invalid-blindtest-unapproved-invariant.json",
    ] {
        let value = load(name);
        // These are intentionally structurally valid; semantic validation must reject them.
        assert!(schema("conformance-fixture.schema.json").is_valid(&value));
        assert!(
            validate(&value).is_err(),
            "{name} escaped the contract checker"
        );
    }
}
#[test]
fn json_003_unknown_fields_missing_metadata_invalid_enums_rejected() {
    for field in [
        "run_id",
        "schema_version",
        "tool_version",
        "seed",
        "target_revision",
        "config_hash",
        "plan_hash",
        "experiment_hash",
        "observer_versions",
    ] {
        let mut value = load("pass.json");
        value["run"]["context"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(validate(&value).is_err(), "missing {field}");
    }
    let mut value = load("pass.json");
    value["run"]["unknown"] = serde_json::json!(true);
    assert!(validate(&value).is_err());
    let mut value = load("pass.json");
    value["run"]["coverage"]["state"]["status"] = serde_json::json!("almost_complete");
    assert!(validate(&value).is_err());
}
#[test]
fn json_004_rust_schema_drift_is_detected() {
    let committed: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("schemas/conformance-fixture.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        conformance_schema(),
        committed,
        "schema-visible changes require a version decision"
    );
}
#[test]
fn json_005_legacy_schemas_reject_invalid_documents() {
    for name in [
        "run-result.schema.json",
        "behavior-baseline.schema.json",
        "sideeffect-effect.schema.json",
        "blindtest-invariant.schema.json",
    ] {
        assert!(!schema(name).is_valid(&serde_json::json!({})));
    }
    let mut value = load("pass.json");
    value["policy"]["plan"]["product_contract"]["baseline"]["created_by"] =
        serde_json::json!("agent");
    assert!(validate(&value).is_err());
}
#[test]
fn json_006_forged_replayability_and_isolation_claims_rejected() {
    let mut value = load("pass.json");
    value["run"]["replay_inputs"]["seed"] = serde_json::Value::Null;
    assert!(validate(&value).is_err());
    let mut value = load("pass.json");
    value["expected"]["isolation_claim"] = serde_json::json!("hardened_linux");
    assert!(validate(&value).is_err());
}
#[test]
fn json_007_invalid_identity_cannot_emit_schema_invalid_result() {
    let (p, mut r) = case();
    r.context.run_id.clear();
    assert!(run_result(&p, &r).is_err());
}
