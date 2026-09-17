#![cfg(unix)]
#[path = "../../verify-mcp/tests/support/behavior.rs"]
mod behavior;
#[path = "../../verify-core/tests/support/blindtest.rs"]
mod blindtest;
#[path = "../../verify-mcp/tests/support/sideeffect.rs"]
mod sideeffect;
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use verify_cli::agent::Product;
use verify_cli::integration::{Entry, Project};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_b2ige"))
}
fn save(p: &Path, value: &impl serde::Serialize) {
    fs::write(p, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn code(out: &Output, expected: i32) {
    assert_eq!(
        out.status.code(),
        Some(expected),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn register(
    root: &Path,
    p: Product,
    config: &impl serde::Serialize,
    auth: Option<std::path::PathBuf>,
) -> std::path::PathBuf {
    let file = root.join("registered.json");
    save(&file, config);
    let registry = root.join("registry.json");
    save(
        &registry,
        &Project {
            schema_version: "1".into(),
            entries: std::collections::BTreeMap::from([(
                "test".into(),
                Entry {
                    product: p,
                    config: file,
                    store: root.join(if p == Product::Blindtest {
                        "sealed/runs"
                    } else {
                        "store"
                    }),
                    authorization: auth,
                },
            )]),
        },
    );
    registry
}
#[test]
fn inspect_is_bounded_read_only_and_never_executes_project_or_docker_scripts() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let c = behavior::Case::new("exit 99");
    let marker = c.dir.join("EXECUTED");
    fs::write(
        c.dir.join("package.json"),
        format!(
            r#"{{"scripts":{{"prepare":"touch {}"}}}}"#,
            marker.display()
        ),
    )
    .unwrap();
    fs::write(c.dir.join("Cargo.toml"), "[package]\nbuild='build.rs'").unwrap();
    fs::write(
        c.dir.join("docker"),
        format!("#!/bin/sh\ntouch {}\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(c.dir.join("docker"), fs::Permissions::from_mode(0o700)).unwrap();
    symlink(c.dir.join("private-canary"), c.dir.join("Dockerfile")).unwrap();
    let before = fs::read_dir(&c.dir).unwrap().count();
    let out = cli()
        .arg("inspect")
        .arg(&c.dir)
        .env("B2IGE_DOCKER", c.dir.join("docker"))
        .env("PATH", &c.dir)
        .output()
        .unwrap();
    code(&out, 0);
    let text = String::from_utf8(out.stdout).unwrap();
    for label in [
        "detected",
        "absent",
        "ambiguous",
        "unsupported",
        "needs operator decision",
        "verification_performed: false",
    ] {
        assert!(text.contains(label), "{text}");
    }
    assert!(!text.contains("private-canary"));
    assert!(!marker.exists());
    assert_eq!(before, fs::read_dir(&c.dir).unwrap().count());
    assert!(!c.dir.join(".b2ige").exists());
    let empty = c.dir.join("empty");
    fs::create_dir(&empty).unwrap();
    let out = cli().arg("inspect").arg(empty).output().unwrap();
    code(&out, 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("unsupported (no recognized"));
}
#[test]
fn init_idempotence_malformed_version_duplicate_identity_and_symlink_controls() {
    use std::os::unix::fs::symlink;
    let c = behavior::Case::new("printf same");
    for action in ["init", "init", "setup"] {
        code(
            &cli()
                .arg(action)
                .arg(&c.dir)
                .current_dir("/")
                .output()
                .unwrap(),
            0,
        );
    }
    let p = c.dir.join(".b2ige/project.json");
    let original = fs::read(&p).unwrap();
    code(
        &cli()
            .arg("init")
            .arg(&c.dir)
            .arg("--dry-run")
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(fs::read(&p).unwrap(), original);
    for data in [
        "{bad",
        r#"{"schema_version":"2","entries":{}}"#,
        r#"{"schema_version":"1","entries":{"x":{"product":"behavior","config":"c","store":"s","authorization":null},"x":{"product":"blindtest","config":"c","store":"s","authorization":null}}}"#,
    ] {
        fs::write(&p, data).unwrap();
        code(&cli().arg("init").arg(&c.dir).output().unwrap(), 3);
        assert_eq!(fs::read_to_string(&p).unwrap(), data);
    }
    fs::remove_file(&p).unwrap();
    let real = c.dir.join("real.json");
    fs::write(&real, &original).unwrap();
    symlink(&real, &p).unwrap();
    code(&cli().arg("init").arg(&c.dir).output().unwrap(), 3);
    assert_eq!(fs::read(&real).unwrap(), original);
    fs::remove_file(&p).unwrap();
    fs::remove_dir(c.dir.join(".b2ige")).unwrap();
    symlink(&c.dir, c.dir.join(".b2ige")).unwrap();
    code(&cli().arg("init").arg(&c.dir).output().unwrap(), 3);
}
fn prepare_behavior(c: &behavior::Case, out: &Path, config: bool) -> Output {
    if config {
        let p = c.dir.join("input.json");
        save(&p, &c.experiment);
        cli()
            .args(["prepare", "--product", "behavior", "--config"])
            .arg(p)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    } else {
        cli()
            .args(["prepare", "--product", "behavior", "--reference"])
            .arg(c.dir.join("before"))
            .arg("--candidate")
            .arg(c.dir.join("after"))
            .args([
                "--case-id",
                "case",
                "--timeout-ms",
                "15000",
                "--seed",
                "42",
                "--out",
            ])
            .arg(out)
            .output()
            .unwrap()
    }
}
#[test]
fn behavior_prepare_has_separate_roles_and_no_authorization_or_baseline_promotion() {
    let c = behavior::Case::new("printf changed");
    let out = c.dir.join("draft");
    code(&prepare_behavior(&c, &out, false), 0);
    let draft: verify_core::behavior::BehaviorExperiment =
        verify_cli::integration::read(&out.join("behavior.json")).unwrap();
    assert_eq!(draft.before.identity, c.experiment.before.identity);
    assert_eq!(draft.after.identity, c.experiment.after.identity);
    assert_eq!(draft.baseline.target_revision, draft.before.identity);
    assert_ne!(draft.baseline.target_revision, draft.after.identity);
    assert_eq!(
        draft.baseline.approval.status,
        verify_core::ApprovalStatus::Unapproved
    );
    assert_eq!(draft.baseline.stability_runs, None);
    assert_eq!(fs::read_dir(&out).unwrap().count(), 2);
    assert!(!c.dir.join(".b2ige").exists());
    assert!(verify_core::behavior::validate_configuration(
        &draft,
        &verify_cli::integration::empty_auth()
    )
    .is_err());
    let review = fs::read_to_string(out.join("REVIEW.md")).unwrap();
    assert!(review.contains("BLOCKED"));
    assert!(review.contains("Stability is never inferred"));
    code(&prepare_behavior(&c, &out, false), 3);
    code(
        &cli()
            .args(["verify"])
            .arg(out.join("behavior.json"))
            .output()
            .unwrap(),
        3,
    );
    code(
        &cli()
            .args(["prepare", "--out"])
            .arg(c.dir.join("implicit"))
            .output()
            .unwrap(),
        3,
    );
    assert!(!c.dir.join("implicit").exists());
}
#[test]
fn candidate_cannot_rebind_an_approved_reference_during_prepare() {
    let mut c = behavior::Case::new("printf changed");
    c.experiment.before = c.experiment.after.clone();
    code(&prepare_behavior(&c, &c.dir.join("poisoned"), true), 3);
    assert!(!c.dir.join("poisoned").exists());
}
#[test]
fn sideeffect_prepare_requires_explicit_disposable_fixture_and_committed_observer() {
    let c = sideeffect::Case::new("safe");
    let input = c.dir.join("input.json");
    save(&input, &c.contract);
    let out = c.dir.join("draft");
    let base = || {
        let mut cmd = cli();
        cmd.args(["prepare", "--product", "sideeffect", "--config"])
            .arg(&input)
            .arg("--out")
            .arg(&out);
        cmd
    };
    code(&base().output().unwrap(), 3);
    assert!(!out.exists());
    code(
        &base()
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .output()
            .unwrap(),
        0,
    );
    let review = fs::read_to_string(out.join("REVIEW.md")).unwrap();
    assert!(review.contains("Attempts/stdout/request logs are NOT committed effects"));
    assert!(review.contains("BLOCKED"));
    assert_eq!(fs::read_dir(&out).unwrap().count(), 2);
    let mut invalid = json!(c.contract);
    invalid["required_observers"][0]["authoritative_source"] = json!("request_log");
    save(&input, &invalid);
    code(
        &cli()
            .args(["prepare", "--product", "sideeffect", "--config"])
            .arg(&input)
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .arg("--out")
            .arg(c.dir.join("bad"))
            .output()
            .unwrap(),
        3,
    );
}
#[test]
fn registered_router_all_verdicts_exact_exit_agreement_and_no_substitution() {
    for (mode, budget, expected) in [("safe", 2, 0), ("unsafe", 2, 1), ("safe", 0, 2)] {
        let mut c = sideeffect::Case::new(mode);
        c.contract.exploration_budget.max_attempts = budget;
        let registry = register(&c.dir, Product::Sideeffect, &c.contract, None);
        let out = cli()
            .args(["verify", "test", "--registry"])
            .arg(&registry)
            .args(["--output", "agent", "--protocol", "1"])
            .current_dir("/")
            .output()
            .unwrap();
        code(&out, expected);
        let response: verify_cli::agent::Response = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(response.verdict.exit_code(), expected as u8);
        assert_eq!(response.product, Product::Sideeffect);
        assert_eq!(
            verify_cli::agent::ci_check(&out.stdout, expected).verdict,
            response.verdict
        );
        code(
            &cli()
                .args(["verify", "unknown", "--registry"])
                .arg(&registry)
                .output()
                .unwrap(),
            3,
        );
        code(
            &cli()
                .args(["verify", "test", "--registry"])
                .arg(&registry)
                .args(["--config", "substitute.json"])
                .output()
                .unwrap(),
            3,
        );
    }
    let c = behavior::Case::new("printf same");
    let reg = register(&c.dir, Product::Sideeffect, &c.experiment, None);
    let out = cli()
        .args(["verify", "test", "--registry"])
        .arg(reg)
        .args(["--output", "agent", "--protocol", "1"])
        .output()
        .unwrap();
    code(&out, 3);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["product"], "sideeffect");
    assert_eq!(r["verdict"], "ERROR");
}
#[test]
fn missing_behavior_stability_checker_and_observers_fail_closed_with_field_blockers() {
    for field in [
        "baseline_stable",
        "approved_checker_bindings",
        "approved_baselines",
    ] {
        let c = behavior::Case::new("printf same");
        let mut auth = json!(c.auth);
        auth[field] = if field == "baseline_stable" {
            json!(false)
        } else {
            json!([])
        };
        save(&c.dir.join("auth.json"), &auth);
        let reg = register(
            &c.dir,
            Product::Behavior,
            &c.experiment,
            Some(c.dir.join("auth.json")),
        );
        code(
            &cli()
                .args(["verify", "test", "--registry"])
                .arg(&reg)
                .output()
                .unwrap(),
            if field == "baseline_stable" { 2 } else { 3 },
        );
        let before = fs::read_dir(c.dir.join("store"))
            .map(|r| r.count())
            .unwrap_or(0);
        let out = cli()
            .args(["doctor", "--registry"])
            .arg(&reg)
            .output()
            .unwrap();
        code(&out, 3);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["verification_performed"], false);
        assert_eq!(v["kind"], "readiness");
        assert_eq!(
            fs::read_dir(c.dir.join("store"))
                .map(|r| r.count())
                .unwrap_or(0),
            before
        );
        let human = cli()
            .args(["doctor", "--registry"])
            .arg(&reg)
            .args(["--output", "human"])
            .output()
            .unwrap();
        code(&human, 3);
        assert!(String::from_utf8_lossy(&human.stdout).contains("BLOCKED"));
        assert!(!String::from_utf8_lossy(&human.stdout).contains("PASS"));
        assert_eq!(
            verify_cli::agent::ci_check(&out.stdout, 0).verdict,
            verify_core::Verdict::Error
        );
    }
}
#[test]
fn sideeffect_unavailable_failed_and_partial_observation_never_pass_or_fake_product_failure() {
    for mode in ["missing", "corrupt", "partial"] {
        let mut c = sideeffect::Case::new("safe");
        match mode {
            "missing" => fs::remove_file(c.dir.join("fixture/ledger.db")).unwrap(),
            "corrupt" => fs::write(c.dir.join("fixture/ledger.db"), b"not SQLite").unwrap(),
            _ => c.contract.exploration_budget.max_attempts = 0,
        }
        c.contract.trigger.fixture.snapshot_identity =
            verify_core::behavior::snapshot_identity(c.contract.trigger.fixture.source.as_deref())
                .unwrap();
        let reg = register(&c.dir, Product::Sideeffect, &c.contract, None);
        let out = cli()
            .args(["verify", "test", "--registry"])
            .arg(reg)
            .args(["--output", "agent", "--protocol", "1"])
            .output()
            .unwrap();
        assert!(
            matches!(out.status.code(), Some(2 | 3)),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
#[test]
fn blindtest_public_preparation_and_readiness_do_not_leak_private_material() {
    let c = blindtest::Corpus::temporary();
    let input = c.root.join("public.json");
    let config = c.config.clone();
    save(&input, &config);
    let out = c.root.join("draft");
    let output = cli()
        .args(["prepare", "--product", "blindtest", "--config"])
        .arg(&input)
        .arg("--workspace")
        .arg(&c.workspace)
        .arg("--out")
        .arg(&out)
        .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
        .output()
        .unwrap();
    code(&output, 0);
    let review = fs::read_to_string(out.join("REVIEW.md")).unwrap();
    for bytes in [String::from_utf8(output.stdout).unwrap(), review] {
        assert!(!bytes.contains(&c.suite.private_canary));
        assert!(!bytes.contains(c.sealed.to_str().unwrap()));
        for case in &c.suite.cases {
            assert!(!bytes.contains(&case.case_id));
        }
    }
    let overlap = c.root.join("overlap");
    code(
        &cli()
            .args(["prepare", "--product", "blindtest", "--config"])
            .arg(&input)
            .arg("--workspace")
            .arg(&c.root)
            .arg("--out")
            .arg(&overlap)
            .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
            .output()
            .unwrap(),
        3,
    );
    assert!(!overlap.exists());
    let reg = register(&c.root, Product::Blindtest, &config, None);
    for command in ["doctor", "verify"] {
        let mut cmd = cli();
        cmd.arg(command);
        if command == "verify" {
            cmd.arg("test")
                .args(["--output", "agent", "--protocol", "1"]);
        }
        cmd.arg("--registry").arg(&reg).env(
            "B2IGE_BLINDTEST_SEALED_ROOT",
            c.root.join("absent-private-canary"),
        );
        let out = cmd.output().unwrap();
        code(&out, 3);
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!all.contains("absent-private-canary"));
        assert!(!all.contains(&c.suite.private_canary));
    }
}

// PTY is exclusively a test harness around synthetic test authorizations. The
// production CLI has no noninteractive approval bypass or hidden test flag.
fn terminal(args: &[String], answer: &str, mutate: Option<&Path>) -> Output {
    terminal_env(args, answer, mutate, None)
}
fn terminal_env(
    args: &[String],
    answer: &str,
    mutate: Option<&Path>,
    sealed: Option<&Path>,
) -> Output {
    let action = mutate
        .map(|p| {
            format!(
                "with open({}, 'ab') as f: f.write(b'CHANGED')",
                serde_json::to_string(p).unwrap()
            )
        })
        .unwrap_or_default();
    terminal_action(args, answer, &action, sealed, "")
}
fn terminal_action(
    args: &[String],
    answer: &str,
    action: &str,
    sealed: Option<&Path>,
    setup: &str,
) -> Output {
    let script = r#"
import os,pty,select,subprocess,sys,time
exec(sys.argv[3])
master,slave=pty.openpty()
p=subprocess.Popen(sys.argv[4:],stdin=slave,stdout=slave,stderr=slave,close_fds=True,restore_signals=False)
os.close(slave)
buf=b''; sent=False; deadline=time.monotonic()+90
while time.monotonic()<deadline:
    ready,_,_=select.select([master],[],[],0.1)
    if ready:
        try: chunk=os.read(master,65536)
        except OSError: break
        if not chunk: break
        buf+=chunk
        if b'Type APPROVE ' in buf and not sent:
            exec(sys.argv[2])
            os.write(master,(sys.argv[1]+'\n').encode()); sent=True
    elif p.poll() is not None: break
else:
    p.kill(); raise RuntimeError('PTY timeout')
p.wait(); os.close(master); sys.stdout.buffer.write(buf); sys.exit(p.returncode)
"#;
    let mut command = Command::new("python3");
    if let Some(sealed) = sealed {
        command.env("B2IGE_BLINDTEST_SEALED_ROOT", sealed);
    }
    command
        .arg("-c")
        .arg(script)
        .arg(answer)
        .arg(action)
        .arg(setup)
        .arg(env!("CARGO_BIN_EXE_b2ige"))
        .args(args)
        .output()
        .unwrap()
}
fn approval_args(draft: &Path, controller: &Path, project: &Path, product: &str) -> Vec<String> {
    vec![
        "trust".into(),
        "approve".into(),
        draft.display().to_string(),
        "--product".into(),
        product.into(),
        "--identity".into(),
        "test".into(),
        "--registry".into(),
        controller.join("registry.json").display().to_string(),
        "--controller".into(),
        controller.display().to_string(),
        "--project-root".into(),
        project.display().to_string(),
        "--store".into(),
        controller.join("store").display().to_string(),
    ]
}
#[test]
fn interactive_approval_rereads_inputs_cancels_and_registers_exact_typed_identity() {
    for mode in [
        "cancel",
        "noninteractive",
        "changed",
        "approve",
        "unstable",
        "checker",
    ] {
        let c = behavior::Case::new("printf same");
        let draft = c.dir.join("draft");
        code(&prepare_behavior(&c, &draft, true), 0);
        let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
        fs::create_dir(&controller).unwrap();
        let mut auth = json!(c.auth);
        if mode == "unstable" {
            auth["baseline_stable"] = json!(false);
        }
        if mode == "checker" {
            auth["approved_checker_bindings"] = json!([]);
        }
        save(&controller.join("auth.json"), &auth);
        let mut args = approval_args(&draft, &controller, &c.dir, "behavior");
        args.extend([
            "--authorization".into(),
            controller.join("auth.json").display().to_string(),
        ]);
        let output = if mode == "noninteractive" {
            cli().args(&args).output().unwrap()
        } else {
            terminal(
                &args,
                if mode == "cancel" {
                    "no"
                } else {
                    "APPROVE test"
                },
                if mode == "changed" {
                    Some(&c.experiment.after.executable)
                } else {
                    None
                },
            )
        };
        code(&output, if mode == "approve" { 0 } else { 3 });
        let registry = controller.join("registry.json");
        assert_eq!(registry.exists(), mode == "approve");
        if mode == "approve" {
            let p: Project = verify_cli::integration::read(&registry).unwrap();
            assert!(p.entries["test"].config.is_absolute());
            let saved: verify_core::behavior::BehaviorAuthorization =
                verify_cli::integration::read(p.entries["test"].authorization.as_ref().unwrap())
                    .unwrap();
            assert_eq!(saved, c.auth);
            code(
                &cli()
                    .args(["verify", "test", "--registry"])
                    .arg(&registry)
                    .current_dir("/")
                    .output()
                    .unwrap(),
                0,
            );
            let before = fs::read(&registry).unwrap();
            code(&terminal(&args, "APPROVE test", None), 3);
            assert_eq!(fs::read(&registry).unwrap(), before);
        } else {
            assert!(!controller.join("test").exists());
        }
        fs::remove_dir_all(controller).unwrap();
    }
}
#[test]
fn interactive_sideeffect_registration_requires_review_then_executes_existing_path() {
    let c = sideeffect::Case::new("safe");
    let input = c.dir.join("input.json");
    save(&input, &c.contract);
    let draft = c.dir.join("draft");
    code(
        &cli()
            .args(["prepare", "--product", "sideeffect", "--config"])
            .arg(input)
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .arg("--out")
            .arg(&draft)
            .output()
            .unwrap(),
        0,
    );
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    fs::create_dir(&controller).unwrap();
    let args = approval_args(&draft, &controller, &c.dir, "sideeffect");
    let out = terminal(&args, "APPROVE test", None);
    code(&out, 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("not attempts/stdout/request logs"));
    code(
        &cli()
            .args(["verify", "test", "--registry"])
            .arg(controller.join("registry.json"))
            .output()
            .unwrap(),
        0,
    );
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn blindtest_registered_docker_paths_preserve_all_verdicts_and_trusted_approval() {
    let mut c = blindtest::Corpus::temporary();
    c.build_images();
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    fs::create_dir(&controller).unwrap();
    for (mode, max_cases, expected) in
        [("correct", 128, 0), ("mutant_a", 128, 1), ("correct", 1, 2)]
    {
        let mut config = c.config_for(mode);
        config.max_cases = max_cases;
        let reg = register(&c.root, Product::Blindtest, &config, None);
        let out = cli()
            .args(["verify", "test", "--registry"])
            .arg(reg)
            .args(["--output", "agent", "--protocol", "1"])
            .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
            .output()
            .unwrap();
        code(&out, expected);
        let r: verify_cli::agent::Response = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r.product, Product::Blindtest);
        assert_eq!(r.verdict.exit_code(), expected as u8);
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(!text.contains(&c.suite.private_canary));
        assert!(!text.contains(c.sealed.to_str().unwrap()));
    }
    let config = c.config_for("correct");
    let input = c.root.join("public.json");
    save(&input, &config);
    let draft = c.root.join("draft");
    code(
        &cli()
            .args(["prepare", "--product", "blindtest", "--config"])
            .arg(&input)
            .arg("--workspace")
            .arg(&config.target.workspace)
            .arg("--out")
            .arg(&draft)
            .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
            .output()
            .unwrap(),
        0,
    );
    let args = approval_args(&draft, &controller, &config.target.workspace, "blindtest");
    let approved = terminal_env(&args, "APPROVE test", None, Some(&c.sealed));
    code(&approved, 0);
    assert!(!String::from_utf8_lossy(&approved.stdout).contains(&c.suite.private_canary));
    assert!(!String::from_utf8_lossy(&approved.stdout).contains(c.sealed.to_str().unwrap()));
    code(
        &cli()
            .args(["verify", "test", "--registry"])
            .arg(controller.join("registry.json"))
            .args(["--output", "agent", "--protocol", "1"])
            .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
            .current_dir("/")
            .output()
            .unwrap(),
        0,
    );
    for defect in ["image", "isolation", "suite"] {
        let mut bad = json!(config);
        match defect {
            "image" => bad["target"]["image"] = json!(format!("sha256:{}", "0".repeat(64))),
            "isolation" => bad["required_isolation"] = json!("NONE"),
            _ => bad["suite_hash"] = json!(format!("sha256:{}", "0".repeat(64))),
        }
        let reg = register(&c.root, Product::Blindtest, &bad, None);
        let out = cli()
            .args(["verify", "test", "--registry"])
            .arg(reg)
            .args(["--output", "agent", "--protocol", "1"])
            .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
            .output()
            .unwrap();
        code(&out, 3);
    }
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn typed_public_flag_adapters_and_conflicting_input_refusal() {
    let c = sideeffect::Case::new("safe");
    let trigger = &c.contract.trigger;
    let out = c.dir.join("flags");
    let output = cli()
        .args(["prepare", "--product", "sideeffect", "--out"])
        .arg(&out)
        .arg("--fixture")
        .arg(c.dir.join("fixture"))
        .arg("--trigger")
        .arg(&trigger.executable)
        .args([
            "--contract-id",
            "checkout",
            "--operation-id",
            "checkout",
            "--idempotency",
            "key",
            "--correlation",
            "correlation",
            "--provider",
            "local",
            "--operation",
            "payment",
            "--expectation",
            "exactly-once",
            "--db",
            "ledger.db",
            "--table",
            "ledger",
            "--external-id-column",
            "external_id",
            "--idempotency-column",
            "idem",
            "--correlation-column",
            "correlation",
            "--operation-column",
            "operation",
            "--commit-order-column",
            "seq",
            "--timeout-ms",
            "3000",
            "--schedules-json",
        ])
        .arg(serde_json::to_string(&c.contract.fault_schedules).unwrap())
        .arg("--budget-json")
        .arg(serde_json::to_string(&c.contract.exploration_budget).unwrap())
        .arg("--args-json")
        .arg(serde_json::to_string(&trigger.args).unwrap())
        .arg("--env-json")
        .arg(serde_json::to_string(&trigger.environment).unwrap())
        .output()
        .unwrap();
    code(&output, 0);
    let contract: verify_core::sideeffect::SideEffectContract =
        verify_cli::integration::read(&out.join("sideeffect.json")).unwrap();
    assert!(contract.validate().is_ok());
    let b = blindtest::Corpus::temporary();
    let c = &b.config;
    let out = b.root.join("flags");
    let result = cli()
        .args(["prepare", "--product", "blindtest", "--out"])
        .arg(out)
        .arg("--workspace")
        .arg(&b.workspace)
        .arg("--suite-hash")
        .arg(&c.suite_hash)
        .arg("--approved-invariants-json")
        .arg(serde_json::to_string(&c.approved_invariants).unwrap())
        .arg("--image")
        .arg(&c.target.image)
        .arg("--command")
        .arg(&c.target.command)
        .arg("--build-identity")
        .arg(&c.target.build_identity)
        .arg("--bounds-json")
        .arg(serde_json::to_string(&c.target.bounds).unwrap())
        .args(["--max-cases", "128", "--max-case-args", "4"])
        .output()
        .unwrap();
    code(&result, 0);
    let c = behavior::Case::new("printf same");
    let input = c.dir.join("input.json");
    save(&input, &c.experiment);
    code(
        &cli()
            .args(["prepare", "--product", "behavior", "--config"])
            .arg(input)
            .arg("--candidate")
            .arg(&c.experiment.after.executable)
            .arg("--out")
            .arg(c.dir.join("conflict"))
            .output()
            .unwrap(),
        3,
    );
}
#[test]
fn registered_draft_is_rejected_and_symlinked_outputs_are_not_written() {
    use std::os::unix::fs::symlink;
    let c = behavior::Case::new("printf same");
    let draft = c.dir.join("draft");
    code(&prepare_behavior(&c, &draft, true), 0);
    let reg = c.dir.join("registry.json");
    save(
        &reg,
        &Project {
            schema_version: "1".into(),
            entries: std::collections::BTreeMap::from([(
                "test".into(),
                Entry {
                    product: Product::Behavior,
                    config: draft.join("behavior.json"),
                    store: c.dir.join("store"),
                    authorization: Some(c.dir.join("auth.json")),
                },
            )]),
        },
    );
    code(
        &cli()
            .args(["verify", "test", "--registry"])
            .arg(reg)
            .output()
            .unwrap(),
        3,
    );
    assert!(!c.dir.join("store").exists());
    let link = c.dir.join("alias");
    symlink(&c.dir, &link).unwrap();
    code(&prepare_behavior(&c, &link.join("new-draft"), true), 3);
    assert!(!c.dir.join("new-draft").exists());
}

// Synthetic public fixtures only; no operator approvals or private holdouts are used.
fn reviewed_behavior(c: &behavior::Case, controller: &Path) -> Vec<String> {
    let draft = c.dir.join("draft");
    code(&prepare_behavior(c, &draft, true), 0);
    fs::create_dir_all(controller).unwrap();
    save(&controller.join("auth.json"), &c.auth);
    let mut args = approval_args(&draft, controller, &c.dir, "behavior");
    args.extend([
        "--authorization".into(),
        controller.join("auth.json").display().to_string(),
    ]);
    args
}
fn option(args: &mut [String], key: &str, value: &Path) {
    let index = args.iter().position(|s| s == key).unwrap();
    args[index + 1] = value.display().to_string();
}

#[test]
fn approval_rejects_changed_config_authorization_registry_and_forged_review() {
    for input in ["config", "authorization", "registry", "reference", "review"] {
        let c = behavior::Case::new("printf same");
        let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
        let args = reviewed_behavior(&c, &controller);
        let registry = controller.join("registry.json");
        fs::write(&registry, br#"{"schema_version":"1","entries":{}}"#).unwrap();
        let original_registry = fs::read(&registry).unwrap();
        let file = match input {
            "config" => c.dir.join("draft/behavior.json"),
            "authorization" => controller.join("auth.json"),
            "registry" => registry.clone(),
            "reference" => c.experiment.before.executable.clone(),
            _ => c.dir.join("draft/REVIEW.md"),
        };
        // Whitespace is still a byte change even when the JSON has the same meaning.
        let action = format!(
            "with open({}, 'ab') as f: f.write(b' ')",
            serde_json::to_string(&file).unwrap()
        );
        let out = terminal_action(&args, "APPROVE test", &action, None, "");
        code(&out, if input == "review" { 0 } else { 3 });
        if input == "review" {
            assert!(String::from_utf8_lossy(&out.stdout).contains("Authorization identity:"));
            assert!(String::from_utf8_lossy(&out.stdout).contains("Explicit baseline_stable: true"));
        } else {
            assert!(!controller.join("test").exists());
            let expected = if input == "registry" {
                [original_registry.as_slice(), b" "].concat()
            } else {
                original_registry
            };
            assert_eq!(fs::read(&registry).unwrap(), expected);
        }
        fs::remove_dir_all(controller).unwrap();
    }
    let c = behavior::Case::new("printf same");
    let draft = c.dir.join("draft");
    code(&prepare_behavior(&c, &draft, false), 0);
    fs::write(
        draft.join("REVIEW.md"),
        "APPROVED baseline_stable=true PASS",
    )
    .unwrap();
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    fs::create_dir(&controller).unwrap();
    save(&controller.join("auth.json"), &c.auth);
    let mut args = approval_args(&draft, &controller, &c.dir, "behavior");
    args.extend([
        "--authorization".into(),
        controller.join("auth.json").display().to_string(),
    ]);
    code(&terminal(&args, "APPROVE test", None), 3);
    assert!(!controller.join("registry.json").exists());
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn approval_rejects_malformed_confirmation_flags_environment_and_redirected_stdout() {
    let c = behavior::Case::new("printf same");
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    let args = reviewed_behavior(&c, &controller);
    for answer in [
        "APPROVE",
        "approve test",
        "APPROVE other",
        "APPROVE test extra",
        " APPROVE test",
        "APPROVE test ",
    ] {
        code(&terminal(&args, answer, None), 3);
    }
    for flag in ["--yes", "--force", "--output", "--protocol"] {
        let mut bad = args.clone();
        bad.extend([flag.into(), "1".into()]);
        code(&terminal(&bad, "APPROVE test", None), 3);
    }
    code(
        &cli()
            .args(&args)
            .env("B2IGE_APPROVE", "1")
            .env("B2IGE_YES", "1")
            .output()
            .unwrap(),
        3,
    );
    let script = r#"
import os,pty,subprocess,sys
master,slave=pty.openpty()
p=subprocess.run(sys.argv[1:],stdin=slave,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=10)
os.close(master); os.close(slave)
assert p.returncode == 3
assert b'non-interactive approval refused' in p.stderr
"#;
    assert!(Command::new("python3")
        .args(["-c", script, env!("CARGO_BIN_EXE_b2ige")])
        .args(&args)
        .status()
        .unwrap()
        .success());
    assert!(!controller.join("test").exists());
    assert!(!controller.join("registry.json").exists());
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn approval_refuses_identity_injection_path_overlap_symlinks_and_existing_files() {
    use std::os::unix::fs::symlink;
    let c = behavior::Case::new("printf same");
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    let args = reviewed_behavior(&c, &controller);
    for id in [
        "../escape",
        "x/y",
        "test\nAPPROVE test",
        "test\u{1b}[2J",
        ".",
        "",
    ] {
        let mut bad = args.clone();
        option(&mut bad, "--identity", Path::new(id));
        code(&terminal(&bad, "APPROVE test", None), 3);
    }
    for (key, path) in [
        ("--controller", c.dir.clone()),
        ("--project-root", controller.clone()),
        ("--store", controller.clone()),
        ("--store", controller.join("test")),
        ("--store", controller.join("registry.json")),
        ("--store", controller.join("auth.json")),
        ("--registry", controller.join("test/config.json")),
        ("--authorization", c.dir.join("auth.json")),
        ("--registry", controller.join("../escape.json")),
    ] {
        let mut bad = args.clone();
        option(&mut bad, key, &path);
        code(&terminal(&bad, "APPROVE test", None), 3);
    }
    let alias = controller.join("alias.json");
    symlink(controller.join("auth.json"), &alias).unwrap();
    for key in ["--authorization", "--registry"] {
        let mut bad = args.clone();
        option(&mut bad, key, &alias);
        code(&terminal(&bad, "APPROVE test", None), 3);
    }
    fs::create_dir(controller.join("test")).unwrap();
    fs::write(
        controller.join("test/config.json"),
        b"retained approved bytes",
    )
    .unwrap();
    code(&terminal(&args, "APPROVE test", None), 3);
    assert_eq!(
        fs::read(controller.join("test/config.json")).unwrap(),
        b"retained approved bytes"
    );
    assert!(!controller.join("registry.json").exists());
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn approval_refuses_controller_inside_actual_fixture_even_with_different_project_root() {
    let c = sideeffect::Case::new("safe");
    let controller = c.dir.join("fixture/controller");
    fs::create_dir(&controller).unwrap();
    let input = c.dir.join("input.json");
    save(&input, &c.contract);
    let draft = c.dir.join("draft");
    code(
        &cli()
            .args(["prepare", "--product", "sideeffect", "--config"])
            .arg(input)
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .arg("--out")
            .arg(&draft)
            .output()
            .unwrap(),
        0,
    );
    let project = c.dir.join("public-project");
    fs::create_dir(&project).unwrap();
    let args = approval_args(&draft, &controller, &project, "sideeffect");
    code(&terminal(&args, "APPROVE test", None), 3);
    assert!(!controller.join("registry.json").exists());
    assert!(!controller.join("test").exists());
}

#[test]
fn registry_lock_is_shared_by_nested_controllers_and_never_removed_by_nonowner() {
    let c = behavior::Case::new("printf same");
    let root = std::env::temp_dir().join(verify_cli::integration::nonce());
    let controller = root.join("nested");
    let args = reviewed_behavior(&c, &controller);
    let lock = controller.join("registry.json.adoption-approval.lock");
    fs::write(&lock, b"other live writer").unwrap();
    for selected_controller in [&root, &controller] {
        let mut bad = args.clone();
        option(&mut bad, "--controller", selected_controller);
        code(&terminal(&bad, "APPROVE test", None), 3);
        assert_eq!(fs::read(&lock).unwrap(), b"other live writer");
        assert!(!selected_controller.join("test").exists());
    }
    assert!(!controller.join("registry.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn registry_is_published_last_on_partial_filesystem_failure() {
    let c = behavior::Case::new("printf same");
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    let args = reviewed_behavior(&c, &controller);
    let mut entries = serde_json::Map::new();
    for n in 0..100 {
        entries.insert(format!("retained-{n}"), json!({"product":"behavior","config":"retained.json","store":"runs","authorization":"auth.json"}));
    }
    let registry = controller.join("registry.json");
    save(&registry, &json!({"schema_version":"1","entries":entries}));
    let original = fs::read(&registry).unwrap();
    assert!(original.len() > 4096);
    assert!(
        fs::metadata(c.dir.join("draft/behavior.json"))
            .unwrap()
            .len()
            < 4096
    );
    let out = terminal_action(&args, "APPROVE test", "", None, "import resource,signal; signal.signal(signal.SIGXFSZ, signal.SIG_IGN); resource.setrlimit(resource.RLIMIT_FSIZE, (4096,4096))");
    code(&out, 3);
    assert!(controller.join("test/config.json").is_file());
    assert!(controller.join("test/authorization.json").is_file());
    assert_eq!(fs::read(&registry).unwrap(), original);
    code(
        &cli()
            .args(["verify", "test", "--registry"])
            .arg(&registry)
            .output()
            .unwrap(),
        3,
    );
    let approved_bytes = fs::read(controller.join("test/config.json")).unwrap();
    code(&terminal(&args, "APPROVE test", None), 3);
    assert_eq!(
        fs::read(controller.join("test/config.json")).unwrap(),
        approved_bytes
    );
    assert_eq!(fs::read(&registry).unwrap(), original);
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn discovery_never_opens_metadata_scripts_databases_or_private_registry_targets() {
    use std::os::unix::fs::symlink;
    let c = behavior::Case::new("exit 99");
    for marker in ["Cargo.toml", "package.json", "Dockerfile", "ledger.db"] {
        assert!(Command::new("mkfifo")
            .arg(c.dir.join(marker))
            .status()
            .unwrap()
            .success());
    }
    fs::create_dir(c.dir.join(".b2ige")).unwrap();
    symlink(c.dir.join("ledger.db"), c.dir.join(".b2ige/project.json")).unwrap();
    let out = cli().arg("inspect").arg(&c.dir).output().unwrap();
    code(&out, 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("unsafe or malformed registry"));
    let sealed = c.dir.join("sealed");
    fs::create_dir(&sealed).unwrap();
    let out = cli()
        .arg("inspect")
        .arg(&sealed)
        .env("B2IGE_BLINDTEST_SEALED_ROOT", &sealed)
        .output()
        .unwrap();
    code(&out, 3);
    assert!(!String::from_utf8_lossy(&out.stderr).contains(sealed.to_str().unwrap()));
}

#[test]
fn prepare_cannot_open_or_write_sealed_material_and_does_not_probe_docker() {
    use std::os::unix::fs::PermissionsExt;
    let c = blindtest::Corpus::temporary();
    let input = c.root.join("public.json");
    save(&input, &c.config);
    fs::remove_file(c.sealed.join("suite.json")).unwrap();
    assert!(Command::new("mkfifo")
        .arg(c.sealed.join("suite.json"))
        .status()
        .unwrap()
        .success());
    let docker = c.root.join("docker");
    fs::write(
        &docker,
        format!("#!/bin/sh\ntouch {}/EXECUTED\n", c.root.display()),
    )
    .unwrap();
    fs::set_permissions(&docker, fs::Permissions::from_mode(0o700)).unwrap();
    for (out, expected) in [(c.root.join("draft"), 0), (c.sealed.join("draft"), 3)] {
        code(
            &cli()
                .args(["prepare", "--product", "blindtest", "--config"])
                .arg(&input)
                .arg("--workspace")
                .arg(&c.workspace)
                .arg("--out")
                .arg(&out)
                .env("B2IGE_BLINDTEST_SEALED_ROOT", &c.sealed)
                .env("B2IGE_DOCKER", &docker)
                .env("PATH", &c.root)
                .output()
                .unwrap(),
            expected,
        );
    }
    assert!(!c.root.join("EXECUTED").exists());
    assert!(!c.sealed.join("draft").exists());
}

#[test]
fn ready_doctor_creates_no_store_executes_no_target_and_is_rejected_by_ci() {
    let c = behavior::Case::new("exit 99");
    let registry = register(
        &c.dir,
        Product::Behavior,
        &c.experiment,
        Some(c.dir.join("auth.json")),
    );
    let out = cli()
        .args(["doctor", "--registry"])
        .arg(&registry)
        .output()
        .unwrap();
    code(&out, 0);
    let readiness: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(readiness["verification_performed"], false);
    assert!(!c.dir.join("store").exists());
    assert_eq!(
        verify_cli::agent::ci_check(&out.stdout, 0).verdict,
        verify_core::Verdict::Error
    );
    let nested = serde_json::to_vec(&readiness["checks"]["test"]).unwrap();
    assert_eq!(
        verify_cli::agent::ci_check(&nested, 0).verdict,
        verify_core::Verdict::Error
    );
    let human = cli()
        .args(["doctor", "--registry"])
        .arg(registry)
        .args(["--output", "human"])
        .output()
        .unwrap();
    code(&human, 0);
    assert!(!String::from_utf8_lossy(&human.stdout).contains("PASS"));
}

#[test]
fn concurrent_reviews_publish_one_registry_and_retry_preserves_the_winner() {
    let c = behavior::Case::new("printf same");
    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    let args = reviewed_behavior(&c, &controller);
    let mut other = args.clone();
    option(&mut other, "--identity", Path::new("second"));
    let commands: Vec<_> = [&args, &other]
        .iter()
        .map(|args| {
            std::iter::once(env!("CARGO_BIN_EXE_b2ige").to_owned())
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
        })
        .collect();
    let script = r#"
import json,os,pty,select,subprocess,sys,time
children=[]
for command in json.loads(sys.argv[1]):
    master,slave=pty.openpty()
    p=subprocess.Popen(command,stdin=slave,stdout=slave,stderr=slave)
    os.close(slave); children.append((p,master,command[command.index('--identity')+1]))
try:
    for p,fd,identity in children:
        buf=b''; deadline=time.monotonic()+30
        while b'Type APPROVE ' not in buf:
            assert time.monotonic()<deadline, 'review timeout'
            if select.select([fd],[],[],0.1)[0]: buf+=os.read(fd,65536)
    for p,fd,identity in children: os.write(fd,('APPROVE '+identity+'\n').encode())
    codes=[p.wait(timeout=30) for p,fd,identity in children]
    assert sorted(codes)==[0,3], codes
finally:
    for p,fd,identity in children:
        if p.poll() is None: p.kill(); p.wait()
        os.close(fd)
"#;
    let out = Command::new("python3")
        .args(["-c", script])
        .arg(serde_json::to_string(&commands).unwrap())
        .output()
        .unwrap();
    code(&out, 0);
    let registry = controller.join("registry.json");
    let p = verify_cli::adoption::registry(&registry).unwrap();
    assert_eq!(p.entries.len(), 1);
    let winner = p.entries.keys().next().unwrap();
    let retained = fs::read(&p.entries[winner].config).unwrap();
    let (retry, answer) = if winner == "test" {
        (&other, "APPROVE second")
    } else {
        (&args, "APPROVE test")
    };
    code(&terminal(retry, answer, None), 0);
    let p = verify_cli::adoption::registry(&registry).unwrap();
    assert_eq!(p.entries.len(), 2);
    assert_eq!(fs::read(&p.entries[winner].config).unwrap(), retained);
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn blindtest_approval_withholds_controller_paths_and_rejects_mutable_image_or_project_overlap() {
    let c = blindtest::Corpus::temporary();
    let draft = c.root.join("draft");
    fs::create_dir(&draft).unwrap();
    let mut config = c.config.clone();
    config.target.image = "public-mutable-tag".into();
    save(&draft.join("blindtest.json"), &config);
    let args = approval_args(&draft, &c.sealed, &c.workspace, "blindtest");
    let out = terminal_env(&args, "APPROVE test", None, Some(&c.sealed));
    code(&out, 3);
    let all = [out.stdout, out.stderr].concat();
    let text = String::from_utf8_lossy(&all);
    assert!(text.contains("paths withheld"));
    assert!(!text.contains(c.sealed.to_str().unwrap()));
    assert!(!text.contains(&c.suite.private_canary));
    assert!(!c.sealed.join("registry.json").exists());

    let controller = std::env::temp_dir().join(verify_cli::integration::nonce());
    fs::create_dir(&controller).unwrap();
    let args = approval_args(&draft, &controller, &c.root, "blindtest");
    let out = terminal_env(&args, "APPROVE test", None, Some(&c.sealed));
    code(&out, 3);
    assert!(String::from_utf8_lossy(&out.stdout).contains("unsafe private path separation"));
    assert!(!controller.join("registry.json").exists());
    fs::remove_dir_all(controller).unwrap();
}

#[test]
fn init_preserves_custom_registry_bytes_and_verify_rejects_ambiguous_or_malformed_entries() {
    let c = behavior::Case::new("printf same");
    fs::create_dir(c.dir.join(".b2ige")).unwrap();
    let registry = c.dir.join(".b2ige/project.json");
    let valid = " { \"entries\": {\"retained\": {\"product\":\"behavior\",\"config\":\"missing.json\",\"store\":\"runs\",\"authorization\":null}},\n\"schema_version\":\"1\" }\n";
    fs::write(&registry, valid).unwrap();
    for command in ["init", "setup"] {
        code(&cli().arg(command).arg(&c.dir).output().unwrap(), 0);
        assert_eq!(fs::read_to_string(&registry).unwrap(), valid);
    }
    for invalid in [
        r#"{"schema_version":"1","schema_version":"1","entries":{}}"#,
        r#"{"schema_version":"1","entries":{},"entries":{}}"#,
        r#"{"schema_version":"1","entries":{"test":{"product":"behavior","config":"x","store":"s"},"test":{"product":"sideeffect","config":"x","store":"s"}}}"#,
        r#"{"schema_version":"1","entries":{"test":{"product":"auto","config":"x","store":"s"}}}"#,
        r#"{"schema_version":"1","entries":{"test":{"product":"behavior","config":false,"store":"s"}}}"#,
    ] {
        fs::write(&registry, invalid).unwrap();
        code(&cli().arg("init").arg(&c.dir).output().unwrap(), 3);
        code(
            &cli()
                .args(["verify", "test", "--registry"])
                .arg(&registry)
                .args(["--output", "agent", "--protocol", "1"])
                .output()
                .unwrap(),
            3,
        );
        assert_eq!(fs::read_to_string(&registry).unwrap(), invalid);
        assert!(!c.dir.join("runs").exists());
    }
}

#[test]
fn system_executable_aliases_resolve_to_the_bytes_pinned_in_prepared_contracts() {
    for executable in ["/usr/bin/python3", "/bin/sh"] {
        let input = Path::new(executable);
        let resolved = verify_cli::adoption::path(input).unwrap();
        assert_eq!(resolved, fs::canonicalize(input).unwrap());
        assert!(!fs::symlink_metadata(&resolved)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            verify_core::behavior::executable_identity(input).unwrap(),
            verify_core::behavior::executable_identity(&resolved).unwrap()
        );
    }
    let c = sideeffect::Case::new("safe");
    let input = c.dir.join("input.json");
    save(&input, &c.contract);
    let out = c.dir.join("draft");
    code(
        &cli()
            .args(["prepare", "--product", "sideeffect", "--config"])
            .arg(&input)
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
        0,
    );
    let prepared: verify_core::sideeffect::SideEffectContract =
        verify_cli::integration::read(&out.join("sideeffect.json")).unwrap();
    assert_eq!(
        prepared.trigger.executable,
        fs::canonicalize("/usr/bin/python3").unwrap()
    );
    assert_eq!(
        prepared.trigger.executable_hash,
        verify_core::behavior::executable_identity(&prepared.trigger.executable).unwrap()
    );
}

#[test]
fn system_alias_support_never_admits_project_controller_or_fixture_links_or_traversal() {
    use std::os::unix::fs::symlink;
    let c = sideeffect::Case::new("safe");
    let controller = c.dir.join("controller");
    fs::create_dir(&controller).unwrap();
    for directory in [&c.dir, &controller, &c.dir.join("fixture")] {
        // Even a link to the genuine system executable remains user indirection.
        let executable = directory.join("python3");
        symlink("/usr/bin/python3", &executable).unwrap();
        assert!(verify_cli::adoption::path(&executable).is_err());
        let alias = directory.join("bin");
        symlink("/usr/bin", &alias).unwrap();
        assert!(verify_cli::adoption::path(&alias.join("python3")).is_err());
    }
    for input in ["/usr/bin/../bin/python3", "/bin/../bin/sh"] {
        assert!(verify_cli::adoption::path(Path::new(input)).is_err());
    }
    let mut contract = c.contract.clone();
    contract.trigger.executable = controller.join("python3");
    let input = c.dir.join("input.json");
    save(&input, &contract);
    let out = c.dir.join("draft");
    code(
        &cli()
            .args(["prepare", "--product", "sideeffect", "--config"])
            .arg(input)
            .arg("--fixture")
            .arg(c.dir.join("fixture"))
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
        3,
    );
    assert!(!out.exists());
}
