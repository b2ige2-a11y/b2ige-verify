//! P3A bounded baseline measurement. Public consumers take a stored profile ID
//! and hash, never caller-provided classifications or process observations.
//! Hashes protect integrity within the existing trusted-local Store boundary;
//! they are not authentication or proof of future deterministic behavior.
use super::*;

const PROFILER: &str = "behavior.process.baseline_stability.v1";
pub const MIN_REPETITIONS: usize = 2;
pub const DEFAULT_REPETITIONS: usize = 5;
const MAX_REPETITIONS: usize = 1000;
const OBSERVABLES: [Observable; 4] = [
    Observable::ExitCode,
    Observable::Signal,
    Observable::Stdout,
    Observable::Stderr,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfilingConfig {
    pub repetitions: usize,
}
impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            repetitions: DEFAULT_REPETITIONS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stability {
    Stable,
    Unstable,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservableStability {
    pub observable: Observable,
    pub stability: Stability,
    /// First-seen distinct values, including partial values if INCOMPLETE.
    /// Classification uses actual raw bytes, never equality of hashes alone.
    pub observed_values: Vec<ObservableValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StabilityProfileRef {
    pub profile_id: String,
    pub profile_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineStabilityProfile {
    pub profile_id: String,
    pub schema_version: String,
    pub case_identity: String,
    pub baseline_identity: String,
    pub baseline_target_identity: String,
    pub input_identity: String,
    pub repetition_count: usize,
    pub run_ids: Vec<String>,
    pub runs: Vec<RunRecord>,
    pub observables: Vec<ObservableStability>,
    pub evidence_refs: Vec<EvidenceRef>,
    /// Canonical baseline-only experiment: AFTER is an exact copy of BEFORE.
    /// Candidate changes neither alter the profile nor update baseline approval.
    pub experiment: BehaviorExperiment,
    pub experiment_hash: String,
    pub config: ProfilingConfig,
    pub config_hash: String,
    pub authorization_hash: String,
}
impl BaselineStabilityProfile {
    pub fn reference(&self) -> io::Result<StabilityProfileRef> {
        Ok(StabilityProfileRef {
            profile_id: self.profile_id.clone(),
            profile_hash: canonical_hash(self)?,
        })
    }
}

/// Separate v1 schema: the P2 exact artifact and its interpretation are unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StabilityComparisonResult {
    pub comparison_id: String,
    pub schema_version: String,
    pub profile: StabilityProfileRef,
    pub case_identity: String,
    pub baseline_target_identity: String,
    pub candidate_identity: String,
    pub input_identity: String,
    pub candidate: RunRecord,
    pub evidence_refs: Vec<EvidenceRef>,
    pub experiment: BehaviorExperiment,
    pub experiment_hash: String,
    pub config_hash: String,
    pub authorization_hash: String,
    pub outcome: BehaviorOutcome,
    pub verdict: Verdict,
    pub divergences: Vec<Divergence>,
    pub reason: String,
    pub replayability: Replayability,
}

fn schema<T: JsonSchema>(name: &str, title: &str) -> serde_json::Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(T))
        .expect("statically defined schema serializes");
    schema["$id"] = format!("https://b2ige.dev/schemas/verify/{name}.v1.json").into();
    schema["title"] = title.into();
    schema["properties"]["schema_version"]["const"] = "1".into();
    schema
}

pub fn profile_schema() -> serde_json::Value {
    schema::<BaselineStabilityProfile>(
        "baseline-stability-profile",
        "Baseline Stability Profile v1",
    )
}

pub fn comparison_schema() -> serde_json::Value {
    schema::<StabilityComparisonResult>(
        "stability-comparison-result",
        "Stability Comparison Result v1",
    )
}

fn baseline_experiment(experiment: &BehaviorExperiment) -> BehaviorExperiment {
    let mut baseline = experiment.clone();
    baseline.after = baseline.before.clone();
    baseline
}

fn validate(experiment: &BehaviorExperiment, auth: &BehaviorAuthorization) -> io::Result<()> {
    validate_contract(experiment, auth)?;
    if experiment.case.required_observers != [OBSERVER] {
        return Err(invalid(
            "P3A supports only the four required cli_process observables",
        ));
    }
    // baseline_stable is deliberately not consulted: P3A measures uncertainty.
    Ok(())
}

fn validate_config(config: &ProfilingConfig) -> io::Result<()> {
    if !(1..=MAX_REPETITIONS).contains(&config.repetitions) {
        return Err(invalid(
            "P3A repetitions must be in 1..=1000; fewer than 2 is INCOMPLETE",
        ));
    }
    Ok(())
}

fn receipt(id: &str, kind: &str, value: &impl Serialize) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(value)?,
    };
    Ok(Evidence {
        evidence_id: kind.into(),
        run_id: id.into(),
        source: PROFILER.into(),
        trust_class: TrustClass::Derived,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec![PROFILER.into()],
    })
}

