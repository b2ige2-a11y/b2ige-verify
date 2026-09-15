#![cfg(unix)]
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use verify_core::{
    acquisition::AcquisitionResult,
    behavior::{self, *},
    checker_binding_hash, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator, Verdict,
};
use verify_evidence::{canonical_bytes, canonical_hash, store::EvidenceStore, ObservationCoverage};
use verify_runner::process::OBSERVER;

struct Case {
    dir: PathBuf,
    experiment: BehaviorExperiment,
    auth: BehaviorAuthorization,
}
impl Case {
    fn new(mode: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "b2ige-p2-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let before = dir.join("before-target");
        let after = dir.join("after-target");
        fs::copy(env!("CARGO_BIN_EXE_p2-behavior-before"), &before).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_p2-behavior-after"), &after).unwrap();
        let case = BehaviorCase {
            schema_version: "1".into(),
            case_id: "explicit-case".into(),
            args: vec![mode.into()],
            environment: BTreeMap::from([(
                "EXECUTION_MARKER".into(),
                dir.join("executions").to_str().unwrap().into(),
            )]),
            // Allow concurrent cold executable launches in the workspace suite.
            // Dedicated timeout cases below keep their explicit short deadline.
            timeout_ms: 15000,
            fixture: LocalFixture {
                source: None,
                snapshot_identity: snapshot_identity(None).unwrap(),
            },
            comparison_policy: ComparisonPolicy::ProcessByteExactV1,
            required_observers: vec![OBSERVER.into()],
        };
        let baseline = Baseline {
            baseline_id: "approved-baseline".into(),
            target_revision: executable_identity(&before).unwrap(),
            created_by: BaselineCreator::Human,
            approval: BaselineApproval {
                status: ApprovalStatus::Approved,
                actor: Some("test-policy".into()),
                reason: Some("explicit fixture baseline approval".into()),
            },
            observation_contract_hash: case.observation_contract_hash().unwrap(),
            stability_runs: Some(1),
            notes: None,
        };
        let experiment = BehaviorExperiment {
            schema_version: "1".into(),
            before: BehaviorTarget {
                identity: executable_identity(&before).unwrap(),
                executable: before,
                input_identity: case.input_identity().unwrap(),
            },
            after: BehaviorTarget {
                identity: executable_identity(&after).unwrap(),
                executable: after,
                input_identity: case.input_identity().unwrap(),
            },
            case,
            baseline,
            seed: 42,
        };
        let mut test = Self {
            dir,
            experiment,
            auth: BehaviorAuthorization {
                approved_baselines: BTreeSet::new(),
                approved_checker_bindings: BTreeSet::new(),
                baseline_stable: true,
            },
        };
        test.approve();
        test
    }
    // Test-only trusted authoring. The engine never creates authorization pins.
    fn approve(&mut self) {
        self.auth.approved_baselines =
            BTreeSet::from([canonical_hash(&self.experiment.baseline).unwrap()]);
        self.auth.approved_checker_bindings = BTreeSet::from([checker_binding_hash(
            &self.experiment.baseline,
            &self.experiment.case.checker_claim().unwrap(),
        )
        .unwrap()]);
    }
    fn bind_inputs(&mut self) {
        let hash = self.experiment.case.input_identity().unwrap();
        self.experiment.before.input_identity = hash.clone();
        self.experiment.after.input_identity = hash;
    }
    fn fixture(&mut self) {
        let source = self.dir.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("input.txt"), b"original input\x00\xff").unwrap();
        fs::create_dir(source.join("empty-directory")).unwrap();
        fs::set_permissions(source.join("input.txt"), fs::Permissions::from_mode(0o640)).unwrap();
        self.experiment.case.fixture = LocalFixture {
            snapshot_identity: snapshot_identity(Some(&source)).unwrap(),
            source: Some(source),
        };
        self.bind_inputs();
    }
    fn store(&self) -> EvidenceStore {
        EvidenceStore::new(self.dir.join("runs"))
    }
    fn execute(&self) -> BehaviorComparisonResult {
        behavior::execute(
            &self.store(),
            &self.dir.join("work"),
            "comparison",
            &self.experiment,
            &self.auth,
        )
        .unwrap()
    }
    fn load(&self) -> std::io::Result<BehaviorComparisonResult> {
        behavior::load(&self.store(), "comparison", &self.auth)
    }
    fn count(&self) -> Vec<u8> {
        fs::read(self.dir.join("executions")).unwrap_or_default()
    }
    fn acquisition(&self, side: &str) -> AcquisitionResult {
        let (value, _) = self.store().load(&format!("comparison-{side}")).unwrap();
        serde_json::from_value(value).unwrap()
    }
    fn rejected_before_execution(&self) {
        let result = self.execute();
        assert_eq!(result.outcome, BehaviorOutcome::Error, "{}", result.reason);
        assert_eq!(result.verdict, Verdict::Error);
        assert!(result.before_run_id.is_none() && result.after_run_id.is_none());
        assert!(result.divergences.is_empty());
        assert!(self.count().is_empty());
        assert!(!self.dir.join("runs/comparison-before").exists());
        assert!(!self.dir.join("runs/comparison-after").exists());
        assert_eq!(self.load().unwrap(), result);
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_identical_raw_streams_and_exit_are_no_divergence_with_two_real_runs() {
    let case = Case::new("equal");
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::NoDivergenceFound,
        "{}",
        result.reason
    );
    assert_eq!(result.verdict, Verdict::Pass);
    assert!(result.divergences.is_empty());
    assert!(result.reason.contains("within tested behavior space"));
    assert_eq!(case.count(), b"ba");
    assert_ne!(result.baseline_target_identity, result.candidate_identity);
    assert_ne!(result.before_run_id, result.after_run_id);
    let before = case.acquisition("before");
    let after = case.acquisition("after");
    assert_eq!(before.observation.stdout, b"ready\n\x00\xff");
    assert_eq!(before.observation.stderr, b"warning\x00\xfe");
    assert_eq!(before.observation.stdout, after.observation.stdout);
    assert_eq!(before.observation.stderr, after.observation.stderr);
    assert_eq!(before.process.args, after.process.args);
    assert_eq!(before.process.environment, after.process.environment);
    assert_eq!(before.process.timeout_ms, after.process.timeout_ms);
    assert_eq!(before.verdict, Verdict::Inconclusive);
    assert_eq!(after.verdict, Verdict::Inconclusive);
    assert_eq!(case.load().unwrap(), result);
}

fn assert_difference(mode: &str, observable: Observable) {
    let case = Case::new(mode);
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::DivergenceProven,
        "{}",
        result.reason
    );
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.divergences.len(), 1);
    let difference = &result.divergences[0];
    assert_eq!(difference.observable, observable);
    assert_eq!(difference.case_identity, result.case_identity);
    assert_eq!(
        Some(&difference.before_run_id),
        result.before_run_id.as_ref()
    );
    assert_eq!(Some(&difference.after_run_id), result.after_run_id.as_ref());
    assert_eq!(
        difference.before_target_hash,
        result.baseline_target_identity
    );
    assert_eq!(difference.after_target_hash, result.candidate_identity);
    assert_ne!(difference.before, difference.after);
    match difference.observable {
        Observable::ExitCode => {
            assert_eq!(
                difference.before,
                ObservableValue::Status { value: Some(0) }
            );
            assert_eq!(difference.after, ObservableValue::Status { value: Some(1) });
        }
        Observable::Signal => {
            assert_eq!(
                difference.before,
                ObservableValue::Status { value: Some(15) }
            );
            assert_eq!(difference.after, ObservableValue::Status { value: Some(9) });
        }
        _ => {}
    }
    assert_eq!(difference.evidence_refs, result.evidence_refs);
    assert!(
        matches!(result.replayability, verify_replay::Replayability::Unavailable { ref reason } if !reason.is_empty())
    );
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn b_stdout_difference_is_proven_from_raw_bytes() {
    assert_difference("stdout", Observable::Stdout);
}

#[test]
fn c_stderr_difference_is_proven_from_raw_bytes() {
    assert_difference("stderr", Observable::Stderr);
}

#[test]
fn d_exit_zero_vs_one_is_proven() {
    assert_difference("exit", Observable::ExitCode);
}

