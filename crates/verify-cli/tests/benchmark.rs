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
