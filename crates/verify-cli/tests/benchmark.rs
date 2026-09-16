use std::process::Command;
use verify_cli::bench::{verify_recorded_verdict, BenchmarkRunResult};
fn run(product: &str) -> BenchmarkRunResult {
    let output = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["bench", product, "--output", "json"])
        .output()
        .unwrap();
    let r: BenchmarkRunResult = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stdout)));
    assert!(output.status.success(), "{}", verify_cli::bench::human(&r));
    assert!(r.products[product].gate_pass);
    assert!(
        !r.summary.gate_pass,
        "A product subset must not grant the full release gate"
    );
    for c in &r.cases {
        verify_recorded_verdict(c).unwrap();
    }
    r
}
#[test]
fn actual_sqlite_benchmark_corpus() {
    let r = run("sideeffect");
    assert_eq!(r.cases.len(), 13);
    assert_eq!(
        r.products["sideeffect"].rates["duplicate_detection"].numerator,
        3
    );
    assert_eq!(
        r.products["sideeffect"].rates["true_bug_detection"].numerator,
        6
    );
}
#[test]
fn actual_docker_benchmark_corpus() {
    let r = run("blindtest");
    assert_eq!(r.cases.len(), 9);
    assert_eq!(
        r.products["blindtest"].rates["Mutation adequacy on benchmark corpus"].numerator,
        2
    );
    assert_eq!(r.summary.agent_leakage, 0);
    assert_eq!(r.summary.hidden_leakage, 0);
}
#[test]
fn empty_selection_is_not_success() {
    let output = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["bench", "--case", "not-a-case", "--output", "json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let r: BenchmarkRunResult = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!r.summary.gate_pass);
    assert_eq!(r.summary.total_cases, 0);
}

#[test]
fn unavailable_docker_is_reported_without_fabricated_measurements() {
    let output = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .env("B2IGE_DOCKER", "/nonexistent/b2ige-proof-docker")
        .args(["bench", "blindtest", "--output", "json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let r: BenchmarkRunResult = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(r.cases.len(), 9);
    assert!(!r.summary.gate_pass);
    let summary = &r.products["blindtest"];
    assert!(!summary.gate_pass);
    assert_eq!(summary.rates["measured_verdicts"].numerator, 0);
    assert_eq!(summary.rates["measured_verdicts"].denominator, 9);
    assert_eq!(summary.rates["true_bug_detection"].denominator, 3);
    for name in ["hidden_leakage_measurement", "agent_leakage_measurement"] {
        assert_eq!(summary.rates[name].numerator, 0);
        assert_eq!(summary.rates[name].denominator, 9);
    }
    for case in &r.cases {
        assert!(case.actual_verdict.is_none());
        assert!(case.evidence_valid.is_none());
        assert!(case.hidden_leakage.is_none());
        assert!(case.agent_leakage.is_none());
        assert!(case
            .harness_error
            .as_ref()
            .unwrap()
            .contains("local engine unavailable"));
        assert!(verify_recorded_verdict(case).is_err());
    }
}