#[test]
fn e_identical_nonzero_exit_is_no_divergence() {
    let case = Case::new("nonzero");
    let result = case.execute();
    assert_eq!(result.outcome, BehaviorOutcome::NoDivergenceFound);
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(case.acquisition("before").observation.exit_code, Some(7));
    assert_eq!(case.acquisition("after").observation.exit_code, Some(7));
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn f_different_terminating_signals_are_proven() {
    assert_difference("signal", Observable::Signal);
}

#[test]
fn g_equal_or_one_sided_timeout_is_inconclusive_never_pass_or_fail() {
    for mode in ["timeout", "one-timeout"] {
        let mut case = Case::new(mode);
        case.experiment.case.timeout_ms = 150;
        case.bind_inputs();
        let result = case.execute();
        assert_eq!(
            result.outcome,
            BehaviorOutcome::Inconclusive,
            "{}",
            result.reason
        );
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert!(result.divergences.is_empty());
        assert!(case.acquisition("before").observation.started);
        assert!(case.acquisition("after").observation.started);
        assert_eq!(result.coverage["before"], ObservationCoverage::Partial);
        assert_eq!(case.load().unwrap(), result);
    }
}

#[test]
fn g_capture_overflow_is_inconclusive_and_p1_error_semantics_are_preserved() {
    let case = Case::new("flood");
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::Inconclusive,
        "{}",
        result.reason
    );
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert!(result.divergences.is_empty());
    let before = case.acquisition("before");
    assert_eq!(before.verdict, Verdict::Error);
    assert_eq!(
        before.bundle.coverage[OBSERVER].status,
        ObservationCoverage::Failed
    );
    assert_eq!(
        before.observation.runner_failure.as_deref(),
        Some("capture limit exceeded; observation incomplete")
    );
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn h_spawn_failure_is_error_even_with_matching_executable_hash() {
    for side in ["before", "after"] {
        let case = Case::new("equal");
        let target = if side == "before" {
            &case.experiment.before
        } else {
            &case.experiment.after
        };
        fs::set_permissions(&target.executable, fs::Permissions::from_mode(0o600)).unwrap();
        let result = case.execute();
        assert_eq!(result.outcome, BehaviorOutcome::Error, "{}", result.reason);
        assert_eq!(result.verdict, Verdict::Error);
        assert!(result.divergences.is_empty());
        assert!(!case.acquisition(side).observation.started);
        assert_eq!(case.count().len(), 1);
        assert_eq!(case.load().unwrap(), result);
    }
}

#[test]
fn h_after_acquisition_reservation_failure_never_passes_with_only_before() {
    let case = Case::new("equal");
    case.store().reserve("comparison-after").unwrap();
    let result = case.execute();
    assert_eq!(result.outcome, BehaviorOutcome::Error);
    assert!(result.before_run_id.is_some() && result.after_run_id.is_none());
    assert_eq!(case.count(), b"b");
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn i_mismatched_input_identities_or_modified_explicit_inputs_refuse_execution() {
    for mutation in [
        "before-id",
        "after-id",
        "args",
        "env",
        "timeout",
        "snapshot",
    ] {
        let mut case = Case::new("equal");
        match mutation {
            "before-id" => {
                case.experiment.before.input_identity = canonical_hash(&"other").unwrap()
            }
            "after-id" => case.experiment.after.input_identity = canonical_hash(&"other").unwrap(),
            "args" => case.experiment.case.args.push("different".into()),
            "env" => {
                case.experiment
                    .case
                    .environment
                    .insert("CHANGED".into(), "1".into());
            }
            "timeout" => case.experiment.case.timeout_ms += 1,
            _ => case.experiment.case.fixture.snapshot_identity = canonical_hash(&"other").unwrap(),
        }
        case.rejected_before_execution();
    }
}

#[test]
fn j_baseline_approval_provenance_hash_and_checker_binding_mismatch_refuse_execution() {
    for mutation in [
        "unapproved",
        "actor",
        "reason",
        "pins",
        "baseline-hash",
        "target",
        "contract",
        "binding",
    ] {
        let mut case = Case::new("equal");
        match mutation {
            "unapproved" => case.experiment.baseline.approval.status = ApprovalStatus::Unapproved,
            "actor" => case.experiment.baseline.approval.actor = None,
            "reason" => case.experiment.baseline.approval.reason = Some(" ".into()),
            "pins" => case.auth.approved_baselines.clear(),
            "baseline-hash" => case.experiment.baseline.baseline_id = "changed".into(),
            "target" => {
                case.experiment.baseline.target_revision = case.experiment.after.identity.clone();
                case.approve();
            }
            "contract" => {
                case.experiment.baseline.observation_contract_hash =
                    canonical_hash(&"other").unwrap();
                case.approve();
            }
            _ => case.auth.approved_checker_bindings.clear(),
        }
        case.rejected_before_execution();
    }
}

#[test]
fn k_before_cwd_changes_cannot_contaminate_after_workspace() {
    let mut case = Case::new("workspace");
    case.fixture();
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::NoDivergenceFound,
        "{}",
        result.reason
    );
    let before = result.before.as_ref().unwrap();
    let after = result.after.as_ref().unwrap();
    assert_ne!(before.working_directory, after.working_directory);
    assert_eq!(
        before.initial_snapshot_identity,
        after.initial_snapshot_identity
    );
    assert_eq!(before.input_identity, after.input_identity);
    assert_eq!(
        case.acquisition("before").observation.stdout,
        b"original input\x00\xff"
    );
    assert_eq!(
        case.acquisition("after").observation.stdout,
        b"original input\x00\xff"
    );
    for link in [before, after] {
        assert!(link.working_directory.join("created/marker").is_file());
        assert!(link.working_directory.join("empty-directory").is_dir());
        assert_eq!(
            fs::metadata(link.working_directory.join("input.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
    assert_eq!(
        fs::read(case.dir.join("source/input.txt")).unwrap(),
        b"original input\x00\xff"
    );
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn k_after_uses_original_snapshot_even_if_before_mutates_source_fixture() {
    let mut case = Case::new("source-change");
    case.fixture();
    case.experiment.case.environment.insert(
        "SOURCE_FILE".into(),
        case.dir.join("source/input.txt").to_str().unwrap().into(),
    );
    case.bind_inputs();
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::NoDivergenceFound,
        "{}",
        result.reason
    );
    assert_eq!(
        fs::read(case.dir.join("source/input.txt")).unwrap(),
        b"mutated source"
    );
    assert_eq!(
        case.acquisition("after").observation.stdout,
        b"original input\x00\xff"
    );
}

#[test]
fn l_comparison_refs_hashes_and_schema_link_to_actual_canonical_artifacts() {
    let case = Case::new("stdout");
    let result = case.execute();
    for link in [
        result.before.as_ref().unwrap(),
        result.after.as_ref().unwrap(),
    ] {
        let (value, evidence) = case.store().load(&link.run_id).unwrap();
        assert_eq!(link.result_hash, canonical_hash(&value).unwrap());
        assert_eq!(link.evidence_refs.len(), 1);
        let reference = &link.evidence_refs[0];
        assert_eq!(reference.run_id, evidence[0].run_id);
        assert_eq!(reference.evidence_id, evidence[0].evidence_id);
        assert_eq!(
            reference.evidence_hash,
            canonical_hash(&evidence[0]).unwrap()
        );
    }
    assert_eq!(
        result.experiment_hash,
        canonical_hash(&case.experiment).unwrap()
    );
    assert_eq!(
        result.config_hash,
        canonical_hash(&case.experiment.case).unwrap()
    );
    let raw = fs::read(case.dir.join("runs/comparison/result.json")).unwrap();
    let value: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(raw, canonical_bytes(&value).unwrap());
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/behavior-comparison-result.schema.json"
    ))
    .unwrap();
    assert_eq!(schema, comparison_schema());
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::to_value(&result).unwrap()));
    let (saved, derived) = case.store().load("comparison").unwrap();
    assert_eq!(derived[0].trust_class, verify_evidence::TrustClass::Derived);
    assert_eq!(
        serde_json::from_value::<BehaviorComparisonResult>(saved).unwrap(),
        result
    );
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn mutable_missing_git_or_changed_executable_identities_never_execute() {
    for mutation in [
        "main",
        "",
        "git:0123456789012345678901234567890123456789",
        "changed",
        "missing",
    ] {
        let mut case = Case::new("equal");
        match mutation {
            "changed" => fs::write(&case.experiment.after.executable, b"changed").unwrap(),
            "missing" => fs::remove_file(&case.experiment.after.executable).unwrap(),
            _ => case.experiment.after.identity = mutation.into(),
        }
        case.rejected_before_execution();
    }
}

#[test]
fn changed_fixture_identity_or_symlink_is_rejected_before_execution() {
    for mutation in ["bytes", "permissions", "symlink"] {
        let mut case = Case::new("workspace");
        case.fixture();
        let path = case.dir.join("source/input.txt");
        match mutation {
            "bytes" => fs::write(path, b"changed").unwrap(),
            "permissions" => fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap(),
            _ => symlink("input.txt", case.dir.join("source/link")).unwrap(),
        }
        case.rejected_before_execution();
    }
}

#[test]
fn unsupported_required_observers_and_unasserted_stability_never_pass() {
    for observer in [
        "http",
        "filesystem",
        "sqlite",
        "postgresql",
        "network",
        "unstable",
    ] {
        let mut case = Case::new("equal");
        if observer == "unstable" {
            case.auth.baseline_stable = false;
        } else {
            case.experiment
                .case
                .required_observers
                .push(observer.into());
            case.experiment.baseline.observation_contract_hash =
                case.experiment.case.observation_contract_hash().unwrap();
            case.bind_inputs();
            case.approve();
        }
        let result = case.execute();
        assert_eq!(
            result.outcome,
            BehaviorOutcome::Inconclusive,
            "{}",
            result.reason
        );
        assert_eq!(result.verdict, Verdict::Inconclusive);
        assert!(case.count().is_empty());
        assert!(result.before_run_id.is_none() && result.after_run_id.is_none());
        assert_eq!(case.load().unwrap(), result);
    }
}

#[test]
fn literal_args_explicit_environment_and_null_stdin_are_identical() {
    let mut case = Case::new("inputs");
    case.experiment
        .case
        .args
        .push("$HOME ; $(touch forbidden) * literal".into());
    case.experiment
        .case
        .environment
        .insert("EXPLICIT_VALUE".into(), "same explicit value".into());
    case.bind_inputs();
    let result = case.execute();
    assert_eq!(
        result.outcome,
        BehaviorOutcome::NoDivergenceFound,
        "{}",
        result.reason
    );
    assert_eq!(
        case.acquisition("before").observation.stdout,
        b"$HOME ; $(touch forbidden) * literal\nsame explicit value\nPATH absent: true\n"
    );
}

#[test]
fn duplicate_comparison_and_existing_workspaces_never_overwrite_or_reexecute() {
    let case = Case::new("equal");
    let first = case.execute();
    let bytes = fs::read(case.dir.join("runs/comparison/result.json")).unwrap();
    assert!(behavior::execute(
        &case.store(),
        &case.dir.join("work"),
        "comparison",
        &case.experiment,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), b"ba");
    assert_eq!(
        bytes,
        fs::read(case.dir.join("runs/comparison/result.json")).unwrap()
    );
    assert_eq!(case.load().unwrap(), first);

    let case = Case::new("equal");
    fs::create_dir_all(case.dir.join("work/comparison/before")).unwrap();
    fs::write(case.dir.join("work/comparison/before/keep"), b"existing").unwrap();
    case.rejected_before_execution();
    assert_eq!(
        fs::read(case.dir.join("work/comparison/before/keep")).unwrap(),
        b"existing"
    );
}

#[test]
fn corrupt_before_evidence_during_after_execution_causes_error_before_comparison() {
    let mut case = Case::new("corrupt-before");
    case.experiment.case.environment.insert(
        "BEFORE_EVIDENCE".into(),
        case.dir
            .join("runs/comparison-before/evidence/process-observation.json")
            .to_str()
            .unwrap()
            .into(),
    );
    case.bind_inputs();
    let result = case.execute();
    assert_eq!(case.count(), b"ba");
    assert_eq!(result.outcome, BehaviorOutcome::Error);
    assert!(result.divergences.is_empty());
    assert!(case.load().is_err());
}

#[test]
fn after_target_changed_by_before_is_rechecked_and_cannot_pass() {
    let mut case = Case::new("change-after-target");
    case.experiment.case.environment.insert(
        "AFTER_TARGET".into(),
        case.experiment.after.executable.to_str().unwrap().into(),
    );
    case.bind_inputs();
    let result = case.execute();
    assert_eq!(result.outcome, BehaviorOutcome::Error);
    assert_eq!(case.count(), b"b");
    assert!(result.before_run_id.is_some() && result.after_run_id.is_none());
    assert_eq!(case.load().unwrap(), result);
}

#[test]
fn missing_corrupt_or_partial_run_artifacts_cannot_reload_a_positive_comparison() {
    for side in ["before", "after"] {
        for mutation in [
            "result-missing",
            "result-corrupt",
            "evidence-missing",
            "evidence-corrupt",
            "pending",
        ] {
            let case = Case::new("equal");
            assert_eq!(case.execute().verdict, Verdict::Pass);
            let root = case.dir.join(format!("runs/comparison-{side}"));
            match mutation {
                "result-missing" => fs::remove_file(root.join("result.json")).unwrap(),
                "result-corrupt" => fs::write(root.join("result.json"), b"{}").unwrap(),
                "evidence-missing" => {
                    fs::remove_file(root.join("evidence/process-observation.json")).unwrap()
                }
                "evidence-corrupt" => {
                    fs::write(root.join("evidence/process-observation.json"), b"{}").unwrap()
                }
                _ => fs::rename(root.join("result.json"), root.join("result.pending")).unwrap(),
            }
            assert!(case.load().is_err(), "{side}/{mutation}");
        }
    }
}

#[test]
fn typed_acquisition_linkage_rejects_rehashed_missing_observation_and_input_mutations() {
    for (pointer, value) in [
        ("/observation/exit_code", Value::Null),
        ("/observation/started", json!(false)),
        ("/observation/timed_out", json!(true)),
        ("/process/args", json!(["changed"])),
        ("/process/environment", json!({})),
        ("/process/timeout_ms", json!(123)),
        ("/bundle/coverage/cli_process/status", json!("partial")),
        ("/bundle/evidence_refs", json!([])),
        ("/context/run_id", json!("comparison-after")),
    ] {
        let case = Case::new("equal");
        case.execute();
        let path = case.dir.join("runs/comparison-before/result.json");
        let mut commit: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        *commit["manifest"]["result"].pointer_mut(pointer).unwrap() = value;
        commit["integrity_hash"] = json!(canonical_hash(&commit["manifest"]).unwrap());
        fs::write(path, canonical_bytes(&commit).unwrap()).unwrap();
        assert!(case.store().load("comparison-before").is_ok());
        assert!(case.load().is_err(), "{pointer}");
    }
}

#[test]
fn unknown_policy_versions_duplicate_environment_and_missing_fields_are_rejected() {
    let case = Case::new("equal");
    let raw = serde_json::to_value(&case.experiment).unwrap();
    for field in [
        "args",
        "environment",
        "timeout_ms",
        "fixture",
        "comparison_policy",
    ] {
        let mut value = raw.clone();
        value["case"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<BehaviorExperiment>(value).is_err());
    }
    let mut value = raw.clone();
    value["case"]["comparison_policy"] = json!("normalized");
    assert!(serde_json::from_value::<BehaviorExperiment>(value).is_err());
    let raw = serde_json::to_string(&case.experiment.case).unwrap();
    let duplicate = raw.replace(
        "\"environment\":{",
        "\"environment\":{\"DUP\":\"1\",\"DUP\":\"2\",",
    );
    assert!(serde_json::from_str::<BehaviorCase>(&duplicate).is_err());
}

// Test-only coherent rewriting defeats generic Store hashes to exercise the
// Behavior validator. This does not make Store hashes an authentication system.
fn rewrite_artifact(
    case: &Case,
    run_id: &str,
    result: &impl serde::Serialize,
    evidence: &verify_evidence::Evidence,
) {
    let root = case.dir.join("runs").join(run_id);
    fs::write(
        root.join("evidence")
            .join(format!("{}.json", evidence.evidence_id)),
        canonical_bytes(evidence).unwrap(),
    )
    .unwrap();
    let path = root.join("result.json");
    let mut commit: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    commit["manifest"]["result"] = serde_json::to_value(result).unwrap();
    commit["manifest"]["evidence_hashes"] =
        json!({&evidence.evidence_id: canonical_hash(evidence).unwrap()});
    commit["integrity_hash"] = json!(canonical_hash(&commit["manifest"]).unwrap());
    fs::write(path, canonical_bytes(&commit).unwrap()).unwrap();
}

fn rewrite_report(case: &Case, report: &BehaviorComparisonResult) {
    let (_, mut evidence) = case.store().load("comparison").unwrap();
    evidence[0].observation = verify_evidence::Observation::Value {
        value: serde_json::to_value(report).unwrap(),
    };
    evidence[0].integrity_hash = canonical_hash(&evidence[0].observation).unwrap();
    rewrite_artifact(case, "comparison", report, &evidence[0]);
    assert!(case.store().load("comparison").is_ok());
}

#[test]
fn rehashed_positive_outcome_cannot_hide_timeout_spawn_failure_or_one_missing_run() {
    for mode in ["timeout", "spawn", "before-only", "after-only"] {
        let mut case = Case::new(if mode == "timeout" {
            "timeout"
        } else {
            "equal"
        });
        if mode == "timeout" {
            case.experiment.case.timeout_ms = 150;
            case.bind_inputs();
        } else if mode == "spawn" {
            fs::set_permissions(
                &case.experiment.after.executable,
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let mut report = case.execute();
        if mode == "before-only" {
            report.after = None;
            report.after_run_id = None;
            report
                .evidence_refs
                .retain(|r| r.run_id == "comparison-before");
            report.coverage.remove("after");
        } else if mode == "after-only" {
            report.before = None;
            report.before_run_id = None;
            report
                .evidence_refs
                .retain(|r| r.run_id == "comparison-after");
            report.coverage.remove("before");
        }
        report.outcome = BehaviorOutcome::NoDivergenceFound;
        report.verdict = Verdict::Pass;
        rewrite_report(&case, &report);
        assert!(case.load().is_err(), "{mode}");
    }
}

#[test]
fn missing_required_terminal_observation_cannot_pass_even_with_consistent_all_hashes() {
    let case = Case::new("equal");
    let mut report = case.execute();
    let mut before = case.acquisition("before");
    before.observation.exit_code = None;
    before.observation.signal = None;
    let (_, mut evidence) = case.store().load("comparison-before").unwrap();
    evidence[0].observation = verify_evidence::Observation::Value {
        value: serde_json::to_value(&before.observation).unwrap(),
    };
    evidence[0].integrity_hash = canonical_hash(&evidence[0].observation).unwrap();
    let evidence_hash = canonical_hash(&evidence[0]).unwrap();
    before
        .bundle
        .evidence_hashes
        .insert(evidence[0].evidence_id.clone(), evidence_hash.clone());
    rewrite_artifact(&case, "comparison-before", &before, &evidence[0]);
    assert!(case.store().load("comparison-before").is_ok());
    let link = report.before.as_mut().unwrap();
    link.result_hash = canonical_hash(&before).unwrap();
    link.evidence_refs[0].evidence_hash = evidence_hash.clone();
    report.evidence_refs[0].evidence_hash = evidence_hash;
    rewrite_report(&case, &report);
    assert!(case.load().is_err());
}

#[test]
fn fabricated_divergence_without_actual_difference_is_rejected_after_rehashing() {
    let case = Case::new("equal");
    let mut report = case.execute();
    report.outcome = BehaviorOutcome::DivergenceProven;
    report.verdict = Verdict::Fail;
    report.divergences = vec![Divergence {
        case_identity: report.case_identity.clone(),
        before_run_id: report.before_run_id.clone().unwrap(),
        after_run_id: report.after_run_id.clone().unwrap(),
        before_target_hash: report.baseline_target_identity.clone(),
        after_target_hash: report.candidate_identity.clone(),
        observable: Observable::ExitCode,
        before: ObservableValue::Status { value: Some(0) },
        after: ObservableValue::Status { value: Some(1) },
        evidence_refs: report.evidence_refs.clone(),
    }];
    rewrite_report(&case, &report);
    assert!(case.load().is_err());
}

// P3A additions: keep every existing P2 test and fixture unchanged.
use behavior::stability::{
    BaselineStabilityProfile, ProfilingConfig, Stability, StabilityProfileRef,
};

const COUNTER: &str = r#"
n=0
if [ -f "$COUNTER_FILE" ]; then read -r n < "$COUNTER_FILE"; fi
n=$((n + 1))
printf '%s\n' "$n" > "$COUNTER_FILE"
"#;

fn p3_scripts(before: &str, after: &str) -> Case {
    let mut case = Case::new("explicit");
    for (target, marker, body) in [
        (&case.experiment.before, "b", before),
        (&case.experiment.after, "a", after),
    ] {
        fs::write(
            &target.executable,
            format!("#!/bin/sh\nset -eu\nprintf {marker} >> \"$EXECUTION_MARKER\"\n{body}\n"),
        )
        .unwrap();
    }
    case.experiment.before.identity =
        executable_identity(&case.experiment.before.executable).unwrap();
    case.experiment.after.identity =
        executable_identity(&case.experiment.after.executable).unwrap();
    case.experiment.baseline.target_revision = case.experiment.before.identity.clone();
    case.experiment.case.environment.insert(
        "COUNTER_FILE".into(),
        case.dir.join("counter").to_str().unwrap().into(),
    );
    case.bind_inputs();
    case.approve();
    case
}

fn p3_profile(case: &Case) -> BaselineStabilityProfile {
    let profile = stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "profile",
        &case.experiment,
        &case.auth,
        &ProfilingConfig::default(),
    )
    .unwrap();
    assert_eq!(
        stability::load_profile(&case.store(), "profile", &case.auth).unwrap(),
        profile
    );
    profile
}

fn p3_compare(
    case: &Case,
    reference: &StabilityProfileRef,
) -> std::io::Result<stability::StabilityComparisonResult> {
    stability::compare(
        &case.store(),
        &case.dir.join("work"),
        "measured-comparison",
        &case.experiment,
        &case.auth,
        reference,
    )
}

fn p3_result(
    case: &Case,
    profile: &BaselineStabilityProfile,
    outcome: BehaviorOutcome,
) -> stability::StabilityComparisonResult {
    let report = p3_compare(case, &profile.reference().unwrap()).unwrap();
    assert_eq!(report.outcome, outcome, "{}", report.reason);
    assert_eq!(report.verdict, outcome.verdict());
    assert_eq!(
        stability::load_comparison(&case.store(), "measured-comparison", &case.auth).unwrap(),
        report
    );
    report
}

fn p3_observable(
    profile: &BaselineStabilityProfile,
    observable: Observable,
) -> &stability::ObservableStability {
    profile
        .observables
        .iter()
        .find(|item| item.observable == observable)
        .unwrap()
}

fn p3_rewrite(case: &Case, id: &str, value: &impl serde::Serialize) {
    let (_, mut evidence) = case.store().load(id).unwrap();
    evidence[0].observation = verify_evidence::Observation::Value {
        value: serde_json::to_value(value).unwrap(),
    };
    evidence[0].integrity_hash = canonical_hash(&evidence[0].observation).unwrap();
    rewrite_artifact(case, id, value, &evidence[0]);
    assert!(case.store().load(id).is_ok());
}

#[test]
fn p3a_a_stable_raw_observables_have_five_distinct_real_acquisitions() {
    let mut case = Case::new("equal");
    // The profiler measures stability without asserting it in the P1A policy.
    case.auth.baseline_stable = false;
    let profile = p3_profile(&case);
    assert_eq!(profile.schema_version, "1");
    assert_eq!(profile.repetition_count, 5);
    assert_eq!(profile.run_ids.iter().collect::<BTreeSet<_>>().len(), 5);
    assert_eq!(
        profile
            .runs
            .iter()
            .map(|run| &run.working_directory)
            .collect::<BTreeSet<_>>()
            .len(),
        5
    );
    assert_eq!(case.count(), b"bbbbb");
    assert_eq!(profile.observables.len(), 4);
    for item in &profile.observables {
        assert_eq!(item.stability, Stability::Stable);
        assert_eq!(item.observed_values.len(), 1);
    }
    assert_eq!(
        p3_observable(&profile, Observable::Signal).observed_values,
        vec![ObservableValue::Status { value: None }]
    );
    for link in &profile.runs {
        let (value, evidence) = case.store().load(&link.run_id).unwrap();
        let run: AcquisitionResult = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(link.result_hash, canonical_hash(&value).unwrap());
        assert_eq!(link.plan_hash, run.context.plan_hash);
        assert_eq!(link.config_hash, run.context.config_hash);
        assert_eq!(
            link.input_identity,
            case.experiment.case.input_identity().unwrap()
        );
        assert_eq!(run.process.executable, case.experiment.before.executable);
        assert_eq!(run.process.args, case.experiment.case.args);
        assert_eq!(run.process.environment, case.experiment.case.environment);
        assert_eq!(run.process.timeout_ms, case.experiment.case.timeout_ms);
        assert_eq!(run.observation.stdout, b"ready\n\x00\xff");
        assert_eq!(run.observation.stderr, b"warning\x00\xfe");
        assert_ne!(run.verdict, Verdict::Pass);
        assert_eq!(
            evidence[0].trust_class,
            verify_evidence::TrustClass::DirectRuntime
        );
        assert_eq!(
            link.evidence_refs[0].evidence_hash,
            canonical_hash(&evidence[0]).unwrap()
        );
    }
    p3_result(&case, &profile, BehaviorOutcome::NoDivergenceFound);
    assert_eq!(case.count(), b"bbbbba");
}

#[test]
fn p3a_b_counter_output_is_unstable_without_normalization() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf 'candidate\\n'",
    );
    let profile = p3_profile(&case);
    let stdout = p3_observable(&profile, Observable::Stdout);
    assert_eq!(stdout.stability, Stability::Unstable);
    assert_eq!(stdout.observed_values.len(), 5);
    for observable in [Observable::ExitCode, Observable::Signal, Observable::Stderr] {
        assert_eq!(
            p3_observable(&profile, observable).stability,
            Stability::Stable
        );
    }
    assert_eq!(fs::read(case.dir.join("counter")).unwrap(), b"5\n");
}

#[test]
fn p3a_c_stable_value_changes_preserve_each_exact_divergence() {
    for (mode, observable) in [
        ("stdout", Observable::Stdout),
        ("stderr", Observable::Stderr),
        ("exit", Observable::ExitCode),
        ("signal", Observable::Signal),
    ] {
        let case = Case::new(mode);
        let profile = p3_profile(&case);
        assert!(profile
            .observables
            .iter()
            .all(|item| item.stability == Stability::Stable));
        let result = p3_result(&case, &profile, BehaviorOutcome::DivergenceProven);
        assert_eq!(result.divergences.len(), 1);
        let difference = &result.divergences[0];
        assert_eq!(difference.observable, observable);
        assert_eq!(difference.before_run_id, profile.run_ids[0]);
        assert_eq!(difference.after_run_id, result.candidate.run_id);
        assert_eq!(difference.evidence_refs.len(), 2);
        assert!(
            matches!(result.replayability, verify_replay::Replayability::Unavailable { ref reason } if !reason.is_empty())
        );
    }
}

#[test]
fn p3a_d_unstable_only_difference_is_inconclusive_never_pass() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf 'candidate\\n'",
    );
    let profile = p3_profile(&case);
    let report = p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
    assert!(report.divergences.is_empty());
    assert_eq!(
        p3_observable(&profile, Observable::Stdout).stability,
        Stability::Unstable
    );
}

