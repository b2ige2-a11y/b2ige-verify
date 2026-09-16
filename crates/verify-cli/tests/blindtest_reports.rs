#[path = "../../verify-core/tests/support/blindtest.rs"]
mod support;
use serde_json::{json, Value};
use std::{fs, process::Command, sync::OnceLock};
use support::*;
use verify_core::{behavior::BehaviorAuthorization, blindtest::*, Verdict};
use verify_evidence::{canonical_hash, store::EvidenceStore, Observation};
fn auth() -> BehaviorAuthorization {
    BehaviorAuthorization {
        approved_baselines: Default::default(),
        approved_checker_bindings: Default::default(),
        baseline_stable: false,
    }
}
fn actual() -> &'static Corpus {
    static C: OnceLock<Corpus> = OnceLock::new();
    C.get_or_init(|| {
        let mut c = Corpus::temporary();
        c.build_images();
        for mode in ["correct", "mutant_a", "mutant_b", "noop", "probe"] {
            execute(&c.config_for(mode), &c.sealed, &c.store, mode).unwrap();
        }
        let mut partial = c.config_for("correct");
        partial.max_cases = 1;
        execute(&partial, &c.sealed, &c.store, "partial").unwrap();
        let mut bad = c.config_for("correct");
        bad.target.image = format!("sha256:{}", "0".repeat(64));
        execute(&bad, &c.sealed, &c.store, "error").unwrap();
        c
    })
}
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn human_agent_verdicts_match_every_actual_outcome() {
    let c = actual();
    for (id, v) in [
        ("correct", Verdict::Pass),
        ("mutant_a", Verdict::Fail),
        ("partial", Verdict::Inconclusive),
        ("error", Verdict::Error),
    ] {
        let r = verify_cli::load(&c.store, id, &auth()).unwrap();
        assert_eq!(r.document().verdict, v);
        assert_eq!(r.agent().verdict, v);
        for (s, v) in [
            (verify_cli::report_schema(), json!(r.document())),
            (verify_cli::agent_schema(), json!(r.agent())),
        ] {
            assert!(jsonschema::validator_for(&s).unwrap().is_valid(&v));
        }
    }
}
#[test]
fn agent_contains_no_hidden_inventory_path_oracle_or_canary() {
    let c = actual();
    for id in [
        "correct", "mutant_a", "mutant_b", "noop", "probe", "partial", "error",
    ] {
        let r = verify_cli::load(&c.store, id, &auth()).unwrap();
        let a = serde_json::to_string(&r.agent()).unwrap();
        for secret in std::iter::once(c.suite.private_canary.as_str())
            .chain(c.suite.cases.iter().map(|c| c.case_id.as_str()))
            .chain(
                c.suite
                    .cases
                    .iter()
                    .flat_map(|c| c.args.iter().map(String::as_str)),
            )
        {
            assert!(!a.contains(secret), "secret reached agent output");
        }
        for forbidden in [
            "hidden_details",
            "oracle",
            "private_metadata",
            "/Users/",
            "/sealed",
            "/var/lib/docker",
            "docker.sock",
            "rejected\\n",
            "session created\\n",
        ] {
            assert!(!a.contains(forbidden), "forbidden field/value: {forbidden}");
        }
        assert!(!a.contains(c.root.to_str().unwrap()));
        assert!(!a.contains(c.workspace.to_str().unwrap()));
    }
}
#[test]
fn human_ui_has_trusted_details_and_sanitized_main_fields() {
    let c = actual();
    let r = verify_cli::load(&c.store, "mutant_a", &auth()).unwrap();
    let html = verify_cli::viewer::render(&r, "/token", "mutant_a");
    for label in [
        "✕ NOT READY",
        "Hidden verification found a contract violation",
        "Expected",
        "Observed",
        "Reproduction",
        "Hidden details · trusted human view",
        "Isolation",
        "Runs",
        "Raw artifact",
    ] {
        assert!(html.contains(label), "missing {label}");
    }
    assert!(html.contains(&c.suite.private_canary));
    assert!(r.human().contains("Runs · 3"));
    let d = r.document();
    for v in [&d.headline, &d.reason] {
        assert!(!v.contains(&c.suite.private_canary));
    }
    let pass = verify_cli::load(&c.store, "correct", &auth()).unwrap();
    let html = verify_cli::viewer::render(&pass, "/token", "correct");
    assert!(html.contains("✓ VERIFIED"));
    assert!(html.contains("No failures detected<br>within the executed hidden suite"));
    assert!(html.contains("Isolation · Docker verified"));
}
#[test]
fn report_cli_exit_codes_and_agent_exports() {
    let c = actual();
    for (id, code) in [
        ("correct", 0),
        ("mutant_a", 1),
        ("partial", 2),
        ("error", 3),
    ] {
        let out = cli(&[
            "report",
            id,
            "--store",
            c.store.root().to_str().unwrap(),
            "--output",
            "agent",
        ]);
        assert_eq!(out.status.code(), Some(code));
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["schema_version"], "3");
        assert!(out.stderr.is_empty());
    }
}
#[test]
fn blindtest_verify_cli_uses_private_store_and_public_config() {
    let c = actual();
    let path = c.root.join("cli-config.json");
    write_json(&path, &c.config_for("mutant_b"));
    let out = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["blindtest", "verify"])
        .arg(&path)
        .args(["--output", "agent"])
        .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["verdict"], "FAIL");
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&c.suite.private_canary));
}
#[test]
fn doctor_checks_actual_docker_without_claiming_hardened_linux() {
    let out = cli(&["blindtest", "doctor"]);
    assert_eq!(out.status.code(), Some(0));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["supported_isolation"], "DOCKER_ISOLATION");
    assert_eq!(v["attestation_required_per_run"], true);
}
#[test]
fn docker_unavailable_never_fake_pass_and_agent_error_is_safe() {
    let c = actual();
    let path = c.root.join("unavailable-config.json");
    write_json(&path, &c.config);
    let out = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["blindtest", "verify"])
        .arg(path)
        .args(["--output", "agent"])
        .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
        .env("B2IGE_DOCKER", "/nonexistent/blindtest-docker")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["verdict"], "ERROR");
    assert_eq!(v["blindtest"]["isolation"], "Not attested");
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&c.suite.private_canary));
    let out = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["blindtest", "doctor"])
        .env("B2IGE_DOCKER", "/nonexistent/blindtest-docker")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
}
#[test]
fn agent_cannot_open_trusted_human_view() {
    let out = cli(&["report", "x", "--output", "agent", "--open"]);
    assert_eq!(out.status.code(), Some(64));
}
#[test]
fn validate_suite_cli_records_actual_corpus() {
    let c = actual();
    let validation = c.validation();
    let p = c.root.join("validation-config.json");
    write_json(&p, &validation);
    let out = Command::new(env!("CARGO_BIN_EXE_b2ige"))
        .args(["blindtest", "validate-suite"])
        .arg(p)
        .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let receipt: BlindTestSuiteValidationReceipt = serde_json::from_slice(&out.stdout).unwrap();
    assert!(receipt.matched);
    assert_eq!(receipt.runs.len(), 6);
    assert!(
        load_validation(&c.store, &receipt.blindtest_validation_id)
            .unwrap()
            .matched
    );
}
#[test]
fn malicious_public_labels_and_run_names_are_sanitized() {
    let a = actual();
    let mut c = Corpus::temporary();
    c.config.target.image = a.images["mutant_a"].clone();
    let secret = c.suite.private_canary.clone();
    let id = secret.clone();
    c.suite.invariants[0].public_summary =
        format!("{} {} /sealed/oracle", secret, c.suite.cases[0].case_id);
    c.suite.invariants[0].expected_semantic = c.suite.cases[0].args[0].clone();
    let fixture_name = "private_fixture_inventory_name";
    c.suite.cases[0].fixture = Some(BoundedFixture {
        name: fixture_name.into(),
        bytes: vec![],
    });
    c.suite.cases[0].reproduction_template = vec![
        secret.clone(),
        c.suite.manifest.suite_id.clone(),
        fixture_name.into(),
        "LOGICAL_NOW".into(),
    ];
    for case in &mut c.suite.cases {
        let inv = c
            .suite
            .invariants
            .iter()
            .find(|i| i.invariant_id == case.invariant_id)
            .unwrap();
        case.oracle.as_mut().unwrap().invariant_hash = canonical_hash(inv).unwrap();
    }
    c.seal();
    execute(&c.config, &c.sealed, &c.store, &id).unwrap();
    let r = verify_cli::load(&c.store, &id, &auth()).unwrap();
    let agent = serde_json::to_string(&r.agent()).unwrap();
    assert!(!agent.contains(&secret));
    assert!(!agent.contains(&c.suite.cases[0].case_id));
    assert!(!agent.contains("/sealed/oracle"));
    assert!(!agent.contains(&c.suite.manifest.suite_id));
    assert!(!agent.contains(fixture_name));
    assert!(!agent.contains("LOGICAL_NOW"));
    assert!(!r.document().reason.contains(&secret));
}
#[test]
fn injected_canary_in_actual_evidence_becomes_fail_and_never_exports() {
    let c = actual();
    let mut r = load(&c.store, "correct").unwrap();
    let (_, mut items) = c.store.load("correct").unwrap();
    r.executions[0]
        .capture
        .stdout
        .extend_from_slice(r.suite.private_canary.as_bytes());
    let item = items
        .iter_mut()
        .find(|e| e.evidence_id == "execution-0")
        .unwrap();
    item.observation = Observation::Value {
        value: json!(r.executions[0]),
    };
    item.integrity_hash = canonical_hash(&item.observation).unwrap();
    let private = Corpus::temporary();
    let store = EvidenceStore::new(private.sealed.join("injected-evidence"));
    let run = store.reserve("correct").unwrap();
    for e in &items {
        run.write_evidence(e).unwrap();
    }
    run.complete(
        &r,
        &items
            .iter()
            .map(|e| e.evidence_id.clone())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let v = verify_cli::load(&store, "correct", &auth()).unwrap();
    assert_eq!(v.document().verdict, Verdict::Fail);
    assert!(v
        .agent()
        .blindtest
        .unwrap()
        .failures
        .iter()
        .any(|f| f.failure_kind == "isolation_leakage"));
    assert!(!serde_json::to_string(&v.agent())
        .unwrap()
        .contains(&r.suite.private_canary));
}
#[test]
fn exported_report_is_never_an_authoritative_source() {
    let c = actual();
    let r = verify_cli::load(&c.store, "correct", &auth()).unwrap();
    let p = c.root.join("export.json");
    fs::write(&p, serde_json::to_vec(r.document()).unwrap()).unwrap();
    let out = cli(&["report", p.to_str().unwrap(), "--output", "agent"]);
    assert_eq!(out.status.code(), Some(3));
}
