//! Explicit maintenance utility for synthetic fixtures, never a Behavior baseline updater.
#[path = "../../../tests/conformance/support.rs"]
mod support;
use support::*;
use verify_core::*;
use verify_evidence::*;
use verify_runner::*;
fn write(name: &str, p: TrustedPolicy, r: Run, forged_pass: bool) {
    let mut expected = evaluate(&p, &r);
    if forged_pass {
        expected.verdict = Verdict::Pass;
    }
    let f = ConformanceFixture {
        harness_schema_version: "1".into(),
        policy: p,
        run: r,
        expected,
    };
    std::fs::write(
        root().join("tests/fixtures").join(name),
        serde_json::to_string_pretty(&f).unwrap() + "\n",
    )
    .unwrap();
}
fn main() {
    std::fs::write(
        root().join("schemas/conformance-fixture.schema.json"),
        serde_json::to_string_pretty(&conformance_schema()).unwrap() + "\n",
    )
    .unwrap();
    if std::env::args().any(|arg| arg == "--schema-only") {
        return;
    }
    let (p, r) = case();
    write("pass.json", p, r, false);
    let (p, mut r) = case();
    set_value(&mut r, serde_json::json!(false));
    write("fail.json", p, r, false);
    let (p, mut r) = case();
    r.evidence.clear();
    write("inconclusive.json", p, r, false);
    let (p, mut r) = case();
    r.execution = ExecutionStatus::RunnerCrash;
    write("error.json", p, r, false);
    let (p, r) = sideeffect();
    write("sideeffect-pass.json", p, r, false);
    let (p, r) = blindtest();
    write("blindtest-pass.json", p, r, false);
    let (p, mut r) = case();
    r.evidence.clear();
    write("invalid-pass-missing-evidence.json", p, r, true);
    let (p, mut r) = case();
    r.coverage.get_mut("state").unwrap().status = ObservationCoverage::Partial;
    write("invalid-pass-incomplete-coverage.json", p, r, true);
    let (p, mut r) = sideeffect();
    r.evidence[0].observation = Observation::Attempt {
        request_id: "http-attempt".into(),
    };
    refresh(&mut r);
    write("invalid-sideeffect-attempt-as-effect.json", p, r, true);
    let (mut p, mut r) = blindtest();
    if let ProductContract::Blindtest { invariants } = &mut p.plan.product_contract {
        invariants[0].authority.status = InvariantStatus::Candidate;
    }
    rebind(&mut p, &mut r);
    write("invalid-blindtest-unapproved-invariant.json", p, r, true);
}
