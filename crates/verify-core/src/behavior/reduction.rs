//! Deterministic deletion of explicit P3B assignments, verified by fresh P2 pairs.
use super::*;
use generation::{
    Assignment, ComparisonRef, DimensionTarget, GeneratedCaseIdentity, GenerationSpec,
};

pub const ALGORITHM: &str = "behavior.assignment-deletion.v1";

/// Exact ordered observable set, target pins, baseline and process semantics.
/// Stream bytes and status values may change; observable kinds may not change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DivergenceSignature {
    pub observables: Vec<Observable>,
    pub baseline_identity: String,
    pub baseline_target_identity: String,
    pub candidate_identity: String,
    pub policy: ComparisonPolicy,
}

pub fn signature(report: &BehaviorComparisonResult) -> io::Result<DivergenceSignature> {
    if report.outcome != BehaviorOutcome::DivergenceProven || report.divergences.is_empty() {
        return Err(invalid("reduction requires DIVERGENCE_PROVEN"));
    }
    Ok(DivergenceSignature {
        observables: report
            .divergences
            .iter()
            .map(|d| d.observable.clone())
            .collect(),
        baseline_identity: report.baseline_identity.clone(),
        baseline_target_identity: report.baseline_target_identity.clone(),
        candidate_identity: report.candidate_identity.clone(),
        policy: report.experiment.case.comparison_policy.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReductionInput {
    pub source_suite_id: String,
    pub source_suite_hash: String,
    pub source_comparison: ComparisonRef,
    pub failing_case: GeneratedCaseIdentity,
    /// Contains the original base case and its immutable execution contract.
    pub generation_spec: GenerationSpec,
    pub expected_signature: DivergenceSignature,
    /// Maximum candidate P2 pair invocations (at most two process starts each).
    pub execution_budget: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReductionStatus {
    Minimized,
    Unreducible,
    BudgetExhausted,
    Inconclusive,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub assignment: Vec<Assignment>,
    pub comparison: ComparisonRef,
    pub outcome: BehaviorOutcome,
    pub signature: Option<DivergenceSignature>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CounterexampleReductionResult {
    pub reduction_id: String,
    pub schema_version: String,
    pub algorithm_version: String,
    pub input: ReductionInput,
    pub authorization_hash: String,
    pub original_assignment: Vec<Assignment>,
    pub reduced_assignment: Vec<Assignment>,
    pub original_signature: DivergenceSignature,
    pub final_signature: DivergenceSignature,
    pub attempted_candidates: Vec<Attempt>,
    pub accepted_reductions: u64,
    pub execution_count: u64,
    pub status: ReductionStatus,
    pub minimal: bool,
    pub minimality_claim: String,
    pub final_comparison: ComparisonRef,
    pub final_before: Option<RunRecord>,
    pub final_after: Option<RunRecord>,
    pub final_evidence_refs: Vec<EvidenceRef>,
    pub trace_hash: String,
    pub limitations: Vec<String>,
}

pub fn result_schema() -> serde_json::Value {
    let mut value = serde_json::to_value(schemars::schema_for!(CounterexampleReductionResult))
        .expect("static schema");
    value["$id"] =
        "https://b2ige.dev/schemas/verify/counterexample-reduction-result.v1.json".into();
    value["properties"]["schema_version"]["const"] = "1".into();
    value
}

fn reference(report: &BehaviorComparisonResult) -> io::Result<ComparisonRef> {
    Ok(ComparisonRef {
        comparison_id: report.comparison_id.clone(),
        comparison_hash: canonical_hash(report)?,
    })
}

fn source(
    store: &EvidenceStore,
    input: &ReductionInput,
    auth: &BehaviorAuthorization,
) -> io::Result<BehaviorComparisonResult> {
    if input.execution_budget > MAX_SAFE_INTEGER {
        return Err(invalid("invalid reduction budget"));
    }
    let suite = generation::load(store, &input.source_suite_id, auth)?;
    let index = suite
        .generated_cases
        .iter()
        .position(|c| c == &input.failing_case)
        .ok_or_else(|| invalid("source generated case mismatch"))?;
    if canonical_hash(&suite)? != input.source_suite_hash
        || suite.generation_spec != input.generation_spec
        || suite.comparisons[index] != input.source_comparison
    {
        return Err(invalid("source suite/spec/comparison mismatch"));
    }
    let report = super::load(store, &input.source_comparison.comparison_id, auth)?;
    if reference(&report)? != input.source_comparison
        || signature(&report)? != input.expected_signature
    {
        return Err(invalid("source failure signature mismatch"));
    }
    Ok(report)
}

fn experiment(input: &ReductionInput, assignment: &[Assignment]) -> io::Result<BehaviorExperiment> {
    let mut experiment = input.generation_spec.base.clone();
    for a in assignment {
        match &a.target {
            DimensionTarget::Argument { index } => {
                experiment.case.args[*index as usize] = a.value.clone()
            }
            DimensionTarget::Environment { name } => {
                experiment
                    .case
                    .environment
                    .insert(name.clone(), a.value.clone());
            }
        }
    }
    let identity = experiment.case.input_identity()?;
    experiment.case.case_id = format!(
        "reduced-{}",
        canonical_hash(&(ALGORITHM, &input.failing_case, assignment, &identity))?
            .trim_start_matches("sha256:")
    );
    experiment.before.input_identity = identity.clone();
    experiment.after.input_identity = identity;
    Ok(experiment)
}

fn candidate_id(id: &str, ordinal: usize, experiment: &BehaviorExperiment) -> io::Result<String> {
    Ok(format!(
        "reduction-comparison-{}",
        canonical_hash(&(ALGORITHM, id, ordinal, experiment))?.trim_start_matches("sha256:")
    ))
}

fn verified_child(
    store: &EvidenceStore,
    id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<BehaviorComparisonResult> {
    let child = super::load(store, id, auth)?;
    if let (Some(before), Some(after)) = (&child.before, &child.after) {
        let a = load_run(
            store,
            &child.experiment,
            &child.experiment.before,
            before,
            true,
        )?;
        let b = load_run(
            store,
            &child.experiment,
            &child.experiment.after,
            after,
            true,
        )?;
        if decision(&child, &a, &b) != (child.outcome, child.divergences.clone()) {
            return Err(invalid(
                "reduction outcome unsupported by actual acquisitions",
            ));
        }
    }
    Ok(child)
}

// One state machine for execution and verified trace reconstruction. Restart at
// the first dimension after acceptance: previously rejected deletions may work now.
fn run(
    store: &EvidenceStore,
    id: &str,
    input: &ReductionInput,
    auth: &BehaviorAuthorization,
    workspace: Option<&Path>,
) -> io::Result<CounterexampleReductionResult> {
    let mut final_report = source(store, input, auth)?;
    let mut assignment = input.failing_case.assignment.clone();
    assignment.sort_by(|a, b| a.target.cmp(&b.target));
    let original = assignment.clone();
    let mut attempts = Vec::new();
    let mut accepted = 0;
    let mut cursor = 0;
    let mut uncertain = false;
    let mut error = false;
    while cursor < assignment.len() && (attempts.len() as u64) < input.execution_budget {
        let mut candidate = assignment.clone();
        candidate.remove(cursor);
        let experiment = experiment(input, &candidate)?;
        let child_id = candidate_id(id, attempts.len(), &experiment)?;
        if let Some(workspace) = workspace {
            super::execute(store, workspace, &child_id, &experiment, auth)?;
        }
        let child = verified_child(store, &child_id, auth)?;
        if child.experiment != experiment {
            return Err(invalid("reduction experiment mismatch"));
        }
        let sig = signature(&child).ok();
        let keep = sig.as_ref() == Some(&input.expected_signature);
        attempts.push(Attempt {
            assignment: candidate.clone(),
            comparison: reference(&child)?,
            outcome: child.outcome,
            signature: sig,
            accepted: keep,
        });
        if keep {
            assignment = candidate;
            final_report = child;
            accepted += 1;
            cursor = 0;
            uncertain = false;
            error = false;
        } else {
            uncertain |= child.outcome == BehaviorOutcome::Inconclusive;
            error |= child.outcome == BehaviorOutcome::Error;
            cursor += 1;
        }
    }
    let status = if cursor < assignment.len() {
        ReductionStatus::BudgetExhausted
    } else if error {
        ReductionStatus::Error
    } else if uncertain {
        ReductionStatus::Inconclusive
    } else if accepted == 0 {
        ReductionStatus::Unreducible
    } else {
        ReductionStatus::Minimized
    };
    let minimal = matches!(
        status,
        ReductionStatus::Minimized | ReductionStatus::Unreducible
    );
    Ok(CounterexampleReductionResult {
        reduction_id: id.into(), schema_version: "1".into(), algorithm_version: ALGORITHM.into(), input: input.clone(), authorization_hash: canonical_hash(auth)?,
        original_assignment: original, reduced_assignment: assignment, original_signature: input.expected_signature.clone(), final_signature: signature(&final_report)?,
        accepted_reductions: accepted, execution_count: attempts.len() as u64, status, minimal,
        minimality_claim: if minimal { "LOCALLY MINIMIZED COUNTEREXAMPLE: every single assignment deletion tested without compatible divergence (1-minimal within recorded executions)." } else { "No minimality claim; best proven counterexample retained." }.into(),
        final_comparison: reference(&final_report)?, final_before: final_report.before, final_after: final_report.after, final_evidence_refs: final_report.evidence_refs,
        trace_hash: canonical_hash(&attempts)?, attempted_candidates: attempts,
        limitations: vec!["Bounded explicit assignment deletion only; not globally minimal or exhaustive proof. Deterministic order/result assumes stable target behavior and execution environment.".into(), "Signature matches the exact observable-kind set, baseline/target pins and process byte-exact policy; values/stream bytes need not match the original.".into(), "Budget counts candidate P2 pair invocations, including ERROR/INCONCLUSIVE; at most twice that many process starts. Source validation uses no executions. Storage/integrity failure returns Err without a completed reduction artifact.".into(), "Trusted local Unix; hashes are not authentication. Existing P2 stability, host, stream and replay limitations remain. Unchanged best result explicitly references the original; accepted reductions always reference fresh acquisitions.".into()],
    })
}

fn receipt(report: &CounterexampleReductionResult) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(report)?,
    };
    Ok(Evidence {
        evidence_id: "counterexample-reduction".into(),
        run_id: report.reduction_id.clone(),
        source: ALGORITHM.into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec![ALGORITHM.into()],
    })
}

pub fn execute(
    store: &EvidenceStore,
    workspace_root: &Path,
    reduction_id: &str,
    input: &ReductionInput,
    auth: &BehaviorAuthorization,
) -> io::Result<CounterexampleReductionResult> {
    source(store, input, auth)?;
    let reserved = store.reserve(reduction_id)?;
    let report = run(store, reduction_id, input, auth, Some(workspace_root))?;
    // Re-read source and the entire trace after the last target has run.
    if report != run(store, reduction_id, input, auth, None)? {
        return Err(invalid("reduction changed during execution"));
    }
    let evidence = receipt(&report)?;
    reserved.write_evidence(&evidence)?;
    reserved.complete(&report, &[evidence.evidence_id])?;
    Ok(report)
}

pub fn load(
    store: &EvidenceStore,
    reduction_id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<CounterexampleReductionResult> {
    let (value, evidence) = store.load(reduction_id)?;
    let saved: CounterexampleReductionResult = serde_json::from_value(value)?;
    let expected = run(store, reduction_id, &saved.input, auth, None)?;
    if saved != expected || evidence != [receipt(&expected)?] {
        return Err(invalid(
            "reduction identity/trace/minimality/evidence mismatch",
        ));
    }
    Ok(expected)
}