#[test]
fn p3a_d_matching_one_unstable_value_still_cannot_pass() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf '1\\n'",
    );
    let profile = p3_profile(&case);
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_e_stable_exit_difference_wins_over_unstable_stdout() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf 'candidate\\n'\nexit 7",
    );
    let profile = p3_profile(&case);
    let report = p3_result(&case, &profile, BehaviorOutcome::DivergenceProven);
    assert_eq!(report.divergences.len(), 1);
    assert_eq!(report.divergences[0].observable, Observable::ExitCode);
    assert_eq!(
        report.divergences[0].after,
        ObservableValue::Status { value: Some(7) }
    );
}

#[test]
fn p3a_f_one_timeout_makes_profile_and_comparison_incomplete() {
    let mut case = p3_scripts(
        &format!("{COUNTER}\nif [ \"$n\" -eq 3 ]; then /bin/sleep 30; fi\nprintf 'ready\\n'"),
        "printf 'different\\n'\nexit 7",
    );
    // Keep the same cold-launch allowance as the other real fixtures even when
    // all workspace tests launch concurrently; only repetition 3 exceeds it.
    case.experiment.case.timeout_ms = 15000;
    case.bind_inputs();
    let profile = p3_profile(&case);
    assert!(profile
        .observables
        .iter()
        .all(|item| item.stability == Stability::Incomplete));
    let runs: Vec<AcquisitionResult> = profile
        .run_ids
        .iter()
        .map(|id| serde_json::from_value(case.store().load(id).unwrap().0).unwrap())
        .collect();
    assert_eq!(
        runs.iter().filter(|run| run.observation.timed_out).count(),
        1
    );
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_f_capture_overflow_never_produces_stable_profile() {
    let mut case = p3_scripts(
        &format!(
            "{COUNTER}\nif [ \"$n\" -eq 3 ]; then exec \"$FLOOD_TARGET\" flood; fi\nprintf ready"
        ),
        "printf ready",
    );
    case.experiment.case.environment.insert(
        "FLOOD_TARGET".into(),
        env!("CARGO_BIN_EXE_p2-behavior-before").into(),
    );
    case.bind_inputs();
    let profile = p3_profile(&case);
    assert!(profile
        .observables
        .iter()
        .all(|item| item.stability == Stability::Incomplete));
    let run: AcquisitionResult =
        serde_json::from_value(case.store().load(&profile.run_ids[2]).unwrap().0).unwrap();
    assert_eq!(run.verdict, Verdict::Error);
    assert_eq!(
        run.observation.runner_failure.as_deref(),
        Some("capture limit exceeded; observation incomplete")
    );
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_g_other_case_input_target_baseline_seed_or_profile_hash_is_rejected() {
    for mutation in [
        "case",
        "args",
        "environment",
        "timeout",
        "fixture",
        "target",
        "baseline",
        "seed",
        "hash",
    ] {
        let mut case = Case::new("equal");
        let profile = p3_profile(&case);
        let mut reference = profile.reference().unwrap();
        match mutation {
            "case" => case.experiment.case.case_id = "different-case".into(),
            "args" => case.experiment.case.args.push("different".into()),
            "environment" => {
                case.experiment
                    .case
                    .environment
                    .insert("OTHER".into(), "1".into());
            }
            "timeout" => case.experiment.case.timeout_ms += 1,
            "fixture" => case.fixture(),
            "target" => {
                case.experiment.before = case.experiment.after.clone();
                case.experiment.baseline.target_revision = case.experiment.before.identity.clone();
                case.approve();
            }
            "baseline" => {
                case.experiment.baseline.baseline_id = "different".into();
                case.approve();
            }
            "seed" => case.experiment.seed += 1,
            _ => reference.profile_hash = canonical_hash(&"wrong").unwrap(),
        }
        case.bind_inputs();
        assert!(p3_compare(&case, &reference).is_err(), "{mutation}");
        assert_eq!(case.count(), b"bbbbb", "{mutation}");
        assert!(!case.dir.join("runs/measured-comparison-candidate").exists());
    }
}

#[test]
fn p3a_h_missing_corrupt_pending_runs_or_profile_cannot_compare_or_reload_pass() {
    for target in [
        "profile",
        "profile-baseline-0",
        "profile-baseline-4",
        "measured-comparison-candidate",
    ] {
        for mutation in [
            "result-missing",
            "result-corrupt",
            "evidence-missing",
            "evidence-corrupt",
            "pending",
        ] {
            let case = Case::new("equal");
            let profile = p3_profile(&case);
            p3_result(&case, &profile, BehaviorOutcome::NoDivergenceFound);
            let root = case.dir.join("runs").join(target);
            let evidence = if target == "profile" {
                "baseline-stability"
            } else {
                "process-observation"
            };
            let evidence_path = root.join("evidence").join(format!("{evidence}.json"));
            match mutation {
                "result-missing" => fs::remove_file(root.join("result.json")).unwrap(),
                "result-corrupt" => fs::write(root.join("result.json"), b"{}").unwrap(),
                "evidence-missing" => fs::remove_file(evidence_path).unwrap(),
                "evidence-corrupt" => fs::write(evidence_path, b"{}").unwrap(),
                _ => fs::rename(root.join("result.json"), root.join("result.pending")).unwrap(),
            }
            assert!(
                stability::load_comparison(&case.store(), "measured-comparison", &case.auth)
                    .is_err(),
                "{target}/{mutation}"
            );
            if target != "measured-comparison-candidate" {
                assert!(stability::compare(
                    &case.store(),
                    &case.dir.join("work"),
                    "second-comparison",
                    &case.experiment,
                    &case.auth,
                    &profile.reference().unwrap()
                )
                .is_err());
                assert!(!case.dir.join("runs/second-comparison-candidate").exists());
            }
        }
    }
}

#[test]
fn p3a_h_rehashed_stable_claim_cannot_hide_actual_unstable_evidence() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf '1\\n'",
    );
    let mut profile = p3_profile(&case);
    let item = profile
        .observables
        .iter_mut()
        .find(|item| item.observable == Observable::Stdout)
        .unwrap();
    item.stability = Stability::Stable;
    item.observed_values.truncate(1);
    p3_rewrite(&case, "profile", &profile);
    assert!(stability::load_profile(&case.store(), "profile", &case.auth).is_err());
    assert!(p3_compare(&case, &profile.reference().unwrap()).is_err());
}

