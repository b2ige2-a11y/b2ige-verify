#![cfg(unix)]
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use verify_core::{
    acquisition::{acquire, AcquisitionResult, RunState},
    ConformanceFixture, Verdict,
};
use verify_evidence::{canonical_bytes, canonical_hash, store::EvidenceStore, ObservationCoverage};
use verify_runner::{
    process::{ProcessSpec, OBSERVER},
    ExecutionStatus, IsolationLevel,
};

struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "b2ige-p1b-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.0.join("runs"))
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn inputs(dir: &TestDir, mode: &str) -> (verify_core::ExperimentPlan, ProcessSpec) {
    let fixture: ConformanceFixture =
        serde_json::from_str(include_str!("../../../tests/fixtures/pass.json")).unwrap();
    let mut plan = fixture.policy.plan;
    plan.required_observers = vec![OBSERVER.into()];
    plan.optional_observers.clear();
    plan.isolation = IsolationLevel::None;
    plan.reset_strategy = "none".into();
    plan.time_budget_ms = 3000;
    let spec = ProcessSpec {
        executable: PathBuf::from(env!("CARGO_BIN_EXE_p1b-process-fixture")),
        args: vec![mode.into()],
        working_directory: dir.0.clone(),
        environment: BTreeMap::new(),
        timeout_ms: plan.time_budget_ms,
    };
    (plan, spec)
}
fn run(dir: &TestDir, mode: &str) -> AcquisitionResult {
    let (plan, spec) = inputs(dir, mode);
    acquire(&plan, &spec, &dir.store(), "run-1").unwrap()
}
#[test]
fn real_exit_zero_bytes_context_hash_and_canonical_roundtrip() {
    let dir = TestDir::new();
    let result = run(&dir, "success");
    assert_eq!(result.observation.stdout, b"hello\n\x00\xff");
    assert!(result.observation.stderr.is_empty());
    assert_eq!(result.observation.exit_code, Some(0));
    assert!(result.observation.started);
    assert!(result.context.valid());
    assert_eq!(result.execution, ExecutionStatus::Completed);
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert_eq!(
        result.lifecycle,
        [
            RunState::Created,
            RunState::Running,
            RunState::Observed,
            RunState::Completed
        ]
    );
    assert_eq!(
        result.bundle.coverage[OBSERVER].status,
        ObservationCoverage::Complete
    );
    assert_eq!(
        result.context.plan_hash,
        canonical_hash(&result.plan).unwrap()
    );
    assert_eq!(
        result.context.config_hash,
        canonical_hash(&result.process).unwrap()
    );
    let (stored, evidence) = dir.store().load("run-1").unwrap();
    assert_eq!(
        serde_json::from_value::<AcquisitionResult>(stored).unwrap(),
        result
    );
    assert_eq!(evidence.len(), 1);
    assert!(evidence[0].valid());
    assert_eq!(evidence[0].run_id, result.context.run_id);
    assert_eq!(evidence[0].related_claim_ids, result.bundle.claim_refs);
    assert_eq!(
        result.bundle.evidence_hashes[&evidence[0].evidence_id],
        canonical_hash(&evidence[0]).unwrap()
    );
    let raw = fs::read(dir.0.join("runs/run-1/result.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(raw, canonical_bytes(&value).unwrap());
    assert_eq!(
        canonical_bytes(&result).unwrap(),
        canonical_bytes(&result.clone()).unwrap()
    );
}
#[test]
fn real_nonzero_is_observed_not_runner_error_and_stderr_persists() {
    let dir = TestDir::new();
    let result = run(&dir, "failure");
    assert_eq!(result.observation.exit_code, Some(7));
    assert_eq!(result.observation.stderr, b"target failure\n");
    assert_eq!(result.execution, ExecutionStatus::Completed);
    assert_eq!(result.verdict, Verdict::Inconclusive);
    let (stored, _) = dir.store().load("run-1").unwrap();
    assert_eq!(
        serde_json::from_value::<AcquisitionResult>(stored)
            .unwrap()
            .observation
            .stderr,
        b"target failure\n"
    );
}
#[test]
fn real_timeout_kills_and_records_partial_output() {
    let dir = TestDir::new();
    let (mut plan, mut spec) = inputs(&dir, "timeout");
    plan.time_budget_ms = 150;
    spec.timeout_ms = 150;
    let start = std::time::Instant::now();
    let result = acquire(&plan, &spec, &dir.store(), "run-1").unwrap();
    assert!(start.elapsed().as_secs() < 3);
    assert!(result.observation.timed_out);
    assert_eq!(result.observation.signal, Some(9));
    assert!(result.observation.runner_failure.is_none());
    assert_eq!(result.observation.stdout, b"before timeout\n");
    assert_eq!(
        result.bundle.coverage[OBSERVER].status,
        ObservationCoverage::Partial
    );
    assert_eq!(result.verdict, Verdict::Inconclusive);
    dir.store().load("run-1").unwrap();
}
#[test]
fn real_spawn_failure_never_claims_running_or_observed() {
    let dir = TestDir::new();
    let (plan, mut spec) = inputs(&dir, "success");
    spec.executable = dir.0.join("missing");
    let result = acquire(&plan, &spec, &dir.store(), "run-1").unwrap();
    assert!(!result.observation.started);
    assert!(result.observation.runner_failure.is_some());
    assert_eq!(result.observation.exit_code, None);
    assert_eq!(result.verdict, Verdict::Error);
    assert_eq!(result.execution, ExecutionStatus::RunnerCrash);
    assert_eq!(
        result.lifecycle,
        [
            RunState::Created,
            RunState::RunnerFailed,
            RunState::Completed
        ]
    );
    assert_eq!(
        result.bundle.coverage[OBSERVER].status,
        ObservationCoverage::Failed
    );
    dir.store().load("run-1").unwrap();
}
#[test]
fn real_environment_cwd_and_literal_arguments() {
    let dir = TestDir::new();
    let (plan, mut spec) = inputs(&dir, "environment");
    spec.environment
        .insert("P1B_VALUE".into(), "explicit".into());
    spec.args.push("$(touch forbidden); *.txt".into());
    let result = acquire(&plan, &spec, &dir.store(), "run-1").unwrap();
    let cwd = fs::canonicalize(&dir.0).unwrap();
    assert_eq!(
        String::from_utf8(result.observation.stdout).unwrap(),
        format!(
            "explicit\n{}\ntrue\n$(touch forbidden); *.txt\n",
            cwd.display()
        )
    );
    assert!(!dir.0.join("forbidden").exists());
}
#[test]
fn real_dual_stream_capture_does_not_deadlock() {
    let dir = TestDir::new();
    let result = run(&dir, "both");
    assert_eq!(result.observation.stdout, vec![b'a'; 262144]);
    assert_eq!(result.observation.stderr, vec![b'b'; 262144]);
    assert_eq!(result.execution, ExecutionStatus::Completed);
}
#[test]
fn real_capture_overflow_is_failure_not_complete_evidence() {
    let dir = TestDir::new();
    let result = run(&dir, "flood");
    assert!(result
        .observation
        .runner_failure
        .as_ref()
        .unwrap()
        .contains("capture limit"));
    assert_eq!(result.verdict, Verdict::Error);
    assert_eq!(
        result.bundle.coverage[OBSERVER].status,
        ObservationCoverage::Failed
    );
}
#[test]
fn real_timeout_cleans_descendant_process_group() {
    let dir = TestDir::new();
    let (mut plan, mut spec) = inputs(&dir, "descendant");
    plan.time_budget_ms = 150;
    spec.timeout_ms = 150;
    let marker = dir.0.join("leaked");
    spec.args.push(marker.to_str().unwrap().into());
    let result = acquire(&plan, &spec, &dir.store(), "run-1").unwrap();
    assert!(result.observation.timed_out);
    assert!(result.observation.runner_failure.is_none());
    std::thread::sleep(std::time::Duration::from_millis(850));
    assert!(!marker.exists());
}
#[test]
fn real_signal_is_recorded_as_target_status() {
    let dir = TestDir::new();
    let result = run(&dir, "signal");
    assert_eq!(result.observation.signal, Some(15));
    assert_eq!(result.observation.exit_code, None);
    assert_eq!(result.verdict, Verdict::Inconclusive);
}
#[test]
fn duplicate_run_is_rejected_before_execution() {
    let dir = TestDir::new();
    run(&dir, "success");
    let before = fs::read(dir.0.join("runs/run-1/result.json")).unwrap();
    let (plan, spec) = inputs(&dir, "failure");
    assert!(acquire(&plan, &spec, &dir.store(), "run-1").is_err());
    assert_eq!(
        before,
        fs::read(dir.0.join("runs/run-1/result.json")).unwrap()
    );
}
#[test]
fn partial_run_and_pending_file_are_not_evidence() {
    let dir = TestDir::new();
    dir.store().reserve("unfinished").unwrap();
    fs::write(
        dir.0.join("runs/unfinished/result.pending"),
        b"{\"manifest\":",
    )
    .unwrap();
    assert!(dir.store().load("unfinished").is_err());
    assert!(dir.store().reserve("unfinished").is_err());
}
#[test]
fn corrupt_truncated_and_missing_evidence_are_rejected() {
    for corruption in ["payload", "truncate", "missing", "run-link", "whitespace"] {
        let dir = TestDir::new();
        run(&dir, "success");
        let path = dir.0.join("runs/run-1/evidence/process-observation.json");
        let bytes = fs::read(&path).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        match corruption {
            "payload" => {
                value["observation"]["value"]["stdout"] = serde_json::json!([99]);
                fs::write(&path, canonical_bytes(&value).unwrap()).unwrap();
            }
            "truncate" => fs::write(&path, &bytes[..bytes.len() / 2]).unwrap(),
            "missing" => fs::remove_file(&path).unwrap(),
            "run-link" => {
                value["run_id"] = "other-run".into();
                fs::write(&path, canonical_bytes(&value).unwrap()).unwrap();
            }
            _ => {
                let mut bytes = bytes;
                bytes.push(b'\n');
                fs::write(&path, bytes).unwrap();
            }
        }
        assert!(dir.store().load("run-1").is_err(), "{corruption}");
    }
}
#[test]
fn corrupt_result_is_rejected() {
    let dir = TestDir::new();
    run(&dir, "success");
    let path = dir.0.join("runs/run-1/result.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["manifest"]["result"]["verdict"] = "PASS".into();
    fs::write(path, canonical_bytes(&value).unwrap()).unwrap();
    assert!(dir.store().load("run-1").is_err());
}
#[test]
fn unsupported_plan_and_unsafe_paths_reject_before_run_creation() {
    let dir = TestDir::new();
    let (mut plan, spec) = inputs(&dir, "success");
    plan.required_observers.push("http".into());
    assert!(acquire(&plan, &spec, &dir.store(), "run-1").is_err());
    assert!(!dir.0.join("runs/run-1").exists());
    for id in ["", "..", "../escape", "/absolute", "a/b"] {
        assert!(dir.store().reserve(id).is_err());
    }
}
#[test]
fn duplicate_evidence_and_commit_are_never_overwritten() {
    let source = TestDir::new();
    run(&source, "success");
    let (result, evidence) = source.store().load("run-1").unwrap();
    let dir = TestDir::new();
    let reserved = dir.store().reserve("run-1").unwrap();
    reserved.write_evidence(&evidence[0]).unwrap();
    assert!(reserved.write_evidence(&evidence[0]).is_err());
    reserved
        .complete(&result, &[evidence[0].evidence_id.clone()])
        .unwrap();
    assert!(reserved
        .complete(&result, &[evidence[0].evidence_id.clone()])
        .is_err());
    assert!(reserved.write_evidence(&evidence[0]).is_err());
    dir.store().load("run-1").unwrap();
}

#[test]
fn incomplete_evidence_cannot_be_committed() {
    let dir = TestDir::new();
    let run = dir.store().reserve("run-1").unwrap();
    assert!(run.complete(&serde_json::json!({}), &[]).is_err());
    assert!(run
        .complete(&serde_json::json!({}), &["missing".into()])
        .is_err());
    fs::write(
        dir.0.join("runs/run-1/evidence/broken.json"),
        b"{\"run_id\":",
    )
    .unwrap();
    assert!(run
        .complete(&serde_json::json!({}), &["broken".into()])
        .is_err());
    assert!(!dir.0.join("runs/run-1/result.json").exists());
    assert!(dir.store().load("run-1").is_err());
}
#[test]
fn concurrent_run_reservation_has_exactly_one_winner() {
    let dir = TestDir::new();
    let store = dir.store();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.reserve("same-run").is_ok()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|t| t.join().unwrap().then_some(()))
            .count(),
        1
    );
    assert!(store.load("same-run").is_err());
}
#[test]
fn symlink_artifact_is_not_accepted_as_evidence() {
    let dir = TestDir::new();
    run(&dir, "success");
    let path = dir.0.join("runs/run-1/evidence/process-observation.json");
    let other = dir.0.join("elsewhere.json");
    fs::rename(&path, &other).unwrap();
    std::os::unix::fs::symlink(&other, &path).unwrap();
    assert!(dir.store().load("run-1").is_err());
}
