#![cfg(unix)]
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use verify_core::{
    acquisition::{
        acquire,
        replay::{execute, ReplayResult},
        AcquisitionResult,
    },
    ConformanceFixture, Verdict,
};
use verify_evidence::{canonical_bytes, canonical_hash, store::EvidenceStore};
use verify_replay::ReplayStatus;
use verify_runner::{
    process::{ProcessSpec, OBSERVER},
    IsolationLevel,
};

struct Case {
    dir: PathBuf,
    original: AcquisitionResult,
}
impl Case {
    fn new(mode: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "b2ige-p1c-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let executable = dir.join("target");
        fs::copy(env!("CARGO_BIN_EXE_p1c-replay-fixture"), &executable).unwrap();
        let fixture: ConformanceFixture =
            serde_json::from_str(include_str!("../../../tests/fixtures/pass.json")).unwrap();
        let mut plan = fixture.policy.plan;
        plan.required_observers = vec![OBSERVER.into()];
        plan.optional_observers.clear();
        plan.isolation = IsolationLevel::None;
        plan.reset_strategy = "none".into();
        plan.time_budget_ms = 3000;
        plan.target_revision = format!(
            "sha256:{:x}",
            Sha256::digest(fs::read(&executable).unwrap())
        );
        let spec = ProcessSpec {
            executable,
            args: vec![mode.into(), "literal argument $HOME ; no shell".into()],
            working_directory: dir.clone(),
            environment: BTreeMap::from([("REPLAY_VALUE".into(), "explicit value".into())]),
            timeout_ms: 3000,
        };
        let original = acquire(
            &plan,
            &spec,
            &EvidenceStore::new(dir.join("runs")),
            "original",
        )
        .unwrap();
        assert_eq!(original.observation.runner_failure, None);
        Self { dir, original }
    }
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.dir.join("runs"))
    }
    fn replay(&self) -> ReplayResult {
        execute(&self.store(), "original", "replay")
    }
    fn count(&self) -> usize {
        fs::read(self.dir.join("executions")).unwrap().len()
    }
    fn assert_unavailable(&self) {
        let result = self.replay();
        assert_eq!(
            result.status,
            ReplayStatus::Unavailable,
            "{}",
            result.reason
        );
        assert!(result.replay_run_id.is_none());
        assert!(result.acquisition.is_none());
        assert!(!self.dir.join("runs/replay").exists());
        assert_eq!(self.count(), 1);
    }
    // Rehash the generic Store envelope so tests exercise typed replay preflight,
    // not merely the already-tested Store corruption checks. This is NOT a
    // process provenance mechanism or an authorized production writer.
    fn edit_result(&self, change: impl FnOnce(&mut Value)) {
        let path = self.dir.join("runs/original/result.json");
        let mut commit: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut commit["manifest"]["result"]);
        commit["integrity_hash"] = json!(canonical_hash(&commit["manifest"]).unwrap());
        fs::write(path, canonical_bytes(&commit).unwrap()).unwrap();
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn same_inputs_execute_and_fresh_evidence_relationship_is_durable() {
    let case = Case::new("stable");
    let original_bytes = fs::read(case.dir.join("runs/original/result.json")).unwrap();
    let result = case.replay();
    assert_eq!(result.status, ReplayStatus::Reproduced, "{}", result.reason);
    assert_eq!(case.count(), 2);
    let replay = result.acquisition.as_ref().unwrap();
    assert_eq!(replay.process, case.original.process);
    assert_eq!(replay.plan, case.original.plan);
    assert_eq!(replay.context.seed, case.original.context.seed);
    assert_eq!(replay.context.plan_hash, case.original.context.plan_hash);
    assert_eq!(
        replay.context.config_hash,
        case.original.context.config_hash
    );
    assert_eq!(
        replay.context.observer_versions,
        case.original.context.observer_versions
    );
    assert_eq!(replay.observation.stdout, case.original.observation.stdout);
    assert_eq!(replay.verdict, Verdict::Inconclusive);
    assert_eq!(result.original_run_id, "original");
    assert_eq!(result.replay_run_id.as_deref(), Some("replay"));
    assert_ne!(replay.context.run_id, case.original.context.run_id);
    assert_eq!(
        result.original_result_hash,
        Some(canonical_hash(&case.original).unwrap())
    );
    let (_, old) = case.store().load("original").unwrap();
    let (saved, new) = case.store().load("replay").unwrap();
    assert_eq!(
        serde_json::from_value::<ReplayResult>(saved).unwrap(),
        result
    );
    assert_eq!(new[0].source, old[0].source);
    assert_eq!(new[0].run_id, "replay");
    assert_eq!(old[0].run_id, "original");
    assert_ne!(new[0].observation, old[0].observation);
    assert_ne!(new[0].integrity_hash, old[0].integrity_hash);
    assert_eq!(
        replay.bundle.evidence_hashes[&new[0].evidence_id],
        canonical_hash(&new[0]).unwrap()
    );
    assert_eq!(
        result.original_evidence_hashes[&old[0].evidence_id],
        canonical_hash(&old[0]).unwrap()
    );
    assert_eq!(
        fs::read(case.dir.join("runs/original/result.json")).unwrap(),
        original_bytes
    );
}

