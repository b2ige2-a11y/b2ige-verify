//! V100-0 trust semantics. These tests pin shared authoritative boundaries;
//! they must fail if a weakening turns uncertainty, untrusted material, or
//! unverifiable artifacts into a successful verdict.

#[path = "support/blindtest.rs"]
mod blindtest_support;
#[path = "../../../tests/conformance/support.rs"]
#[allow(dead_code)]
mod conformance;

use conformance::{blindtest, case, rebind, set_value};
use serde_json::json;
use std::fs;
use verify_core::{
    blindtest::{execute, read_suite, validate_suite},
    evaluate, run_result, InvariantStatus, ProductContract, Verdict,
};
use verify_evidence::{
    canonical_hash, store::EvidenceStore, Evidence, Observation, ObservationCoverage, TrustClass,
};
use verify_runner::ExecutionStatus;

#[test]
fn missing_verdict_critical_evidence_cannot_pass() {
    for status in [
        ObservationCoverage::Partial,
        ObservationCoverage::Unavailable,
        ObservationCoverage::Failed,
    ] {
        let (policy, mut run) = case();
        run.coverage.get_mut("state").unwrap().status = status;
        assert_ne!(evaluate(&policy, &run).verdict, Verdict::Pass);
    }

    let (policy, mut run) = case();
    run.coverage.clear();
    assert_ne!(evaluate(&policy, &run).verdict, Verdict::Pass);

    let (policy, mut run) = case();
    run.evidence.clear();
    assert_ne!(evaluate(&policy, &run).verdict, Verdict::Pass);
}

#[test]
fn required_checker_and_observer_failures_cannot_pass() {
    let (policy, mut run) = case();
    run.completed_checkers.clear();
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Inconclusive);

    let (policy, mut run) = case();
    run.execution = ExecutionStatus::RunnerCrash;
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Error);

    let (policy, mut run) = case();
    run.execution = ExecutionStatus::ObserverInitializationFailure;
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Error);

    let (policy, mut run) = case();
    run.coverage.get_mut("state").unwrap().status = ObservationCoverage::Failed;
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Inconclusive);
}

#[test]
fn advisory_and_unapproved_llm_material_have_no_verdict_authority() {
    for trust_class in [TrustClass::Derived, TrustClass::Advisory] {
        for value in [true, false] {
            let (policy, mut run) = case();
            run.evidence[0].trust_class = trust_class;
            set_value(&mut run, json!(value));
            assert_eq!(evaluate(&policy, &run).verdict, Verdict::Inconclusive);
        }
    }

    // LLM candidates may be promoted only by an independently approved,
    // artifact-bound authority. A raw candidate/rejection cannot decide.
    for status in [InvariantStatus::Candidate, InvariantStatus::Rejected] {
        let (mut policy, mut run) = blindtest();
        if let ProductContract::Blindtest { invariants } = &mut policy.plan.product_contract {
            invariants[0].authority.status = status;
        }
        rebind(&mut policy, &mut run);
        for value in [true, false] {
            set_value(&mut run, json!(value));
            assert_eq!(evaluate(&policy, &run).verdict, Verdict::Inconclusive);
        }
    }
}

#[test]
fn integrity_and_identity_mismatch_cannot_silently_pass() {
    let (policy, mut run) = case();
    run.evidence[0].integrity_hash = "sha256:corrupt".into();
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Error);

    let (policy, mut run) = case();
    run.evidence[0].run_id = "older-run".into();
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Error);

    let (policy, mut run) = case();
    run.evidence[0].source = "other-observer".into();
    assert_ne!(evaluate(&policy, &run).verdict, Verdict::Pass);

    let (policy, mut run) = case();
    run.context.target_revision = "other-target".into();
    assert_eq!(evaluate(&policy, &run).verdict, Verdict::Error);
}

#[test]
fn hidden_suite_absence_or_corruption_cannot_validate() {
    let corpus = blindtest_support::Corpus::temporary();
    fs::remove_file(corpus.sealed.join("suite.json")).unwrap();
    assert!(read_suite(&corpus.sealed).is_err());
    assert!(execute(
        &corpus.config,
        &corpus.sealed,
        &corpus.store,
        "missing-suite"
    )
    .is_err());

    let corpus = blindtest_support::Corpus::temporary();
    fs::write(corpus.sealed.join("suite.json"), b"corrupt hidden suite").unwrap();
    assert!(read_suite(&corpus.sealed).is_err());
    assert!(execute(
        &corpus.config,
        &corpus.sealed,
        &corpus.store,
        "corrupt-suite"
    )
    .is_err());

    let mut corpus = blindtest_support::Corpus::temporary();
    corpus.suite.cases[0].oracle = None;
    assert!(validate_suite(&corpus.suite, &corpus.config).is_err());

    let mut corpus = blindtest_support::Corpus::temporary();
    corpus.suite.manifest.case_hashes.clear();
    assert!(validate_suite(&corpus.suite, &corpus.config).is_err());
}

#[test]
fn bounded_output_cannot_claim_exhaustive_correctness() {
    let (policy, run) = case();
    let evaluation = evaluate(&policy, &run);
    assert_eq!(evaluation.verdict, Verdict::Pass);
    assert!(evaluation
        .limitations
        .iter()
        .any(|limitation| limitation.contains("not exhaustive")));

    let result = run_result(&policy, &run).unwrap();
    assert!(result["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|limitation| limitation.as_str().unwrap().contains("not exhaustive")));
    assert_eq!(result["scope"]["exploration_budget"]["cases"], json!(1));
}

fn committed_store() -> (tempfile::TempDir, EvidenceStore) {
    let directory = tempfile::tempdir().unwrap();
    let store = EvidenceStore::new(directory.path().join("runs"));
    let run = store.reserve("run").unwrap();
    let observation = Observation::Value { value: json!(true) };
    let evidence = Evidence {
        evidence_id: "evidence".into(),
        run_id: "run".into(),
        source: "trusted-observer".into(),
        trust_class: TrustClass::DirectRuntime,
        order: 0,
        integrity_hash: canonical_hash(&observation).unwrap(),
        observation,
        related_claim_ids: vec!["claim".into()],
    };
    run.write_evidence(&evidence).unwrap();
    run.complete(&json!({"verdict": "PASS"}), &["evidence".into()])
        .unwrap();
    (directory, store)
}

#[test]
fn authoritative_store_loader_fails_closed_on_missing_or_corrupt_artifacts() {
    let (directory, store) = committed_store();
    fs::remove_file(directory.path().join("runs/run/result.json")).unwrap();
    assert!(store.load("run").is_err());

    let (directory, store) = committed_store();
    fs::write(
        directory.path().join("runs/run/evidence/evidence.json"),
        b"corrupt evidence",
    )
    .unwrap();
    assert!(store.load("run").is_err());

    let (directory, store) = committed_store();
    fs::write(
        directory.path().join("runs/run/result.json"),
        b"corrupt commit",
    )
    .unwrap();
    assert!(store.load("run").is_err());
}