#[test]
fn p3a_h_rehashed_profile_cannot_omit_or_duplicate_runs_or_required_observables() {
    for mutation in [
        "drop-run",
        "duplicate-run",
        "drop-observable",
        "drop-evidence",
        "schema",
        "case",
        "input",
        "target",
        "experiment-hash",
        "config-hash",
    ] {
        let case = Case::new("equal");
        let mut profile = p3_profile(&case);
        match mutation {
            "drop-run" => {
                profile.runs.pop();
                profile.run_ids.pop();
            }
            "duplicate-run" => {
                profile.runs[4] = profile.runs[0].clone();
                profile.run_ids[4] = profile.run_ids[0].clone();
            }
            "drop-observable" => {
                profile.observables.pop();
            }
            "drop-evidence" => profile.evidence_refs.clear(),
            "schema" => profile.schema_version = "2".into(),
            "case" => profile.case_identity = canonical_hash(&"wrong").unwrap(),
            "input" => profile.input_identity = canonical_hash(&"wrong").unwrap(),
            "target" => profile.baseline_target_identity = canonical_hash(&"wrong").unwrap(),
            "experiment-hash" => profile.experiment_hash = canonical_hash(&"wrong").unwrap(),
            _ => profile.config_hash = canonical_hash(&"wrong").unwrap(),
        }
        p3_rewrite(&case, "profile", &profile);
        assert!(
            stability::load_profile(&case.store(), "profile", &case.auth).is_err(),
            "{mutation}"
        );
        assert!(
            p3_compare(&case, &profile.reference().unwrap()).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn p3a_i_one_repetition_is_incomplete_and_zero_is_rejected() {
    let case = Case::new("equal");
    assert!(stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "zero",
        &case.experiment,
        &case.auth,
        &ProfilingConfig { repetitions: 0 }
    )
    .is_err());
    assert!(case.count().is_empty());
    let profile = stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "profile",
        &case.experiment,
        &case.auth,
        &ProfilingConfig { repetitions: 1 },
    )
    .unwrap();
    assert_eq!(profile.repetition_count, 1);
    assert!(profile
        .observables
        .iter()
        .all(|item| item.stability == Stability::Incomplete));
    assert_eq!(
        stability::load_profile(&case.store(), "profile", &case.auth).unwrap(),
        profile
    );
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_i_rehashed_profile_cannot_lower_requested_repetitions_to_discard_outlier() {
    let case = p3_scripts(
        &format!("{COUNTER}\nif [ \"$n\" -eq 5 ]; then printf 'outlier'; else printf 'same'; fi"),
        "printf 'same'",
    );
    let mut profile = p3_profile(&case);
    assert_eq!(
        p3_observable(&profile, Observable::Stdout).stability,
        Stability::Unstable
    );
    profile.config.repetitions = 4;
    profile.repetition_count = 4;
    profile.run_ids.pop();
    profile.runs.pop();
    profile.evidence_refs.pop();
    profile.config_hash = canonical_hash(&(
        "behavior.process.baseline_stability.v1",
        &profile.experiment.case,
        &profile.config,
    ))
    .unwrap();
    let item = profile
        .observables
        .iter_mut()
        .find(|item| item.observable == Observable::Stdout)
        .unwrap();
    item.stability = Stability::Stable;
    item.observed_values.truncate(1);
    p3_rewrite(&case, "profile", &profile);
    assert!(stability::load_profile(&case.store(), "profile", &case.auth).is_err());
    assert!(p3_compare(&case, &profile.reference().unwrap()).is_err());
}

#[test]
fn p3a_j_profile_presence_does_not_change_original_p2_exact_mode() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
        "printf 'candidate\\n'",
    );
    let profile = p3_profile(&case);
    let exact = case.execute();
    assert_eq!(exact.schema_version, "1");
    assert_eq!(exact.outcome, BehaviorOutcome::DivergenceProven);
    assert_eq!(case.load().unwrap(), exact);
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
    assert_eq!(case.load().unwrap(), exact);
}