#[test]
fn changed_stdout_is_divergence_not_infrastructure_error_or_product_fail() {
    let case = Case::new("diverge");
    let result = case.replay();
    assert_eq!(result.status, ReplayStatus::ExecutedButDiverged);
    let replay = result.acquisition.unwrap();
    assert_ne!(replay.observation.stdout, case.original.observation.stdout);
    assert_eq!(replay.observation.runner_failure, None);
    assert_eq!(replay.verdict, Verdict::Inconclusive);
    assert_eq!(case.count(), 2);
}

#[test]
fn nonzero_target_result_can_reproduce_without_product_fail() {
    let case = Case::new("nonzero");
    let result = case.replay();
    assert_eq!(result.status, ReplayStatus::Reproduced);
    let replay = result.acquisition.unwrap();
    assert_eq!(replay.observation.exit_code, Some(7));
    assert_eq!(replay.verdict, Verdict::Inconclusive);
}

#[test]
fn mutable_missing_and_unbound_git_targets_are_rejected_before_spawn() {
    for target in [
        "main",
        "HEAD",
        "v1.0",
        "",
        "git:0123456789012345678901234567890123456789",
    ] {
        let case = Case::new("stable");
        case.edit_result(|r| {
            r["plan"]["target_revision"] = json!(target);
            r["context"]["target_revision"] = json!(target);
            let hash = canonical_hash(&r["plan"]).unwrap();
            r["context"]["plan_hash"] = json!(hash);
            r["context"]["experiment_hash"] = json!(hash);
        });
        case.assert_unavailable();
    }
}

#[test]
fn plan_config_seed_hash_and_observer_mismatches_are_rejected() {
    for (pointer, value) in [
        ("/plan/plan_id", json!("other")),
        ("/plan/seed", json!(123)),
        ("/context/seed", json!(123)),
        (
            "/context/plan_hash",
            json!(format!("sha256:{}", "0".repeat(64))),
        ),
        (
            "/context/experiment_hash",
            json!(format!("sha256:{}", "0".repeat(64))),
        ),
        (
            "/context/config_hash",
            json!(format!("sha256:{}", "0".repeat(64))),
        ),
        ("/process/args", json!(["diverge"])),
        ("/process/environment", json!({})),
        ("/process/working_directory", json!("/")),
        ("/process/timeout_ms", json!(1)),
        ("/context/observer_versions/cli_process", json!("2")),
        ("/bundle/run_id", json!("other")),
        ("/context/run_id", json!("other")),
        ("/observation/stdout", json!([1, 2, 3])),
        ("/verdict", json!("PASS")),
        ("/acquisition_schema_version", json!("2")),
    ] {
        let case = Case::new("stable");
        case.edit_result(|r| *r.pointer_mut(pointer).unwrap() = value);
        case.assert_unavailable();
    }
}

