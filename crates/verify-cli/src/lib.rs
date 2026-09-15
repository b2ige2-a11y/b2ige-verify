//! Read-only presentation. Only product verified loaders may construct VerifiedReport.
//! Report/agent JSON is an export, never accepted as authoritative input.
pub mod agent;
pub mod bench;
mod blindtest;
pub mod integration;
mod sideeffect;
pub use blindtest::{BlindTestAgent, BlindTestReport};
pub mod viewer;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{json, Value};
pub use sideeffect::{SideEffectAgent, SideEffectReport};
use std::{
    io,
    path::{Path, PathBuf},
};
use verify_core::{
    behavior::{
        self, generation, reduction, stability, BehaviorAuthorization, BehaviorComparisonResult,
        Divergence, EvidenceRef, RunRecord,
    },
    Verdict,
};
use verify_evidence::{canonical_hash, store::EvidenceStore};
use verify_replay::Replayability;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    BehaviorExact,
    BehaviorStability,
    GeneratedSuite,
    CounterexampleReduction,
    SideEffectProof,
    BlindTest,
}
#[derive(Debug, Clone, Serialize, serde::Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub integrity_hash: String,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub comparison_id: String,
    pub divergence: Divergence,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reproduction {
    pub label: String,
    pub comparison: ArtifactRef,
    pub experiment: behavior::BehaviorExperiment,
    pub original_assignments: Option<usize>,
    pub retained_assignments: Option<usize>,
    pub reduction_status: Option<reduction::ReductionStatus>,
    pub minimality_claim: Option<String>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReportDocument {
    pub schema_version: String,
    pub projection_version: String,
    pub report_id: String,
    pub product: String,
    pub kind: ReportKind,
    pub source: ArtifactRef,
    pub verdict: Verdict,
    pub outcome: String,
    pub headline: String,
    pub reason: String,
    pub expected: Option<Value>,
    pub observed: Option<Value>,
    pub primary_failure: Option<Failure>,
    pub other_failures: Vec<Failure>,
    pub coverage: Value,
    pub reproduction: Option<Reproduction>,
    pub evidence_summaries: Vec<EvidenceRef>,
    pub run_references: Vec<RunRecord>,
    pub limitations: Vec<String>,
    pub replayability: Replayability,
    pub related_artifacts: Vec<ArtifactRef>,
    pub raw_artifact: ArtifactRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sideeffect: Option<SideEffectReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blindtest: Option<BlindTestReport>,
}
/// Explicit allowlist, no raw artifact, paths, environment, inventory or oracle.
/// Future products must opt into a disclosure policy before agent export.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentReport {
    pub schema_version: String,
    pub disclosure_policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sideeffect: Option<SideEffectAgent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blindtest: Option<BlindTestAgent>,
    pub source: ArtifactRef,
    pub verdict: Verdict,
    pub kind: ReportKind,
    pub outcome: String,
    pub summary: String,
    pub expected: Option<Value>,
    pub observed: Option<Value>,
    pub reproduction: Option<ArtifactRef>,
    pub reduction_status: Option<reduction::ReductionStatus>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub evidence_count: usize,
    pub limitations: Vec<String>,
    pub replayability: Replayability,
}
/// Cannot be deserialized or constructed outside the verified loading boundary.
pub struct VerifiedReport {
    document: ReportDocument,
    raw: Value,
}
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn reference(id: &str, raw: &impl Serialize) -> io::Result<ArtifactRef> {
    Ok(ArtifactRef {
        artifact_id: id.into(),
        integrity_hash: canonical_hash(raw)?,
    })
}
fn headline(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "No divergence found\nwithin tested behavior space",
        Verdict::Fail => "Behavior divergence proven",
        Verdict::Inconclusive => "Verification evidence incomplete",
        Verdict::Error => "Verification could not complete",
    }
}
pub fn badge(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "✓ VERIFIED",
        Verdict::Fail => "✕ FAILED",
        Verdict::Inconclusive => "? INCONCLUSIVE",
        Verdict::Error => "! ERROR",
    }
}
fn exact_details(d: &mut ReportDocument, r: &BehaviorComparisonResult) -> io::Result<()> {
    d.reason = r.reason.clone();
    d.coverage = json!({"scope":r.scope,"observers":r.coverage,"cases":1});
    d.limitations.extend(r.limitations.clone());
    d.replayability = r.replayability.clone();
    d.evidence_summaries.extend(r.evidence_refs.clone());
    d.run_references
        .extend(r.before.iter().chain(r.after.iter()).cloned());
    d.other_failures
        .extend(r.divergences.iter().cloned().map(|divergence| Failure {
            comparison_id: r.comparison_id.clone(),
            divergence,
        }));
    d.reproduction = Some(Reproduction {
        label: "Recorded reproduction (unreduced)".into(),
        comparison: reference(&r.comparison_id, r)?,
        experiment: r.experiment.clone(),
        original_assignments: None,
        retained_assignments: None,
        reduction_status: None,
        minimality_claim: None,
    });
    Ok(())
}
/// Store reads below are used ONLY to dispatch the product loader. No raw field
/// is projected before its product loader has verified/recomputed the artifact.
pub fn load(
    store: &EvidenceStore,
    id: &str,
    auth: &BehaviorAuthorization,
) -> io::Result<VerifiedReport> {
    let (hint, _) = store.load(id)?;
    if hint.get("blindtest_result_id").is_some() {
        return blindtest::load(store, id);
    }
    if hint.get("sideeffect_result_id").is_some() {
        return sideeffect::load(store, id);
    }
    let (kind, raw, verdict, outcome) = if hint.get("reduction_id").is_some() {
        let r = reduction::load(store, id, auth)?;
        // Reduction status is not a verdict. The best proven comparison owns it.
        let c = behavior::load(store, &r.final_comparison.comparison_id, auth)?;
        (
            ReportKind::CounterexampleReduction,
            serde_json::to_value(&r)?,
            c.verdict,
            serde_json::to_value(r.status)?.as_str().unwrap().into(),
        )
    } else if hint.get("suite_id").is_some() {
        let r = generation::load(store, id, auth)?;
        (
            ReportKind::GeneratedSuite,
            serde_json::to_value(&r)?,
            r.aggregate_verdict,
            "GENERATED_SUITE".into(),
        )
    } else if hint.get("profile").is_some() {
        let r = stability::load_comparison(store, id, auth)?;
        (
            ReportKind::BehaviorStability,
            serde_json::to_value(&r)?,
            r.verdict,
            serde_json::to_value(r.outcome)?.as_str().unwrap().into(),
        )
    } else if hint.get("comparison_id").is_some() {
        let r = behavior::load(store, id, auth)?;
        (
            ReportKind::BehaviorExact,
            serde_json::to_value(&r)?,
            r.verdict,
            serde_json::to_value(r.outcome)?.as_str().unwrap().into(),
        )
    } else {
        return Err(invalid(
            "unsupported source: reports are projections, not verified artifacts",
        ));
    };
    let source = reference(id, &raw)?;
    let mut d = ReportDocument {
        schema_version:"3".into(),projection_version:"3".into(),report_id:format!("report-{}",source.integrity_hash.trim_start_matches("sha256:")),product:"behavior".into(),kind,
        source:source.clone(),verdict,outcome,headline:headline(verdict).into(),reason:String::new(),expected:None,observed:None,primary_failure:None,other_failures:vec![],coverage:Value::Null,reproduction:None,evidence_summaries:vec![],run_references:vec![],limitations:vec!["Presentation projection; source loaders own the verdict. Hashes establish integrity, not authentication.".into()],
        replayability:Replayability::Unavailable{reason:"Automatic paired replay is unavailable; recorded experiments retain controllable inputs.".into()},related_artifacts:vec![],raw_artifact:source,sideeffect:None,blindtest:None,
    };
    match d.kind {
        ReportKind::SideEffectProof | ReportKind::BlindTest => {
            unreachable!("separate verified projection")
        }
        ReportKind::BehaviorExact => {
            let r = serde_json::from_value(raw.clone())?;
            exact_details(&mut d, &r)?;
        }
        ReportKind::BehaviorStability => {
            let r: stability::StabilityComparisonResult = serde_json::from_value(raw.clone())?;
            let p = stability::load_profile(store, &r.profile.profile_id, auth)?;
            d.reason = r.reason;
            d.coverage = json!({"baseline_repetitions":p.repetition_count,"observables":p.observables,"profiling_budget":p.config,"cases":1});
            d.evidence_summaries = r.evidence_refs;
            d.run_references = p.runs;
            d.run_references.push(r.candidate);
            d.other_failures = r
                .divergences
                .into_iter()
                .map(|divergence| Failure {
                    comparison_id: id.into(),
                    divergence,
                })
                .collect();
            d.replayability = r.replayability;
            d.limitations.push("Bounded baseline stability profiling; no exhaustive proof or future stability guarantee.".into());
            d.reproduction = Some(Reproduction {
                label: "Recorded stability experiment".into(),
                comparison: d.source.clone(),
                experiment: r.experiment,
                original_assignments: None,
                retained_assignments: None,
                reduction_status: None,
                minimality_claim: None,
            });
        }
        ReportKind::GeneratedSuite => {
            let r: generation::BehaviorGeneratedSuiteResult = serde_json::from_value(raw.clone())?;
            for c in &r.comparisons {
                let child = behavior::load(store, &c.comparison_id, auth)?;
                if canonical_hash(&child)? != c.comparison_hash {
                    return Err(invalid("suite child changed during projection"));
                }
                d.related_artifacts
                    .push(reference(&c.comparison_id, &child)?);
                d.evidence_summaries.extend(child.evidence_refs.clone());
                d.run_references
                    .extend(child.before.iter().chain(child.after.iter()).cloned());
                d.other_failures
                    .extend(
                        child
                            .divergences
                            .iter()
                            .take(1)
                            .cloned()
                            .map(|divergence| Failure {
                                comparison_id: c.comparison_id.clone(),
                                divergence,
                            }),
                    );
                if d.reproduction.is_none() && child.verdict == Verdict::Fail {
                    d.reproduction = Some(Reproduction {
                        label: "Recorded reproduction (unreduced)".into(),
                        comparison: reference(&child.comparison_id, &child)?,
                        experiment: child.experiment.clone(),
                        original_assignments: None,
                        retained_assignments: None,
                        reduction_status: None,
                        minimality_claim: None,
                    });
                }
            }
            d.reason = format!(
                "{} proven divergences · {} incomplete · {} errors",
                r.outcome_counts.divergence_proven,
                r.outcome_counts.inconclusive,
                r.outcome_counts.error
            );
            d.coverage = json!({"generation":r.coverage,"exploration_budget":r.generation_budget,"outcomes":r.outcome_counts});
            d.limitations.extend(r.limitations);
        }
        ReportKind::CounterexampleReduction => {
            let r: reduction::CounterexampleReductionResult = serde_json::from_value(raw.clone())?;
            let c = behavior::load(store, &r.final_comparison.comparison_id, auth)?;
            exact_details(&mut d, &c)?;
            d.outcome = serde_json::to_value(r.status)?.as_str().unwrap().into();
            d.reason = format!("Reduction: {}. {}", d.outcome, r.minimality_claim);
            d.related_artifacts.push(reference(&c.comparison_id, &c)?);
            let suite = generation::load(store, &r.input.source_suite_id, auth)?;
            d.related_artifacts
                .push(reference(&suite.suite_id, &suite)?);
            if let Some(repro) = &mut d.reproduction {
                repro.label = if r.status == reduction::ReductionStatus::Minimized {
                    "Locally minimized reproduction"
                } else {
                    "Best proven reproduction (no minimized claim)"
                }
                .into();
                repro.original_assignments = Some(r.original_assignment.len());
                repro.retained_assignments = Some(r.reduced_assignment.len());
                repro.reduction_status = Some(r.status);
                repro.minimality_claim = Some(r.minimality_claim);
            }
            d.coverage = json!({"comparison":d.coverage,"reduction_executions":r.execution_count,"execution_budget":r.input.execution_budget});
            d.limitations.extend(r.limitations);
        }
    }
    if !d.other_failures.is_empty() {
        d.primary_failure = Some(d.other_failures.remove(0));
    }
    if let Some(f) = &d.primary_failure {
        d.expected = Some(serde_json::to_value(&f.divergence.before)?);
        d.observed = Some(serde_json::to_value(&f.divergence.after)?);
    }
    d.evidence_summaries
        .sort_by(|a, b| (&a.run_id, &a.evidence_id).cmp(&(&b.run_id, &b.evidence_id)));
    d.evidence_summaries.dedup();
    d.run_references.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    d.run_references.dedup();
    Ok(VerifiedReport { document: d, raw })
}
impl VerifiedReport {
    pub fn document(&self) -> &ReportDocument {
        &self.document
    }
    pub fn raw(&self) -> &Value {
        &self.raw
    }
    pub fn agent(&self) -> AgentReport {
        if self.document.blindtest.is_some() {
            return blindtest::agent(self);
        }
        if self.document.sideeffect.is_some() {
            return sideeffect::agent(self);
        }
        let d = &self.document;
        let refs = d
            .primary_failure
            .as_ref()
            .map(|f| f.divergence.evidence_refs.clone())
            .unwrap_or_default();
        AgentReport {
            schema_version: "3".into(),
            sideeffect: None,
            blindtest: None,
            disclosure_policy: "behavior.public-references.v1".into(),
            source: d.source.clone(),
            verdict: d.verdict,
            kind: d.kind.clone(),
            outcome: d.outcome.clone(),
            summary: match d.verdict {
                Verdict::Pass => "No divergence in tested scope",
                Verdict::Fail => "Proven behavior divergence",
                Verdict::Inconclusive => "Evidence incomplete",
                Verdict::Error => "Verifier execution error",
            }
            .into(),
            expected: d.expected.clone(),
            observed: d.observed.clone(),
            reproduction: d.reproduction.as_ref().map(|r| r.comparison.clone()),
            reduction_status: d.reproduction.as_ref().and_then(|r| r.reduction_status),
            evidence_refs: refs,
            evidence_count: d.evidence_summaries.len(),
            limitations: vec![
                "Bounded evidence; inspect source for scope, reasons and recorded inputs.".into(),
            ],
            replayability: d.replayability.clone(),
        }
    }
    pub fn human(&self) -> String {
        let d = &self.document;
        let mut s = format!(
            "{}\n\n{}\n\nReason\n{}\n",
            if d.blindtest.is_some() && d.verdict == Verdict::Fail {
                "✕ NOT READY"
            } else {
                badge(d.verdict)
            },
            d.headline,
            d.reason
        );
        if let Some(blind) = &d.blindtest {
            s.push_str(&blindtest::human(blind));
        }
        if let Some(effect) = &d.sideeffect {
            s.push_str(&sideeffect::human(effect));
        }
        if let Some(f) = &d.primary_failure {
            s.push_str(&format!(
                "\nExpected\n{}\n\nObserved\n{}\n",
                pretty(&f.divergence.before),
                pretty(&f.divergence.after)
            ));
        }
        if d.verdict != Verdict::Pass {
            if let Some(r) = &d.reproduction {
                s.push_str(&format!("\n{}\n{}\n", r.label, pretty(r)));
            }
        }
        s.push_str(&format!("\nCoverage\n{}\n\nLimitations\n{}\nReplayability\n{}\n\nEvidence · {}\nRuns · {}\nDetails available: --open / --output json\n",pretty(&d.coverage),d.limitations.join("\n"),pretty(&d.replayability),d.evidence_summaries.len(),d.blindtest.as_ref().map_or_else(|| d.sideeffect.as_ref().map_or(d.run_references.len(), |s|s.schedules.iter().map(|s|s.attempts).sum()), |b| b.runs.as_array().map_or(0, Vec::len))));
        s.push_str(&format!("\nSource artifact\n{}\n", d.source.artifact_id));
        // Escape target-controlled terminal control sequences (including OSC).
        s.chars()
            .flat_map(|c| {
                if c.is_control() && c != '\n' && c != '\t' {
                    c.escape_default().collect::<Vec<_>>()
                } else {
                    vec![c]
                }
            })
            .collect()
    }
}
pub fn pretty(v: &impl Serialize) -> String {
    serde_json::to_string_pretty(v).expect("serializable report")
}
pub fn report_schema() -> Value {
    schema::<ReportDocument>("report-document")
}
pub fn agent_schema() -> Value {
    schema::<AgentReport>("agent-report")
}
fn schema<T: JsonSchema>(name: &str) -> Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).expect("static schema");
    v["$id"] = format!("https://b2ige.dev/schemas/verify/{name}.v3.json").into();
    v["properties"]["schema_version"]["const"] = "3".into();
    if v["properties"].get("projection_version").is_some() {
        v["properties"]["projection_version"]["const"] = "3".into();
    }
    v
}
/// Paths must point to a store artifact directory or its committed result.json.
/// Standalone copied JSON cannot bypass sibling evidence and child validation.
pub fn resolve(input: &str, root: &Path) -> io::Result<(PathBuf, String)> {
    let path = Path::new(input);
    if path.exists() || input.contains('/') {
        let directory = if path.file_name().is_some_and(|n| n == "result.json") {
            path.parent().ok_or_else(|| invalid("missing parent"))?
        } else {
            path
        };
        let id = directory
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| invalid("invalid artifact path"))?;
        Ok((
            directory
                .parent()
                .ok_or_else(|| invalid("missing store"))?
                .to_owned(),
            id.into(),
        ))
    } else {
        Ok((root.to_owned(), input.into()))
    }
}
