#![cfg(unix)]
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use verify_cli::{load, viewer, VerifiedReport};
use verify_core::{
    behavior::{self, generation::*, reduction::*, stability::*, *},
    checker_binding_hash, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator, Verdict,
};
use verify_evidence::{canonical_hash, store::EvidenceStore};
struct Case {
    dir: PathBuf,
    experiment: BehaviorExperiment,
    auth: BehaviorAuthorization,
}
impl Case {
    fn new(after: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "b2ige-p4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        for (name, body) in [("before", "printf same"), ("after", after)] {
            let path = dir.join(name);
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let case = BehaviorCase {
            schema_version: "1".into(),
            case_id: "case".into(),
            args: vec!["base".into()],
            environment: BTreeMap::new(),
            timeout_ms: 15000,
            fixture: LocalFixture {
                source: None,
                snapshot_identity: snapshot_identity(None).unwrap(),
            },
            comparison_policy: ComparisonPolicy::ProcessByteExactV1,
            required_observers: vec!["cli_process".into()],
        };
        let baseline = Baseline {
            baseline_id: "baseline".into(),
            target_revision: executable_identity(&dir.join("before")).unwrap(),
            created_by: BaselineCreator::Human,
            approval: BaselineApproval {
                status: ApprovalStatus::Approved,
                actor: Some("test-author".into()),
                reason: Some("test fixture approval".into()),
            },
            observation_contract_hash: case.observation_contract_hash().unwrap(),
            stability_runs: Some(1),
            notes: None,
        };
        let auth = BehaviorAuthorization {
            approved_baselines: BTreeSet::from([canonical_hash(&baseline).unwrap()]),
            approved_checker_bindings: BTreeSet::from([checker_binding_hash(
                &baseline,
                &case.checker_claim().unwrap(),
            )
            .unwrap()]),
            baseline_stable: true,
        };
        let target = |name: &str| BehaviorTarget {
            identity: executable_identity(&dir.join(name)).unwrap(),
            executable: dir.join(name),
            input_identity: case.input_identity().unwrap(),
        };
        let experiment = BehaviorExperiment {
            schema_version: "1".into(),
            before: target("before"),
            after: target("after"),
            case,
            baseline,
            seed: 42,
        };
        fs::write(dir.join("auth.json"), serde_json::to_vec(&auth).unwrap()).unwrap();
        Self {
            dir,
            experiment,
            auth,
        }
    }
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.dir.join("runs"))
    }
    fn exact(&self) -> VerifiedReport {
        behavior::execute(
            &self.store(),
            &self.dir.join("work"),
            "comparison",
            &self.experiment,
            &self.auth,
        )
        .unwrap();
        load(&self.store(), "comparison", &self.auth).unwrap()
    }
    fn suite(&self, values: &[&str]) -> BehaviorGeneratedSuiteResult {
        let spec = GenerationSpec {
            generation_id: "spec".into(),
            schema_version: "1".into(),
            base: self.experiment.clone(),
            mode: GenerationMode::Cartesian,
            dimensions: vec![
                Dimension {
                    target: DimensionTarget::Argument { index: 0 },
                    candidates: values.iter().map(|s| (*s).into()).collect(),
                },
                Dimension {
                    target: DimensionTarget::Environment { name: "Y".into() },
                    candidates: vec!["1".into()],
                },
            ],
            max_cases: 20,
        };
        behavior::generation::execute(
            &self.store(),
            &self.dir.join("work"),
            "suite",
            &spec,
            &self.auth,
        )
        .unwrap()
    }
    fn reduce(&self, budget: u64) -> VerifiedReport {
        let suite = self.suite(&["changed"]);
        let child = behavior::load(
            &self.store(),
            &suite.comparisons[0].comparison_id,
            &self.auth,
        )
        .unwrap();
        let input = ReductionInput {
            source_suite_id: suite.suite_id.clone(),
            source_suite_hash: canonical_hash(&suite).unwrap(),
            source_comparison: suite.comparisons[0].clone(),
            failing_case: suite.generated_cases[0].clone(),
            generation_spec: suite.generation_spec,
            expected_signature: signature(&child).unwrap(),
            execution_budget: budget,
        };
        behavior::reduction::execute(
            &self.store(),
            &self.dir.join("work"),
            "reduction",
            &input,
            &self.auth,
        )
        .unwrap();
        load(&self.store(), "reduction", &self.auth).unwrap()
    }
    fn cli(&self, input: &str, format: &str) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_b2ige"))
            .args([
                "report",
                input,
                "--store",
                self.dir.join("runs").to_str().unwrap(),
                "--authorization",
                self.dir.join("auth.json").to_str().unwrap(),
                "--output",
                format,
            ])
            .output()
            .unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if std::env::var_os("B2IGE_P4_KEEP_FIXTURES").is_none() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}
