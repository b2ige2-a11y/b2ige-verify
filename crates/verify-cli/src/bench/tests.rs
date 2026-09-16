use super::*;
fn row(c: &BenchmarkCase, v: Verdict) -> BenchmarkCaseResult {
    let mut r = BenchmarkCaseResult::empty(c);
    r.actual_verdict = Some(v);
    r.evidence_valid = Some(true);
    r.verified_reload = Some(true);
    r
}
#[test]
fn zero_denominator_is_na() {
    assert_eq!(Rate::new(0, 0).fraction, None);
    assert_eq!(Rate::new(0, 0).human(), "N/A");
    assert_eq!(Rate::new(0, 2).fraction, Some(0.));
}
#[test]
fn zero_cases_cannot_pass() {
    assert!(!summarize(&[], &[]).gate_pass);
}
#[test]
fn missing_case_cannot_pass() {
    let c = catalog().unwrap();
    assert!(!summarize(&c, &[]).gate_pass);
}
#[test]
fn duplicate_case_result_is_error() {
    let c = catalog().unwrap().remove(0);
    let r = row(&c, Verdict::Pass);
    let s = summarize(&[c], &[r.clone(), r]);
    assert!(!s.gate_pass);
    assert!(s.blockers.iter().any(|s| s.contains("duplicate")));
}
#[test]
fn partial_execution_cannot_pass() {
    let c = catalog().unwrap();
    assert!(!summarize(&c, &[row(&c[0], Verdict::Pass)]).gate_pass);
}
#[test]
fn missing_results_keep_planned_metric_denominators() {
    let c = catalog().unwrap();
    let s = summarize(&c, &[]);
    let count = |classification| {
        c.iter()
            .filter(|case| case.expected_classification == classification)
            .count()
    };
    assert_eq!(s.total_cases, c.len());
    assert_eq!(
        s.rates["false_pass"].denominator,
        c.len() - count(Classification::Correct)
    );
    assert_eq!(
        s.rates["false_fail"].denominator,
        count(Classification::Correct)
    );
    assert_eq!(
        s.rates["true_bug_detection"].denominator,
        count(Classification::Buggy)
    );
    assert_eq!(s.rates["inconclusive"].denominator, c.len());
    assert_eq!(s.rates["error"].denominator, c.len());
    assert!(!s.gate_pass);
}
#[test]
fn duplicate_results_do_not_inflate_measured_counts() {
    let c = catalog()
        .unwrap()
        .into_iter()
        .find(|case| case.expected_classification == Classification::Buggy)
        .unwrap();
    let r = row(&c, Verdict::Pass);
    let s = summarize(std::slice::from_ref(&c), &[r.clone(), r]);
    assert_eq!(s.false_pass, 1);
    assert_eq!(s.rates["false_pass"], Rate::new(1, 1));
    assert!(!s.gate_pass);
}
#[test]
fn known_false_pass_candidate_is_counted() {
    let c = catalog()
        .unwrap()
        .into_iter()
        .find(|c| c.expected_classification == Classification::Buggy)
        .unwrap();
    let s = summarize(std::slice::from_ref(&c), &[row(&c, Verdict::Pass)]);
    assert_eq!(s.false_pass, 1);
    assert!(!s.gate_pass);
}
#[test]
fn correct_false_fail_is_counted() {
    let c = catalog().unwrap().remove(0);
    let s = summarize(std::slice::from_ref(&c), &[row(&c, Verdict::Fail)]);
    assert_eq!(s.false_fail, 1);
    assert!(!s.gate_pass);
}
#[test]
fn error_does_not_count_as_killed_mutant() {
    let c = catalog().unwrap().remove(1);
    let s = summarize(std::slice::from_ref(&c), &[row(&c, Verdict::Error)]);
    assert_eq!(s.rates["true_bug_detection"].numerator, 0);
    assert!(!s.gate_pass);
}
#[test]
fn missing_evidence_cannot_pass() {
    let c = catalog().unwrap().remove(0);
    let mut r = row(&c, Verdict::Pass);
    r.evidence_valid = None;
    assert!(!summarize(&[c], &[r]).gate_pass);
}
#[test]
fn schemas_reject_unknown_and_invalid_enum() {
    let mut r = serde_json::to_value(row(&catalog().unwrap()[0], Verdict::Pass)).unwrap();
    r["actual_verdict"] = "SUCCESS".into();
    assert!(validate(&schema::<BenchmarkCaseResult>(), &r).is_err());
    r["actual_verdict"] = "PASS".into();
    r["surprise"] = true.into();
    assert!(validate(&schema::<BenchmarkCaseResult>(), &r).is_err());
}
#[test]
fn semantic_comparison_ignores_times_and_paths() {
    let c = catalog().unwrap().remove(0);
    let a = row(&c, Verdict::Pass);
    let mut b = a.clone();
    b.execution_ms = 999;
    b.result_refs = vec!["random/path".into()];
    assert_eq!(
        semantic_hash(std::slice::from_ref(&a)).unwrap(),
        semantic_hash(std::slice::from_ref(&b)).unwrap()
    );
    b.actual_verdict = Some(Verdict::Fail);
    assert_ne!(semantic_hash(&[a]).unwrap(), semantic_hash(&[b]).unwrap());
}
#[test]
fn actual_behavior_corpus_and_wrong_label_and_verdict_tamper() {
    let run = execute(&Options {
        product: Some("behavior".into()),
        ..Default::default()
    })
    .unwrap();
    assert!(run.products["behavior"].gate_pass, "{}", human(&run));
    assert_eq!(run.cases.len(), 9);
    assert_eq!(
        run.products["behavior"].rates["generator_bug_discovery"].numerator,
        2
    );
    assert_eq!(
        run.products["behavior"].rates["reduction_success"].numerator,
        1
    );
    let mut changed = run.clone();
    changed.cases[0].actual_verdict = Some(Verdict::Fail);
    changed.semantic_hash = semantic_hash(&changed.cases).unwrap();
    assert_eq!(
        compare(&run, &changed).unwrap()["cases"]["behavior.preserving"],
        "regression"
    );
    assert_eq!(
        compare(&changed, &run).unwrap()["cases"]["behavior.preserving"],
        "improvement"
    );
    let mut timed = run.clone();
    timed.elapsed_ms += 1000;
    assert_eq!(
        compare(&run, &timed).unwrap()["cases"]["behavior.preserving"],
        "unchanged"
    );
    let r = &run.cases[0];
    let mut wrong = r.case.clone();
    wrong.expected_classification = Classification::Buggy;
    wrong.allowed_verdicts = vec![Verdict::Fail];
    let mut rr = r.clone();
    rr.case = wrong.clone();
    let summary = summarize(&[wrong], &[rr]);
    assert!(!summary.gate_pass);
    assert_eq!(summary.false_pass, 1);
    verify_recorded_verdict(r).unwrap();
    let mut tampered = r.clone();
    tampered.actual_verdict = Some(Verdict::Fail);
    assert!(verify_recorded_verdict(&tampered).is_err());
    let path = Path::new(&r.result_refs[0]);
    let mut commit: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let intact = commit.clone();
    commit["manifest"]["result"]["verdict"] = "FAIL".into();
    commit["integrity_hash"] = canonical_hash(&commit["manifest"]).unwrap().into();
    fs::write(path, verify_evidence::canonical_bytes(&commit).unwrap()).unwrap();
    // Behavior rejects a cached verdict inconsistent with its evidence receipt.
    assert!(verify_recorded_verdict(r).is_err());
    assert!(verify_recorded_verdict(&tampered).is_err());
    fs::write(path, verify_evidence::canonical_bytes(&intact).unwrap()).unwrap();
    verify_recorded_verdict(r).unwrap();
    let (store, id) = source(r).unwrap();
    corrupt_evidence(&store, &id).unwrap();
    assert!(verify_recorded_verdict(r).is_err());
}
#[test]
fn labels_are_independent_and_stable() {
    let c = catalog().unwrap();
    assert_eq!(c, catalog().unwrap());
    assert_eq!(c.len(), 31);
    assert!(c.iter().all(|c| c.allowed_verdicts.len() == 1));
}