fn observable_value(observable: &Observable, run: &ProcessObservation) -> ObservableValue {
    match observable {
        Observable::ExitCode => ObservableValue::Status {
            value: run.exit_code,
        },
        Observable::Signal => ObservableValue::Status { value: run.signal },
        Observable::Stdout => ObservableValue::Bytes {
            sha256: raw_hash(&run.stdout),
            length: run.stdout.len(),
        },
        Observable::Stderr => ObservableValue::Bytes {
            sha256: raw_hash(&run.stderr),
            length: run.stderr.len(),
        },
    }
}

fn equal(observable: &Observable, a: &ProcessObservation, b: &ProcessObservation) -> bool {
    match observable {
        Observable::ExitCode => a.exit_code == b.exit_code,
        Observable::Signal => a.signal == b.signal,
        Observable::Stdout => a.stdout == b.stdout,
        Observable::Stderr => a.stderr == b.stderr,
    }
}

fn summarize(
    profile_id: &str,
    experiment: &BehaviorExperiment,
    auth: &BehaviorAuthorization,
    config: &ProfilingConfig,
    acquisitions: &[AcquisitionResult],
) -> io::Result<BaselineStabilityProfile> {
    let incomplete = acquisitions.len() < MIN_REPETITIONS
        || acquisitions.len() != config.repetitions
        || acquisitions
            .iter()
            .any(|run| readiness(&run.observation) != BehaviorOutcome::NoDivergenceFound);
    let observables = OBSERVABLES
        .into_iter()
        .map(|observable| {
            let mut observed_values = Vec::new();
            for run in acquisitions {
                let value = observable_value(&observable, &run.observation);
                if !observed_values.contains(&value) {
                    observed_values.push(value);
                }
            }
            let stability = if incomplete {
                Stability::Incomplete
            } else if acquisitions
                .iter()
                .all(|run| equal(&observable, &acquisitions[0].observation, &run.observation))
            {
                Stability::Stable
            } else {
                Stability::Unstable
            };
            ObservableStability {
                observable,
                stability,
                observed_values,
            }
        })
        .collect();
    let runs: Vec<_> = acquisitions
        .iter()
        .map(|run| record(run, &experiment.case))
        .collect::<io::Result<_>>()?;
    Ok(BaselineStabilityProfile {
        profile_id: profile_id.into(),
        schema_version: "1".into(),
        case_identity: experiment.case.identity()?,
        baseline_identity: canonical_hash(&experiment.baseline)?,
        baseline_target_identity: experiment.before.identity.clone(),
        input_identity: experiment.case.input_identity()?,
        repetition_count: runs.len(),
        run_ids: runs.iter().map(|run| run.run_id.clone()).collect(),
        evidence_refs: runs
            .iter()
            .flat_map(|run| run.evidence_refs.clone())
            .collect(),
        runs,
        observables,
        experiment: experiment.clone(),
        experiment_hash: canonical_hash(experiment)?,
        config: config.clone(),
        config_hash: canonical_hash(&(PROFILER, &experiment.case, config))?,
        authorization_hash: canonical_hash(auth)?,
    })
}

fn check_workspace(link: &RunRecord, id: &str, leaf: &str) -> io::Result<()> {
    if link.run_id != format!("{id}-{leaf}")
        || !link.working_directory.is_absolute()
        || link.working_directory.file_name().and_then(|s| s.to_str()) != Some(leaf)
        || link
            .working_directory
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            != Some(id)
    {
        return Err(invalid(
            "profile/comparison requires distinct run IDs and fresh workspaces",
        ));
    }
    Ok(())
}

