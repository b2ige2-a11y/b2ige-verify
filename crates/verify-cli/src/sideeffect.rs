use super::*;
use serde::{Deserialize, Serialize};
use verify_core::sideeffect as se;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectSummary {
    pub effect_id: String,
    pub expectation: se::Expectation,
    pub committed: Option<usize>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleSummary {
    pub schedule_id: String,
    pub attempts: usize,
    pub executed: bool,
    pub effects: Vec<EffectSummary>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SideEffectReport {
    pub focused_schedule: String,
    pub attempts: usize,
    pub committed: Option<usize>,
    pub effects: Vec<EffectSummary>,
    pub schedules: Vec<ScheduleSummary>,
    pub timeline: Vec<se::HistoryEvent>,
    pub counterexample: Option<se::SideEffectCounterexample>,
    pub violations: Vec<se::Violation>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SideEffectAgent {
    pub attempts: usize,
    pub effect_ids: Vec<String>,
    pub reproduction_status: Option<se::ReductionStatus>,
}
pub(super) fn load(store: &EvidenceStore, id: &str) -> io::Result<VerifiedReport> {
    let r = se::load(store, id)?;
    let raw = serde_json::to_value(&r)?;
    let source = reference(id, &r)?;
    let mut evidence_summaries = vec![];
    for aid in std::iter::once(id).chain(
        r.schedules
            .iter()
            .flat_map(|s| s.attempts.iter().map(|a| a.artifact_id.as_str())),
    ) {
        let (_, evidence) = store.load(aid)?;
        for e in evidence {
            evidence_summaries.push(EvidenceRef {
                run_id: aid.into(),
                evidence_id: e.evidence_id.clone(),
                evidence_hash: canonical_hash(&e)?,
            });
        }
    }
    let mut schedules = vec![];
    for s in &r.schedules {
        let last = s
            .attempts
            .last()
            .map(|a| store.load(&a.artifact_id))
            .transpose()?
            .map(|(v, _)| serde_json::from_value::<se::AttemptRecord>(v))
            .transpose()?;
        let effects = r
            .contract
            .effects
            .iter()
            .map(|e| EffectSummary {
                effect_id: e.effect_id.clone(),
                expectation: e.expectation,
                committed: last
                    .as_ref()
                    .filter(|a| {
                        a.after.iter().any(|o| {
                            o.observer_id == e.authoritative_observer
                                && o.coverage == verify_evidence::ObservationCoverage::Complete
                        })
                    })
                    .map(|_| {
                        s.committed
                            .iter()
                            .filter(|r| r.effect_id == e.effect_id)
                            .count()
                    }),
            })
            .collect();
        schedules.push(ScheduleSummary {
            schedule_id: s.schedule.schedule_id.clone(),
            attempts: s.attempts.len(),
            executed: s.complete,
            effects,
        });
    }
    let focus = r.violations.first().map(|v| v.schedule_id.as_str());
    let focused = schedules
        .iter()
        .find(|s| Some(s.schedule_id.as_str()) == focus)
        .or_else(|| schedules.last());
    let headline = match r.verdict {
        Verdict::Pass => "Committed effects satisfy contract",
        Verdict::Fail
            if r.violations
                .iter()
                .any(|v| v.kind == se::ViolationKind::DuplicateCommittedEffect) =>
        {
            "Duplicate committed effect proven"
        }
        Verdict::Fail => "Committed-effect contract violation proven",
        Verdict::Inconclusive => "Committed-effect evidence incomplete",
        Verdict::Error => "SideEffect verification could not complete",
    };
    let effect_report = SideEffectReport {
        focused_schedule: focused.map(|s| s.schedule_id.clone()).unwrap_or_default(),
        attempts: focused.map_or(0, |s| s.attempts),
        committed: focused.and_then(|s| s.effects.iter().map(|e| e.committed).sum()),
        effects: focused.map(|s| s.effects.clone()).unwrap_or_default(),
        schedules,
        timeline: r
            .schedules
            .iter()
            .flat_map(|s| s.history.events.clone())
            .collect(),
        counterexample: r.counterexample.clone(),
        violations: r.violations.clone(),
    };
    let related_artifacts = r
        .counterexample
        .iter()
        .flat_map(|x| &x.trials)
        .map(|t| ArtifactRef {
            artifact_id: t.result.artifact_id.clone(),
            integrity_hash: t.result.integrity_hash.clone(),
        })
        .collect();
    let mut expected: Vec<Value> = effect_report
        .effects
        .iter()
        .map(|e| json!({"effect_id":e.effect_id,"expectation":e.expectation}))
        .collect();
    expected.extend(
        r.contract
            .relations
            .iter()
            .map(|relation| json!({"relation":relation})),
    );
    let mut observed: Vec<Value> = effect_report
        .effects
        .iter()
        .map(|e| json!({"effect_id":e.effect_id,"committed":e.committed}))
        .collect();
    observed.extend(
        r.violations
            .iter()
            .filter(|v| {
                matches!(
                    v.kind,
                    se::ViolationKind::CommitOrder | se::ViolationKind::HalfCommit
                )
            })
            .map(|v| json!({"violation":v.kind,"effect_ids":v.effect_ids,"observed":v.observed})),
    );
    let document = ReportDocument {
        schema_version:"3".into(),projection_version:"3".into(),report_id:format!("report-{}",source.integrity_hash.trim_start_matches("sha256:")),product:"sideeffect".into(),kind:ReportKind::SideEffectProof,source:source.clone(),verdict:r.verdict,outcome:"SIDEEFFECT_PROOF".into(),headline:headline.into(),reason:if let Some(v) = r.violations.first() {format!("Expected {}. Observed {}.",v.expected,v.observed)} else if r.reasons.is_empty() {"All counts refer to committed ledger effects within each independently reset schedule.".into()} else {r.reasons.join("; ")},expected:Some(json!(expected)),observed:Some(json!(observed)),primary_failure:None,other_failures:vec![],coverage:json!({"required_schedules":r.contract.fault_schedules.len(),"executed_schedules":r.schedules.iter().filter(|s|s.complete).count(),"budget":r.contract.exploration_budget,"schedules":effect_report.schedules}),reproduction:None,evidence_summaries,run_references:vec![],limitations:r.limitations,replayability:Replayability::Unavailable {reason:"Actual reproduction artifacts are linked when present; automatic replay CLI is unavailable.".into()},related_artifacts,raw_artifact:source,sideeffect:Some(effect_report),blindtest:None,
    };
    Ok(VerifiedReport { document, raw })
}
pub(super) fn agent(report: &VerifiedReport) -> AgentReport {
    let d = &report.document;
    let s = d.sideeffect.as_ref().unwrap();
    AgentReport {
        schema_version: "3".into(),
        blindtest: None,
        disclosure_policy: "sideeffect.public-references.v1".into(),
        source: d.source.clone(),
        verdict: d.verdict,
        kind: d.kind.clone(),
        outcome: d.outcome.clone(),
        summary: d.headline.clone(),
        expected: d.expected.clone(),
        observed: d.observed.clone(),
        reproduction: s
            .counterexample
            .as_ref()
            .and_then(|r| r.reproduction.as_ref())
            .map(|r| ArtifactRef {
                artifact_id: r.artifact_id.clone(),
                integrity_hash: r.integrity_hash.clone(),
            }),
        reduction_status: None,
        evidence_refs: d
            .evidence_summaries
            .iter()
            .filter(|e| e.evidence_id == "committed")
            .take(8)
            .cloned()
            .collect(),
        evidence_count: d.evidence_summaries.len(),
        limitations: vec![
            "Bounded committed-state evidence; counts are per focused schedule.".into(),
        ],
        replayability: d.replayability.clone(),
        sideeffect: Some(SideEffectAgent {
            attempts: s.attempts,
            effect_ids: s.effects.iter().map(|e| e.effect_id.clone()).collect(),
            reproduction_status: s.counterexample.as_ref().map(|x| x.status),
        }),
    }
}
pub(super) fn expectation(e: se::Expectation) -> &'static str {
    match e {
        se::Expectation::ExactlyOnce => "× 1",
        se::Expectation::AtMostOnce => "≤ 1",
        se::Expectation::AtLeastOnce => "≥ 1",
        se::Expectation::Never => "× 0",
    }
}
pub(super) fn human(s: &SideEffectReport) -> String {
    let mut out = format!(
        "\nSchedule\n{}\n\nAttempts      {}\nCommitted     {}\n",
        s.focused_schedule,
        s.attempts,
        s.committed
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unknown".into())
    );
    for e in &s.effects {
        out.push_str(&format!(
            "\nExpected\n{} {}\n\nObserved\n{} × {}\n",
            e.effect_id,
            expectation(e.expectation),
            e.effect_id,
            e.committed
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unknown".into())
        ));
    }
    if let Some(x) = &s.counterexample {
        out.push_str(&format!(
            "\n{}\n",
            if x.status == se::ReductionStatus::LocallyMinimized {
                "Locally minimized reproduction"
            } else {
                "Recorded reproduction (no minimality claim)"
            }
        ));
        for (i, step) in x.steps.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, step));
        }
        out.push_str(&format!("{:?} · {}\n", x.status, x.replayability));
    }
    out.push_str(&format!(
        "\nTimeline · {} events\nRuns · {}\n",
        s.timeline.len(),
        s.schedules.iter().map(|s| s.attempts).sum::<usize>()
    ));
    out
}