#[test]
fn negative_fixture_preserves_unexpected_loader_pass() {
    let (verdict, rejected) = loader_observation(Ok(Verdict::Pass));
    assert_eq!(verdict, Verdict::Pass);
    assert!(!rejected);
    let c = catalog()
        .unwrap()
        .into_iter()
        .find(|c| c.benchmark_case_id == "blindtest.missing")
        .unwrap();
    let mut r = row(&c, verdict);
    r.negative_evidence_rejected = Some(rejected);
    let s = summarize(&[c], &[r]);
    assert_eq!(s.false_pass, 1);
    assert!(!s.gate_pass);
    assert_eq!(
        loader_observation(Err(invalid("missing required evidence"))),
        (Verdict::Error, true)
    );
}

#[test]
fn comparison_rejects_versions_and_recomputes_untrusted_hashes() {
    let run: BenchmarkRunResult = serde_json::from_str(include_str!(
        "../../../../benchmarks/baseline-v1/result.json"
    ))
    .unwrap();
    let mut other = run.clone();
    other.benchmark_version = "future".into();
    assert!(compare(&run, &other).is_err());
    other = run.clone();
    other.corpus_hash = "different".into();
    assert!(compare(&run, &other).is_err());
    other = run.clone();
    other.schema_version = "future".into();
    assert!(compare(&run, &other).is_err());
    other = run.clone();
    other.cases[0].actual_verdict = Some(Verdict::Error);
    assert_eq!(compare(&run, &other).unwrap()["semantic_equal"], false);
    other = run.clone();
    other.semantic_hash = "forged".into();
    assert_eq!(compare(&run, &other).unwrap()["semantic_equal"], true);
}
#[test]
fn comparison_rejects_case_identity_tamper() {
    let run: BenchmarkRunResult = serde_json::from_str(include_str!(
        "../../../../benchmarks/baseline-v1/result.json"
    ))
    .unwrap();
    let mut other = run.clone();
    other.cases[0].case.category = "tampered".into();
    assert!(compare(&run, &other).is_err());
}