fn html(r: &VerifiedReport) -> String {
    viewer::render(r, "/token", "comparison")
}
#[test]
fn behavior_pass_human_agent() {
    let c = Case::new("printf same");
    let r = c.exact();
    assert_eq!(r.document().verdict, Verdict::Pass);
    assert!(r.human().contains("✓ VERIFIED"));
    assert!(r.human().contains("within tested behavior space"));
    assert_eq!(r.agent().verdict, Verdict::Pass);
    assert!(html(&r).contains("<summary>Details</summary>"));
}
#[test]
fn behavior_fail_expected_observed_evidence() {
    let c = Case::new("printf changed");
    let r = c.exact();
    assert_eq!(r.document().verdict, Verdict::Fail);
    assert_ne!(r.document().expected, r.document().observed);
    assert!(!r.document().evidence_summaries.is_empty());
    assert!(r.human().contains("Expected"));
    assert!(html(&r).contains("Primary failure"));
}
#[test]
fn inconclusive_is_not_pass() {
    let mut c = Case::new("/bin/sleep 1; printf same");
    c.experiment.case.timeout_ms = 150;
    let h = c.experiment.case.input_identity().unwrap();
    c.experiment.before.input_identity = h.clone();
    c.experiment.after.input_identity = h;
    let r = c.exact();
    assert_eq!(r.document().verdict, Verdict::Inconclusive);
    assert!(html(&r).contains("? INCONCLUSIVE"));
    assert!(!html(&r).contains("✓ VERIFIED"));
    assert_eq!(c.cli("comparison", "human").status.code(), Some(2));
}
#[test]
fn infrastructure_error_is_not_failure() {
    let c = Case::new("printf same");
    fs::set_permissions(
        &c.experiment.after.executable,
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let r = c.exact();
    assert_eq!(r.document().verdict, Verdict::Error);
    assert!(html(&r).contains("! ERROR"));
    assert!(!html(&r).contains("✕ FAILED"));
    assert_eq!(c.cli("comparison", "human").status.code(), Some(3));
}
#[test]
fn suite_primary_and_collapsed_others() {
    let c = Case::new("printf changed");
    c.suite(&["a", "b", "c"]);
    let r = load(&c.store(), "suite", &c.auth).unwrap();
    assert_eq!(r.document().verdict, Verdict::Fail);
    assert_eq!(r.document().other_failures.len(), 2);
    assert!(html(&r).contains("<details><summary>Other failures · 2</summary>"));
    assert!(r.document().reason.contains("3 proven divergences"));
}
#[test]
fn reducer_minimized() {
    let c = Case::new("if [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi");
    let r = c.reduce(20);
    let p = r.document().reproduction.as_ref().unwrap();
    assert_eq!(p.reduction_status, Some(ReductionStatus::Minimized));
    assert_eq!(p.retained_assignments, Some(1));
    assert!(html(&r).contains("Locally minimized reproduction"));
    assert_eq!(r.document().verdict, Verdict::Fail);
}
#[test]
fn reducer_budget_exhausted_is_not_minimal() {
    let c = Case::new("if [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi");
    let r = c.reduce(0);
    assert_eq!(r.document().outcome, "BUDGET_EXHAUSTED");
    assert!(!html(&r).contains("Locally minimized reproduction"));
    assert!(!r.human().contains("Minimal reproduction"));
    assert_eq!(
        r.agent().reduction_status,
        Some(ReductionStatus::BudgetExhausted)
    );
}
#[test]
fn missing_source_rejected() {
    let c = Case::new("printf same");
    assert!(load(&c.store(), "missing", &c.auth).is_err());
    assert_eq!(c.cli("missing", "json").status.code(), Some(3));
}
#[test]
fn corrupt_source_rejected() {
    let c = Case::new("printf same");
    c.exact();
    fs::write(c.dir.join("runs/comparison/result.json"), "{}").unwrap();
    assert!(load(&c.store(), "comparison", &c.auth).is_err());
}
#[test]
fn corrupt_child_prevents_suite_pass() {
    let c = Case::new("printf same");
    let s = c.suite(&["a"]);
    assert_eq!(s.aggregate_verdict, Verdict::Pass);
    fs::write(
        c.dir
            .join("runs")
            .join(&s.comparisons[0].comparison_id)
            .join("result.json"),
        "{}",
    )
    .unwrap();
    assert!(load(&c.store(), "suite", &c.auth).is_err());
}
#[test]
fn forged_report_is_never_input() {
    let c = Case::new("printf changed");
    let r = c.exact();
    let mut export = serde_json::to_value(r.document()).unwrap();
    export["verdict"] = "PASS".into();
    fs::write(
        c.dir.join("report.json"),
        serde_json::to_vec(&export).unwrap(),
    )
    .unwrap();
    assert_eq!(
        c.cli(c.dir.join("report.json").to_str().unwrap(), "json")
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        load(&c.store(), "comparison", &c.auth)
            .unwrap()
            .document()
            .verdict,
        Verdict::Fail
    );
}
#[test]
fn deterministic_compact_allowlisted_agent() {
    let c = Case::new("printf changed");
    let a = c.exact();
    let b = load(&c.store(), "comparison", &c.auth).unwrap();
    let x = serde_json::to_string(&a.agent()).unwrap();
    assert_eq!(x, serde_json::to_string(&b.agent()).unwrap());
    assert!(!x.contains("environment"));
    assert!(!x.contains(c.dir.to_str().unwrap()));
    assert!(x.len() < serde_json::to_string(a.document()).unwrap().len());
}
#[test]
fn cli_pass_fail_exit_codes_and_json_schemas() {
    for (body, code) in [("printf same", 0), ("printf changed", 1)] {
        let c = Case::new(body);
        c.exact();
        for (format, schema) in [
            ("json", verify_cli::report_schema()),
            ("agent", verify_cli::agent_schema()),
        ] {
            let o = c.cli("comparison", format);
            assert_eq!(o.status.code(), Some(code));
            let value: Value = serde_json::from_slice(&o.stdout).unwrap();
            assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
        }
    }
}
#[test]
fn path_input_uses_verified_loader() {
    let c = Case::new("printf same");
    c.exact();
    let path = c.dir.join("runs/comparison/result.json");
    assert_eq!(
        c.cli(path.to_str().unwrap(), "agent").status.code(),
        Some(0)
    );
    fs::remove_file(c.dir.join("runs/comparison-before/result.json")).unwrap();
    assert_eq!(
        c.cli(path.to_str().unwrap(), "agent").status.code(),
        Some(3)
    );
}
#[test]
fn disclosure_and_html_injection_contract() {
    let mut c = Case::new("printf changed");
    c.experiment.case.args = vec!["</pre><script>alert(1)</script>\u{1b}[2J".into()];
    let h = c.experiment.case.input_identity().unwrap();
    c.experiment.before.input_identity = h.clone();
    c.experiment.after.input_identity = h;
    let r = c.exact();
    let page = html(&r);
    assert!(!page.contains("<script>"));
    assert!(page.contains("&lt;script&gt;"));
    assert!(!page.contains("<details open"));
    assert!(page.contains("<details><summary>Raw artifact</summary>"));
    assert!(page.find("Expected").unwrap() < page.find("<summary>Evidence").unwrap());
    assert!(!r.human().contains('\u{1b}'));
    assert!(!page.contains("More details"));
}
#[test]
fn untrusted_authorization_cannot_be_derived_from_artifact() {
    let c = Case::new("printf same");
    c.exact();
    let mut auth = c.auth.clone();
    auth.approved_baselines.clear();
    assert!(load(&c.store(), "comparison", &auth).is_err());
}
#[test]
fn stability_verified_projection() {
    let c = Case::new("printf same");
    let p = behavior::stability::profile(
        &c.store(),
        &c.dir.join("work"),
        "profile",
        &c.experiment,
        &c.auth,
        &ProfilingConfig { repetitions: 3 },
    )
    .unwrap();
    behavior::stability::compare(
        &c.store(),
        &c.dir.join("work"),
        "stable-comparison",
        &c.experiment,
        &c.auth,
        &p.reference().unwrap(),
    )
    .unwrap();
    let r = load(&c.store(), "stable-comparison", &c.auth).unwrap();
    assert_eq!(r.document().verdict, Verdict::Pass);
    assert_eq!(r.document().run_references.len(), 4);
    assert_eq!(r.document().coverage["baseline_repetitions"], 3);
    fs::write(c.dir.join("runs/profile/result.json"), "{}").unwrap();
    assert!(load(&c.store(), "stable-comparison", &c.auth).is_err());
}
#[test]
fn schemas_match_saved_files() {
    for (saved, schema) in [
        (
            include_str!("../../../schemas/report-document.schema.json"),
            verify_cli::report_schema(),
        ),
        (
            include_str!("../../../schemas/agent-report.schema.json"),
            verify_cli::agent_schema(),
        ),
    ] {
        assert_eq!(serde_json::from_str::<Value>(saved).unwrap(), schema);
    }
}
#[test]
fn cli_argument_misuse_is_distinct() {
    let out = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["report", "x", "--output", "yaml"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("! ERROR"));
}

fn start(c: &Case, id: &str) -> String {
    let server = viewer::Viewer::bind(c.store(), c.auth.clone(), id).unwrap();
    let url = server.url().unwrap();
    std::thread::spawn(move || {
        let _ = server.serve();
    });
    url
}
fn http(url: &str, host_override: Option<&str>) -> String {
    use std::io::{Read, Write};
    let (host, path) = url
        .strip_prefix("http://")
        .unwrap()
        .split_once('/')
        .unwrap();
    let mut stream = std::net::TcpStream::connect(host).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET /{path} HTTP/1.1\r\nHost: {}\r\n\r\n",
        host_override.unwrap_or(host)
    )
    .unwrap();
    let mut result = String::new();
    stream.read_to_string(&mut result).unwrap();
    result
}
#[test]
fn http_navigation_revalidates_root_and_child() {
    let c = Case::new("printf changed");
    let suite = c.suite(&["a", "b"]);
    let url = start(&c, "suite");
    let root = http(&url, None);
    assert!(root.starts_with("HTTP/1.1 200"));
    assert!(root.contains("Cache-Control: no-store"));
    let child_url = format!(
        "{}/{}",
        url.rsplit_once('/').unwrap().0,
        suite.comparisons[0].comparison_id
    );
    assert!(http(&child_url, None).contains("✕ FAILED"));
    fs::remove_file(c.dir.join("runs/suite/result.json")).unwrap();
    let invalid = http(&child_url, None);
    assert!(invalid.starts_with("HTTP/1.1 422"));
    assert!(!invalid.contains("✕ FAILED"));
    assert!(invalid.contains("! ERROR"));
}
#[test]
fn http_rejects_cross_origin_and_arbitrary_paths() {
    let c = Case::new("printf same");
    c.exact();
    let url = start(&c, "comparison");
    assert!(http(&url, Some("attacker.example")).starts_with("HTTP/1.1 403"));
    let prefix = url.rsplit_once('/').unwrap().0;
    assert!(http(&format!("{prefix}/../auth.json"), None).starts_with("HTTP/1.1 422"));
    let bad_token = url.replacen("/comparison", "/other", 1);
    assert!(http(&bad_token, None).starts_with("HTTP/1.1 422"));
    let host = url
        .strip_prefix("http://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    assert!(http(&format!("http://{host}/wrong/comparison"), None).starts_with("HTTP/1.1 404"));
}
#[test]
fn http_reducer_suite_child_navigation() {
    let c = Case::new("if [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi");
    c.reduce(0);
    let url = start(&c, "reduction");
    let prefix = url.rsplit_once('/').unwrap().0;
    let suite = behavior::generation::load(&c.store(), "suite", &c.auth).unwrap();
    assert!(http(&format!("{prefix}/suite"), None).starts_with("HTTP/1.1 200"));
    assert!(http(
        &format!("{prefix}/{}", suite.comparisons[0].comparison_id),
        None
    )
    .starts_with("HTTP/1.1 200"));
}