#[test]
fn p3a_fresh_workspaces_restore_one_frozen_fixture_for_all_repetitions() {
    let mut case = Case::new("source-change");
    case.fixture();
    case.experiment.case.environment.insert(
        "SOURCE_FILE".into(),
        case.dir.join("source/input.txt").to_str().unwrap().into(),
    );
    case.bind_inputs();
    let profile = p3_profile(&case);
    assert_eq!(
        p3_observable(&profile, Observable::Stdout).stability,
        Stability::Stable
    );
    for link in &profile.runs {
        let run: AcquisitionResult =
            serde_json::from_value(case.store().load(&link.run_id).unwrap().0).unwrap();
        assert_eq!(run.observation.stdout, b"original input\x00\xff");
        assert_eq!(
            link.initial_snapshot_identity,
            case.experiment.case.fixture.snapshot_identity
        );
    }
    assert_eq!(
        fs::read(case.dir.join("source/input.txt")).unwrap(),
        b"mutated source"
    );
    // A new comparison refuses a source that no longer matches the profiled input.
    assert!(p3_compare(&case, &profile.reference().unwrap()).is_err());
}

#[test]
fn p3a_rehashed_comparison_cannot_convert_uncertainty_or_divergence_to_pass() {
    for scenario in [
        "unstable",
        "divergence",
        "incomplete",
        "candidate-timeout",
        "candidate-spawn",
    ] {
        let mut case = match scenario {
            "unstable" => p3_scripts(
                &format!("{COUNTER}\nprintf '%s\\n' \"$n\""),
                "printf '1\\n'",
            ),
            "incomplete" => p3_scripts("/bin/sleep 5", "printf 'ready'"),
            "candidate-timeout" => p3_scripts("printf 'ready'", "/bin/sleep 5"),
            "candidate-spawn" => Case::new("equal"),
            _ => Case::new("stdout"),
        };
        if scenario.contains("timeout") || scenario == "incomplete" {
            case.experiment.case.timeout_ms = 1500;
            case.bind_inputs();
        }
        let profile = p3_profile(&case);
        if scenario == "candidate-timeout" || scenario == "candidate-spawn" {
            assert!(profile
                .observables
                .iter()
                .all(|item| item.stability == Stability::Stable));
        }
        if scenario == "candidate-spawn" {
            fs::set_permissions(
                &case.experiment.after.executable,
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let mut report = p3_compare(&case, &profile.reference().unwrap()).unwrap();
        let expected = match scenario {
            "divergence" => BehaviorOutcome::DivergenceProven,
            "candidate-spawn" => BehaviorOutcome::Error,
            _ => BehaviorOutcome::Inconclusive,
        };
        assert_eq!(report.outcome, expected);
        assert_eq!(
            stability::load_comparison(&case.store(), "measured-comparison", &case.auth).unwrap(),
            report
        );
        report.outcome = BehaviorOutcome::NoDivergenceFound;
        report.verdict = Verdict::Pass;
        report.divergences.clear();
        p3_rewrite(&case, "measured-comparison", &report);
        assert!(
            stability::load_comparison(&case.store(), "measured-comparison", &case.auth).is_err(),
            "{scenario}"
        );
    }
}

#[test]
fn p3a_candidate_corruption_of_profile_evidence_is_detected_after_execution() {
    let mut case = p3_scripts(
        "printf ready",
        "printf '{}' > \"$PROFILE_EVIDENCE\"\nprintf ready",
    );
    case.experiment.case.environment.insert(
        "PROFILE_EVIDENCE".into(),
        case.dir
            .join("runs/profile-baseline-0/evidence/process-observation.json")
            .to_str()
            .unwrap()
            .into(),
    );
    case.bind_inputs();
    let profile = p3_profile(&case);
    assert!(p3_compare(&case, &profile.reference().unwrap()).is_err());
    assert_eq!(case.count(), b"bbbbba");
    assert!(case.store().load("measured-comparison").is_err());
}

#[test]
fn p3a_duplicate_ids_or_existing_workspaces_never_overwrite_or_reexecute() {
    let case = Case::new("equal");
    let profile = p3_profile(&case);
    let bytes = fs::read(case.dir.join("runs/profile/result.json")).unwrap();
    assert!(stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "profile",
        &case.experiment,
        &case.auth,
        &ProfilingConfig::default()
    )
    .is_err());
    assert_eq!(case.count(), b"bbbbb");
    assert_eq!(
        fs::read(case.dir.join("runs/profile/result.json")).unwrap(),
        bytes
    );
    assert_eq!(
        stability::load_profile(&case.store(), "profile", &case.auth).unwrap(),
        profile
    );
    fs::create_dir_all(case.dir.join("work/occupied/baseline-0")).unwrap();
    fs::write(case.dir.join("work/occupied/baseline-0/keep"), b"keep").unwrap();
    assert!(stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "occupied",
        &case.experiment,
        &case.auth,
        &ProfilingConfig::default()
    )
    .is_err());
    assert_eq!(case.count(), b"bbbbb");
    assert_eq!(
        fs::read(case.dir.join("work/occupied/baseline-0/keep")).unwrap(),
        b"keep"
    );
}

#[test]
fn p3a_executable_identity_is_rechecked_before_each_baseline_acquisition() {
    let mut case = p3_scripts(
        "printf ready\nprintf '# changed executable\\n' >> \"$SELF_TARGET\"",
        "printf ready",
    );
    case.experiment.case.environment.insert(
        "SELF_TARGET".into(),
        case.experiment.before.executable.to_str().unwrap().into(),
    );
    case.bind_inputs();
    assert!(stability::profile(
        &case.store(),
        &case.dir.join("work"),
        "profile",
        &case.experiment,
        &case.auth,
        &ProfilingConfig::default()
    )
    .is_err());
    assert_eq!(case.count(), b"b");
    assert!(case.store().load("profile").is_err());
    assert!(stability::load_profile(&case.store(), "profile", &case.auth).is_err());
}

#[test]
fn p3a_artifact_schemas_and_canonical_receipts_match_actual_saved_evidence() {
    let case = Case::new("equal");
    let profile = p3_profile(&case);
    let report = p3_result(&case, &profile, BehaviorOutcome::NoDivergenceFound);
    for (id, value, schema, stored) in [
        (
            "profile",
            serde_json::to_value(&profile).unwrap(),
            stability::profile_schema(),
            include_str!("../../../schemas/baseline-stability-profile.schema.json"),
        ),
        (
            "measured-comparison",
            serde_json::to_value(&report).unwrap(),
            stability::comparison_schema(),
            include_str!("../../../schemas/stability-comparison-result.schema.json"),
        ),
    ] {
        assert_eq!(serde_json::from_str::<Value>(stored).unwrap(), schema);
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
        let raw = fs::read(case.dir.join("runs").join(id).join("result.json")).unwrap();
        assert_eq!(
            raw,
            canonical_bytes(&serde_json::from_slice::<Value>(&raw).unwrap()).unwrap()
        );
        let (saved, evidence) = case.store().load(id).unwrap();
        assert_eq!(saved, value);
        assert_eq!(
            evidence[0].trust_class,
            verify_evidence::TrustClass::Derived
        );
    }
    assert_eq!(
        report.profile.profile_hash,
        canonical_hash(&profile).unwrap()
    );
    assert_eq!(
        profile.experiment_hash,
        canonical_hash(&profile.experiment).unwrap()
    );
    assert_eq!(
        report.experiment_hash,
        canonical_hash(&case.experiment).unwrap()
    );
    assert_eq!(report.evidence_refs.len(), 6);
}

