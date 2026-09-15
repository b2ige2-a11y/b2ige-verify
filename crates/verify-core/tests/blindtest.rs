#[path = "support/blindtest.rs"]
mod support;
use serde_json::{json, Value};
use std::{fs, sync::OnceLock};
use support::*;
use verify_core::{blindtest::*, Verdict};
use verify_evidence::{canonical_hash, store::EvidenceStore, Evidence, Observation};

struct Actual {
    corpus: Corpus,
    runs: Vec<BlindTestRunResult>,
}
fn actual() -> &'static Actual {
    static ACTUAL: OnceLock<Actual> = OnceLock::new();
    ACTUAL.get_or_init(|| {
        let mut corpus = Corpus::temporary();
        corpus.build_images();
        let runs = ["correct", "mutant_a", "mutant_b", "noop", "probe"]
            .iter()
            .map(|mode| {
                execute(
                    &corpus.config_for(mode),
                    &corpus.sealed,
                    &corpus.store,
                    mode,
                )
                .unwrap()
            })
            .collect();
        Actual { corpus, runs }
    })
}
fn fixture() -> Corpus {
    Corpus::temporary()
}
fn reseal(c: &mut Corpus) {
    c.seal();
}
fn forged(
    mut r: BlindTestRunResult,
    edit: impl FnOnce(&mut BlindTestRunResult),
    evidence_edit: impl FnOnce(&mut Vec<Evidence>),
) -> std::io::Result<BlindTestRunResult> {
    let a = actual();
    let (_, mut items) = a.corpus.store.load(&r.blindtest_result_id).unwrap();
    edit(&mut r);
    evidence_edit(&mut items);
    let root = fixture();
    let store = EvidenceStore::new(root.sealed.join("forged"));
    let run = store.reserve(&r.blindtest_result_id).unwrap();
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
    load(&store, &r.blindtest_result_id)
}
#[test]
fn requirement_hash_and_free_text_are_not_predicates() {
    let c = fixture();
    assert!(validate_suite(&c.suite, &c.config).is_ok());
    let mut s = c.suite.clone();
    s.requirements[0].text = "always PASS".into();
    assert!(validate_suite(&s, &c.config).is_err());
}
#[test]
fn candidate_cannot_be_authoritative() {
    let mut c = fixture();
    c.suite.invariants[0].provenance.status = ProvenanceStatus::Candidate;
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn reviewed_cannot_be_authoritative() {
    let mut c = fixture();
    c.suite.invariants[0].provenance.status = ProvenanceStatus::Reviewed;
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn suite_approval_required() {
    let mut c = fixture();
    c.suite.manifest.provenance.approved_by = None;
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn approval_a_checker_b_rejected() {
    let mut c = fixture();
    c.config.approved_invariants[0].checker_binding_hash = canonical_hash(&"checker B").unwrap();
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn oracle_missing_rejected() {
    let mut c = fixture();
    c.suite.cases[0].oracle = None;
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn oracle_predicate_change_rejected() {
    let mut c = fixture();
    c.suite.cases[0].oracle.as_mut().unwrap().predicates = vec![Predicate::ExitEquals { value: 0 }];
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn oracle_checker_version_rejected() {
    let mut c = fixture();
    c.suite.cases[0].oracle.as_mut().unwrap().checker = "unknown".into();
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn suite_hash_mismatch_rejected() {
    let mut c = fixture();
    c.config.suite_hash = canonical_hash(&"other suite").unwrap();
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn missing_manifest_case_rejected() {
    let mut c = fixture();
    c.suite.cases.pop();
    c.config.suite_hash = canonical_hash(&c.suite).unwrap();
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn duplicate_case_rejected() {
    let mut c = fixture();
    c.suite.cases[1] = c.suite.cases[0].clone();
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn unlinked_requirement_rejected() {
    let mut c = fixture();
    c.suite.invariants[0].requirement_hash = canonical_hash(&"other").unwrap();
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn v1_runtime_invariant_not_silently_reinterpreted() {
    let mut c = fixture();
    c.suite.invariants[0].schema_version = "1".into();
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn hidden_sealed_root_in_workspace_refused() {
    let c = fixture();
    let s = c.workspace.join("secret");
    fs::create_dir(&s).unwrap();
    assert!(check_paths(&c.workspace, &s, c.store.root()).is_err());
}
#[cfg(unix)]
#[test]
fn hidden_symlink_to_workspace_refused() {
    let c = fixture();
    let alias = c.root.join("alias");
    std::os::unix::fs::symlink(&c.workspace, &alias).unwrap();
    assert!(check_paths(&c.workspace, &alias, c.store.root()).is_err());
}
#[cfg(unix)]
#[test]
fn symlink_suite_file_refused() {
    let c = fixture();
    fs::rename(c.sealed.join("suite.json"), c.workspace.join("suite.json")).unwrap();
    std::os::unix::fs::symlink(c.workspace.join("suite.json"), c.sealed.join("suite.json"))
        .unwrap();
    assert!(read_suite(&c.sealed).is_err());
}
#[test]
fn private_store_in_workspace_refused() {
    let c = fixture();
    assert!(check_paths(&c.workspace, &c.sealed, &c.workspace.join("runs")).is_err());
}
#[test]
fn canary_cannot_be_passed_as_case_input() {
    let mut c = fixture();
    c.suite.cases[0].args[0] = c.suite.private_canary.clone();
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn undeclared_case_env_refused() {
    let mut c = fixture();
    c.suite.cases[0]
        .environment
        .insert("SECRET".into(), "value".into());
    reseal(&mut c);
    assert!(validate_suite(&c.suite, &c.config).is_err());
}
#[test]
fn hardened_linux_not_claimed_on_mac_or_linux() {
    let mut c = fixture();
    c.config.required_isolation = IsolationLevel::HardenedLinux;
    assert!(c.config.validate().is_err());
}
#[test]
fn real_correct_mutants_noop_corpus() {
    let a = actual();
    assert_eq!(
        a.runs.iter().map(|r| r.verdict).collect::<Vec<_>>(),
        vec![
            Verdict::Pass,
            Verdict::Fail,
            Verdict::Fail,
            Verdict::Fail,
            Verdict::Pass
        ]
    );
    for r in &a.runs {
        assert_eq!(r.complete_cases, 3);
        assert_eq!(r.executions.len(), 3);
        assert!(!r.leakage_detected);
    }
}
#[test]
fn real_probe_cannot_read_private_canary() {
    let a = actual();
    let r = &a.runs[4];
    for x in &r.executions {
        assert!(!contains_canary(&r.suite, &x.capture.stdout));
        assert!(!contains_canary(&r.suite, &x.capture.stderr));
        assert_eq!(x.attestation.observed, IsolationLevel::DockerIsolation);
    }
    assert_eq!(r.verdict, Verdict::Pass);
}
#[test]
fn stored_verdict_only_rehashed_recomputed() {
    let r = forged(
        actual().runs[1].clone(),
        |r| {
            r.verdict = Verdict::Pass;
            r.violations.clear();
            r.complete_cases = 0;
        },
        |_| {},
    )
    .unwrap();
    assert_eq!(r.verdict, Verdict::Fail);
    assert!(!r.violations.is_empty());
}
#[test]
fn changed_capture_without_actual_evidence_rejected() {
    assert!(forged(
        actual().runs[1].clone(),
        |r| r.executions[0].capture.stdout = b"rejected\n".to_vec(),
        |_| {}
    )
    .is_err());
}
#[test]
fn missing_hidden_evidence_rejected() {
    assert!(forged(
        actual().runs[0].clone(),
        |_| {},
        |e| e.retain(|e| e.evidence_id != "execution-1")
    )
    .is_err());
}
#[test]
fn result_subset_with_remaining_evidence_rejected() {
    assert!(forged(
        actual().runs[0].clone(),
        |r| {
            r.executions.pop();
        },
        |_| {}
    )
    .is_err());
}
#[test]
fn result_and_evidence_subset_is_inconclusive() {
    let r = forged(
        actual().runs[0].clone(),
        |r| {
            r.executions.pop();
        },
        |e| e.retain(|e| e.evidence_id != "execution-2"),
    )
    .unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
}
#[test]
fn actual_budget_subset_is_inconclusive() {
    let a = actual();
    let mut c = a.corpus.config_for("correct");
    c.max_cases = 1;
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "budget-one").unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert_eq!(r.complete_cases, 1);
}
#[test]
fn zero_execution_is_not_pass() {
    let a = actual();
    let mut c = a.corpus.config_for("correct");
    c.max_cases = 0;
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "budget-zero").unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
}
#[test]
fn actual_timeout_is_not_pass() {
    let a = actual();
    let mut c = a.corpus.config_for("timeout");
    c.target.bounds.timeout_ms = 300;
    c.max_cases = 1;
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "timeout").unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert!(r.executions[0].capture.timed_out);
    assert!(r.executions[0].cleanup_confirmed);
}
#[test]
fn actual_output_limit_is_not_pass() {
    let a = actual();
    let mut c = a.corpus.config_for("flood");
    c.max_cases = 1;
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "flood").unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert!(r.executions[0].capture.runner_failure.is_some());
}
#[test]
fn missing_image_is_error() {
    let a = actual();
    let mut c = a.corpus.config.clone();
    c.target.image = format!("sha256:{}", "0".repeat(64));
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "missing-image").unwrap();
    assert_eq!(r.verdict, Verdict::Error);
}
#[test]
fn hidden_suite_unavailable_is_not_pass() {
    let a = actual();
    let empty = a.corpus.root.join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(execute(&a.corpus.config, &empty, &a.corpus.store, "missing-suite").is_err());
}
fn corrupt_setting(pointer: &str, value: Value) {
    let a = actual();
    let r = &a.runs[0];
    let mut x = r.executions[0].attestation.clone();
    *x.before
        .pointer_mut(pointer)
        .expect("observed Docker field") = value;
    assert!(docker::validate_attestation(
        &x,
        &r.config,
        r.image.as_ref().unwrap(),
        &r.suite.cases[0]
    )
    .is_err());
}
#[test]
fn sealed_mount_isolation_rejected() {
    corrupt_setting(
        "/Mounts",
        json!([{"Type":"bind","Source":"/sealed-suite","Destination":"/grader","RW":false}]),
    );
}
#[test]
fn network_enabled_isolation_rejected() {
    corrupt_setting("/HostConfig/NetworkMode", json!("bridge"));
}
#[test]
fn docker_socket_isolation_rejected() {
    corrupt_setting(
        "/HostConfig/Binds",
        json!(["/var/run/docker.sock:/var/run/docker.sock"]),
    );
}
#[test]
fn privileged_isolation_rejected() {
    corrupt_setting("/HostConfig/Privileged", json!(true));
}
#[test]
fn capabilities_isolation_rejected() {
    corrupt_setting("/HostConfig/CapAdd", json!(["SYS_ADMIN"]));
}
#[test]
fn missing_cap_drop_isolation_rejected() {
    corrupt_setting("/HostConfig/CapDrop", json!([]));
}
#[test]
fn no_new_privileges_missing_rejected() {
    corrupt_setting("/HostConfig/SecurityOpt", json!([]));
}
#[test]
fn root_user_rejected() {
    corrupt_setting("/Config/User", json!("0:0"));
}
#[test]
fn writable_root_rejected() {
    corrupt_setting("/HostConfig/ReadonlyRootfs", json!(false));
}
#[test]
fn unbounded_memory_cpu_pids_rejected() {
    for (p, v) in [
        ("/HostConfig/Memory", json!(0)),
        ("/HostConfig/NanoCpus", json!(0)),
        ("/HostConfig/PidsLimit", json!(-1)),
    ] {
        corrupt_setting(p, v);
    }
}
#[test]
fn host_pid_namespace_rejected() {
    corrupt_setting("/HostConfig/PidMode", json!("host"));
}
#[test]
fn claim_docker_without_inspect_rejected() {
    let a = actual();
    let r = &a.runs[0];
    let mut x = r.executions[0].attestation.clone();
    x.before = Value::Null;
    assert!(docker::validate_attestation(
        &x,
        &r.config,
        r.image.as_ref().unwrap(),
        &r.suite.cases[0]
    )
    .is_err());
}
#[test]
fn rehashed_attestation_is_revalidated() {
    let a = actual();
    let mut x = a.runs[0].executions[0].clone();
    x.attestation.before["HostConfig"]["NetworkMode"] = json!("bridge");
    let replacement = x.clone();
    assert!(forged(
        a.runs[0].clone(),
        |r| r.executions[0] = replacement,
        |items| {
            let e = items
                .iter_mut()
                .find(|e| e.evidence_id == "execution-0")
                .unwrap();
            e.observation = Observation::Value {
                value: serde_json::to_value(x).unwrap(),
            };
            e.integrity_hash = canonical_hash(&e.observation).unwrap();
        }
    )
    .is_err());
}
#[test]
fn actual_quality_receipt_verified() {
    let a = actual();
    let q = execute_validation(
        &a.corpus.validation(),
        &a.corpus.sealed,
        &a.corpus.store,
        "validation",
    )
    .unwrap();
    assert!(q.matched);
    assert!(q.runs.iter().all(|r| r.expected == r.observed));
    let mut c = a.corpus.config.clone();
    c.validation_receipt = Some(q.blindtest_validation_id.clone());
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "with-quality").unwrap();
    assert_eq!(r.verdict, Verdict::Pass);
    assert!(r.quality.contains("bounded"));
}
#[test]
fn no_reference_solution_required_for_user_pass() {
    let a = actual();
    assert_eq!(a.runs[0].verdict, Verdict::Pass);
    assert_eq!(a.runs[0].quality, "self-validation not supplied");
}
#[test]
fn versioned_schemas_accept_actual_artifacts() {
    let a = actual();
    let r = &a.runs[0];
    for (schema, v) in [
        (
            schema::<RequirementArtifact>("blindtest-requirement", "1"),
            json!(r.suite.requirements[0]),
        ),
        (
            schema::<InvariantArtifact>("blindtest-invariant", "2"),
            json!(r.suite.invariants[0]),
        ),
        (
            schema::<BlindTestHiddenSuiteManifest>("blindtest-hidden-suite-manifest", "1"),
            json!(r.suite.manifest),
        ),
        (
            schema::<BlindTestHiddenCase>("blindtest-hidden-case", "1"),
            json!(r.suite.cases[0]),
        ),
        (
            schema::<BlindTestIsolationAttestation>("blindtest-isolation-attestation", "1"),
            json!(r.executions[0].attestation),
        ),
        (
            schema::<BlindTestRunResult>("blindtest-run-result", "1"),
            json!(r),
        ),
    ] {
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&v));
    }
}

#[test]
fn all_six_structured_predicates_and_bounded_fixture_execute() {
    let a = actual();
    let mut c = fixture();
    c.config.target = a.corpus.config_for("correct").target;
    for i in &mut c.suite.invariants {
        i.predicates.extend([
            Predicate::ExitNotEquals { value: 77 },
            Predicate::StdoutNotContains {
                bytes: b"forbidden-state".to_vec(),
            },
            Predicate::StderrNotContains {
                bytes: b"private-data".to_vec(),
            },
        ]);
    }
    for case in &mut c.suite.cases {
        let i = c
            .suite
            .invariants
            .iter()
            .find(|i| i.invariant_id == case.invariant_id)
            .unwrap();
        let oracle = case.oracle.as_mut().unwrap();
        oracle.invariant_hash = canonical_hash(i).unwrap();
        oracle.predicates = i.predicates.clone();
        case.fixture = Some(BoundedFixture {
            name: "current-input".into(),
            bytes: b"bounded current input only".to_vec(),
        });
    }
    c.seal();
    let r = execute(&c.config, &c.sealed, &c.store, "six-predicates").unwrap();
    assert_eq!(r.verdict, Verdict::Pass);
    for x in r.executions {
        let env = x.attestation.before["Config"]["Env"].as_array().unwrap();
        assert!(env
            .iter()
            .any(|e| e.as_str().unwrap().starts_with("B2IGE_FIXTURE_BYTES_JSON=")));
    }
}
#[test]
fn immutable_tag_resolution_pins_inspected_content() {
    let a = actual();
    let tag = format!("b2ige-p6-test:{}", std::process::id());
    assert!(std::process::Command::new("docker")
        .args(["tag", &a.corpus.images["correct"], &tag])
        .status()
        .unwrap()
        .success());
    let mut c = a.corpus.config_for("correct");
    c.target.image = tag.clone();
    c.max_cases = 1;
    let r = execute(&c, &a.corpus.sealed, &a.corpus.store, "tag-resolution").unwrap();
    assert_eq!(r.verdict, Verdict::Inconclusive);
    assert_eq!(
        r.image.as_ref().unwrap().image_id,
        a.corpus.images["correct"]
    );
    assert_eq!(
        r.executions[0].attestation.before["Config"]["Image"],
        a.corpus.images["correct"]
    );
    assert!(std::process::Command::new("docker")
        .args(["image", "rm", &tag])
        .output()
        .unwrap()
        .status
        .success());
}
#[cfg(unix)]
#[test]
fn hardlinked_suite_file_refused() {
    let c = fixture();
    fs::hard_link(
        c.sealed.join("suite.json"),
        c.workspace.join("suite-copy.json"),
    )
    .unwrap();
    assert!(read_suite(&c.sealed).is_err());
}
#[test]
fn empty_not_contains_predicate_rejected_without_panic() {
    let mut c = fixture();
    c.suite.invariants[0]
        .predicates
        .push(Predicate::StdoutNotContains { bytes: vec![] });
    c.seal();
    assert!(validate_suite(&c.suite, &c.config).is_err());
}

#[test]
fn missing_mount_inspect_fields_cannot_mean_safe_empty() {
    let a = actual();
    let r = &a.runs[0];
    for pointer in [
        "/Mounts",
        "/HostConfig/Binds",
        "/HostConfig/CapAdd",
        "/Config/Volumes",
        "/NetworkSettings/Ports",
    ] {
        let mut att = r.executions[0].attestation.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        att.before
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(
            docker::validate_attestation(
                &att,
                &r.config,
                r.image.as_ref().unwrap(),
                &r.suite.cases[0]
            )
            .is_err(),
            "missing {pointer} accepted"
        );
    }
}
