//! SideEffect Proof v1: trusted local execution, durable SQLite evidence and
//! deterministic checking. Neither target exits nor request counts assign verdicts.
mod model;
mod reduction;
mod sqlite;
use crate::{
    behavior::{self, workspace::Snapshot},
    Verdict,
};
pub use model::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Component, Path},
};
use verify_evidence::{
    canonical_hash,
    store::{EvidenceStore, RunStore},
    Evidence, Observation, ObservationCoverage, TrustClass,
};
use verify_runner::process::{observe_controlled, ProcessSpec};

fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn text_id(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256
}
fn schedule_valid(s: &FaultSchedule) -> bool {
    !s.primitives.is_empty()
        && s.primitives.len() <= 64
        && s.primitives.iter().enumerate().all(|(i, p)| match p {
            FaultPrimitive::Retry | FaultPrimitive::DuplicateDelivery => i > 0,
            FaultPrimitive::KillAfterCommit => {
                i == 0 && s.primitives.get(i + 1) == Some(&FaultPrimitive::Retry)
            }
            FaultPrimitive::None => true,
        })
}
impl SideEffectContract {
    pub fn validate(&self) -> io::Result<()> {
        let op = &self.operation;
        let t = &self.trigger;
        if self.schema_version != "1"
            || !identifier(&self.contract_id)
            || [
                &op.operation_id,
                &op.idempotency_identity,
                &op.correlation_identity,
            ]
            .iter()
            .any(|v| !text_id(v))
            || self.effects.is_empty()
            || self.effects.len() > 64
            || self.required_observers.is_empty()
            || self.required_observers.len() > 16
            || self.fault_schedules.is_empty()
            || self.fault_schedules.len() > 64
            || t.timeout_ms == 0
            || t.timeout_ms > 60_000
            || !t.executable.is_absolute()
            || !verify_evidence::valid_hash(&t.executable_hash)
            || !verify_evidence::valid_hash(&t.fixture.snapshot_identity)
            || t.fixture.source.as_ref().is_some_and(|p| !p.is_absolute())
            || self.exploration_budget.max_schedules > 64
            || self.exploration_budget.max_attempts > 4096
            || self.exploration_budget.reduction_executions > 128
            || t.environment
                .keys()
                .any(|k| k.is_empty() || k.contains(['=', '\0']))
            || t.environment
                .values()
                .chain(t.args.iter())
                .any(|v| v.contains('\0'))
        {
            return Err(invalid("invalid or unsupported SideEffectContract v1"));
        }
        let mut ids = BTreeSet::new();
        for o in &self.required_observers {
            if !ids.insert(&o.observer_id)
                || [
                    &o.observer_id,
                    &o.table,
                    &o.external_effect_id_column,
                    &o.idempotency_column,
                    &o.correlation_column,
                    &o.operation_column,
                    &o.commit_order_column,
                ]
                .iter()
                .any(|s| !identifier(s))
                || o.db_path.as_os_str().is_empty()
                || o.db_path
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || [
                    &o.external_effect_id_column,
                    &o.idempotency_column,
                    &o.correlation_column,
                    &o.operation_column,
                    &o.commit_order_column,
                ]
                .into_iter()
                .collect::<BTreeSet<_>>()
                .len()
                    != 5
            {
                return Err(invalid("invalid SQLite observer identity/path/columns"));
            }
        }
        let mut effects = BTreeSet::new();
        let mut domains = BTreeSet::new();
        for e in &self.effects {
            if !identifier(&e.effect_id)
                || !effects.insert(&e.effect_id)
                || !domains.insert((&e.authoritative_observer, &e.operation))
                || !text_id(&e.provider)
                || !text_id(&e.operation)
                || !ids.contains(&e.authoritative_observer)
                || e.identity.idempotency_identity != op.idempotency_identity
                || e.identity.correlation_identity != op.correlation_identity
            {
                return Err(invalid(
                    "invalid committed effect identity/observer contract",
                ));
            }
        }
        if ids.iter().any(|id| {
            !self
                .effects
                .iter()
                .any(|e| &e.authoritative_observer == *id)
        }) {
            return Err(invalid("required observer is not bound to an effect"));
        }
        for relation in &self.relations {
            let (a, b) = match relation {
                Relation::OrderedAfter { effect, after } => (effect, after),
                Relation::AtomicWith { effect, other } => (effect, other),
            };
            if a == b || !effects.contains(a) || !effects.contains(b) {
                return Err(invalid("invalid relation effects"));
            }
            if matches!(relation, Relation::OrderedAfter { .. }) {
                let observer = |id: &str| {
                    let e = self.effects.iter().find(|e| e.effect_id == id).unwrap();
                    self.required_observers
                        .iter()
                        .find(|o| o.observer_id == e.authoritative_observer)
                        .unwrap()
                };
                let (a, b) = (observer(a), observer(b));
                if (&a.db_path, &a.table, &a.commit_order_column)
                    != (&b.db_path, &b.table, &b.commit_order_column)
                {
                    return Err(invalid(
                        "ordered_after requires a shared ledger commit sequence",
                    ));
                }
            }
        }
        let mut schedules = BTreeSet::new();
        if self.fault_schedules.iter().any(|s| {
            !identifier(&s.schedule_id) || !schedules.insert(&s.schedule_id) || !schedule_valid(s)
        }) {
            return Err(invalid("invalid fault schedule: kill requires following retry; retry/delivery require an earlier attempt"));
        }
        Ok(())
    }
}
fn evidence<T: Serialize>(
    run_id: &str,
    id: &str,
    source: &str,
    trust_class: TrustClass,
    value: &T,
) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(value)?,
    };
    Ok(Evidence {
        evidence_id: id.into(),
        run_id: run_id.into(),
        source: source.into(),
        trust_class,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec!["sideeffect".into()],
    })
}
fn save<T: Serialize>(
    run: &RunStore,
    id: &str,
    name: &str,
    source: &str,
    trust: TrustClass,
    value: &T,
    ids: &mut Vec<String>,
) -> io::Result<()> {
    run.write_evidence(&evidence(id, name, source, trust, value)?)?;
    ids.push(name.into());
    Ok(())
}
fn require_evidence<T: Serialize>(
    items: &[Evidence],
    id: &str,
    name: &str,
    source: &str,
    trust: TrustClass,
    value: &T,
) -> io::Result<()> {
    if items.iter().find(|e| e.evidence_id == name)
        != Some(&evidence(id, name, source, trust, value)?)
    {
        return Err(invalid(
            "source result contradicts bound runtime/SQLite/history evidence",
        ));
    }
    Ok(())
}
fn reference(id: &str, value: &impl Serialize) -> io::Result<ArtifactRef> {
    Ok(ArtifactRef {
        artifact_id: id.into(),
        integrity_hash: canonical_hash(value)?,
    })
}
fn parse<T: DeserializeOwned>(v: serde_json::Value) -> io::Result<T> {
    Ok(serde_json::from_value(v)?)
}
fn complete(s: &[SQLiteSnapshot]) -> bool {
    !s.is_empty()
        && s.iter()
            .all(|s| s.coverage == ObservationCoverage::Complete)
}
fn rows(s: &[SQLiteSnapshot]) -> Vec<CommittedRow> {
    s.iter().flat_map(|s| s.rows.clone()).collect()
}
fn killed(a: &AttemptRecord) -> bool {
    a.requested == FaultPrimitive::KillAfterCommit
        && a.runtime.termination_requested
        && a.runtime.termination_delivered
        && a.runtime.observation.started
        && a.runtime.observation.signal == Some(9)
        && !a.runtime.observation.timed_out
        && a.runtime.observation.runner_failure.is_none()
        && complete(&a.commit_before_kill)
        && !rows(&a.commit_before_kill).is_empty()
}
fn attempt_evidence(run: &RunStore, record: &AttemptRecord) -> io::Result<()> {
    let id = &record.identity.run_id;
    let mut ids = vec![];
    save(
        run,
        id,
        "process",
        "cli_process",
        TrustClass::DirectRuntime,
        &(
            &record.identity,
            &record.contract_hash,
            &record.schedule_id,
            record.action_index,
            record.requested,
            &record.process,
            &record.runtime,
        ),
        &mut ids,
    )?;
    save(
        run,
        id,
        "committed",
        "sqlite_committed",
        TrustClass::AuthoritativeTargetState,
        &(&record.commit_before_kill, &record.after),
        &mut ids,
    )?;
    run.complete(record, &ids)
}
fn load_attempt(
    store: &EvidenceStore,
    r: &ArtifactRef,
    c: &SideEffectContract,
    parent: &str,
    si: usize,
    ai: usize,
) -> io::Result<AttemptRecord> {
    let expected_id = format!("{parent}-s{si}-a{ai}");
    if r.artifact_id != expected_id {
        return Err(invalid("cross-run attempt reference"));
    }
    let (raw, evidence) = store.load(&r.artifact_id)?;
    let a: AttemptRecord = parse(raw)?;
    if canonical_hash(&a)? != r.integrity_hash
        || a.schema_version != "1"
        || a.identity.run_id != expected_id
        || a.identity.attempt_id != format!("attempt-{ai}")
        || a.identity.operation_id != c.operation.operation_id
        || a.contract_hash != canonical_hash(c)?
        || a.schedule_id != c.fault_schedules[si].schedule_id
        || a.action_index != ai
        || Some(&a.requested) != c.fault_schedules[si].primitives.get(ai)
        || evidence.len() != 2
    {
        return Err(invalid("invalid attempt identity/hash/schedule binding"));
    }
    a.process.validate().map_err(invalid)?;
    if a.process.executable != c.trigger.executable
        || a.process.args != c.trigger.args
        || a.process.environment != c.trigger.environment
        || a.process.timeout_ms != c.trigger.timeout_ms
    {
        return Err(invalid("attempt input does not match contract"));
    }
    require_evidence(
        &evidence,
        &expected_id,
        "process",
        "cli_process",
        TrustClass::DirectRuntime,
        &(
            &a.identity,
            &a.contract_hash,
            &a.schedule_id,
            a.action_index,
            a.requested,
            &a.process,
            &a.runtime,
        ),
    )?;
    require_evidence(
        &evidence,
        &expected_id,
        "committed",
        "sqlite_committed",
        TrustClass::AuthoritativeTargetState,
        &(&a.commit_before_kill, &a.after),
    )?;
    if !a.commit_before_kill.is_empty() {
        sqlite::verify_snapshots(&a.commit_before_kill, c)?;
    }
    sqlite::verify_snapshots(&a.after, c)?;
    if (a.runtime.termination_delivered && !a.runtime.termination_requested)
        || a.runtime.termination_requested != !a.commit_before_kill.is_empty()
        || (!a.commit_before_kill.is_empty()
            && (a.requested != FaultPrimitive::KillAfterCommit
                || !complete(&a.commit_before_kill)
                || rows(&a.commit_before_kill).is_empty()))
    {
        return Err(invalid("termination lacks pre-kill committed evidence"));
    }
    Ok(a)
}
fn push_event(
    h: &mut SideEffectHistory,
    kind: HistoryKind,
    a: Option<&AttemptRecord>,
    effect: Option<CommittedEffectIdentity>,
    refs: Vec<String>,
) {
    let n = h.events.len() as u64;
    h.events.push(HistoryEvent {
        event_id: format!("{}-event-{n}", h.run_id),
        logical_order: n,
        run_id: a.map_or_else(|| h.run_id.clone(), |a| a.identity.run_id.clone()),
        attempt_id: a.map(|a| a.identity.attempt_id.clone()),
        kind,
        related_effect: effect,
        evidence_refs: refs,
    });
}
fn observe_events(
    h: &mut SideEffectHistory,
    snapshots: &[SQLiteSnapshot],
    a: Option<&AttemptRecord>,
    reference: &str,
) {
    for s in snapshots {
        if s.coverage != ObservationCoverage::Complete {
            push_event(
                h,
                HistoryKind::ObserverFailure,
                a,
                None,
                vec![reference.into()],
            );
        }
        for r in &s.rows {
            push_event(
                h,
                HistoryKind::EffectObservedCommitted,
                a,
                Some(r.identity.clone()),
                vec![reference.into()],
            );
        }
    }
}
fn derive_schedule(
    c: &SideEffectContract,
    parent: &str,
    si: usize,
    initial: Vec<SQLiteSnapshot>,
    attempts: &[AttemptRecord],
    refs: Vec<ArtifactRef>,
) -> io::Result<FaultScheduleResult> {
    let schedule = c.fault_schedules[si].clone();
    let mut h = SideEffectHistory {
        schema_version: "1".into(),
        run_id: format!("{parent}-s{si}"),
        events: vec![],
    };
    push_event(
        &mut h,
        HistoryKind::OperationStarted,
        None,
        None,
        vec![format!("{parent}/contract")],
    );
    observe_events(&mut h, &initial, None, &format!("{parent}/initial-{si}"));
    let mut known: BTreeMap<(String, String), CommittedRow> = BTreeMap::new();
    let mut schedule_complete = attempts.len() == schedule.primitives.len();
    for a in attempts {
        let runtime = &a.runtime.observation;
        let process_ref = format!("{}/process", a.identity.run_id);
        let committed_ref = format!("{}/committed", a.identity.run_id);
        if runtime.started {
            match a.requested {
                FaultPrimitive::Retry => push_event(
                    &mut h,
                    HistoryKind::RetryStarted,
                    Some(a),
                    None,
                    vec![process_ref.clone()],
                ),
                FaultPrimitive::DuplicateDelivery => push_event(
                    &mut h,
                    HistoryKind::DuplicateDeliveryStarted,
                    Some(a),
                    None,
                    vec![process_ref.clone()],
                ),
                _ => {}
            }
            push_event(
                &mut h,
                HistoryKind::AttemptStarted,
                Some(a),
                None,
                vec![process_ref.clone()],
            );
        }
        observe_events(&mut h, &a.commit_before_kill, Some(a), &committed_ref);
        if runtime.started && (runtime.exit_code.is_some() || runtime.signal.is_some()) {
            push_event(
                &mut h,
                if runtime.signal.is_some() {
                    HistoryKind::TargetKilled
                } else {
                    HistoryKind::TargetCompleted
                },
                Some(a),
                None,
                vec![process_ref],
            );
        }
        observe_events(&mut h, &a.after, Some(a), &committed_ref);
        schedule_complete &= runtime.started
            && runtime.runner_failure.is_none()
            && !runtime.timed_out
            && (runtime.exit_code.is_some() || runtime.signal.is_some())
            && (a.requested != FaultPrimitive::KillAfterCommit || killed(a));
        for snapshots in [&a.commit_before_kill, &a.after] {
            // Each complete snapshot must retain every previously committed row of its observer.
            for s in snapshots {
                if s.coverage != ObservationCoverage::Complete {
                    continue;
                }
                for old in known.values().filter(|r| {
                    c.effects.iter().any(|e| {
                        e.effect_id == r.effect_id && e.authoritative_observer == s.observer_id
                    })
                }) {
                    if !s.rows.contains(old) {
                        return Err(invalid(
                            "append-only committed ledger regressed or changed identity",
                        ));
                    }
                }
                for row in &s.rows {
                    let key = (
                        row.effect_id.clone(),
                        row.identity.external_effect_id.clone(),
                    );
                    if known
                        .insert(key, row.clone())
                        .is_some_and(|old| old != *row)
                    {
                        return Err(invalid("same committed ID has conflicting evidence"));
                    }
                }
            }
        }
    }
    push_event(
        &mut h,
        HistoryKind::ExperimentCompleted,
        None,
        None,
        refs.iter().map(|r| r.artifact_id.clone()).collect(),
    );
    let executed_faults = h
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                HistoryKind::RetryStarted | HistoryKind::DuplicateDeliveryStarted
            ) || (e.kind == HistoryKind::TargetKilled
                && attempts
                    .iter()
                    .any(|a| a.identity.run_id == e.run_id && killed(a)))
        })
        .cloned()
        .collect();
    Ok(FaultScheduleResult {
        schema_version: "1".into(),
        schedule,
        initial,
        attempts: refs,
        executed_faults,
        complete: schedule_complete,
        committed: known.into_values().collect(),
        history: h,
    })
}
fn evaluate(
    c: &SideEffectContract,
    schedules: &[FaultScheduleResult],
    attempts: &[Vec<AttemptRecord>],
) -> (Verdict, Vec<Violation>, Vec<String>) {
    let mut violations = vec![];
    let mut reasons = vec![];
    let mut error = false;
    if schedules.len() != c.fault_schedules.len() {
        reasons.push("Required exploration schedules incomplete (budget exhausted)".into());
    }
    for (s, runs) in schedules.iter().zip(attempts) {
        if !s.complete {
            reasons.push(format!(
                "{}: requested fault/attempt schedule not fully executed",
                s.schedule.schedule_id
            ));
        }
        let snapshots = s.initial.iter().chain(
            runs.iter()
                .flat_map(|a| a.commit_before_kill.iter().chain(&a.after)),
        );
        if snapshots
            .clone()
            .any(|s| s.coverage == ObservationCoverage::Failed)
        {
            error = true;
            reasons.push("SQLite observer schema/data/initialization failure".into());
        }
        if !complete(&s.initial) || runs.last().is_none_or(|a| !complete(&a.after)) {
            reasons.push("Committed-effect evidence incomplete".into());
        }
        if !rows(&s.initial).is_empty() {
            error = true;
            reasons.push(
                "Initial fixture already contains this operation: refusing baseline poisoning"
                    .into(),
            );
        }
        if runs
            .iter()
            .any(|a| a.runtime.observation.runner_failure.is_some())
        {
            error = true;
            reasons.push("Process runner infrastructure failure".into());
        }
        let covered = |effect: &EffectContract| {
            complete(&s.initial)
                && runs.last().is_some_and(|a| {
                    a.after.iter().any(|o| {
                        o.observer_id == effect.authoritative_observer
                            && o.coverage == ObservationCoverage::Complete
                    })
                })
                && s.complete
        };
        let selected = |id: &str| {
            s.committed
                .iter()
                .filter(|r| r.effect_id == id)
                .collect::<Vec<_>>()
        };
        let refs: Vec<_> = s
            .attempts
            .iter()
            .map(|a| format!("{}/committed", a.artifact_id))
            .collect();
        for e in &c.effects {
            let count = selected(&e.effect_id).len();
            let kind = match e.expectation {
                Expectation::ExactlyOnce | Expectation::AtMostOnce if count > 1 => {
                    Some(ViolationKind::DuplicateCommittedEffect)
                }
                Expectation::ExactlyOnce | Expectation::AtLeastOnce if count == 0 && covered(e) => {
                    Some(ViolationKind::MissingCommittedEffect)
                }
                Expectation::Never if count > 0 => Some(ViolationKind::ForbiddenCommittedEffect),
                _ => None,
            };
            if let Some(kind) = kind {
                violations.push(Violation {
                    schedule_id: s.schedule.schedule_id.clone(),
                    effect_ids: vec![e.effect_id.clone()],
                    kind,
                    expected: format!("{}: {:?}", e.effect_id, e.expectation),
                    observed: format!("{} committed effects", count),
                    evidence_refs: refs.clone(),
                });
            }
        }
        for relation in &c.relations {
            let (a, b, kind) = match relation {
                Relation::OrderedAfter { effect, after } => {
                    (effect, after, ViolationKind::CommitOrder)
                }
                Relation::AtomicWith { effect, other } => {
                    (effect, other, ViolationKind::HalfCommit)
                }
            };
            let (ar, br) = (selected(a), selected(b));
            let both_covered = [a, b]
                .iter()
                .all(|id| covered(c.effects.iter().find(|e| &e.effect_id == *id).unwrap()));
            let broken = match kind {
                ViolationKind::CommitOrder => {
                    ar.iter().any(|a| {
                        br.iter()
                            .any(|b| a.identity.commit_order <= b.identity.commit_order)
                    }) || (both_covered && !ar.is_empty() && br.is_empty())
                }
                ViolationKind::HalfCommit => both_covered && (ar.is_empty() != br.is_empty()),
                _ => false,
            };
            if broken {
                violations.push(Violation {
                    schedule_id: s.schedule.schedule_id.clone(),
                    effect_ids: vec![a.clone(), b.clone()],
                    kind,
                    expected: format!("{relation:?}"),
                    observed: format!("{}: {} commits; {}: {} commits", a, ar.len(), b, br.len()),
                    evidence_refs: refs.clone(),
                });
            }
        }
    }
    reasons.sort();
    reasons.dedup();
    (
        if error {
            Verdict::Error
        } else if !violations.is_empty() {
            Verdict::Fail
        } else if !reasons.is_empty() {
            Verdict::Inconclusive
        } else {
            Verdict::Pass
        },
        violations,
        reasons,
    )
}
fn replayability() -> verify_replay::Replayability {
    verify_replay::Replayability::Unavailable { reason:"Automatic replay is unavailable; recorded inputs and actually re-executed counterexample artifacts are retained when reduction budget allows.".into() }
}
fn limitations() -> Vec<String> {
    vec!["Bounded local Unix exploration; no exhaustive proof, sandbox or secrecy claim.".into(),"SQLite table must be an append-only durable committed ledger; one shared integer commit sequence is required for ordering. Detached writers and hostile targets are outside this trust boundary.".into(),"Hashes detect corruption, not authenticated wholesale artifact rewriting. Agent export omits raw paths, environment and ledger data.".into(),"Reduction establishes local single-action minimality only after actual reruns; automatic replay CLI is unavailable.".into()]
}

