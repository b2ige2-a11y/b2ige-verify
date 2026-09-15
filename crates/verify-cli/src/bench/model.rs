use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use verify_core::Verdict;
use verify_evidence::canonical_hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Classification {
    Correct,
    Buggy,
    Nondeterministic,
    Incomplete,
    InfraErrorExpected,
}
macro_rules! model { ($($item:item)*) => {$ (#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)] #[serde(deny_unknown_fields)] $item)*}; }
model! {
    pub struct BenchmarkCase {
        pub schema_version: String,
        pub benchmark_case_id: String,
        pub product: String,
        pub category: String,
        pub expected_classification: Classification,
        pub fixture_identity: String,
        pub config_identity: String,
        pub allowed_verdicts: Vec<Verdict>,
        pub tags: Vec<String>,
        pub timeout_ms: u64,
        pub execution_budget: u64,
        pub notes: String,
    }
    pub struct Rate { pub numerator: usize, pub denominator: usize, pub fraction: Option<f64> }
    pub struct Reproduction {
        pub semantics: String,
        pub exists: bool,
        pub verified: Option<bool>,
        pub replay_available: bool,
        pub replay_reproduced: Option<bool>,
        pub reduction_available: bool,
        pub locally_minimized: Option<bool>,
        pub original_size: Option<usize>,
        pub reduced_size: Option<usize>,
        pub refs: Vec<String>,
        pub unavailable_reason: Option<String>,
    }
    pub struct BenchmarkCaseResult {
        pub schema_version: String,
        pub case: BenchmarkCase,
        pub actual_verdict: Option<Verdict>,
        pub product_outcome: Option<String>,
        pub harness_error: Option<String>,
        pub evidence_valid: Option<bool>,
        pub verified_reload: Option<bool>,
        pub negative_evidence_rejected: Option<bool>,
        pub setup_ms: u64,
        pub execution_ms: u64,
        pub observation_coverage: Rate,
        pub reproduction: Reproduction,
        pub metrics: BTreeMap<String, Rate>,
        pub hidden_leakage: Option<usize>,
        pub agent_leakage: Option<usize>,
        pub result_refs: Vec<String>,
        pub actual_config_hash: Option<String>,
        pub actual_fixture_hash: Option<String>,
        pub limitations: Vec<String>,
    }
    pub struct BenchmarkSummary {
        pub schema_version: String,
        pub total_cases: usize,
        pub verdict_counts: BTreeMap<String, usize>,
        pub false_pass: usize,
        pub false_fail: usize,
        pub mismatches: usize,
        pub rates: BTreeMap<String, Rate>,
        pub setup_ms: u64,
        pub execution_ms: u64,
        pub hidden_leakage: usize,
        pub agent_leakage: usize,
        pub gate_pass: bool,
        pub blockers: Vec<String>,
    }
    pub struct BenchmarkRunResult {
        pub schema_version: String,
        pub benchmark_version: String,
        pub tool_version: String,
        pub platform: String,
        pub rust_version: String,
        pub docker_version: Option<String>,
        pub corpus_hash: String,
        pub requested_case_ids: Vec<String>,
        pub complete_release_corpus: bool,
        pub elapsed_ms: u64,
        pub summary: BenchmarkSummary,
        pub products: BTreeMap<String, BenchmarkSummary>,
        pub cases: Vec<BenchmarkCaseResult>,
        pub semantic_hash: String,
        pub limitations: Vec<String>,
    }
}
impl Rate {
    pub fn new(numerator: usize, denominator: usize) -> Self {
        Self {
            numerator,
            denominator,
            fraction: (denominator != 0).then(|| numerator as f64 / denominator as f64),
        }
    }
    pub fn human(&self) -> String {
        if self.denominator == 0 {
            "N/A".into()
        } else {
            format!(
                "{} / {} ({:.1}%)",
                self.numerator,
                self.denominator,
                self.fraction.unwrap_or(0.) * 100.
            )
        }
    }
}
impl Default for Reproduction {
    fn default() -> Self {
        Self {
            semantics: "unavailable".into(),
            exists: false,
            verified: None,
            replay_available: false,
            replay_reproduced: None,
            reduction_available: false,
            locally_minimized: None,
            original_size: None,
            reduced_size: None,
            refs: vec![],
            unavailable_reason: Some("Automatic replay or reduction is unavailable or unmeasured; see the explicit availability fields".into()),
        }
    }
}
impl BenchmarkCaseResult {
    pub fn empty(case: &BenchmarkCase) -> Self {
        Self {
            schema_version: "1".into(),
            case: case.clone(),
            actual_verdict: None,
            product_outcome: None,
            harness_error: None,
            evidence_valid: None,
            verified_reload: None,
            negative_evidence_rejected: None,
            setup_ms: 0,
            execution_ms: 0,
            observation_coverage: Rate::new(0, 0),
            reproduction: Reproduction::default(),
            metrics: BTreeMap::new(),
            hidden_leakage: None,
            agent_leakage: None,
            result_refs: vec![],
            actual_config_hash: None,
            actual_fixture_hash: None,
            limitations: vec![],
        }
    }
}
pub fn summarize(expected: &[BenchmarkCase], rows: &[BenchmarkCaseResult]) -> BenchmarkSummary {
    let mut s = BenchmarkSummary {
        schema_version: "1".into(),
        total_cases: rows.len(),
        verdict_counts: ["PASS", "FAIL", "INCONCLUSIVE", "ERROR"]
            .map(|x| (x.into(), 0))
            .into(),
        false_pass: 0,
        false_fail: 0,
        mismatches: 0,
        rates: BTreeMap::new(),
        setup_ms: 0,
        execution_ms: 0,
        hidden_leakage: 0,
        agent_leakage: 0,
        gate_pass: false,
        blockers: vec![],
    };
    let inventory: BTreeSet<_> = expected.iter().map(|c| &c.benchmark_case_id).collect();
    let seen: BTreeSet<_> = rows.iter().map(|r| &r.case.benchmark_case_id).collect();
    if expected.is_empty() {
        s.blockers.push("zero-case suite".into());
    }
    if inventory.len() != expected.len() || seen.len() != rows.len() {
        s.blockers.push("duplicate case identity/result".into());
    }
    if seen != inventory {
        s.blockers
            .push("missing or unexpected case result: partial execution".into());
    }
    let mut bug = 0;
    let mut killed = 0;
    let mut correct = 0;
    let mut accepted = 0;
    let mut noncorrect = 0;
    for r in rows {
        let Some(c) = expected
            .iter()
            .find(|c| c.benchmark_case_id == r.case.benchmark_case_id)
        else {
            continue;
        };
        if &r.case != c {
            s.blockers
                .push(format!("{}: case definition tampered", c.benchmark_case_id));
        }
        if let Some(v) = r.actual_verdict {
            *s.verdict_counts
                .get_mut(serde_json::to_value(v).unwrap().as_str().unwrap())
                .unwrap() += 1;
        }
        if c.expected_classification == Classification::Buggy {
            bug += 1;
            killed += usize::from(r.actual_verdict == Some(Verdict::Fail));
        }
        if c.expected_classification == Classification::Correct {
            correct += 1;
            accepted += usize::from(r.actual_verdict == Some(Verdict::Pass));
            s.false_fail += usize::from(r.actual_verdict == Some(Verdict::Fail));
        } else {
            noncorrect += 1;
            s.false_pass += usize::from(r.actual_verdict == Some(Verdict::Pass));
        }
        let mismatch = r
            .actual_verdict
            .is_none_or(|v| !c.allowed_verdicts.contains(&v));
        s.mismatches += usize::from(mismatch);
        if mismatch
            || r.harness_error.is_some()
            || r.evidence_valid == Some(false)
            || r.verified_reload != Some(true)
            || r.negative_evidence_rejected == Some(false)
            || (r.evidence_valid.is_none() && r.negative_evidence_rejected != Some(true))
        {
            s.blockers.push(format!(
                "{}: mismatch, incomplete execution, or invalid evidence",
                c.benchmark_case_id
            ));
        }
        if c.product == "blindtest"
            && r.actual_verdict
                .is_some_and(|v| v == Verdict::Pass || v == Verdict::Fail)
            && (r.hidden_leakage.is_none() || r.agent_leakage.is_none())
        {
            s.blockers.push(format!(
                "{}: leakage measurement missing",
                c.benchmark_case_id
            ));
        }
        let mut required = vec![];
        if c.tags.iter().any(|t| t == "noise") {
            required.push("noise_handling");
        }
        if c.tags.iter().any(|t| t == "generator") {
            required.extend(["deterministic_generation", "generator_bug_discovery"]);
        }
        if c.tags.iter().any(|t| t == "reducer") {
            required.push("reduction_success");
        }
        if c.product == "sideeffect" {
            required.extend([
                "requested_fault_execution_coverage",
                "committed_effect_confirmation_coverage",
            ]);
        }
        if c.category == "duplicate" {
            required.push("duplicate_detection");
        }
        if c.category == "relationship" || c.category == "lost" {
            required.push("relationship_or_lost_detection");
        }
        if c.benchmark_case_id == "blindtest.correct" {
            required.push("suite_quality_receipt_accuracy");
        }
        if c.category == "known_mutant" {
            required.push("Mutation adequacy on benchmark corpus");
        }
        if required
            .iter()
            .any(|k| r.metrics.get(*k).is_none_or(|rate| rate.denominator == 0))
        {
            s.blockers.push(format!(
                "{}: required measurement missing",
                c.benchmark_case_id
            ));
        }
        if r.actual_verdict == Some(Verdict::Fail)
            && (!r.reproduction.exists || r.reproduction.verified != Some(true))
        {
            s.blockers.push(format!(
                "{}: corpus failure reproduction not verified",
                c.benchmark_case_id
            ));
        }
        for (k, v) in &r.metrics {
            let e = s.rates.entry(k.clone()).or_insert_with(|| Rate::new(0, 0));
            *e = Rate::new(e.numerator + v.numerator, e.denominator + v.denominator);
        }
        s.setup_ms += r.setup_ms;
        s.execution_ms += r.execution_ms;
        s.hidden_leakage += r.hidden_leakage.unwrap_or(0);
        s.agent_leakage += r.agent_leakage.unwrap_or(0);
    }
    let bool_rate = |f: fn(&BenchmarkCaseResult) -> Option<bool>| {
        let values: Vec<_> = rows.iter().filter_map(f).collect();
        Rate::new(values.iter().filter(|x| **x).count(), values.len())
    };
    for (k, v) in [
        ("false_pass", Rate::new(s.false_pass, noncorrect)),
        ("false_fail", Rate::new(s.false_fail, correct)),
        ("true_bug_detection", Rate::new(killed, bug)),
        ("correct_acceptance", Rate::new(accepted, correct)),
        (
            "inconclusive",
            Rate::new(s.verdict_counts["INCONCLUSIVE"], rows.len()),
        ),
        ("error", Rate::new(s.verdict_counts["ERROR"], rows.len())),
        ("evidence_validity", bool_rate(|r| r.evidence_valid)),
        ("verified_reload", bool_rate(|r| r.verified_reload)),
        (
            "negative_evidence_rejection",
            bool_rate(|r| r.negative_evidence_rejected),
        ),
        (
            "verified_reproduction",
            bool_rate(|r| r.reproduction.verified),
        ),
        (
            "deterministic_replay",
            bool_rate(|r| r.reproduction.replay_reproduced),
        ),
        (
            "local_minimization",
            bool_rate(|r| r.reproduction.locally_minimized),
        ),
        (
            "minimal_reproduction_availability",
            Rate::new(
                rows.iter()
                    .filter(|r| r.reproduction.locally_minimized == Some(true))
                    .count(),
                rows.iter()
                    .filter(|r| r.actual_verdict == Some(Verdict::Fail))
                    .count(),
            ),
        ),
        (
            "observation_coverage",
            Rate::new(
                rows.iter().map(|r| r.observation_coverage.numerator).sum(),
                rows.iter()
                    .map(|r| r.observation_coverage.denominator)
                    .sum(),
            ),
        ),
    ] {
        s.rates.insert(k.into(), v);
    }
    for k in [
        "noise_handling",
        "deterministic_generation",
        "generator_bug_discovery",
        "reduction_success",
        "duplicate_detection",
        "relationship_or_lost_detection",
        "suite_quality_receipt_accuracy",
    ] {
        if s.rates.get(k).is_some_and(|v| v.numerator != v.denominator) {
            s.blockers
                .push(format!("{k}: required corpus capability failed"));
        }
    }
    let count_rate = |select: fn(&BenchmarkCase) -> bool, verdict: Verdict| {
        let selected: Vec<_> = rows.iter().filter(|r| select(&r.case)).collect();
        Rate::new(
            selected
                .iter()
                .filter(|r| r.actual_verdict == Some(verdict))
                .count(),
            selected.len(),
        )
    };
    for (name, rate) in [
        (
            "known_bug_false_pass",
            count_rate(
                |c| c.expected_classification == Classification::Buggy,
                Verdict::Pass,
            ),
        ),
        (
            "known_unsafe_false_pass",
            count_rate(
                |c| c.product == "sideeffect" && c.expected_classification == Classification::Buggy,
                Verdict::Pass,
            ),
        ),
        (
            "missing_evidence_pass",
            count_rate(
                |c| c.expected_classification == Classification::Incomplete,
                Verdict::Pass,
            ),
        ),
        (
            "partial_hidden_suite_pass",
            count_rate(
                |c| c.product == "blindtest" && c.category == "partial_suite",
                Verdict::Pass,
            ),
        ),
        (
            "isolation_unattested_pass",
            count_rate(
                |c| c.product == "blindtest" && c.category == "isolation_unavailable",
                Verdict::Pass,
            ),
        ),
        (
            "safe_control_false_fail",
            count_rate(
                |c| c.expected_classification == Classification::Correct,
                Verdict::Fail,
            ),
        ),
        (
            "correct_blindtest_false_fail",
            count_rate(
                |c| {
                    c.product == "blindtest" && c.expected_classification == Classification::Correct
                },
                Verdict::Fail,
            ),
        ),
    ] {
        s.rates.insert(name.into(), rate);
    }
    if s.false_pass + s.false_fail + s.hidden_leakage + s.agent_leakage > 0 {
        s.blockers
            .push("false verdict or leakage release invariant violated".into());
    }
    s.gate_pass = s.blockers.is_empty();
    s
}
/// Deliberately excludes durations, random hidden identities, paths and evidence hashes.
/// Case/source identities and every measured semantic result remain in comparison.
pub fn semantic_hash(rows: &[BenchmarkCaseResult]) -> std::io::Result<String> {
    let mut values = BTreeMap::new();
    for r in rows {
        let mut v = serde_json::to_value(r)?;
        for k in [
            "setup_ms",
            "execution_ms",
            "result_refs",
            "actual_config_hash",
            "actual_fixture_hash",
        ] {
            v.as_object_mut().unwrap().remove(k);
        }
        v["reproduction"]["refs"] = serde_json::json!([]);
        values.insert(&r.case.benchmark_case_id, v);
    }
    Ok(canonical_hash(&values)?)
}
pub fn schema<T: JsonSchema>() -> serde_json::Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    v["properties"]["schema_version"]["const"] = "1".into();
    v
}
