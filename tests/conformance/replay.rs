use super::support::*;
use verify_core::*;
use verify_replay::*;

#[test]
fn rp_001_all_controllable_inputs_required() {
    let (p, r) = case();
    assert_eq!(evaluate(&p, &r).replayability, Replayability::Available);
    for field in 0..6 {
        let mut r = r.clone();
        match field {
            0 => r.replay_inputs.seed = None,
            1 => r.replay_inputs.experiment_plan = None,
            2 => r.replay_inputs.config_hash = None,
            3 => r.replay_inputs.fault_schedule = None,
            4 => r.replay_inputs.target_revision = None,
            _ => r.replay_inputs.config = None,
        }
        assert!(
            matches!(evaluate(&p, &r).replayability, Replayability::Unavailable { reason } if !reason.is_empty())
        );
    }
}
#[test]
fn rp_002_mismatched_inputs_are_not_replayable() {
    let (p, r) = case();
    for field in 0..5 {
        let mut r = r.clone();
        match field {
            0 => r.replay_inputs.seed = Some(0),
            1 => r.replay_inputs.experiment_plan = Some(serde_json::json!({"wrong":true})),
            2 => r.replay_inputs.target_revision = Some("wrong".into()),
            3 => r.replay_inputs.fault_schedule = Some(vec![serde_json::json!("injected")]),
            _ => r.replay_inputs.config = Some(serde_json::json!({"changed":true})),
        }
        assert!(matches!(
            evaluate(&p, &r).replayability,
            Replayability::Unavailable { .. }
        ));
    }
}
#[test]
fn rp_003_fail_retains_evidence_and_unavailable_reason() {
    let (p, mut r) = case();
    set_value(&mut r, serde_json::json!(false));
    r.replay_inputs.seed = None;
    let result = evaluate(&p, &r);
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(
        matches!(&result.counterexamples[0].replayability, Replayability::Unavailable { reason } if !reason.is_empty())
    );
    assert!(!result.counterexamples[0].replay_reference.is_empty());
}
#[test]
fn rp_004_invalid_run_identity_is_error() {
    let (p, r) = case();
    for field in 0..6 {
        let mut r = r.clone();
        match field {
            0 => r.context.tool_version.clear(),
            1 => r.context.experiment_hash.clear(),
            2 => r.context.schema_version = "2".into(),
            3 => r.context.target_revision = "unrelated".into(),
            4 => r.context.observer_versions.clear(),
            _ => r.context.seed = 99,
        }
        assert_eq!(evaluate(&p, &r).verdict, Verdict::Error);
    }
}
