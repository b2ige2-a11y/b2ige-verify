//! P2 explicit process differential. Only `execute` acquires comparison inputs;
//! `load` revalidates saved evidence. Neither accepts caller-supplied observations.
//! Trusted local Unix boundary: hashes establish integrity, not authentication.
pub mod generation;
pub mod reduction;
pub mod stability;
pub(crate) mod workspace;

use crate::{
    acquisition::{acquire, assemble, validate_inputs, AcquisitionResult},
    checker_binding_hash, pinned, present, ApprovalStatus, Baseline, Claim, ExperimentPlan,
    Predicate, ProductContract, Scope, Verdict,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use verify_evidence::{
    canonical_hash, store::EvidenceStore, unique_map, valid_hash, Evidence, Observation,
    ObservationCoverage, TrustClass, MAX_SAFE_INTEGER,
};
use verify_replay::Replayability;
use verify_runner::{
    process::{ProcessObservation, ProcessSpec, CAPTURE_LIMIT, OBSERVER},
    IsolationLevel,
};
use workspace::Snapshot;

const CLAIM: &str = "behavior.process.byte_exact.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BehaviorOutcome {
    DivergenceProven,
    NoDivergenceFound,
    Inconclusive,
    Error,
}
impl BehaviorOutcome {
    pub fn verdict(self) -> Verdict {
        match self {
            Self::DivergenceProven => Verdict::Fail,
            Self::NoDivergenceFound => Verdict::Pass,
            Self::Inconclusive => Verdict::Inconclusive,
            Self::Error => Verdict::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonPolicy {
    ProcessByteExactV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalFixture {
    /// None creates an empty workspace. stdin remains P1's /dev/null.
    pub source: Option<PathBuf>,
    /// Content/mode/directory snapshot identity, checked before either execution.
    pub snapshot_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BehaviorCase {
    pub schema_version: String,
    pub case_id: String,
    pub args: Vec<String>,
    #[serde(deserialize_with = "unique_map")]
    pub environment: BTreeMap<String, String>,
    pub timeout_ms: u64,
    pub fixture: LocalFixture,
    pub comparison_policy: ComparisonPolicy,
    pub required_observers: Vec<String>,
}
impl BehaviorCase {
    /// Same logical input; physical executable/cwd paths are not input identity.
    pub fn input_identity(&self) -> io::Result<String> {
        Ok(canonical_hash(&serde_json::json!({
            "input_version": self.schema_version,
            "args": self.args, "environment": self.environment,
            "timeout_ms": self.timeout_ms, "stdin": "null",
            "snapshot_identity": self.fixture.snapshot_identity,
            "comparison_policy": self.comparison_policy,
            "required_observers": self.required_observers,
            "reset_strategy": "local_snapshot_copy_v1",
        }))?)
    }

    pub fn identity(&self) -> io::Result<String> {
        Ok(canonical_hash(&(
            self.case_id.as_str(),
            self.input_identity()?,
        ))?)
    }

    /// The approved baseline pins this versioned observation/checker contract.
    pub fn observation_contract_hash(&self) -> io::Result<String> {
        Ok(canonical_hash(&serde_json::json!({
            "contract_id": CLAIM,
            "comparison_policy": self.comparison_policy,
            "required_observers": self.required_observers,
            "observables": ["exit_code", "signal", "stdout_raw_bytes", "stderr_raw_bytes"],
            "required_coverage": "complete",
        }))?)
    }

    pub fn checker_claim(&self) -> io::Result<Claim> {
        // P1A's artifact/claim binding pins the P2 checker descriptor. The P1A
        // synthetic Equals evaluator is never used to judge these process runs.
        Ok(Claim {
            id: CLAIM.into(),
            observer: OBSERVER.into(),
            accepted_trust: vec![TrustClass::DirectRuntime],
            predicate: Predicate::Equals {
                expected: serde_json::json!({
                    "checker": CLAIM, "observation_contract_hash": self.observation_contract_hash()?,
                }),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BehaviorTarget {
    pub executable: PathBuf,
    /// SHA-256 of the actual executable bytes, not Git or a mutable label.
    pub identity: String,
    pub input_identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BehaviorExperiment {
    pub schema_version: String,
    pub case: BehaviorCase,
    pub baseline: Baseline,
    pub before: BehaviorTarget,
    pub after: BehaviorTarget,
    pub seed: u64,
}

/// Supplied by a trusted human/policy boundary, never learned or updated here.
/// This reuses P1A's baseline provenance, artifact pins and checker binding hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorAuthorization {
    pub approved_baselines: BTreeSet<String>,
    pub approved_checker_bindings: BTreeSet<String>,
    /// P1A's external stability assertion; P2 performs no noise/stability learning.
    pub baseline_stable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub run_id: String,
    pub evidence_id: String,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Observable {
    ExitCode,
    Signal,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservableValue {
    Status { value: Option<i32> },
    Bytes { sha256: String, length: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    pub case_identity: String,
    pub before_run_id: String,
    pub after_run_id: String,
    pub before_target_hash: String,
    pub after_target_hash: String,
    pub observable: Observable,
    pub before: ObservableValue,
    pub after: ObservableValue,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub run_id: String,
    pub result_hash: String,
    pub plan_hash: String,
    pub config_hash: String,
    pub input_identity: String,
    pub initial_snapshot_identity: String,
    pub working_directory: PathBuf,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BehaviorComparisonResult {
    pub comparison_id: String,
    pub schema_version: String,
    pub case_identity: String,
    /// Full approved baseline artifact hash; executable identity is separate.
    pub baseline_identity: String,
    pub baseline_target_identity: String,
    pub candidate_identity: String,
    /// None means no durable acquisition was obtained for that side.
    pub before_run_id: Option<String>,
    pub after_run_id: Option<String>,
    pub before: Option<RunRecord>,
    pub after: Option<RunRecord>,
    pub outcome: BehaviorOutcome,
    pub verdict: Verdict,
    pub reason: String,
    pub divergences: Vec<Divergence>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub experiment: BehaviorExperiment,
    pub experiment_hash: String,
    pub config_hash: String,
    pub authorization_hash: String,
    pub scope: Scope,
    pub coverage: BTreeMap<String, ObservationCoverage>,
    pub replayability: Replayability,
    pub limitations: Vec<String>,
}

/// New P2 schema only. Existing baseline, acquisition and replay schemas stay v1.
pub fn comparison_schema() -> serde_json::Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(BehaviorComparisonResult))
        .expect("statically defined schema serializes");
    schema["$id"] = "https://b2ige.dev/schemas/verify/behavior-comparison-result.v1.json".into();
    schema["title"] = "Behavior Comparison Result v1".into();
    schema["properties"]["schema_version"]["const"] = "1".into();
    schema
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn raw_hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Explicit authoring helper only; does not approve or update a baseline/case.
pub fn snapshot_identity(source: Option<&Path>) -> io::Result<String> {
    Snapshot::capture(source)?.identity()
}

/// Streaming executable-byte identity, shared in meaning with P1C target preflight.
pub fn executable_identity(path: &Path) -> io::Result<String> {
    if !path.is_absolute() {
        return Err(invalid("executable must be absolute"));
    }
    let mut file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("target must be a regular executable file"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

/// Readiness validation reuses the execution contract without executing a target.
pub fn validate_configuration(
    experiment: &BehaviorExperiment,
    authorization: &BehaviorAuthorization,
) -> io::Result<()> {
    validate_contract(experiment, authorization)?;
    supported(experiment, authorization)?;
    verify_target(&experiment.before)?;
    verify_target(&experiment.after)?;
    if snapshot_identity(experiment.case.fixture.source.as_deref())?
        != experiment.case.fixture.snapshot_identity
    {
        return Err(invalid("fixture identity mismatch"));
    }
    Ok(())
}

fn validate_contract(
    experiment: &BehaviorExperiment,
    authorization: &BehaviorAuthorization,
) -> io::Result<()> {
    let case = &experiment.case;
    let baseline = &experiment.baseline;
    if experiment.schema_version != "1"
        || case.schema_version != "1"
        || case.case_id.trim().is_empty()
        || case.timeout_ms == 0
        || case.timeout_ms > MAX_SAFE_INTEGER
        || experiment.seed > MAX_SAFE_INTEGER
        || !valid_hash(&case.fixture.snapshot_identity)
        || baseline.baseline_id.trim().is_empty()
        || baseline
            .stability_runs
            .is_some_and(|n| n == 0 || n > MAX_SAFE_INTEGER)
    {
        return Err(invalid("invalid Behavior v1 experiment/case/baseline"));
    }
    let input = case.input_identity()?;
    for target in [&experiment.before, &experiment.after] {
        if !valid_hash(&target.identity) || !target.executable.is_absolute() {
            return Err(invalid(
                "target requires an absolute executable and raw SHA-256 identity",
            ));
        }
        if target.input_identity != input {
            return Err(invalid("BEFORE/AFTER input identity mismatch"));
        }
    }
    if baseline.target_revision != experiment.before.identity
        || baseline.observation_contract_hash != case.observation_contract_hash()?
        || baseline.approval.status != ApprovalStatus::Approved
        || !present(&baseline.approval.actor)
        || !present(&baseline.approval.reason)
        || !pinned(baseline, &authorization.approved_baselines)
        || !authorization
            .approved_checker_bindings
            .contains(&checker_binding_hash(baseline, &case.checker_claim()?)?)
    {
        return Err(invalid(
            "baseline approval, target identity or checker binding mismatch",
        ));
    }
    Ok(())
}

fn supported(experiment: &BehaviorExperiment, auth: &BehaviorAuthorization) -> io::Result<()> {
    if experiment.case.required_observers != [OBSERVER] {
        return Err(invalid("unsupported required observer: P2 supports cli_process only; network/DB/external state reset and observation are unavailable"));
    }
    if !auth.baseline_stable {
        return Err(invalid(
            "baseline stability is not asserted by trusted policy; P2 performs no noise learning",
        ));
    }
    Ok(())
}

fn verify_target(target: &BehaviorTarget) -> io::Result<()> {
    if executable_identity(&target.executable)? != target.identity {
        return Err(invalid(
            "actual executable SHA-256 differs from pinned target identity",
        ));
    }
    Ok(())
}

fn inputs(
    experiment: &BehaviorExperiment,
    target: &BehaviorTarget,
    cwd: &Path,
    baseline_stable: bool,
) -> io::Result<(ExperimentPlan, ProcessSpec)> {
    let case = &experiment.case;
    let plan = ExperimentPlan {
        plan_id: format!("behavior:{}", case.case_id),
        version: "1".into(),
        target_revision: target.identity.clone(),
        seed: experiment.seed,
        required_observers: vec![OBSERVER.into()],
        optional_observers: vec![],
        // The P2 outer experiment already supplies a fresh snapshot workspace.
        // P1 acquisition itself still performs no reset and no product checking.
        reset_strategy: "none".into(),
        time_budget_ms: case.timeout_ms,
        scope: scope(),
        claims: vec![case.checker_claim()?],
        isolation: IsolationLevel::None,
        product_contract: ProductContract::Behavior {
            baseline: experiment.baseline.clone(),
            // Exact P2 passes its checked external assertion. P3A passes false:
            // acquisition is evidence, never an assertion of measured stability.
            stable: baseline_stable,
        },
    };
    let spec = ProcessSpec {
        executable: target.executable.clone(),
        args: case.args.clone(),
        working_directory: cwd.to_owned(),
        environment: case.environment.clone(),
        timeout_ms: case.timeout_ms,
    };
    validate_inputs(&plan, &spec)?;
    Ok((plan, spec))
}

fn scope() -> Scope {
    Scope {
        contract_ids: vec![CLAIM.into()],
        exploration_budget: BTreeMap::from([("explicit_cases".into(), 1)]),
    }
}

fn record(run: &AcquisitionResult, case: &BehaviorCase) -> io::Result<RunRecord> {
    Ok(RunRecord {
        run_id: run.context.run_id.clone(),
        result_hash: canonical_hash(run)?,
        plan_hash: run.context.plan_hash.clone(),
        config_hash: run.context.config_hash.clone(),
        input_identity: case.input_identity()?,
        initial_snapshot_identity: case.fixture.snapshot_identity.clone(),
        working_directory: run.process.working_directory.clone(),
        evidence_refs: run
            .bundle
            .evidence_hashes
            .iter()
            .map(|(id, hash)| EvidenceRef {
                run_id: run.context.run_id.clone(),
                evidence_id: id.clone(),
                evidence_hash: hash.clone(),
            })
            .collect(),
    })
}

fn load_run(
    store: &EvidenceStore,
    experiment: &BehaviorExperiment,
    target: &BehaviorTarget,
    link: &RunRecord,
    baseline_stable: bool,
) -> io::Result<AcquisitionResult> {
    let (plan, spec) = inputs(experiment, target, &link.working_directory, baseline_stable)?;
    load_acquisition(store, &plan, &spec, &experiment.case, link)
}

fn load_acquisition(
    store: &EvidenceStore,
    plan: &ExperimentPlan,
    spec: &ProcessSpec,
    case: &BehaviorCase,
    link: &RunRecord,
) -> io::Result<AcquisitionResult> {
    let (value, evidence) = store.load(&link.run_id)?;
    let run: AcquisitionResult = serde_json::from_value(value)?;
    let (expected, expected_evidence) =
        assemble(plan, spec, &link.run_id, run.observation.clone())?;
    if !run.context.valid()
        || run != expected
        || evidence != [expected_evidence]
        || record(&run, case)? != *link
    {
        return Err(invalid(
            "corrupt or mismatched acquisition/evidence/input linkage",
        ));
    }
    Ok(run)
}

fn readiness(observation: &ProcessObservation) -> BehaviorOutcome {
    if let Some(failure) = &observation.runner_failure {
        // P1 v1 records these two bounded capture conditions as runner failures.
        // Keep that artifact unchanged; only this product outcome is inconclusive.
        return if observation.started
            && matches!(
                failure.as_str(),
                "capture limit exceeded; observation incomplete"
                    | "capture did not close after timeout cleanup"
            ) {
            BehaviorOutcome::Inconclusive
        } else {
            BehaviorOutcome::Error
        };
    }
    if !observation.started
        || observation.timed_out
        || (observation.exit_code.is_some() == observation.signal.is_some())
        || observation
            .exit_code
            .is_some_and(|code| !(0..=255).contains(&code))
        || observation.signal.is_some_and(|signal| signal <= 0)
        || observation.stdout.len() > CAPTURE_LIMIT
        || observation.stderr.len() > CAPTURE_LIMIT
    {
        BehaviorOutcome::Inconclusive
    } else {
        BehaviorOutcome::NoDivergenceFound
    }
}

fn decision(
    report: &BehaviorComparisonResult,
    before: &AcquisitionResult,
    after: &AcquisitionResult,
) -> (BehaviorOutcome, Vec<Divergence>) {
    let readiness = [
        readiness(&before.observation),
        readiness(&after.observation),
    ];
    for outcome in [BehaviorOutcome::Error, BehaviorOutcome::Inconclusive] {
        if readiness.contains(&outcome) {
            return (outcome, vec![]);
        }
    }
    let a = &before.observation;
    let b = &after.observation;
    let mut differences = Vec::new();
    let mut push = |observable, before, after| {
        differences.push(Divergence {
            case_identity: report.case_identity.clone(),
            before_run_id: report.before_run_id.clone().expect("validated BEFORE"),
            after_run_id: report.after_run_id.clone().expect("validated AFTER"),
            before_target_hash: report.baseline_target_identity.clone(),
            after_target_hash: report.candidate_identity.clone(),
            observable,
            before,
            after,
            evidence_refs: report.evidence_refs.clone(),
        });
    };
    for (observable, a, b) in [
        (Observable::ExitCode, a.exit_code, b.exit_code),
        (Observable::Signal, a.signal, b.signal),
    ] {
        if a != b {
            push(
                observable,
                ObservableValue::Status { value: a },
                ObservableValue::Status { value: b },
            );
        }
    }
    for (observable, a, b) in [
        (Observable::Stdout, &a.stdout, &b.stdout),
        (Observable::Stderr, &a.stderr, &b.stderr),
    ] {
        if a != b {
            push(
                observable,
                ObservableValue::Bytes {
                    sha256: raw_hash(a),
                    length: a.len(),
                },
                ObservableValue::Bytes {
                    sha256: raw_hash(b),
                    length: b.len(),
                },
            );
        }
    }
    (
        if differences.is_empty() {
            BehaviorOutcome::NoDivergenceFound
        } else {
            BehaviorOutcome::DivergenceProven
        },
        differences,
    )
}

fn comparison_evidence(report: &BehaviorComparisonResult) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(report)?,
    };
    Ok(Evidence {
        evidence_id: "behavior-comparison".into(),
        run_id: report.comparison_id.clone(),
        source: CLAIM.into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec![CLAIM.into()],
    })
}

fn set_outcome(report: &mut BehaviorComparisonResult, outcome: BehaviorOutcome, reason: String) {
    report.outcome = outcome;
    report.verdict = outcome.verdict();
    report.reason = reason;
}

/// Runs exactly one explicit case twice through P1 acquisition. Preflight and
/// acquisition errors become durable ERROR reports when storage is writable.
/// An io::Err means ERROR with no promised comparison commit (e.g. occupied ID).
pub fn execute(
    store: &EvidenceStore,
    workspace_root: &Path,
    comparison_id: &str,
    experiment: &BehaviorExperiment,
    authorization: &BehaviorAuthorization,
) -> io::Result<BehaviorComparisonResult> {
    // Store IDs have a 128-byte bound; derived BEFORE/AFTER IDs must fit too.
    if comparison_id.len() > 121 {
        return Err(invalid("comparison_id exceeds derived run ID limit"));
    }
    let committed = store.reserve(comparison_id)?;
    let mut report = BehaviorComparisonResult {
        comparison_id: comparison_id.into(), schema_version: "1".into(),
        case_identity: experiment.case.identity()?,
        baseline_identity: canonical_hash(&experiment.baseline)?,
        baseline_target_identity: experiment.before.identity.clone(),
        candidate_identity: experiment.after.identity.clone(),
        before_run_id: None, after_run_id: None, before: None, after: None,
        outcome: BehaviorOutcome::Error, verdict: Verdict::Error, reason: String::new(),
        divergences: vec![], evidence_refs: vec![],
        experiment: experiment.clone(), experiment_hash: canonical_hash(experiment)?,
        config_hash: canonical_hash(&experiment.case)?, authorization_hash: canonical_hash(authorization)?,
        scope: scope(), coverage: BTreeMap::new(),
        replayability: Replayability::Unavailable {
            reason: "P2 has no paired snapshot replay executor. Rerun execute with the recorded explicit case, approved baseline, matching executable/fixture hashes and a fresh comparison_id; P1C alone does not reset these workspaces.".into(),
        },
        limitations: vec![
            "NO DIVERGENCE FOUND within tested behavior space: one explicit case, one execution per target; no equivalence or exhaustive proof.".into(),
            "Exact exit/signal and raw stream bytes only; no normalization, stability learning, generation or reduction.".into(),
            "Local fixture paths, bytes, directories and permission bits reset from one snapshot; access/modified times fixed to Unix epoch. Absolute cwd, inode, ctime and host/runtime dependencies are not identical.".into(),
            "No network, DB or external service observation/reset; stdin is null; streams retain P1's 1 MiB limit.".into(),
            "Trusted local Unix targets/store/approval policy; no isolation, secret protection, library pinning or hostile same-user hash-to-spawn replacement defense. Explicit environment is stored in clear text.".into(),
        ],
    };
    let result = (|| -> io::Result<()> {
        validate_contract(experiment, authorization)?;
        if let Err(error) = supported(experiment, authorization) {
            set_outcome(
                &mut report,
                BehaviorOutcome::Inconclusive,
                error.to_string(),
            );
            return Ok(());
        }
        // Verify BOTH targets and the single frozen input before either spawn.
        verify_target(&experiment.before)?;
        verify_target(&experiment.after)?;
        let snapshot = Snapshot::capture(experiment.case.fixture.source.as_deref())?;
        if snapshot.identity()? != experiment.case.fixture.snapshot_identity {
            return Err(invalid("actual fixture snapshot/input identity mismatch"));
        }
        let work = workspace::reserve(workspace_root, comparison_id)?;
        for (side, target) in [("before", &experiment.before), ("after", &experiment.after)] {
            let cwd = work.join(side);
            // AFTER is created after BEFORE exits, from the original in-memory
            // snapshot, never from BEFORE's cwd or a reread of mutable source.
            snapshot.materialize(&cwd)?;
            verify_target(target)?;
            let (plan, spec) = inputs(experiment, target, &cwd, true)?;
            let run = acquire(&plan, &spec, store, &format!("{comparison_id}-{side}"))?;
            let link = record(&run, &experiment.case)?;
            report
                .coverage
                .insert(side.into(), run.bundle.coverage[OBSERVER].status);
            report.evidence_refs.extend(link.evidence_refs.clone());
            if side == "before" {
                report.before_run_id = Some(link.run_id.clone());
                report.before = Some(link);
            } else {
                report.after_run_id = Some(link.run_id.clone());
                report.after = Some(link);
            }
        }
        let before = load_run(
            store,
            experiment,
            &experiment.before,
            report
                .before
                .as_ref()
                .ok_or_else(|| invalid("missing BEFORE"))?,
            true,
        )?;
        let after = load_run(
            store,
            experiment,
            &experiment.after,
            report
                .after
                .as_ref()
                .ok_or_else(|| invalid("missing AFTER"))?,
            true,
        )?;
        let (outcome, differences) = decision(&report, &before, &after);
        report.divergences = differences;
        let reason = match outcome {
            BehaviorOutcome::NoDivergenceFound => "NO DIVERGENCE FOUND within tested behavior space; both process observations are complete.",
            BehaviorOutcome::DivergenceProven => "DIVERGENCE PROVEN by byte-exact comparison of two complete process observations.",
            BehaviorOutcome::Inconclusive => "Required process observation incomplete or timed out; comparison withheld.",
            BehaviorOutcome::Error => "Runner/acquisition failed; comparison withheld. Inspect referenced P1 observations for the failure reason.",
        };
        set_outcome(&mut report, outcome, reason.into());
        Ok(())
    })();
    if let Err(error) = result {
        report.divergences.clear();
        set_outcome(
            &mut report,
            BehaviorOutcome::Error,
            format!("Behavior experiment failed: {error}"),
        );
    }
    // The existing Store rereads this derived receipt and atomically publishes
    // result.json without replacing any acquisition, evidence or comparison.
    let evidence = comparison_evidence(&report)?;
    committed.write_evidence(&evidence)?;
    committed.complete(&report, &[evidence.evidence_id])?;
    Ok(report)
}

/// Read-only integrity/linkage revalidation, not replay or authentication. A
/// positive result is returned only while BOTH saved acquisitions still verify.
pub fn load(
    store: &EvidenceStore,
    comparison_id: &str,
    authorization: &BehaviorAuthorization,
) -> io::Result<BehaviorComparisonResult> {
    let (value, evidence) = store.load(comparison_id)?;
    let report: BehaviorComparisonResult = serde_json::from_value(value)?;
    if report.schema_version != "1"
        || report.comparison_id != comparison_id
        || report.case_identity != report.experiment.case.identity()?
        || report.baseline_identity != canonical_hash(&report.experiment.baseline)?
        || report.baseline_target_identity != report.experiment.before.identity
        || report.candidate_identity != report.experiment.after.identity
        || report.experiment_hash != canonical_hash(&report.experiment)?
        || report.config_hash != canonical_hash(&report.experiment.case)?
        || report.authorization_hash != canonical_hash(authorization)?
        || report.verdict != report.outcome.verdict()
        || report.scope != scope()
        || evidence != [comparison_evidence(&report)?]
    {
        return Err(invalid("invalid Behavior comparison identity/hash/receipt"));
    }
    let mut runs = vec![];
    let mut refs = vec![];
    let mut coverage = BTreeMap::new();
    for (side, target, run_id, link) in [
        (
            "before",
            &report.experiment.before,
            &report.before_run_id,
            &report.before,
        ),
        (
            "after",
            &report.experiment.after,
            &report.after_run_id,
            &report.after,
        ),
    ] {
        if run_id.as_ref() != link.as_ref().map(|link| &link.run_id) {
            return Err(invalid("missing or inconsistent comparison run link"));
        }
        if let Some(link) = link {
            if link.run_id != format!("{comparison_id}-{side}")
                || link.working_directory.file_name().and_then(|s| s.to_str()) != Some(side)
                || link
                    .working_directory
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|s| s.to_str())
                    != Some(comparison_id)
            {
                return Err(invalid(
                    "comparison requires distinct run IDs and fresh workspaces",
                ));
            }
            let run = load_run(store, &report.experiment, target, link, true)?;
            refs.extend(link.evidence_refs.clone());
            coverage.insert(side.into(), run.bundle.coverage[OBSERVER].status);
            runs.push(run);
        }
    }
    if refs != report.evidence_refs || coverage != report.coverage {
        return Err(invalid(
            "comparison evidence references/coverage differ from acquisitions",
        ));
    }
    if matches!(
        report.outcome,
        BehaviorOutcome::DivergenceProven | BehaviorOutcome::NoDivergenceFound
    ) {
        validate_contract(&report.experiment, authorization)?;
        supported(&report.experiment, authorization)?;
        if runs.len() != 2 {
            return Err(invalid(
                "both acquisitions are required for a completed comparison",
            ));
        }
        let (outcome, divergences) = decision(&report, &runs[0], &runs[1]);
        if outcome != report.outcome || divergences != report.divergences {
            return Err(invalid(
                "stored outcome/divergences are not supported by actual evidence",
            ));
        }
    } else if !report.divergences.is_empty() {
        return Err(invalid(
            "incomplete/error comparison cannot claim a divergence",
        ));
    }
    Ok(report)
}