/// Check initial observer readiness without triggering an effect or assigning a verdict.
pub fn check_prerequisites(contract: &SideEffectContract) -> io::Result<()> {
    contract.validate()?;
    if behavior::executable_identity(&contract.trigger.executable)?
        != contract.trigger.executable_hash
        || behavior::snapshot_identity(contract.trigger.fixture.source.as_deref())?
            != contract.trigger.fixture.snapshot_identity
    {
        return Err(invalid("target or fixture identity mismatch"));
    }
    let workspace = contract
        .trigger
        .fixture
        .source
        .as_deref()
        .ok_or_else(|| invalid("SQLite fixture required for readiness"))?;
    if sqlite::observe_all(workspace, contract)
        .iter()
        .any(|s| s.coverage != ObservationCoverage::Complete || !s.rows.is_empty())
    {
        return Err(invalid("initial SQLite observer prerequisites unavailable"));
    }
    Ok(())
}

pub fn execute(
    contract: &SideEffectContract,
    store: &EvidenceStore,
    id: &str,
) -> io::Result<SideEffectExperimentResult> {
    execute_inner(contract, store, id, true)
}
fn execute_inner(
    c: &SideEffectContract,
    store: &EvidenceStore,
    id: &str,
    reduce: bool,
) -> io::Result<SideEffectExperimentResult> {
    c.validate()?;
    if id.is_empty()
        || id.len() > (if reduce { 64 } else { 96 })
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("invalid SideEffect experiment ID"));
    }
    let run = store.reserve(id)?; // reserve before any target executes
    let snapshot = Snapshot::capture(c.trigger.fixture.source.as_deref())?;
    if snapshot.identity()? != c.trigger.fixture.snapshot_identity
        || behavior::executable_identity(&c.trigger.executable)? != c.trigger.executable_hash
    {
        return Err(invalid("fixture or executable identity changed"));
    }
    let mut result = SideEffectExperimentResult {
        schema_version: "1".into(),
        sideeffect_result_id: id.into(),
        contract: c.clone(),
        schedules: vec![],
        verdict: Verdict::Error,
        violations: vec![],
        reasons: vec![],
        counterexample: None,
        limitations: limitations(),
        replayability: replayability(),
    };
    let mut all_attempts = vec![];
    let mut remaining = c.exploration_budget.max_attempts;
    for (si, schedule) in c
        .fault_schedules
        .iter()
        .take(c.exploration_budget.max_schedules)
        .enumerate()
    {
        let directory = tempfile::tempdir()?;
        let workspace = directory.path().join("fixture");
        snapshot.materialize(&workspace)?;
        let initial = sqlite::observe_all(&workspace, c);
        let mut attempts = vec![];
        let mut refs = vec![];
        for (ai, primitive) in schedule.primitives.iter().enumerate() {
            if remaining == 0 {
                break;
            }
            remaining -= 1;
            let aid = format!("{id}-s{si}-a{ai}");
            let child = store.reserve(&aid)?;
            let process = ProcessSpec {
                executable: c.trigger.executable.clone(),
                args: c.trigger.args.clone(),
                working_directory: workspace.clone(),
                environment: c.trigger.environment.clone(),
                timeout_ms: c.trigger.timeout_ms,
            };
            let mut commit_before_kill = vec![];
            let runtime = observe_controlled(&process, &mut || {
                if *primitive != FaultPrimitive::KillAfterCommit {
                    return Ok(false);
                }
                let observed = sqlite::observe_all(&workspace, c);
                if complete(&observed) && !rows(&observed).is_empty() {
                    commit_before_kill = observed;
                    Ok(true)
                } else {
                    Ok(false)
                }
            });
            let a = AttemptRecord {
                schema_version: "1".into(),
                identity: AttemptIdentity {
                    attempt_id: format!("attempt-{ai}"),
                    run_id: aid.clone(),
                    operation_id: c.operation.operation_id.clone(),
                },
                contract_hash: canonical_hash(c)?,
                schedule_id: schedule.schedule_id.clone(),
                action_index: ai,
                requested: *primitive,
                process,
                runtime,
                commit_before_kill,
                after: sqlite::observe_all(&workspace, c),
            };
            attempt_evidence(&child, &a)?;
            refs.push(reference(&aid, &a)?);
            attempts.push(a);
        }
        if behavior::executable_identity(&c.trigger.executable)? != c.trigger.executable_hash {
            return Err(invalid("executable changed during experiment"));
        }
        result
            .schedules
            .push(derive_schedule(c, id, si, initial, &attempts, refs)?);
        all_attempts.push(attempts);
    }
    (result.verdict, result.violations, result.reasons) =
        evaluate(c, &result.schedules, &all_attempts);
    if reduce && result.verdict == Verdict::Fail {
        result.counterexample = Some(reduction::execute(&result, store)?);
    }
    let mut ids = vec![];
    save(
        &run,
        id,
        "contract",
        "sideeffect_contract",
        TrustClass::DirectRuntime,
        c,
        &mut ids,
    )?;
    for (si, s) in result.schedules.iter().enumerate() {
        save(
            &run,
            id,
            &format!("initial-{si}"),
            "sqlite_committed",
            TrustClass::AuthoritativeTargetState,
            &s.initial,
            &mut ids,
        )?;
        save(
            &run,
            id,
            &format!("history-{si}"),
            "sideeffect_history",
            TrustClass::DirectRuntime,
            &s.history,
            &mut ids,
        )?;
    }
    save(
        &run,
        id,
        "execution",
        "sideeffect_scheduler",
        TrustClass::DirectRuntime,
        &(
            result
                .schedules
                .iter()
                .map(|s| &s.attempts)
                .collect::<Vec<_>>(),
            &result.counterexample,
        ),
        &mut ids,
    )?;
    run.complete(&result, &ids)?;
    load(store, id)
}
pub fn load(store: &EvidenceStore, id: &str) -> io::Result<SideEffectExperimentResult> {
    load_inner(store, id, true)
}
fn load_inner(
    store: &EvidenceStore,
    id: &str,
    allow_reduction: bool,
) -> io::Result<SideEffectExperimentResult> {
    let (raw, evidence) = store.load(id)?;
    let mut r: SideEffectExperimentResult = parse(raw)?;
    let c = &r.contract;
    c.validate()?;
    if r.schema_version != "1"
        || r.sideeffect_result_id != id
        || r.schedules.len()
            != c.fault_schedules
                .len()
                .min(c.exploration_budget.max_schedules)
        || evidence.len() != 2 + r.schedules.len() * 2
        || r.limitations != limitations()
        || r.replayability != replayability()
        || (!allow_reduction && r.counterexample.is_some())
    {
        return Err(invalid("invalid SideEffect experiment envelope"));
    }
    require_evidence(
        &evidence,
        id,
        "contract",
        "sideeffect_contract",
        TrustClass::DirectRuntime,
        c,
    )?;
    require_evidence(
        &evidence,
        id,
        "execution",
        "sideeffect_scheduler",
        TrustClass::DirectRuntime,
        &(
            r.schedules.iter().map(|s| &s.attempts).collect::<Vec<_>>(),
            &r.counterexample,
        ),
    )?;
    let mut attempts = vec![];
    let mut remaining = c.exploration_budget.max_attempts;
    for (si, s) in r.schedules.iter().enumerate() {
        if s.schedule != c.fault_schedules[si]
            || s.attempts.len() != s.schedule.primitives.len().min(remaining)
        {
            return Err(invalid("required attempt/history schedule missing"));
        }
        remaining -= s.attempts.len();
        require_evidence(
            &evidence,
            id,
            &format!("initial-{si}"),
            "sqlite_committed",
            TrustClass::AuthoritativeTargetState,
            &s.initial,
        )?;
        require_evidence(
            &evidence,
            id,
            &format!("history-{si}"),
            "sideeffect_history",
            TrustClass::DirectRuntime,
            &s.history,
        )?;
        sqlite::verify_snapshots(&s.initial, c)?;
        let runs = s
            .attempts
            .iter()
            .enumerate()
            .map(|(ai, a)| load_attempt(store, a, c, id, si, ai))
            .collect::<io::Result<Vec<_>>>()?;
        if runs
            .windows(2)
            .any(|w| w[0].process.working_directory != w[1].process.working_directory)
            || derive_schedule(c, id, si, s.initial.clone(), &runs, s.attempts.clone())? != *s
        {
            return Err(invalid(
                "history/committed effects/fault execution contradict actual run evidence",
            ));
        }
        attempts.push(runs);
    }
    // Stored verdict and checker output are only caches. Recompute from verified inputs.
    (r.verdict, r.violations, r.reasons) = evaluate(c, &r.schedules, &attempts);
    if let Some(counterexample) = &r.counterexample {
        reduction::verify(&r, counterexample, store)?;
    }
    Ok(r)
}
pub fn contract_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(SideEffectContract)).unwrap()
}
pub fn result_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(SideEffectExperimentResult)).unwrap()
}
pub fn history_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(SideEffectHistory)).unwrap()
}
