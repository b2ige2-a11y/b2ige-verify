//! P3B explicit, bounded input enumeration over the unchanged P2 exact engine.
//! Vector order is semantic; no sorting, randomness, inference or deduplication.
use super::*;

const GENERATOR: &str = "behavior.process.generation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationMode {
    OneAtATime,
    Cartesian,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DimensionTarget {
    Argument { index: u64 },
    Environment { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    pub target: DimensionTarget,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationSpec {
    pub generation_id: String,
    pub schema_version: String,
    /// Includes the approved baseline and immutable execution/observation contract.
    pub base: BehaviorExperiment,
    pub mode: GenerationMode,
    pub dimensions: Vec<Dimension>,
    pub max_cases: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub dimension_index: u64,
    pub candidate_index: u64,
    pub target: DimensionTarget,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedCaseIdentity {
    pub generated_case_id: String,
    pub parent_case_identity: String,
    pub generation_spec_hash: String,
    pub assignment: Vec<Assignment>,
    pub input_identity: String,
    pub case_identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedCase {
    pub identity: GeneratedCaseIdentity,
    pub experiment: BehaviorExperiment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRef {
    pub comparison_id: String,
    pub comparison_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeCounts {
    pub no_divergence_found: u64,
    pub divergence_proven: u64,
    pub inconclusive: u64,
    pub error: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationCoverage {
    pub expected_cases: u64,
    pub verified_comparisons: u64,
    pub completed_pairs: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BehaviorGeneratedSuiteResult {
    pub suite_id: String,
    pub schema_version: String,
    pub generation_spec: GenerationSpec,
    pub generation_spec_hash: String,
    pub generated_cases: Vec<GeneratedCaseIdentity>,
    pub comparisons: Vec<ComparisonRef>,
    pub authorization_hash: String,
    pub outcome_counts: OutcomeCounts,
    pub aggregate_verdict: Verdict,
    pub generation_budget: u64,
    pub coverage: GenerationCoverage,
    pub limitations: Vec<String>,
}

pub fn spec_schema() -> serde_json::Value {
    schema::<GenerationSpec>("generation-spec")
}
pub fn suite_schema() -> serde_json::Value {
    schema::<BehaviorGeneratedSuiteResult>("behavior-generated-suite-result")
}
fn schema<T: JsonSchema>(name: &str) -> serde_json::Value {
    let mut value = serde_json::to_value(schemars::schema_for!(T)).expect("static schema");
    value["$id"] = format!("https://b2ige.dev/schemas/verify/{name}.v1.json").into();
    value["properties"]["schema_version"]["const"] = "1".into();
    value
}

fn expected_count(spec: &GenerationSpec) -> io::Result<u64> {
    if spec.schema_version != "1"
        || spec.generation_id.trim().is_empty()
        || spec.dimensions.is_empty()
        || spec.max_cases == 0
        || spec.max_cases > MAX_SAFE_INTEGER
    {
        return Err(invalid("invalid or empty GenerationSpec v1"));
    }
    let mut targets = BTreeSet::new();
    let mut count = if spec.mode == GenerationMode::Cartesian {
        1u64
    } else {
        0u64
    };
    for dimension in &spec.dimensions {
        if !targets.insert(&dimension.target) || dimension.candidates.is_empty() {
            return Err(invalid("duplicate dimension target or empty dimension"));
        }
        match &dimension.target {
            DimensionTarget::Argument { index } if *index >= spec.base.case.args.len() as u64 => {
                return Err(invalid("invalid argument index"))
            }
            DimensionTarget::Environment { name }
                if name.is_empty() || name.contains(['=', '\0']) =>
            {
                return Err(invalid("invalid environment variable name"))
            }
            _ => {}
        }
        let mut values = BTreeSet::new();
        for value in &dimension.candidates {
            if value.contains('\0') || !values.insert(value) {
                return Err(invalid("invalid or duplicate candidate"));
            }
        }
        let size = dimension.candidates.len() as u64;
        count = match spec.mode {
            GenerationMode::OneAtATime => count.checked_add(size),
            GenerationMode::Cartesian => count.checked_mul(size),
        }
        .ok_or_else(|| invalid("generation count overflow"))?;
        if count > spec.max_cases {
            return Err(invalid("generation exceeds max_cases; no cases executed"));
        }
    }
    Ok(count)
}

/// Pure deterministic generation. Base is not implicitly included. Candidate equal
/// to base is allowed, but duplicate resulting inputs reject the entire generation.
/// Cartesian uses dimension order with the last dimension varying fastest.
pub fn generate(spec: &GenerationSpec) -> io::Result<Vec<GeneratedCase>> {
    let count = expected_count(spec)?;
    let count = usize::try_from(count).map_err(|_| invalid("case count unsupported on host"))?;
    let spec_hash = canonical_hash(spec)?;
    let parent = spec.base.case.identity()?;
    let mut generated = Vec::new();
    generated
        .try_reserve(count)
        .map_err(|_| invalid("generation allocation budget unavailable"))?;
    let mut identities = BTreeSet::new();
    let mut inputs_seen = BTreeSet::new();
    for ordinal in 0..count {
        let mut assignment = Vec::new();
        let mut remaining = ordinal;
        match spec.mode {
            GenerationMode::OneAtATime => {
                for (index, dimension) in spec.dimensions.iter().enumerate() {
                    if remaining < dimension.candidates.len() {
                        assignment.push(assign(index, remaining, dimension));
                        break;
                    }
                    remaining -= dimension.candidates.len();
                }
            }
            GenerationMode::Cartesian => {
                for (index, dimension) in spec.dimensions.iter().enumerate().rev() {
                    assignment.push(assign(
                        index,
                        remaining % dimension.candidates.len(),
                        dimension,
                    ));
                    remaining /= dimension.candidates.len();
                }
                assignment.reverse();
            }
        }
        let mut experiment = spec.base.clone();
        for item in &assignment {
            match &item.target {
                DimensionTarget::Argument { index } => {
                    experiment.case.args[*index as usize] = item.value.clone()
                }
                DimensionTarget::Environment { name } => {
                    experiment
                        .case
                        .environment
                        .insert(name.clone(), item.value.clone());
                }
            }
        }
        let input_identity = experiment.case.input_identity()?;
        let hash = canonical_hash(&(GENERATOR, &spec_hash, &parent, &assignment, &input_identity))?;
        let id = format!("generated-{}", hash.trim_start_matches("sha256:"));
        if !identities.insert(id.clone()) || !inputs_seen.insert(input_identity.clone()) {
            return Err(invalid("duplicate generated case identity/input"));
        }
        experiment.case.case_id = id.clone();
        experiment.before.input_identity = input_identity.clone();
        experiment.after.input_identity = input_identity.clone();
        generated.push(GeneratedCase {
            identity: GeneratedCaseIdentity {
                generated_case_id: id,
                parent_case_identity: parent.clone(),
                generation_spec_hash: spec_hash.clone(),
                assignment,
                input_identity,
                case_identity: experiment.case.identity()?,
            },
            experiment,
        });
    }
    Ok(generated)
}

fn assign(index: usize, candidate: usize, dimension: &Dimension) -> Assignment {
    Assignment {
        dimension_index: index as u64,
        candidate_index: candidate as u64,
        target: dimension.target.clone(),
        value: dimension.candidates[candidate].clone(),
    }
}

fn child_id(suite: &str, generated: &GeneratedCaseIdentity) -> io::Result<String> {
    Ok(format!(
        "generated-comparison-{}",
        canonical_hash(&(GENERATOR, suite, generated))?.trim_start_matches("sha256:")
    ))
}

fn receipt(report: &BehaviorGeneratedSuiteResult) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(report)?,
    };
    Ok(Evidence {
        evidence_id: "behavior-generated-suite".into(),
        run_id: report.suite_id.clone(),
        source: GENERATOR.into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec![GENERATOR.into()],
    })
}

fn summarize(
    store: &EvidenceStore,
    suite_id: &str,
    spec: &GenerationSpec,
    auth: &BehaviorAuthorization,
) -> io::Result<BehaviorGeneratedSuiteResult> {
    validate_contract(&spec.base, auth)?;
    let generated = generate(spec)?;
    let mut counts = OutcomeCounts::default();
    let mut comparisons = Vec::new();
    let mut completed_pairs = 0;
    for item in &generated {
        let id = child_id(suite_id, &item.identity)?;
        let child = super::load(store, &id, auth)?;
        if child.experiment != item.experiment {
            return Err(invalid(
                "generated child experiment/input identity mismatch",
            ));
        }
        // P2 load validates positive outcomes. Also recompute negative outcomes
        // when both acquisitions exist, without changing P2's public semantics.
        if let (Some(before), Some(after)) = (&child.before, &child.after) {
            let before = load_run(
                store,
                &child.experiment,
                &child.experiment.before,
                before,
                true,
            )?;
            let after = load_run(
                store,
                &child.experiment,
                &child.experiment.after,
                after,
                true,
            )?;
            let (outcome, divergences) = decision(&child, &before, &after);
            if outcome != child.outcome || divergences != child.divergences {
                return Err(invalid("child outcome not supported by acquisitions"));
            }
            if matches!(
                outcome,
                BehaviorOutcome::DivergenceProven | BehaviorOutcome::NoDivergenceFound
            ) {
                completed_pairs += 1;
            }
        }
        match child.outcome {
            BehaviorOutcome::NoDivergenceFound => counts.no_divergence_found += 1,
            BehaviorOutcome::DivergenceProven => counts.divergence_proven += 1,
            BehaviorOutcome::Inconclusive => counts.inconclusive += 1,
            BehaviorOutcome::Error => counts.error += 1,
        }
        comparisons.push(ComparisonRef {
            comparison_id: id,
            comparison_hash: canonical_hash(&child)?,
        });
    }
    let verdict = if counts.divergence_proven > 0 {
        Verdict::Fail
    } else if counts.error > 0 {
        Verdict::Error
    } else if counts.inconclusive > 0 {
        Verdict::Inconclusive
    } else {
        Verdict::Pass
    };
    let expected = generated.len() as u64;
    Ok(BehaviorGeneratedSuiteResult {
        suite_id: suite_id.into(), schema_version: "1".into(), generation_spec: spec.clone(),
        generation_spec_hash: canonical_hash(spec)?, generated_cases: generated.into_iter().map(|item| item.identity).collect(),
        comparisons, authorization_hash: canonical_hash(auth)?, outcome_counts: counts,
        aggregate_verdict: verdict, generation_budget: spec.max_cases,
        coverage: GenerationCoverage { expected_cases: expected, verified_comparisons: expected,
            completed_pairs, complete: completed_pairs == expected },
        limitations: vec![
            "Bounded explicit inputs only; NO DIVERGENCE FOUND is not equivalence or exhaustive proof.".into(),
            "P2 exact semantics and external stability assertion; no automatic P3A comparison, fuzzing, mutation, inference or reducer.".into(),
            "Trusted local Unix; hashes are not authentication. P2 fixture/stream/host/runtime limitations apply to every child.".into(),
            "FAIL replayability unavailable: P2 has no paired replay executor; rerun recorded spec with matching approved targets/fixture and a fresh suite ID.".into(),
        ],
    })
}

/// Invalid generation rejects before reservation or acquisition. Storage/integrity
/// failures return Err (ERROR, no completed suite promised); never a partial PASS.
pub fn execute(
    store: &EvidenceStore,
    workspace_root: &Path,
    suite_id: &str,
    spec: &GenerationSpec,
    auth: &BehaviorAuthorization,
) -> io::Result<BehaviorGeneratedSuiteResult> {
    validate_contract(&spec.base, auth)?;
    let generated = generate(spec)?;
    let committed = store.reserve(suite_id)?;
    for item in generated {
        super::execute(
            store,
            workspace_root,
            &child_id(suite_id, &item.identity)?,
            &item.experiment,
            auth,
        )?;
    }
    // Reread all children after the final execution, including their P1 evidence.
    let report = summarize(store, suite_id, spec, auth)?;
    let evidence = receipt(&report)?;
    committed.write_evidence(&evidence)?;
    committed.complete(&report, &[evidence.evidence_id])?;
    Ok(report)
}

/// Re-enumerate the spec, verify every exact child/run/evidence reference and
/// recompute counts, completeness and verdict. Generic Store::load is insufficient.
pub fn load(
    store: &EvidenceStore,
    suite_id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<BehaviorGeneratedSuiteResult> {
    let (value, evidence) = store.load(suite_id)?;
    let saved: BehaviorGeneratedSuiteResult = serde_json::from_value(value)?;
    let expected = summarize(store, suite_id, &saved.generation_spec, auth)?;
    if saved != expected || evidence != [receipt(&expected)?] {
        return Err(invalid(
            "suite identity/spec/children/counts/verdict/receipt mismatch",
        ));
    }
    Ok(expected)
}