fn profiling_inputs(
    experiment: &BehaviorExperiment,
    profile_id: &str,
    config: &ProfilingConfig,
    cwd: &Path,
) -> io::Result<(ExperimentPlan, ProcessSpec)> {
    let (mut plan, spec) = inputs(experiment, &experiment.before, cwd, false)?;
    // Bind requested count to every P1 plan/context/evidence, preventing a
    // coherently rehashed profile from selecting a smaller, stable subset.
    plan.plan_id = format!("{PROFILER}:{profile_id}:{}", canonical_hash(config)?);
    plan.scope
        .exploration_budget
        .insert("baseline_repetitions".into(), config.repetitions as u64);
    validate_inputs(&plan, &spec)?;
    Ok((plan, spec))
}

fn load_baseline_run(
    store: &EvidenceStore,
    experiment: &BehaviorExperiment,
    profile_id: &str,
    config: &ProfilingConfig,
    link: &RunRecord,
) -> io::Result<AcquisitionResult> {
    let (plan, spec) = profiling_inputs(experiment, profile_id, config, &link.working_directory)?;
    load_acquisition(store, &plan, &spec, &experiment.case, link)
}

fn read_profile(
    store: &EvidenceStore,
    profile_id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<(BaselineStabilityProfile, Vec<AcquisitionResult>)> {
    let (value, evidence) = store.load(profile_id)?;
    let profile: BaselineStabilityProfile = serde_json::from_value(value)?;
    validate_config(&profile.config)?;
    validate(&profile.experiment, auth)?;
    if profile.experiment != baseline_experiment(&profile.experiment)
        || profile.runs.len() != profile.config.repetitions
    {
        return Err(invalid(
            "profile is not a complete set of requested baseline acquisitions",
        ));
    }
    let mut runs = Vec::new();
    for (index, link) in profile.runs.iter().enumerate() {
        check_workspace(link, profile_id, &format!("baseline-{index}"))?;
        if link.working_directory.parent() != profile.runs[0].working_directory.parent() {
            return Err(invalid(
                "profile runs must share one reserved workspace root",
            ));
        }
        runs.push(load_baseline_run(
            store,
            &profile.experiment,
            profile_id,
            &profile.config,
            link,
        )?);
    }
    let expected = summarize(
        profile_id,
        &profile.experiment,
        auth,
        &profile.config,
        &runs,
    )?;
    if profile != expected || evidence != [receipt(profile_id, "baseline-stability", &expected)?] {
        return Err(invalid(
            "profile identity/classifications/hashes are not supported by stored acquisitions",
        ));
    }
    Ok((profile, runs))
}

/// Read and recompute every classification from linked, committed P1 evidence.
/// Generic EvidenceStore::load alone is not a stability validator.
pub fn load_profile(
    store: &EvidenceStore,
    profile_id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<BaselineStabilityProfile> {
    Ok(read_profile(store, profile_id, auth)?.0)
}

/// Execute BEFORE exactly config.repetitions times. Candidate is not run or pinned
/// into the profile. Each repetition receives one frozen snapshot in a fresh cwd.
/// I/O/contract/store failures return Err (ERROR, no promised profile commit).
/// Timeout, incomplete capture and runner failure yield INCOMPLETE classifications.
pub fn profile(
    store: &EvidenceStore,
    workspace_root: &Path,
    profile_id: &str,
    experiment: &BehaviorExperiment,
    auth: &BehaviorAuthorization,
    config: &ProfilingConfig,
) -> io::Result<BaselineStabilityProfile> {
    if profile_id.len() > 110 {
        return Err(invalid("profile_id exceeds derived run ID limit"));
    }
    validate_config(config)?;
    let experiment = baseline_experiment(experiment);
    validate(&experiment, auth)?;
    verify_target(&experiment.before)?;
    let snapshot = Snapshot::capture(experiment.case.fixture.source.as_deref())?;
    if snapshot.identity()? != experiment.case.fixture.snapshot_identity {
        return Err(invalid("actual fixture snapshot/input identity mismatch"));
    }
    let committed = store.reserve(profile_id)?;
    let work = workspace::reserve(workspace_root, profile_id)?;
    let mut links = Vec::new();
    for index in 0..config.repetitions {
        let leaf = format!("baseline-{index}");
        let cwd = work.join(&leaf);
        snapshot.materialize(&cwd)?;
        verify_target(&experiment.before)?;
        let (plan, spec) = profiling_inputs(&experiment, profile_id, config, &cwd)?;
        let run = acquire(&plan, &spec, store, &format!("{profile_id}-{leaf}"))?;
        links.push(record(&run, &experiment.case)?);
    }
    // Re-read ALL runs after the final execution: later targets can corrupt an
    // earlier run in this trusted-local mode. Returned observations are not enough.
    let runs = links
        .iter()
        .map(|link| load_baseline_run(store, &experiment, profile_id, config, link))
        .collect::<io::Result<Vec<_>>>()?;
    let profile = summarize(profile_id, &experiment, auth, config, &runs)?;
    let evidence = receipt(profile_id, "baseline-stability", &profile)?;
    committed.write_evidence(&evidence)?;
    committed.complete(&profile, &[evidence.evidence_id])?;
    Ok(profile)
}

fn bound_profile(
    store: &EvidenceStore,
    reference: &StabilityProfileRef,
    experiment: &BehaviorExperiment,
    auth: &BehaviorAuthorization,
) -> io::Result<(BaselineStabilityProfile, Vec<AcquisitionResult>)> {
    let (profile, runs) = read_profile(store, &reference.profile_id, auth)?;
    if profile.reference()? != *reference || profile.experiment != baseline_experiment(experiment) {
        return Err(invalid(
            "stale or wrong profile: hash/case/input/baseline/executable/seed/config mismatch",
        ));
    }
    Ok((profile, runs))
}

fn decide(
    profile: &BaselineStabilityProfile,
    baseline: &AcquisitionResult,
    candidate: &AcquisitionResult,
    candidate_identity: &str,
) -> (BehaviorOutcome, Vec<Divergence>) {
    if profile
        .observables
        .iter()
        .any(|item| item.stability == Stability::Incomplete)
    {
        return (BehaviorOutcome::Inconclusive, vec![]);
    }
    let ready = readiness(&candidate.observation);
    if ready != BehaviorOutcome::NoDivergenceFound {
        return (ready, vec![]);
    }
    let mut divergences = Vec::new();
    for item in &profile.observables {
        if item.stability == Stability::Stable
            && !equal(
                &item.observable,
                &baseline.observation,
                &candidate.observation,
            )
        {
            divergences.push(Divergence {
                case_identity: profile.case_identity.clone(),
                before_run_id: baseline.context.run_id.clone(),
                after_run_id: candidate.context.run_id.clone(),
                before_target_hash: profile.baseline_target_identity.clone(),
                after_target_hash: candidate_identity.into(),
                observable: item.observable.clone(),
                before: observable_value(&item.observable, &baseline.observation),
                after: observable_value(&item.observable, &candidate.observation),
                evidence_refs: profile.runs[0]
                    .evidence_refs
                    .iter()
                    .cloned()
                    .chain(
                        candidate
                            .bundle
                            .evidence_hashes
                            .iter()
                            .map(|(id, hash)| EvidenceRef {
                                run_id: candidate.context.run_id.clone(),
                                evidence_id: id.clone(),
                                evidence_hash: hash.clone(),
                            }),
                    )
                    .collect(),
            });
        }
    }
    let outcome = if !divergences.is_empty() {
        BehaviorOutcome::DivergenceProven
    } else if profile
        .observables
        .iter()
        .any(|item| item.stability == Stability::Unstable)
    {
        // Even matching one observed value cannot convert learned uncertainty to PASS.
        BehaviorOutcome::Inconclusive
    } else {
        BehaviorOutcome::NoDivergenceFound
    };
    (outcome, divergences)
}

fn comparison(
    id: &str,
    experiment: &BehaviorExperiment,
    auth: &BehaviorAuthorization,
    profile: &BaselineStabilityProfile,
    baseline: &AcquisitionResult,
    candidate: &AcquisitionResult,
) -> io::Result<StabilityComparisonResult> {
    let (outcome, divergences) = decide(profile, baseline, candidate, &experiment.after.identity);
    let candidate = record(candidate, &experiment.case)?;
    let reference = profile.reference()?;
    Ok(StabilityComparisonResult {
        comparison_id: id.into(), schema_version: "1".into(),
        case_identity: experiment.case.identity()?, baseline_target_identity: experiment.before.identity.clone(),
        candidate_identity: experiment.after.identity.clone(), input_identity: experiment.case.input_identity()?,
        evidence_refs: profile.evidence_refs.iter().cloned().chain(candidate.evidence_refs.clone()).collect(),
        candidate, experiment: experiment.clone(), experiment_hash: canonical_hash(experiment)?,
        config_hash: canonical_hash(&(PROFILER, &experiment.case, &reference))?,
        authorization_hash: canonical_hash(auth)?, profile: reference,
        outcome, verdict: outcome.verdict(), divergences,
        reason: match outcome {
            BehaviorOutcome::DivergenceProven => "DIVERGENCE PROVEN: candidate differs byte-exactly on an observable stable across the recorded baseline runs; sampled difference, not proof of its cause.",
            BehaviorOutcome::NoDivergenceFound => "NO DIVERGENCE FOUND within tested behavior space; all four required observables were stable across the recorded baseline runs and equal to the complete candidate observation.",
            BehaviorOutcome::Inconclusive => "Required baseline/candidate observation incomplete, repetition count insufficient, or learned baseline uncertainty remains; no automatic ignore rules.",
            BehaviorOutcome::Error => "Candidate runner/acquisition failed; comparison withheld.",
        }.into(),
        replayability: Replayability::Unavailable {
            reason: "No paired snapshot replay executor. Rerun explicit profiling/comparison with recorded inputs, matching executable/fixture hashes, approval pins and fresh IDs. Bounded stability is not proof of future determinism; host/runtime/external state remain uncontrolled.".into(),
        },
    })
}

/// Optional P3A comparison mode. With no profile, continue using behavior::execute
/// (unchanged P2 exact semantics). A supplied profile must be stored and hash-pinned.
/// Err means ERROR with no promised comparison commit; never fall back to exact
/// mode on a stale, missing or corrupt profile. One fresh candidate is acquired.
pub fn compare(
    store: &EvidenceStore,
    workspace_root: &Path,
    comparison_id: &str,
    experiment: &BehaviorExperiment,
    auth: &BehaviorAuthorization,
    reference: &StabilityProfileRef,
) -> io::Result<StabilityComparisonResult> {
    if comparison_id.len() > 110 {
        return Err(invalid("comparison_id exceeds derived run ID limit"));
    }
    validate(experiment, auth)?;
    bound_profile(store, reference, experiment, auth)?;
    verify_target(&experiment.before)?;
    verify_target(&experiment.after)?;
    let snapshot = Snapshot::capture(experiment.case.fixture.source.as_deref())?;
    if snapshot.identity()? != experiment.case.fixture.snapshot_identity {
        return Err(invalid("actual fixture snapshot/input identity mismatch"));
    }
    let committed = store.reserve(comparison_id)?;
    let work = workspace::reserve(workspace_root, comparison_id)?;
    let cwd = work.join("candidate");
    snapshot.materialize(&cwd)?;
    verify_target(&experiment.after)?;
    let (plan, spec) = inputs(experiment, &experiment.after, &cwd, false)?;
    let run = acquire(&plan, &spec, store, &format!("{comparison_id}-candidate"))?;
    let link = record(&run, &experiment.case)?;
    // Check profile again after candidate execution, including all its evidence.
    let (profile, baseline_runs) = bound_profile(store, reference, experiment, auth)?;
    let run = load_run(store, experiment, &experiment.after, &link, false)?;
    let report = comparison(
        comparison_id,
        experiment,
        auth,
        &profile,
        &baseline_runs[0],
        &run,
    )?;
    let evidence = receipt(comparison_id, "stability-comparison", &report)?;
    committed.write_evidence(&evidence)?;
    committed.complete(&report, &[evidence.evidence_id])?;
    Ok(report)
}

/// Revalidates every profile and candidate run and recomputes the entire result.
pub fn load_comparison(
    store: &EvidenceStore,
    comparison_id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<StabilityComparisonResult> {
    let (value, evidence) = store.load(comparison_id)?;
    let report: StabilityComparisonResult = serde_json::from_value(value)?;
    validate(&report.experiment, auth)?;
    let (profile, baseline_runs) = bound_profile(store, &report.profile, &report.experiment, auth)?;
    check_workspace(&report.candidate, comparison_id, "candidate")?;
    let run = load_run(
        store,
        &report.experiment,
        &report.experiment.after,
        &report.candidate,
        false,
    )?;
    let expected = comparison(
        comparison_id,
        &report.experiment,
        auth,
        &profile,
        &baseline_runs[0],
        &run,
    )?;
    if report != expected
        || evidence != [receipt(comparison_id, "stability-comparison", &expected)?]
    {
        return Err(invalid(
            "stability comparison is not supported by stored profile/acquisition evidence",
        ));
    }
    Ok(report)
}