#[test]
fn p3a_exit_and_signal_variation_are_observed_without_hiding_uncertainty() {
    let case = p3_scripts(
        &format!("{COUNTER}\nif [ \"$n\" -eq 3 ]; then kill -TERM \"$$\"; fi"),
        "exit 0",
    );
    let profile = p3_profile(&case);
    for observable in [Observable::ExitCode, Observable::Signal] {
        let item = p3_observable(&profile, observable);
        assert_eq!(item.stability, Stability::Unstable);
        assert_eq!(item.observed_values.len(), 2);
    }
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_stderr_variation_is_unstable_even_when_stdout_is_stable() {
    let case = p3_scripts(
        &format!("{COUNTER}\nprintf '%s\\n' \"$n\" >&2\nprintf ready"),
        "printf ready\nprintf other >&2",
    );
    let profile = p3_profile(&case);
    assert_eq!(
        p3_observable(&profile, Observable::Stdout).stability,
        Stability::Stable
    );
    assert_eq!(
        p3_observable(&profile, Observable::Stderr).stability,
        Stability::Unstable
    );
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
}

#[test]
fn p3a_missing_terminal_observation_cannot_be_rehashed_into_stable_evidence() {
    let case = Case::new("equal");
    let mut profile = p3_profile(&case);
    let id = profile.run_ids[0].clone();
    let (value, mut evidence) = case.store().load(&id).unwrap();
    let mut run: AcquisitionResult = serde_json::from_value(value).unwrap();
    run.observation.exit_code = None;
    run.observation.signal = None;
    evidence[0].observation = verify_evidence::Observation::Value {
        value: serde_json::to_value(&run.observation).unwrap(),
    };
    evidence[0].integrity_hash = canonical_hash(&evidence[0].observation).unwrap();
    let evidence_hash = canonical_hash(&evidence[0]).unwrap();
    run.bundle
        .evidence_hashes
        .insert(evidence[0].evidence_id.clone(), evidence_hash.clone());
    rewrite_artifact(&case, &id, &run, &evidence[0]);
    assert!(case.store().load(&id).is_ok());
    profile.runs[0].result_hash = canonical_hash(&run).unwrap();
    profile.runs[0].evidence_refs[0].evidence_hash = evidence_hash.clone();
    profile.evidence_refs[0].evidence_hash = evidence_hash;
    let exit = profile
        .observables
        .iter_mut()
        .find(|item| item.observable == Observable::ExitCode)
        .unwrap();
    exit.observed_values
        .push(ObservableValue::Status { value: None });
    p3_rewrite(&case, "profile", &profile);
    assert!(stability::load_profile(&case.store(), "profile", &case.auth).is_err());
    assert!(p3_compare(&case, &profile.reference().unwrap()).is_err());
}

// P3B reuses the real P2/P3A fixture and artifact tampering helpers above.
use behavior::generation::{self, Dimension, DimensionTarget, GenerationMode, GenerationSpec};

fn g_spec(case: &Case, values: &[&str]) -> GenerationSpec {
    GenerationSpec {
        generation_id: "explicit-domain".into(),
        schema_version: "1".into(),
        base: case.experiment.clone(),
        mode: GenerationMode::OneAtATime,
        dimensions: vec![Dimension {
            target: DimensionTarget::Argument { index: 0 },
            candidates: values.iter().map(|s| (*s).into()).collect(),
        }],
        max_cases: 20,
    }
}
fn g_run(case: &Case, spec: &GenerationSpec) -> generation::BehaviorGeneratedSuiteResult {
    let report = generation::execute(
        &case.store(),
        &case.dir.join("work"),
        "suite",
        spec,
        &case.auth,
    )
    .unwrap();
    assert_eq!(
        generation::load(&case.store(), "suite", &case.auth).unwrap(),
        report
    );
    report
}
fn g_env(name: &str, values: &[&str]) -> Dimension {
    Dimension {
        target: DimensionTarget::Environment { name: name.into() },
        candidates: values.iter().map(|s| (*s).into()).collect(),
    }
}

#[test]
fn p3b_a_same_spec_has_identical_ids_order_and_input_hashes() {
    let case = Case::new("equal");
    let spec = g_spec(&case, &["equal", "stdout", "exit"]);
    let first = generation::generate(&spec).unwrap();
    let serialized = canonical_bytes(&spec).unwrap();
    let restored = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(generation::generate(&restored).unwrap(), first);
    assert_eq!(generation::generate(&spec).unwrap(), first);
    assert_eq!(first.len(), 3);
    assert_eq!(
        first
            .iter()
            .map(|g| &g.identity.generated_case_id)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    for item in first {
        assert_eq!(
            item.identity.generation_spec_hash,
            canonical_hash(&spec).unwrap()
        );
        assert_eq!(
            item.identity.parent_case_identity,
            case.experiment.case.identity().unwrap()
        );
        assert_eq!(
            item.identity.input_identity,
            item.experiment.case.input_identity().unwrap()
        );
    }
}

#[test]
fn p3b_b_one_at_a_time_changes_only_one_dimension_from_base() {
    let mut case = Case::new("equal");
    case.experiment.case.args.push("base-second".into());
    case.bind_inputs();
    let mut spec = g_spec(&case, &["first", "second"]);
    spec.dimensions.push(Dimension {
        target: DimensionTarget::Argument { index: 1 },
        candidates: vec!["third".into()],
    });
    let generated = generation::generate(&spec).unwrap();
    let args: Vec<_> = generated
        .iter()
        .map(|g| g.experiment.case.args.clone())
        .collect();
    assert_eq!(
        args,
        vec![
            vec!["first", "base-second"],
            vec!["second", "base-second"],
            vec!["equal", "third"]
        ]
    );
    assert!(generated.iter().all(|g| g.identity.assignment.len() == 1));
}

#[test]
fn p3b_c_env_dimension_is_acquired_by_both_real_processes() {
    let case = p3_scripts("printf '%s' \"$VALUE\"", "printf '%s' \"$VALUE\"");
    let mut spec = g_spec(&case, &["unused"]);
    spec.dimensions = vec![g_env("VALUE", &["one", "two", ""])];
    let suite = g_run(&case, &spec);
    assert_eq!(suite.aggregate_verdict, Verdict::Pass);
    for (i, reference) in suite.comparisons.iter().enumerate() {
        let child = behavior::load(&case.store(), &reference.comparison_id, &case.auth).unwrap();
        for run in [child.before.unwrap(), child.after.unwrap()] {
            let run: AcquisitionResult =
                serde_json::from_value(case.store().load(&run.run_id).unwrap().0).unwrap();
            assert_eq!(
                run.observation.stdout,
                spec.dimensions[0].candidates[i].as_bytes()
            );
        }
    }
    assert_eq!(case.count(), b"bababa");
}

#[test]
fn p3b_d_cartesian_last_dimension_varies_fastest() {
    let case = Case::new("equal");
    let mut spec = g_spec(&case, &["a", "b"]);
    spec.mode = GenerationMode::Cartesian;
    spec.dimensions.push(g_env("VALUE", &["1", "2", "3"]));
    let generated = generation::generate(&spec).unwrap();
    assert_eq!(generated.len(), 6);
    let pairs: Vec<_> = generated
        .iter()
        .map(|g| {
            (
                g.experiment.case.args[0].as_str(),
                g.experiment.case.environment["VALUE"].as_str(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("a", "1"),
            ("a", "2"),
            ("a", "3"),
            ("b", "1"),
            ("b", "2"),
            ("b", "3")
        ]
    );
    assert!(generated.iter().all(|g| g.identity.assignment.len() == 2));
}

#[test]
fn p3b_e_budget_overflow_rejects_without_any_execution_or_suite_reservation() {
    for mode in [GenerationMode::OneAtATime, GenerationMode::Cartesian] {
        let case = Case::new("equal");
        let mut spec = g_spec(&case, &["a", "b"]);
        spec.mode = mode;
        spec.dimensions.push(g_env("VALUE", &["1", "2"]));
        spec.max_cases = 3;
        assert!(generation::execute(
            &case.store(),
            &case.dir.join("work"),
            "suite",
            &spec,
            &case.auth
        )
        .is_err());
        assert!(case.count().is_empty());
        assert!(!case.dir.join("runs/suite").exists());
    }
}

#[test]
fn p3b_f_invalid_dimensions_and_empty_generation_reject_atomically() {
    for mutation in [
        "none",
        "empty",
        "duplicate-target",
        "duplicate-value",
        "index",
        "env-name",
        "nul",
        "zero",
        "schema",
        "duplicate-input",
        "oversized-budget",
    ] {
        let case = Case::new("equal");
        let mut spec = g_spec(&case, &["equal", "stdout"]);
        match mutation {
            "none" => spec.dimensions.clear(),
            "empty" => spec.dimensions[0].candidates.clear(),
            "duplicate-target" => spec.dimensions.push(spec.dimensions[0].clone()),
            "duplicate-value" => spec.dimensions[0].candidates.push("equal".into()),
            "index" => spec.dimensions[0].target = DimensionTarget::Argument { index: u64::MAX },
            "env-name" => spec.dimensions = vec![g_env("BAD=NAME", &["x"])],
            "nul" => spec.dimensions[0].candidates.push("\0".into()),
            "zero" => spec.max_cases = 0,
            "schema" => spec.schema_version = "2".into(),
            "duplicate-input" => spec.dimensions.push(g_env(
                "EXECUTION_MARKER",
                &[case.experiment.case.environment["EXECUTION_MARKER"].as_str()],
            )),
            _ => spec.max_cases = u64::MAX,
        }
        assert!(
            generation::execute(
                &case.store(),
                &case.dir.join("work"),
                "suite",
                &spec,
                &case.auth
            )
            .is_err(),
            "{mutation}"
        );
        assert!(case.count().is_empty());
        assert!(!case.dir.join("runs/suite").exists());
    }
}

#[test]
fn p3b_g_all_exact_children_pass_with_complete_coverage() {
    let case = p3_scripts("printf '%s' \"$1\"", "printf '%s' \"$1\"");
    let spec = g_spec(&case, &["a", "b", "c"]);
    let suite = g_run(&case, &spec);
    assert_eq!(suite.aggregate_verdict, Verdict::Pass);
    assert_eq!(suite.outcome_counts.no_divergence_found, 3);
    assert_eq!(suite.coverage.expected_cases, 3);
    assert_eq!(suite.coverage.completed_pairs, 3);
    assert!(suite.coverage.complete);
    assert_eq!(case.count(), b"bababa");
}

#[test]
fn p3b_h_one_generated_stdout_difference_is_fail() {
    let case = p3_scripts(
        "printf same",
        "if [ \"$1\" = changed ]; then printf other; else printf same; fi",
    );
    let suite = g_run(&case, &g_spec(&case, &["equal", "changed"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Fail);
    assert_eq!(suite.outcome_counts.divergence_proven, 1);
    let child = behavior::load(
        &case.store(),
        &suite.comparisons[1].comparison_id,
        &case.auth,
    )
    .unwrap();
    assert_eq!(child.divergences[0].observable, Observable::Stdout);
    assert!(suite.coverage.complete);
}

#[test]
fn p3b_i_one_generated_exit_difference_is_fail() {
    let case = p3_scripts("exit 0", "if [ \"$1\" = changed ]; then exit 7; fi");
    let suite = g_run(&case, &g_spec(&case, &["equal", "changed"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Fail);
    assert_eq!(suite.outcome_counts.divergence_proven, 1);
    let child = behavior::load(
        &case.store(),
        &suite.comparisons[1].comparison_id,
        &case.auth,
    )
    .unwrap();
    assert_eq!(child.divergences[0].observable, Observable::ExitCode);
}

#[test]
fn p3b_j_timeout_without_fail_is_inconclusive() {
    let mut case = p3_scripts(
        "printf same",
        "if [ \"$1\" = slow ]; then /bin/sleep 5; fi\nprintf same",
    );
    case.experiment.case.timeout_ms = 1500;
    case.bind_inputs();
    let suite = g_run(&case, &g_spec(&case, &["equal", "slow"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Inconclusive);
    assert_eq!(suite.outcome_counts.inconclusive, 1);
    assert!(!suite.coverage.complete);
}

#[test]
fn p3b_k_real_spawn_error_without_fail_is_error() {
    let case = Case::new("equal");
    fs::set_permissions(
        &case.experiment.after.executable,
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let suite = g_run(&case, &g_spec(&case, &["equal", "stdout"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Error);
    assert_eq!(suite.outcome_counts.error, 2);
    assert!(!suite.coverage.complete);
    for reference in suite.comparisons {
        let child = behavior::load(&case.store(), &reference.comparison_id, &case.auth).unwrap();
        assert!(child.before.is_some() && child.after.is_some());
    }
}

#[test]
fn p3b_l_fail_and_timeout_preserves_fail_with_incomplete_coverage() {
    let mut case = p3_scripts(
        "printf same",
        "if [ \"$1\" = slow ]; then /bin/sleep 5; fi\nprintf other",
    );
    case.experiment.case.timeout_ms = 1500;
    case.bind_inputs();
    let suite = g_run(&case, &g_spec(&case, &["changed", "slow"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Fail);
    assert_eq!(suite.outcome_counts.divergence_proven, 1);
    assert_eq!(suite.outcome_counts.inconclusive, 1);
    assert!(!suite.coverage.complete);
    assert_eq!(suite.coverage.completed_pairs, 1);
}

#[test]
fn p3b_m_generation_preserves_all_non_input_contract_fields() {
    let case = Case::new("equal");
    let mut spec = g_spec(&case, &["new"]);
    spec.dimensions.push(g_env("NEW_ENV", &["new"]));
    for mode in [GenerationMode::OneAtATime, GenerationMode::Cartesian] {
        spec.mode = mode;
        for generated in generation::generate(&spec).unwrap() {
            let mut restored = generated.experiment;
            restored.case.case_id = spec.base.case.case_id.clone();
            restored.case.args = spec.base.case.args.clone();
            restored.case.environment = spec.base.case.environment.clone();
            restored.before.input_identity = spec.base.before.input_identity.clone();
            restored.after.input_identity = spec.base.after.input_identity.clone();
            assert_eq!(restored, spec.base);
        }
    }
}

#[test]
fn p3b_n_missing_corrupt_or_tampered_child_and_run_evidence_cannot_load_pass() {
    for artifact in ["child", "before", "after"] {
        for mutation in ["missing", "corrupt", "evidence", "rehashed"] {
            let case = Case::new("equal");
            let suite = g_run(&case, &g_spec(&case, &["equal"]));
            let child_id = &suite.comparisons[0].comparison_id;
            let id = if artifact == "child" {
                child_id.clone()
            } else {
                format!("{child_id}-{artifact}")
            };
            let root = case.dir.join("runs").join(&id);
            match mutation {
                "missing" => fs::remove_file(root.join("result.json")).unwrap(),
                "corrupt" => fs::write(root.join("result.json"), b"{}").unwrap(),
                "evidence" => fs::remove_dir_all(root.join("evidence")).unwrap(),
                _ => {
                    let (mut value, _) = case.store().load(&id).unwrap();
                    if artifact == "child" {
                        value["experiment"]["case"]["args"] = json!(["tampered"]);
                    } else {
                        value["observation"]["stdout"] = json!([88]);
                    }
                    p3_rewrite(&case, &id, &value);
                }
            }
            assert!(
                generation::load(&case.store(), "suite", &case.auth).is_err(),
                "{artifact}/{mutation}"
            );
        }
    }
}

#[test]
fn p3b_o_rehashed_suite_cannot_lie_about_verdict_counts_spec_inputs_or_completeness() {
    for mutation in [
        "verdict",
        "counts",
        "empty",
        "missing",
        "duplicate",
        "order",
        "spec",
        "spec-hash",
        "input",
        "assignment",
        "ref-hash",
        "coverage",
        "budget",
        "schema",
        "authorization",
    ] {
        let case = p3_scripts(
            "printf same",
            "if [ \"$1\" = changed ]; then printf other; else printf same; fi",
        );
        let mut suite = g_run(&case, &g_spec(&case, &["equal", "changed"]));
        match mutation {
            "verdict" => suite.aggregate_verdict = Verdict::Pass,
            "counts" => {
                suite.outcome_counts.divergence_proven = 0;
                suite.outcome_counts.no_divergence_found = 2;
                suite.aggregate_verdict = Verdict::Pass;
            }
            "empty" => {
                suite.generated_cases.clear();
                suite.comparisons.clear();
            }
            "missing" => {
                suite.generated_cases.pop();
                suite.comparisons.pop();
            }
            "duplicate" => suite.generated_cases[1] = suite.generated_cases[0].clone(),
            "order" => {
                suite.generated_cases.reverse();
                suite.comparisons.reverse();
            }
            "spec" => {
                suite.generation_spec.dimensions[0].candidates.pop();
                suite.generation_spec_hash = canonical_hash(&suite.generation_spec).unwrap();
            }
            "spec-hash" => suite.generation_spec_hash = canonical_hash(&"wrong").unwrap(),
            "input" => suite.generated_cases[0].input_identity = canonical_hash(&"wrong").unwrap(),
            "assignment" => suite.generated_cases[0].assignment[0].value = "wrong".into(),
            "ref-hash" => suite.comparisons[0].comparison_hash = canonical_hash(&"wrong").unwrap(),
            "coverage" => suite.coverage.completed_pairs = 0,
            "budget" => suite.generation_budget = 1,
            "schema" => suite.schema_version = "2".into(),
            _ => suite.authorization_hash = canonical_hash(&"wrong").unwrap(),
        }
        p3_rewrite(&case, "suite", &suite);
        assert!(
            generation::load(&case.store(), "suite", &case.auth).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn p3b_p_existing_profile_and_exact_semantics_remain_independent() {
    let case = p3_scripts(&format!("{COUNTER}\nprintf '%s' \"$n\""), "printf 1");
    let profile = p3_profile(&case);
    let suite = g_run(&case, &g_spec(&case, &["equal"]));
    assert_eq!(suite.aggregate_verdict, Verdict::Fail);
    p3_result(&case, &profile, BehaviorOutcome::Inconclusive);
    assert_eq!(
        generation::load(&case.store(), "suite", &case.auth).unwrap(),
        suite
    );
    assert_eq!(
        stability::load_profile(&case.store(), "profile", &case.auth).unwrap(),
        profile
    );
}

#[test]
fn p3b_schema_v1_and_atomic_receipt_match_generated_suite() {
    let case = Case::new("equal");
    let spec = g_spec(&case, &["equal"]);
    let suite = g_run(&case, &spec);
    for (schema, value) in [
        (
            generation::spec_schema(),
            serde_json::to_value(&spec).unwrap(),
        ),
        (
            generation::suite_schema(),
            serde_json::to_value(&suite).unwrap(),
        ),
    ] {
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
        assert_eq!(schema["properties"]["schema_version"]["const"], "1");
    }
    let raw = fs::read(case.dir.join("runs/suite/result.json")).unwrap();
    assert_eq!(
        raw,
        canonical_bytes(&serde_json::from_slice::<Value>(&raw).unwrap()).unwrap()
    );
    assert!(generation::execute(
        &case.store(),
        &case.dir.join("work"),
        "suite",
        &spec,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), b"ba");
    assert_eq!(
        fs::read(case.dir.join("runs/suite/result.json")).unwrap(),
        raw
    );
}

#[test]
fn p3b_unapproved_base_rejects_before_execution() {
    let mut case = Case::new("equal");
    case.auth.approved_baselines.clear();
    let spec = g_spec(&case, &["equal"]);
    assert!(generation::execute(
        &case.store(),
        &case.dir.join("work"),
        "suite",
        &spec,
        &case.auth
    )
    .is_err());
    assert!(case.count().is_empty());
}

#[test]
fn p3b_fail_precedes_error_and_error_precedes_inconclusive() {
    for first in ["fail", "slow"] {
        let mut case = p3_scripts(
            "printf same",
            "/bin/chmod 600 \"$0\"\nif [ \"$1\" = slow ]; then /bin/sleep 5; fi\nprintf other",
        );
        case.experiment.case.timeout_ms = 1500;
        case.bind_inputs();
        let suite = g_run(&case, &g_spec(&case, &[first, "next"]));
        assert_eq!(suite.outcome_counts.error, 1);
        assert_eq!(
            suite.aggregate_verdict,
            if first == "fail" {
                Verdict::Fail
            } else {
                Verdict::Error
            }
        );
        assert!(!suite.coverage.complete);
    }
}

#[test]
fn p3b_later_execution_corrupting_earlier_evidence_prevents_suite_commit() {
    let mut case = p3_scripts("printf same", "if [ \"$1\" = corrupt ]; then\nfor file in \"$STORE_ROOT\"/generated-comparison-*/evidence/behavior-comparison.json; do printf '{}' > \"$file\"; done\nfi\nprintf same");
    case.experiment.case.environment.insert(
        "STORE_ROOT".into(),
        case.dir.join("runs").to_str().unwrap().into(),
    );
    case.bind_inputs();
    let spec = g_spec(&case, &["equal", "corrupt"]);
    assert!(generation::execute(
        &case.store(),
        &case.dir.join("work"),
        "suite",
        &spec,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), b"baba");
    assert!(generation::load(&case.store(), "suite", &case.auth).is_err());
    assert!(!case.dir.join("runs/suite/result.json").exists());
}

#[test]
fn p3b_rehashed_child_uncertainty_cannot_be_promoted_to_pass() {
    let mut case = p3_scripts("printf same", "/bin/sleep 5\nprintf same");
    case.experiment.case.timeout_ms = 1500;
    case.bind_inputs();
    let mut suite = g_run(&case, &g_spec(&case, &["slow"]));
    let id = suite.comparisons[0].comparison_id.clone();
    let mut child = behavior::load(&case.store(), &id, &case.auth).unwrap();
    child.outcome = BehaviorOutcome::NoDivergenceFound;
    child.verdict = Verdict::Pass;
    p3_rewrite(&case, &id, &child);
    suite.comparisons[0].comparison_hash = canonical_hash(&child).unwrap();
    suite.aggregate_verdict = Verdict::Pass;
    suite.outcome_counts.inconclusive = 0;
    suite.outcome_counts.no_divergence_found = 1;
    suite.coverage.complete = true;
    suite.coverage.completed_pairs = 1;
    p3_rewrite(&case, "suite", &suite);
    assert!(generation::load(&case.store(), "suite", &case.auth).is_err());
}

#[test]
fn p3b_saved_schema_files_equal_runtime_v1_schemas() {
    assert_eq!(
        serde_json::from_str::<Value>(include_str!("../../../schemas/generation-spec.schema.json"))
            .unwrap(),
        generation::spec_schema()
    );
    assert_eq!(
        serde_json::from_str::<Value>(include_str!(
            "../../../schemas/behavior-generated-suite-result.schema.json"
        ))
        .unwrap(),
        generation::suite_schema()
    );
}

use behavior::reduction::{self, ReductionInput, ReductionStatus};

fn r_input(case: &Case, budget: u64) -> ReductionInput {
    let mut spec = g_spec(case, &["changed"]);
    spec.mode = GenerationMode::Cartesian;
    spec.dimensions
        .extend([g_env("X", &["1"]), g_env("Y", &["1"])]);
    let suite = g_run(case, &spec);
    let child = behavior::load(
        &case.store(),
        &suite.comparisons[0].comparison_id,
        &case.auth,
    )
    .unwrap();
    ReductionInput {
        source_suite_id: suite.suite_id.clone(),
        source_suite_hash: canonical_hash(&suite).unwrap(),
        source_comparison: suite.comparisons[0].clone(),
        failing_case: suite.generated_cases[0].clone(),
        generation_spec: spec,
        expected_signature: reduction::signature(&child).unwrap(),
        execution_budget: budget,
    }
}
fn r_run(
    case: &Case,
    input: &ReductionInput,
    id: &str,
) -> reduction::CounterexampleReductionResult {
    let report =
        reduction::execute(&case.store(), &case.dir.join("work"), id, input, &case.auth).unwrap();
    assert_eq!(
        reduction::load(&case.store(), id, &case.auth).unwrap(),
        report
    );
    report
}
fn r_case() -> Case {
    p3_scripts(
        "printf same",
        "if [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi",
    )
}

#[test]
fn p3c_a_three_assignments_reduce_to_one_with_fresh_pairs() {
    let case = r_case();
    let input = r_input(&case, 20);
    let report = r_run(&case, &input, "reduce");
    assert_eq!(report.status, ReductionStatus::Minimized);
    assert!(report.minimal);
    assert_eq!(report.reduced_assignment.len(), 1);
    assert_eq!(
        report.reduced_assignment[0].target,
        DimensionTarget::Environment { name: "Y".into() }
    );
    assert_eq!(report.accepted_reductions, 2);
    assert_eq!(case.count().len() as u64, 2 + 2 * report.execution_count);
    assert_ne!(report.final_comparison, input.source_comparison);
    let final_child = behavior::load(
        &case.store(),
        &report.final_comparison.comparison_id,
        &case.auth,
    )
    .unwrap();
    assert_eq!(final_child.outcome, BehaviorOutcome::DivergenceProven);
    assert_eq!(final_child.before, report.final_before);
    assert_eq!(final_child.after, report.final_after);
    assert_eq!(final_child.evidence_refs, report.final_evidence_refs);
    for link in [report.final_before.unwrap(), report.final_after.unwrap()] {
        assert!(link
            .run_id
            .starts_with(&report.final_comparison.comparison_id));
        assert!(!link
            .run_id
            .starts_with(&input.source_comparison.comparison_id));
    }
}

#[test]
fn p3c_b_joint_cause_keeps_two_assignments() {
    let case = p3_scripts(
        "printf same",
        "if [ \"${X:-}\" = 1 ] && [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi",
    );
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    assert!(report.minimal);
    assert_eq!(report.reduced_assignment.len(), 2);
}

#[test]
fn p3c_c_no_deletion_preserves_failure_is_unreducible() {
    let case = p3_scripts("printf same", "if [ \"$1\" = changed ] && [ \"${X:-}\" = 1 ] && [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; fi");
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    assert_eq!(report.status, ReductionStatus::Unreducible);
    assert!(report.minimal);
    assert_eq!(report.original_assignment, report.reduced_assignment);
    assert_eq!(report.execution_count, 3);
}

#[test]
fn p3c_d_no_divergence_is_not_accepted() {
    let case = r_case();
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    let last = report.attempted_candidates.last().unwrap();
    assert_eq!(last.outcome, BehaviorOutcome::NoDivergenceFound);
    assert!(!last.accepted);
}

#[test]
fn p3c_e_timeout_cannot_prove_minimality_or_preservation() {
    let mut case = p3_scripts(
        "printf same",
        "if [ \"${Y:-}\" = 1 ]; then printf changed; else /bin/sleep 5; fi",
    );
    case.experiment.case.timeout_ms = 1500;
    case.bind_inputs();
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    assert_eq!(report.status, ReductionStatus::Inconclusive);
    assert!(!report.minimal);
    assert_eq!(report.reduced_assignment.len(), 1);
    assert!(!report.attempted_candidates.last().unwrap().accepted);
}

#[test]
fn p3c_f_spawn_error_cannot_preserve_failure() {
    let case = r_case();
    let input = r_input(&case, 20);
    fs::set_permissions(
        &case.experiment.after.executable,
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let report = r_run(&case, &input, "reduce");
    assert_eq!(report.status, ReductionStatus::Error);
    assert!(!report.minimal);
    assert_eq!(report.accepted_reductions, 0);
    assert!(report
        .attempted_candidates
        .iter()
        .all(|a| a.outcome == BehaviorOutcome::Error && !a.accepted));
}

#[test]
fn p3c_g_non_failing_source_rejected_before_reservation() {
    let case = r_case();
    let mut input = r_input(&case, 20);
    let mut spec = input.generation_spec.clone();
    spec.dimensions[2].candidates = vec!["0".into()];
    let suite = generation::execute(
        &case.store(),
        &case.dir.join("work"),
        "passing",
        &spec,
        &case.auth,
    )
    .unwrap();
    input.source_suite_id = suite.suite_id.clone();
    input.source_suite_hash = canonical_hash(&suite).unwrap();
    input.source_comparison = suite.comparisons[0].clone();
    input.failing_case = suite.generated_cases[0].clone();
    input.generation_spec = spec;
    let count = case.count();
    assert!(reduction::execute(
        &case.store(),
        &case.dir.join("work"),
        "reduce",
        &input,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), count);
    assert!(!case.dir.join("runs/reduce").exists());
}

#[test]
fn p3c_h_corrupt_source_evidence_rejects_before_execution() {
    let case = r_case();
    let input = r_input(&case, 20);
    let child = behavior::load(
        &case.store(),
        &input.source_comparison.comparison_id,
        &case.auth,
    )
    .unwrap();
    let link = &child.evidence_refs[0];
    fs::write(
        case.dir
            .join("runs")
            .join(&link.run_id)
            .join("evidence")
            .join(format!("{}.json", link.evidence_id)),
        b"{}",
    )
    .unwrap();
    let count = case.count();
    assert!(reduction::execute(
        &case.store(),
        &case.dir.join("work"),
        "reduce",
        &input,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), count);
}

#[test]
fn p3c_i_budget_retains_proven_best_without_minimal_claim() {
    let case = r_case();
    let report = r_run(&case, &r_input(&case, 1), "reduce");
    assert_eq!(report.status, ReductionStatus::BudgetExhausted);
    assert!(!report.minimal);
    assert_eq!(report.execution_count, 1);
    assert_eq!(report.reduced_assignment.len(), 2);
    assert_eq!(report.accepted_reductions, 1);
}

#[test]
fn p3c_j_same_input_budget_has_same_order_and_assignment() {
    let case = r_case();
    let input = r_input(&case, 20);
    let a = r_run(&case, &input, "first");
    let b = r_run(&case, &input, "second");
    assert_eq!(a.reduced_assignment, b.reduced_assignment);
    assert_eq!(
        a.attempted_candidates
            .iter()
            .map(|a| (&a.assignment, a.accepted))
            .collect::<Vec<_>>(),
        b.attempted_candidates
            .iter()
            .map(|a| (&a.assignment, a.accepted))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.status, b.status);
}

#[test]
fn p3c_k_different_exit_failure_is_not_original_stdout_failure() {
    let case = p3_scripts(
        "printf same",
        "if [ \"${Y:-}\" = 1 ]; then printf changed; else printf same; exit 7; fi",
    );
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    assert_eq!(report.reduced_assignment.len(), 1);
    let last = report.attempted_candidates.last().unwrap();
    assert_eq!(last.outcome, BehaviorOutcome::DivergenceProven);
    assert_eq!(
        last.signature.as_ref().unwrap().observables,
        vec![Observable::ExitCode]
    );
    assert!(!last.accepted);
}

#[test]
fn p3c_m_rehashed_assignment_minimal_status_signature_and_trace_tampering_rejected() {
    let case = r_case();
    let report = r_run(&case, &r_input(&case, 1), "reduce");
    for kind in 0..6 {
        let mut forged = report.clone();
        match kind {
            0 => forged.reduced_assignment.clear(),
            1 => forged.minimal = true,
            2 => forged.status = ReductionStatus::Minimized,
            3 => forged.final_signature.observables = vec![Observable::ExitCode],
            4 => forged.attempted_candidates[0].accepted = false,
            _ => forged.final_before = None,
        }
        p3_rewrite(&case, "reduce", &forged);
        assert!(
            reduction::load(&case.store(), "reduce", &case.auth).is_err(),
            "tamper {kind}"
        );
    }
}

#[test]
fn p3c_n_source_suite_and_immutable_execution_contract_unchanged() {
    let case = r_case();
    let input = r_input(&case, 20);
    let report = r_run(&case, &input, "reduce");
    assert_eq!(
        canonical_hash(&generation::load(&case.store(), "suite", &case.auth).unwrap()).unwrap(),
        input.source_suite_hash
    );
    for a in report.attempted_candidates {
        let child = behavior::load(&case.store(), &a.comparison.comparison_id, &case.auth).unwrap();
        assert_eq!(child.experiment.baseline, case.experiment.baseline);
        assert_eq!(
            child.experiment.before.executable,
            case.experiment.before.executable
        );
        assert_eq!(
            child.experiment.after.executable,
            case.experiment.after.executable
        );
        assert_eq!(
            child.experiment.case.timeout_ms,
            case.experiment.case.timeout_ms
        );
        assert_eq!(child.experiment.case.fixture, case.experiment.case.fixture);
        assert_eq!(
            child.experiment.case.required_observers,
            case.experiment.case.required_observers
        );
        assert_eq!(child.experiment.seed, case.experiment.seed);
    }
}

#[test]
fn p3c_zero_budget_preserves_original_without_execution() {
    let case = r_case();
    let input = r_input(&case, 0);
    let count = case.count();
    let report = r_run(&case, &input, "reduce");
    assert_eq!(report.status, ReductionStatus::BudgetExhausted);
    assert!(!report.minimal);
    assert_eq!(report.final_comparison, input.source_comparison);
    assert_eq!(case.count(), count);
}

#[test]
fn p3c_removal_restarts_to_establish_one_minimality() {
    let case = p3_scripts("printf same", "if { [ \"$1\" = changed ] && [ \"${X:-}\" = 1 ]; } || { [ \"${X:-}\" != 1 ] && [ \"${Y:-}\" = 1 ]; }; then printf changed; else printf same; fi");
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    assert!(!report.attempted_candidates[0].accepted);
    assert!(report.attempted_candidates[1].accepted);
    assert!(report.attempted_candidates[2].accepted);
    assert_eq!(report.reduced_assignment.len(), 1);
    assert!(report.minimal);
}

#[test]
fn p3c_base_can_itself_be_a_fresh_proven_empty_assignment() {
    let case = p3_scripts("printf same", "printf changed");
    let report = r_run(&case, &r_input(&case, 3), "reduce");
    assert!(report.reduced_assignment.is_empty());
    assert!(report.minimal);
    assert_eq!(report.status, ReductionStatus::Minimized);
}

#[test]
fn p3c_final_evidence_damage_rejected_on_load() {
    let case = r_case();
    let report = r_run(&case, &r_input(&case, 20), "reduce");
    let link = &report.final_evidence_refs[0];
    fs::remove_dir_all(case.dir.join("runs").join(&link.run_id)).unwrap();
    assert!(reduction::load(&case.store(), "reduce", &case.auth).is_err());
}

#[test]
fn p3c_schema_and_non_overwrite() {
    let case = r_case();
    let input = r_input(&case, 20);
    let report = r_run(&case, &input, "reduce");
    jsonschema::validator_for(&reduction::result_schema())
        .unwrap()
        .validate(&serde_json::to_value(&report).unwrap())
        .unwrap();
    let count = case.count();
    assert!(reduction::execute(
        &case.store(),
        &case.dir.join("work"),
        "reduce",
        &input,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), count);
    assert_eq!(
        reduction::load(&case.store(), "reduce", &case.auth).unwrap(),
        report
    );
}

#[test]
fn p3c_source_identity_and_signature_mismatches_reject_before_execution() {
    let case = r_case();
    let input = r_input(&case, 20);
    let count = case.count();
    for kind in 0..5 {
        let mut wrong = input.clone();
        match kind {
            0 => wrong.source_suite_hash = canonical_hash(&"wrong").unwrap(),
            1 => wrong.source_comparison.comparison_hash = canonical_hash(&"wrong").unwrap(),
            2 => wrong.failing_case.assignment[0].value = "wrong".into(),
            3 => wrong.generation_spec.base.seed += 1,
            _ => wrong.expected_signature.observables = vec![Observable::ExitCode],
        }
        assert!(reduction::execute(
            &case.store(),
            &case.dir.join("work"),
            "reduce",
            &wrong,
            &case.auth
        )
        .is_err());
        assert_eq!(case.count(), count);
        assert!(!case.dir.join("runs/reduce").exists());
    }
}

#[test]
fn p3c_signature_allows_different_bytes_of_same_observable() {
    let case = p3_scripts(
        "printf same",
        "if [ \"${Y:-}\" = 1 ]; then printf 'changed-%s' \"${X:-absent}\"; else printf same; fi",
    );
    let input = r_input(&case, 20);
    let original = behavior::load(
        &case.store(),
        &input.source_comparison.comparison_id,
        &case.auth,
    )
    .unwrap();
    let report = r_run(&case, &input, "reduce");
    let final_child = behavior::load(
        &case.store(),
        &report.final_comparison.comparison_id,
        &case.auth,
    )
    .unwrap();
    assert_ne!(
        original.divergences[0].after,
        final_child.divergences[0].after
    );
    assert_eq!(report.original_signature, report.final_signature);
    assert_eq!(report.reduced_assignment.len(), 1);
}

#[test]
fn p3c_missing_source_child_rejected() {
    let case = r_case();
    let input = r_input(&case, 20);
    fs::remove_dir_all(
        case.dir
            .join("runs")
            .join(&input.source_comparison.comparison_id),
    )
    .unwrap();
    let count = case.count();
    assert!(reduction::execute(
        &case.store(),
        &case.dir.join("work"),
        "reduce",
        &input,
        &case.auth
    )
    .is_err());
    assert_eq!(case.count(), count);
}

#[test]
fn p3c_saved_schema_equals_runtime() {
    assert_eq!(
        serde_json::from_str::<Value>(include_str!(
            "../../../schemas/counterexample-reduction-result.schema.json"
        ))
        .unwrap(),
        reduction::result_schema()
    );
}