#[test]
fn comparison_rejects_missing_duplicate_foreign_and_reclassified_rows() {
    let run: BenchmarkRunResult = serde_json::from_str(include_str!(
        "../../../../benchmarks/baseline-v1/result.json"
    ))
    .unwrap();
    validate_snapshot_shape(&run).unwrap();
    let mut variants = vec![run.clone(); 6];
    variants[0].cases.pop();
    variants[1].cases.push(run.cases[0].clone());
    variants[2].cases[0].case.benchmark_case_id = "foreign".into();
    variants[3].cases[0].case.expected_classification = Classification::Incomplete;
    variants[4].requested_case_ids.pop();
    variants[5].cases[0].actual_config_hash = None;
    for invalid in variants {
        assert!(compare(&run, &invalid).is_err());
    }
}

#[test]
fn late_harness_failure_remains_serializable_and_blocks_success_counts() {
    let mut run: BenchmarkRunResult = serde_json::from_str(include_str!(
        "../../../../benchmarks/baseline-v1/result.json"
    ))
    .unwrap();
    let r = run
        .cases
        .iter_mut()
        .find(|r| r.actual_verdict == Some(Verdict::Pass))
        .unwrap();
    let refs = r.result_refs.clone();
    harness_failure(r, invalid("required follow-up verification failed"));
    assert_eq!(r.result_refs, refs);
    assert!(r.actual_verdict.is_none());
    assert!(r.evidence_valid.is_none());
    assert!(r.verified_reload.is_none());
    assert!(verify_recorded_verdict(r).is_err());
    validate_snapshot_shape(&run).unwrap();
    let s = summarize(&catalog().unwrap(), &run.cases);
    assert!(!s.gate_pass);
    assert_eq!(s.rates["measured_verdicts"], Rate::new(30, 31));
    assert_eq!(s.rates["correct_acceptance"], Rate::new(6, 7));
}

#[test]
fn qualification_each_public_case_missing_or_unverified_blocks_gate() {
    // Historical rows are input vectors for the measurement controller only.
    // This does not rerun products or turn the snapshot into fresh proof.
    let run: BenchmarkRunResult = serde_json::from_str(include_str!(
        "../../../../benchmarks/baseline-v1/result.json"
    ))
    .unwrap();
    let expected = catalog().unwrap();
    let control = summarize(&expected, &run.cases);
    assert!(control.gate_pass);
    let mut reversed = run.cases.clone();
    reversed.reverse();
    assert_eq!(summarize(&expected, &reversed), control);
    for index in 0..run.cases.len() {
        for mutation in ["omit", "unmeasured", "unverified", "harness_error"] {
            let mut rows = run.cases.clone();
            match mutation {
                "omit" => {
                    rows.remove(index);
                }
                "unmeasured" => rows[index].actual_verdict = None,
                "unverified" => rows[index].verified_reload = None,
                _ => harness_failure(&mut rows[index], invalid("qualification failure")),
            }
            let summary = summarize(&expected, &rows);
            assert!(
                !summary.gate_pass,
                "{}: {mutation}",
                run.cases[index].case.benchmark_case_id
            );
            assert_eq!(summary.rates["false_pass"].denominator, 24);
            assert_eq!(summary.rates["false_fail"].denominator, 7);
            assert_eq!(summary.rates["measured_verdicts"].denominator, 31);
        }
    }
}