#[test]
fn missing_required_metadata_is_rejected_before_spawn() {
    for (object, key) in [
        ("", "plan"),
        ("context", "plan_hash"),
        ("context", "config_hash"),
        ("context", "seed"),
        ("context", "target_revision"),
        ("context", "observer_versions"),
        ("process", "executable"),
        ("process", "args"),
        ("process", "working_directory"),
        ("process", "environment"),
        ("process", "timeout_ms"),
    ] {
        let case = Case::new("stable");
        case.edit_result(|r| {
            let object = if object.is_empty() { r } else { &mut r[object] };
            object.as_object_mut().unwrap().remove(key);
        });
        case.assert_unavailable();
    }
}

#[test]
fn corrupt_original_commit_forbids_replay() {
    let case = Case::new("stable");
    fs::write(case.dir.join("runs/original/result.json"), b"{broken").unwrap();
    case.assert_unavailable();
}

#[test]
fn corrupt_or_missing_original_evidence_forbids_replay() {
    for missing in [false, true] {
        let case = Case::new("stable");
        let path = case
            .dir
            .join("runs/original/evidence/process-observation.json");
        if missing {
            fs::remove_file(path).unwrap();
        } else {
            fs::write(path, b"{}").unwrap();
        }
        case.assert_unavailable();
    }
}

#[test]
fn changed_executable_bytes_forbid_replay() {
    let case = Case::new("stable");
    fs::write(&case.original.process.executable, b"different executable").unwrap();
    case.assert_unavailable();
}

#[test]
fn missing_executable_forbids_replay() {
    let case = Case::new("stable");
    fs::remove_file(&case.original.process.executable).unwrap();
    case.assert_unavailable();
}

#[test]
fn spawn_failure_with_matching_bytes_is_durable_replay_error() {
    let case = Case::new("stable");
    fs::set_permissions(
        &case.original.process.executable,
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let result = case.replay();
    assert_eq!(result.status, ReplayStatus::Error);
    let acquisition = result.acquisition.as_ref().unwrap();
    assert!(!acquisition.observation.started);
    assert!(acquisition.observation.runner_failure.is_some());
    assert_eq!(acquisition.verdict, Verdict::Error);
    assert_eq!(case.count(), 1);
    let (saved, evidence) = case.store().load("replay").unwrap();
    assert_eq!(
        serde_json::from_value::<ReplayResult>(saved).unwrap(),
        result
    );
    assert_eq!(evidence[0].run_id, "replay");
}

#[test]
fn same_run_id_never_executes_or_overwrites() {
    let case = Case::new("stable");
    let result = execute(&case.store(), "original", "original");
    assert_eq!(result.status, ReplayStatus::Unavailable);
    assert_eq!(case.count(), 1);
    let (saved, _) = case.store().load("original").unwrap();
    assert_eq!(
        serde_json::from_value::<AcquisitionResult>(saved).unwrap(),
        case.original
    );
}

#[test]
fn occupied_replay_id_is_error_without_execution_or_overwrite() {
    let case = Case::new("stable");
    let first = case.replay();
    assert_eq!(first.status, ReplayStatus::Reproduced);
    let second = case.replay();
    assert_eq!(second.status, ReplayStatus::Error);
    assert!(second.acquisition.is_none());
    assert_eq!(case.count(), 2);
    let (saved, _) = case.store().load("replay").unwrap();
    assert_eq!(
        serde_json::from_value::<ReplayResult>(saved).unwrap(),
        first
    );
}

#[test]
fn unsupported_plan_with_consistent_hashes_is_unavailable() {
    for (key, value) in [
        ("required_observers", json!(["other"])),
        ("reset_strategy", json!("reset")),
    ] {
        let case = Case::new("stable");
        case.edit_result(|r| {
            r["plan"][key] = value;
            let hash = canonical_hash(&r["plan"]).unwrap();
            r["context"]["plan_hash"] = json!(hash);
            r["context"]["experiment_hash"] = json!(hash);
        });
        case.assert_unavailable();
    }
}

#[test]
fn bundle_evidence_hash_mismatch_is_rejected_even_when_store_commit_is_valid() {
    let case = Case::new("stable");
    case.edit_result(|r| {
        r["bundle"]["evidence_hashes"]["process-observation"] =
            json!(format!("sha256:{}", "0".repeat(64)))
    });
    assert!(case.store().load("original").is_ok());
    case.assert_unavailable();
}
