#![cfg(unix)]
#[path = "../../verify-mcp/tests/support/behavior.rs"]
mod behavior;
#[path = "../../verify-mcp/tests/support/sideeffect.rs"]
mod sideeffect;
use serde_json::{json, Value};
use std::{fs, process::Command};
use verify_cli::agent::{self, Operation, Product, Response};
use verify_core::Verdict;
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_b2ige"))
}
#[test]
fn actual_behavior_cli_human_json_agent_and_protocol() {
    for (body, code) in [("printf same", 0), ("printf changed", 1)] {
        let c = behavior::Case::new(body);
        let path = c.dir.join("config.json");
        fs::write(&path, serde_json::to_vec(&c.experiment).unwrap()).unwrap();
        for (mode, protocol) in [
            ("human", false),
            ("json", false),
            ("agent", false),
            ("agent", true),
        ] {
            let mut command = cli();
            command
                .args(["behavior", "verify"])
                .arg(&path)
                .arg("--authorization")
                .arg(c.dir.join("auth.json"))
                .arg("--store")
                .arg(c.dir.join("runs"))
                .args(["--output", mode]);
            if protocol {
                command.args(["--protocol", "1"]);
            }
            let out = command.output().unwrap();
            assert_eq!(
                out.status.code(),
                Some(code),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            if mode != "human" {
                let value: Value = serde_json::from_slice(&out.stdout).unwrap();
                let schema = if protocol {
                    agent::response_schema()
                } else if mode == "agent" {
                    verify_cli::agent_schema()
                } else {
                    verify_cli::report_schema()
                };
                assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
            }
        }
    }
}
#[test]
fn malformed_missing_config_machine_errors_are_json() {
    for product in ["behavior", "sideeffect", "blindtest"] {
        let out = cli()
            .args([
                product,
                "verify",
                "/nonexistent/private-canary",
                "--output",
                "agent",
                "--protocol",
                "1",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["verdict"], "ERROR");
        assert!(!String::from_utf8_lossy(&out.stdout).contains("private-canary"));
        assert!(out.stderr.is_empty());
    }
}
#[test]
fn argument_misuse_stays_64() {
    for args in [
        vec!["behavior", "verify"],
        vec!["behavior", "verify", "x", "--protocol", "2"],
        vec!["report", "x", "--output", "agent", "--open"],
        vec!["init", "--overwrite"],
    ] {
        assert_eq!(cli().args(args).status().unwrap().code(), Some(64));
    }
}
#[test]
fn init_dry_run_and_no_overwrite() {
    let c = behavior::Case::new("printf same");
    assert!(cli()
        .arg("init")
        .arg("--dry-run")
        .current_dir(&c.dir)
        .output()
        .unwrap()
        .status
        .success());
    assert!(!c.dir.join(".b2ige").exists());
    assert!(cli()
        .arg("init")
        .current_dir(&c.dir)
        .output()
        .unwrap()
        .status
        .success());
    let path = c.dir.join(".b2ige/project.json");
    let original = fs::read(&path).unwrap();
    assert_eq!(
        cli()
            .arg("init")
            .current_dir(&c.dir)
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    assert_eq!(fs::read(path).unwrap(), original);
    let setup = cli().arg("setup").current_dir(&c.dir).output().unwrap();
    assert!(setup.status.success());
    let setup_value: Value = serde_json::from_slice(&setup.stdout).unwrap();
    assert_eq!(setup_value["kind"], "setup");
    assert_eq!(setup_value["verification_performed"], false);
    assert_eq!(
        fs::read(c.dir.join(".b2ige/project.json")).unwrap(),
        original
    );
    let doctor = cli().arg("doctor").current_dir(&c.dir).output().unwrap();
    assert_eq!(doctor.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(v["verification_performed"], false);
}
#[test]
fn ci_all_verdicts_preserve_codes_and_only_pass_green() {
    let c = behavior::Case::new("printf same");
    let store = verify_evidence::store::EvidenceStore::new(c.dir.join("runs"));
    verify_core::behavior::execute(
        &store,
        &c.dir.join("work"),
        "source",
        &c.experiment,
        &c.auth,
    )
    .unwrap();
    let verified = verify_cli::load(&store, "source", &c.auth).unwrap();
    for (v, code) in [
        (Verdict::Pass, 0),
        (Verdict::Fail, 1),
        (Verdict::Inconclusive, 2),
        (Verdict::Error, 3),
    ] {
        // This tests transport agreement, not authoritative verdict assignment.
        let mut response = Response::from_verified(&verified, Operation::Verify);
        response.verdict = v;
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(agent::ci_check(&bytes, code).verdict, v);
        assert_eq!(agent::ci_check(&bytes, 64).verdict, Verdict::Error);
        let path = c.dir.join("machine.json");
        fs::write(&path, &bytes).unwrap();
        let out = cli()
            .arg("ci-check")
            .arg(path)
            .arg(code.to_string())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(code));
    }
}
#[test]
fn ci_rejects_malformed_missing_mismatch_and_doctor() {
    for bytes in [
        b"".as_slice(),
        b"{}",
        b"{\"verdict\":\"PASS\"}",
        b"not json",
    ] {
        assert_eq!(agent::ci_check(bytes, 0).verdict, Verdict::Error);
    }
    let mut r = Response::error(Product::Behavior, Operation::Doctor);
    r.verdict = Verdict::Pass;
    assert_eq!(
        agent::ci_check(&serde_json::to_vec(&r).unwrap(), 0).verdict,
        Verdict::Error
    );
    r.operation = Operation::Verify;
    assert_eq!(
        agent::ci_check(&serde_json::to_vec(&r).unwrap(), 0).verdict,
        Verdict::Error
    );
}
#[test]
fn request_schema_rejects_injected_command_and_unknown_version() {
    let base = json!({"protocol_version":"1","product":"behavior","operation":"verify","identity":"target","output":"agent","execution_budget":null});
    let validator = jsonschema::validator_for(&agent::request_schema()).unwrap();
    assert!(validator.is_valid(&base));
    for (key, value) in [
        ("protocol_version", json!("2")),
        ("command", json!("sh")),
        ("output", json!("human")),
    ] {
        let mut v = base.clone();
        v[key] = value;
        assert!(!validator.is_valid(&v));
        assert!(serde_json::from_value::<agent::Request>(v).is_err());
    }
}
#[test]
fn actual_ci_wrapper_sideeffect_all_results() {
    for (mode, code) in [("safe", 0), ("unsafe", 1), ("partial", 2), ("missing", 3)] {
        let mut c = sideeffect::Case::new(mode);
        if mode == "partial" {
            c.contract.exploration_budget.max_attempts = 0;
        }
        if mode == "missing" {
            c.contract.trigger.executable = "/nonexistent".into();
        }
        let path = c.dir.join("contract.json");
        fs::write(&path, serde_json::to_vec(&c.contract).unwrap()).unwrap();
        let report = c.dir.join("ci.json");
        let out = Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../scripts/ci-verify.py"
            ))
            .arg("sideeffect")
            .arg(path)
            .arg("--b2ige")
            .arg(env!("CARGO_BIN_EXE_b2ige"))
            .arg("--store")
            .arg(c.dir.join("runs"))
            .arg("--report")
            .arg(&report)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let r: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert!(jsonschema::validator_for(&agent::response_schema())
            .unwrap()
            .is_valid(&r));
    }
}
#[test]
fn ci_missing_verifier_no_stale_success_artifact() {
    let c = behavior::Case::new("printf same");
    let report = c.dir.join("report.json");
    fs::write(&report, b"stale PASS").unwrap();
    let out = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/ci-verify.py"
        ))
        .args(["behavior", "missing", "--b2ige", "/nonexistent", "--report"])
        .arg(&report)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(!report.exists());
}
