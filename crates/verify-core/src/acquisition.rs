//! P1B acquisition only. The existing plan is recorded without executing its
//! product checker. No PASS/FAIL is inferred from process exit.
pub mod replay;
use crate::{ExperimentPlan, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use verify_evidence::{
    canonical_hash, store::EvidenceStore, Coverage, Evidence, Observation, ObservationCoverage,
    TrustClass, MAX_SAFE_INTEGER,
};
use verify_replay::Replayability;
use verify_runner::{
    process::{observe, ProcessObservation, ProcessSpec, OBSERVER},
    ExecutionStatus, IsolationLevel, RunContext,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Created,
    Running,
    Observed,
    Completed,
    RunnerFailed,
}

/// Separate acquisition envelope v1; existing P0/P1A schemas are unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    pub bundle_schema_version: String,
    pub run_id: String,
    pub evidence_refs: Vec<String>,
    pub claim_refs: Vec<String>,
    pub coverage: BTreeMap<String, Coverage>,
    pub counterexamples: Vec<crate::Counterexample>,
    pub replayability: Replayability,
    pub replay_instructions: String,
    pub evidence_hashes: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionResult {
    pub acquisition_schema_version: String,
    pub context: RunContext,
    pub plan: ExperimentPlan,
    pub process: ProcessSpec,
    pub lifecycle: Vec<RunState>,
    pub execution: ExecutionStatus,
    pub verdict: Verdict,
    pub observation: ProcessObservation,
    pub bundle: EvidenceBundle,
    pub limitations: Vec<String>,
}

/// Reserve before executing so a duplicate run ID never reruns a side-effecting
/// target. Store errors are returned as errors; no completed artifact is promised.
pub fn acquire(
    plan: &ExperimentPlan,
    spec: &ProcessSpec,
    store: &EvidenceStore,
    run_id: &str,
) -> io::Result<AcquisitionResult> {
    acquire_with_completion(plan, spec, store, run_id, |result, run| {
        run.complete(result, &result.bundle.evidence_refs)
    })
}

pub(crate) fn validate_inputs(plan: &ExperimentPlan, spec: &ProcessSpec) -> io::Result<()> {
    spec.validate().map_err(io::Error::other)?;
    if plan.version != "1"
        || plan.plan_id.trim().is_empty()
        || plan.target_revision.trim().is_empty()
        || plan.seed > MAX_SAFE_INTEGER
        || plan.time_budget_ms > MAX_SAFE_INTEGER
        || spec.timeout_ms != plan.time_budget_ms
        || plan.isolation != IsolationLevel::None
        || plan.reset_strategy != "none"
        || plan.required_observers != [OBSERVER]
        || !plan.optional_observers.is_empty()
        || plan
            .scope
            .exploration_budget
            .values()
            .any(|v| *v > MAX_SAFE_INTEGER)
    {
        return Err(io::Error::other("unsupported P1B plan: requires v1, cli_process only, no reset/isolation, matching timeout"));
    }
    Ok(())
}

// Shared acquisition path: replay may change only the result envelope at commit.
// Neither caller can inject an observation or bypass the actual process observer.
fn acquire_with_completion(
    plan: &ExperimentPlan,
    spec: &ProcessSpec,
    store: &EvidenceStore,
    run_id: &str,
    complete: impl FnOnce(&AcquisitionResult, &verify_evidence::store::RunStore) -> io::Result<()>,
) -> io::Result<AcquisitionResult> {
    validate_inputs(plan, spec)?;
    let run = store.reserve(run_id)?;
    let (result, evidence) = assemble(plan, spec, run_id, observe(spec))?;
    run.write_evidence(&evidence)?;
    complete(&result, &run)?;
    Ok(result)
}

pub(crate) fn assemble(
    plan: &ExperimentPlan,
    spec: &ProcessSpec,
    run_id: &str,
    observation: ProcessObservation,
) -> io::Result<(AcquisitionResult, Evidence)> {
    let plan_hash = canonical_hash(plan)?;
    let config_hash = canonical_hash(spec)?;
    let failure = observation.runner_failure.is_some();
    let mut lifecycle = vec![RunState::Created];
    if observation.started {
        lifecycle.push(RunState::Running);
    }
    lifecycle.push(if failure {
        RunState::RunnerFailed
    } else {
        RunState::Observed
    });
    // These are acquisition claims, never the plan's unexecuted product claims.
    let claim_refs = vec!["process.acquisition".into()];
    let payload = Observation::Value {
        value: serde_json::to_value(&observation)?,
    };
    let integrity_hash = canonical_hash(&payload)?;
    let evidence = Evidence {
        evidence_id: "process-observation".into(),
        run_id: run_id.into(),
        source: OBSERVER.into(),
        trust_class: TrustClass::DirectRuntime,
        order: 0,
        observation: payload,
        integrity_hash,
        related_claim_ids: claim_refs.clone(),
    };
    let coverage = BTreeMap::from([(
        OBSERVER.into(),
        Coverage {
            status: if failure {
                ObservationCoverage::Failed
            } else if observation.timed_out {
                ObservationCoverage::Partial
            } else {
                ObservationCoverage::Complete
            },
            verdict_critical: true,
            reason: if failure {
                "runner or capture failure; output may be incomplete"
            } else if observation.timed_out {
                "deadline expired; captured bytes are only the observed prefix"
            } else {
                "leader exit and both stream EOFs observed; no cross-stream order guarantee"
            }
            .into(),
        },
    )]);
    lifecycle.push(RunState::Completed);
    let result = AcquisitionResult {
        acquisition_schema_version: "1".into(),
        context: RunContext {
            run_id: run_id.into(), schema_version: "1".into(), tool_version: env!("CARGO_PKG_VERSION").into(),
            seed: plan.seed, target_revision: plan.target_revision.clone(), config_hash,
            plan_hash: plan_hash.clone(), experiment_hash: plan_hash,
            platform_runtime: format!("{}-{} / std::process", std::env::consts::ARCH, std::env::consts::OS),
            observer_versions: BTreeMap::from([(OBSERVER.into(), "1".into())]),
            started_at: observation.started_at.clone(), ended_at: observation.ended_at.clone(),
        },
        plan: plan.clone(), process: spec.clone(), lifecycle,
        execution: if failure { ExecutionStatus::RunnerCrash } else { ExecutionStatus::Completed },
        verdict: if failure { Verdict::Error } else { Verdict::Inconclusive },
        observation,
        bundle: EvidenceBundle {
            bundle_schema_version: "1".into(), run_id: run_id.into(), evidence_refs: vec![evidence.evidence_id.clone()], claim_refs, coverage,
            counterexamples: vec![], replayability: Replayability::Unavailable { reason: "P1B records inputs but does not verify target identity or execute replay".into() },
            replay_instructions: "Recorded plan and process config are acquisition inputs only; external state and executable bytes are not pinned by this runner.".into(),
            evidence_hashes: BTreeMap::from([(evidence.evidence_id.clone(), canonical_hash(&evidence)?)]),
        },
        limitations: vec!["No product checker executed; acquisition completion is not PASS.".into(), "Unix local trusted targets only; process-group cleanup is not isolation.".into(), "Streams capped at 1 MiB each; overflow is runner failure. Environment is explicit and stored in clear text.".into()],
    };
    Ok((result, evidence))
}
