//! Deterministic single-action deletion with actual executions. P3's assignment
//! reducer remains unchanged. Minimality is local, bounded and signature-relative.
use super::*;
fn signature(v: &Violation) -> ViolationSignature {
    ViolationSignature {
        effect_ids: v.effect_ids.clone(),
        kind: v.kind,
    }
}
fn candidates(schedule: &FaultSchedule) -> Vec<FaultSchedule> {
    let mut result = vec![];
    for i in 0..schedule.primitives.len() {
        let mut c = schedule.clone();
        c.primitives.remove(i);
        if matches!(
            c.primitives.first(),
            Some(FaultPrimitive::Retry | FaultPrimitive::DuplicateDelivery)
        ) {
            c.primitives[0] = FaultPrimitive::None;
        }
        if schedule_valid(&c) && !result.contains(&c) {
            result.push(c);
        }
    }
    for (i, p) in schedule.primitives.iter().enumerate() {
        if *p == FaultPrimitive::KillAfterCommit {
            let mut c = schedule.clone();
            c.primitives[i] = FaultPrimitive::None;
            if !result.contains(&c) {
                result.push(c);
            }
        }
    }
    result
}
fn child_contract(c: &SideEffectContract, schedule: &FaultSchedule) -> SideEffectContract {
    let mut c = c.clone();
    c.fault_schedules = vec![schedule.clone()];
    c.exploration_budget = ExplorationBudget {
        max_schedules: 1,
        max_attempts: schedule.primitives.len(),
        reduction_executions: 0,
    };
    c
}
fn steps(child: &SideEffectExperimentResult) -> Vec<String> {
    let mut observed = BTreeSet::new();
    child
        .schedules
        .iter()
        .flat_map(|s| &s.history.events)
        .filter_map(|e| match e.kind {
            HistoryKind::AttemptStarted => Some("Start operation attempt".into()),
            HistoryKind::EffectObservedCommitted => {
                let effect = e.related_effect.as_ref()?;
                if observed.insert((
                    &effect.provider,
                    &effect.operation,
                    &effect.external_effect_id,
                )) {
                    Some(format!("{} committed", effect.operation))
                } else {
                    None
                }
            }
            HistoryKind::TargetKilled => {
                Some("Terminate target after committed effect was observed".into())
            }
            HistoryKind::RetryStarted => Some("Retry the same logical operation".into()),
            HistoryKind::DuplicateDeliveryStarted => {
                Some("Deliver the same operation input again".into())
            }
            _ => None,
        })
        .collect()
}
fn reduce(
    root: &SideEffectExperimentResult,
    mut execute: impl FnMut(usize, &SideEffectContract) -> io::Result<SideEffectExperimentResult>,
) -> io::Result<SideEffectCounterexample> {
    let first = root
        .violations
        .first()
        .ok_or_else(|| invalid("counterexample has no proven violation"))?;
    let original = root
        .contract
        .fault_schedules
        .iter()
        .find(|s| s.schedule_id == first.schedule_id)
        .ok_or_else(|| invalid("counterexample schedule missing"))?
        .clone();
    let sig = signature(first);
    let mut result = SideEffectCounterexample {
        original_schedule: original.clone(),
        retained_schedule: original.clone(),
        signature: sig.clone(),
        status: ReductionStatus::BudgetExhausted,
        trials: vec![],
        reproduction: None,
        steps: vec![],
        replayability: "unavailable: no successful reproduction within reduction budget".into(),
    };
    let mut queue = vec![original];
    let mut index = 0;
    while index < queue.len() {
        if result.trials.len() >= root.contract.exploration_budget.reduction_executions {
            return Ok(result);
        }
        let candidate = queue[index].clone();
        let child = execute(
            result.trials.len(),
            &child_contract(&root.contract, &candidate),
        )?;
        let preserved = child.verdict == Verdict::Fail
            && child.schedules.iter().all(|s| s.complete)
            && child.violations.iter().any(|v| signature(v) == sig);
        let child_ref = reference(&child.sideeffect_result_id, &child)?;
        result.trials.push(ReductionTrial {
            schedule: candidate.clone(),
            result: child_ref.clone(),
            preserved,
        });
        if result.trials.len() == 1 && !preserved {
            result.status = ReductionStatus::Unreproduced;
            return Ok(result);
        }
        if preserved {
            result.retained_schedule = candidate.clone();
            result.reproduction = Some(child_ref);
            result.steps = steps(&child);
            result.replayability =
                "reproduced by recorded local execution; future outcome is not guaranteed".into();
            queue = candidates(&candidate);
            index = 0;
        } else {
            index += 1;
        }
    }
    result.status = ReductionStatus::LocallyMinimized;
    Ok(result)
}
pub(super) fn execute(
    root: &SideEffectExperimentResult,
    store: &EvidenceStore,
) -> io::Result<SideEffectCounterexample> {
    reduce(root, |i, c| {
        execute_inner(
            c,
            store,
            &format!("{}-r{i}", root.sideeffect_result_id),
            false,
        )
    })
}
pub(super) fn verify(
    root: &SideEffectExperimentResult,
    recorded: &SideEffectCounterexample,
    store: &EvidenceStore,
) -> io::Result<()> {
    if root.verdict != Verdict::Fail {
        return Err(invalid("counterexample without FAIL"));
    }
    let mut read = 0;
    let derived = reduce(root, |i, c| {
        let trial = recorded
            .trials
            .get(i)
            .ok_or_else(|| invalid("reduction trial missing"))?;
        let id = format!("{}-r{i}", root.sideeffect_result_id);
        if trial.result.artifact_id != id {
            return Err(invalid("cross-run reduction child"));
        }
        let child = load_inner(store, &id, false)?;
        if child.contract != *c || canonical_hash(&child)? != trial.result.integrity_hash {
            return Err(invalid("reproduction contract or evidence changed"));
        }
        read += 1;
        Ok(child)
    })?;
    if derived != *recorded || read != recorded.trials.len() {
        return Err(invalid("reduction/minimality contradicts actual reruns"));
    }
    Ok(())
}
