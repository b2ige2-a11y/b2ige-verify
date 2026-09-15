#![cfg(unix)]
#[path = "support/behavior.rs"]
mod behavior;
#[path = "../../verify-core/tests/support/blindtest.rs"]
mod blindtest;
#[path = "support/sideeffect.rs"]
mod sideeffect;
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Client {
    fn new(registry: &Path, sealed: Option<&Path>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_b2ige-mcp"));
        command
            .arg("--registry")
            .arg(registry)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(s) = sealed {
            command.env("B2IGE_BLINDTEST_SEALED_ROOT", s);
        }
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut c = Self {
            child,
            input,
            output,
        };
        let v=c.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}));
        assert_eq!(v["result"]["protocolVersion"], "2025-11-25");
        writeln!(
            c.input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        c.input.flush().unwrap();
        c
    }
    fn send(&mut self, v: Value) -> Value {
        writeln!(self.input, "{v}").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn call(&mut self, tool: &str, product: &str, operation: &str, identity: &str) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":{"protocol_version":"1","product":product,"operation":operation,"identity":identity,"output":"agent","execution_budget":null}}}))
    }
}
fn write(path: &Path, v: Value) {
    fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
}
fn registry(root: &Path, product: &str, config: Value, auth: Value) -> std::path::PathBuf {
    write(&root.join("config.json"), config);
    write(&root.join("authorization.json"), auth);
    let path = root.join("project.json");
    write(
        &path,
        json!({"schema_version":"1","entries":{"target":{"product":product,"config":root.join("config.json"),"store":if product=="blindtest" {root.join("sealed/runs")} else {root.join("runs")},"authorization":root.join("authorization.json")}}}),
    );
    path
}
fn assert_result(v: &Value, verdict: &str) {
    assert!(v.get("error").is_none(), "{v}");
    let r = &v["result"]["structuredContent"];
    assert_eq!(r["verdict"], verdict, "{v}");
    assert_eq!(v["result"]["isError"], verdict != "PASS");
    assert_eq!(
        serde_json::from_str::<Value>(v["result"]["content"][0]["text"].as_str().unwrap()).unwrap(),
        *r
    );
    assert!(
        jsonschema::validator_for(&verify_cli::agent::response_schema())
            .unwrap()
            .is_valid(r),
        "{r}"
    );
}
#[test]
fn behavior_actual_pass_fail_reproduction_and_reload() {
    for (body, verdict) in [("printf same", "PASS"), ("printf changed", "FAIL")] {
        let c = behavior::Case::new(body);
        let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
        let mut m = Client::new(&p, None);
        let v = m.call("b2ige_behavior_verify", "behavior", "verify", "target");
        assert_result(&v, verdict);
        let r = &v["result"]["structuredContent"];
        assert!(r["reproduction"]["artifact"].is_object());
        let id = r["source"]["artifact_id"].as_str().unwrap();
        let a = m.call("b2ige_report", "behavior", "report", id);
        let b = m.call("b2ige_report", "behavior", "report", id);
        assert_eq!(a, b);
        assert_result(&a, verdict);
        if verdict == "FAIL" {
            assert!(!r["evidence_refs"].as_array().unwrap().is_empty());
            assert!(r["next_action"].as_str().unwrap().contains("repair"));
        }
        // Required evidence removal cannot become PASS on report reload.
        fs::remove_dir_all(c.dir.join("runs").join(id)).unwrap();
        assert_result(&m.call("b2ige_report", "behavior", "report", id), "ERROR");
    }
}
#[test]
fn sideeffect_actual_pass_fail_inconclusive() {
    for (mode, verdict) in [
        ("safe", "PASS"),
        ("unsafe", "FAIL"),
        ("partial", "INCONCLUSIVE"),
    ] {
        let mut c = sideeffect::Case::new(mode);
        if mode == "partial" {
            c.contract.exploration_budget.max_attempts = 0;
        }
        let p = registry(&c.dir, "sideeffect", json!(c.contract), Value::Null);
        let mut m = Client::new(&p, None);
        let v = m.call("b2ige_sideeffect_verify", "sideeffect", "verify", "target");
        assert_result(&v, verdict);
        assert!(!v.to_string().contains("secret-do-not-export"));
    }
}
#[test]
fn blindtest_actual_docker_all_verdicts_no_hidden_leakage() {
    let mut c = blindtest::Corpus::temporary();
    c.build_images();
    for (mode, verdict) in [
        ("correct", "PASS"),
        ("mutant_a", "FAIL"),
        ("partial", "INCONCLUSIVE"),
        ("error", "ERROR"),
    ] {
        let mut config = c.config_for(if mode == "mutant_a" { mode } else { "correct" });
        if mode == "partial" {
            config.max_cases = 1;
        }
        if mode == "error" {
            config.target.image = format!("sha256:{}", "0".repeat(64));
        }
        let p = registry(&c.root, "blindtest", json!(config), Value::Null);
        let mut m = Client::new(&p, Some(&c.sealed));
        let v = m.call("b2ige_blindtest_verify", "blindtest", "verify", "target");
        assert_result(&v, verdict);
        let text = v.to_string();
        for secret in std::iter::once(c.suite.private_canary.as_str())
            .chain(c.suite.cases.iter().map(|c| c.case_id.as_str()))
            .chain(
                c.suite
                    .cases
                    .iter()
                    .flat_map(|c| c.args.iter().map(String::as_str)),
            )
        {
            assert!(!text.contains(secret));
        }
        for field in [
            "hidden_details",
            "private_metadata",
            "oracle",
            "sealed",
            "docker.sock",
            "/Users/",
            "/var/",
        ] {
            assert!(!text.contains(field), "{field}");
        }
        let id = v["result"]["structuredContent"]["source"]["artifact_id"]
            .as_str()
            .unwrap();
        assert_result(&m.call("b2ige_report", "blindtest", "report", id), verdict);
    }
}
#[test]
fn no_shell_file_or_config_substitution_surface() {
    let c = behavior::Case::new("printf same");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut m = Client::new(&p, None);
    let list = m.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}));
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 5);
    for tool in [
        "shell",
        "read_file",
        "docker_exec",
        "sqlite_query",
        "human_report",
    ] {
        assert!(m
            .call(tool, "behavior", "verify", "target")
            .get("error")
            .is_some());
    }
    assert_result(
        &m.call("b2ige_behavior_verify", "behavior", "verify", "/etc/passwd"),
        "ERROR",
    );
    let v=m.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"b2ige_behavior_verify","arguments":{"protocol_version":"1","product":"behavior","operation":"verify","identity":"target","output":"agent","command":"touch /tmp/forbidden"}}}));
    assert!(v.get("error").is_some());
    assert_result(
        &m.call("b2ige_blindtest_verify", "behavior", "verify", "target"),
        "ERROR",
    );
    assert!(m.send(json!({"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"file:///etc/passwd"}})).get("error").is_some());
}
#[test]
fn pinned_config_and_malformed_config_fail_closed() {
    let c = behavior::Case::new("printf changed");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut m = Client::new(&p, None);
    fs::write(c.dir.join("config.json"), b"malformed private canary").unwrap();
    assert_result(
        &m.call("b2ige_behavior_verify", "behavior", "verify", "target"),
        "FAIL",
    );
    let mut fresh = Client::new(&p, None);
    let v = fresh.call("b2ige_behavior_verify", "behavior", "verify", "target");
    assert_result(&v, "ERROR");
    assert!(!v.to_string().contains("canary"));
}
#[test]
fn target_launch_failure_is_infrastructure_error() {
    let c = behavior::Case::new("printf same");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut m = Client::new(&p, None);
    fs::remove_file(&c.experiment.after.executable).unwrap();
    assert_result(
        &m.call("b2ige_behavior_verify", "behavior", "verify", "target"),
        "ERROR",
    );
}
#[test]
fn doctor_readiness_never_verification_completion() {
    let c = behavior::Case::new("printf changed");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut m = Client::new(&p, None);
    let d = m.call("b2ige_doctor", "behavior", "doctor", "target");
    assert_result(&d, "PASS");
    assert_eq!(d["result"]["structuredContent"]["kind"], "readiness");
    assert_eq!(d["result"]["structuredContent"]["source"], Value::Null);
    assert_result(
        &m.call("b2ige_behavior_verify", "behavior", "verify", "target"),
        "FAIL",
    );
}
#[test]
fn malformed_transport_and_initialization_fail_closed() {
    let c = behavior::Case::new("printf same");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut s = verify_mcp::Server::open(&p).unwrap();
    let mut out = Vec::new();
    s.serve(
        std::io::Cursor::new(
            b"bad\n[]\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
        ),
        &mut out,
    )
    .unwrap();
    for line in String::from_utf8(out).unwrap().lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        assert!(v.get("error").is_some());
    }
}

#[test]
fn doctor_missing_sqlite_observer_is_not_ready() {
    let mut c = sideeffect::Case::new("safe");
    fs::remove_file(c.dir.join("fixture/ledger.db")).unwrap();
    c.contract.trigger.fixture.snapshot_identity =
        verify_core::behavior::snapshot_identity(c.contract.trigger.fixture.source.as_deref())
            .unwrap();
    let p = registry(&c.dir, "sideeffect", json!(c.contract), Value::Null);
    let mut m = Client::new(&p, None);
    assert_result(
        &m.call("b2ige_doctor", "sideeffect", "doctor", "target"),
        "ERROR",
    );
    assert_result(
        &m.call("b2ige_sideeffect_verify", "sideeffect", "verify", "target"),
        "ERROR",
    );
}
#[test]
fn budget_override_and_unknown_report_are_errors() {
    let c = behavior::Case::new("printf same");
    let p = registry(&c.dir, "behavior", json!(c.experiment), json!(c.auth));
    let mut m = Client::new(&p, None);
    let v=m.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"b2ige_behavior_verify","arguments":{"protocol_version":"1","product":"behavior","operation":"verify","identity":"target","output":"agent","execution_budget":0}}}));
    assert_result(&v, "ERROR");
    assert_result(
        &m.call("b2ige_report", "behavior", "report", "/etc/passwd"),
        "ERROR",
    );
}
