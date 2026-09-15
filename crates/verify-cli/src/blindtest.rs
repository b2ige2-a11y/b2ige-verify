use super::*;
use verify_core::blindtest as bt;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicFailure {
    pub invariant: String,
    pub failure_kind: String,
    pub expected: String,
    pub observed: String,
    pub reproduction: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub isolation_attestation_ref: String,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlindTestReport {
    pub hidden_checks: usize,
    pub complete_checks: usize,
    pub isolation: String,
    pub quality: String,
    pub target_image: Option<String>,
    pub failures: Vec<PublicFailure>,
    pub isolation_attestations: Value,
    pub runs: Value,
    /// Only the trusted human projection has this field.
    pub hidden_details: Value,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlindTestAgent {
    pub hidden_checks: usize,
    pub complete_checks: usize,
    pub isolation: String,
    pub quality: String,
    pub target_image: Option<String>,
    pub failures: Vec<PublicFailure>,
}
fn safe_text(text: &str, fallback: &str, r: &bt::BlindTestRunResult) -> String {
    let mut secrets = vec![
        r.suite.private_canary.clone(),
        r.suite.private_metadata.clone(),
        r.config.target.workspace.to_string_lossy().into_owned(),
    ];
    for case in &r.suite.cases {
        secrets.push(case.case_id.clone());
        secrets.extend(case.args.clone());
        secrets.extend(case.environment.values().cloned());
        if let Some(f) = &case.fixture {
            if let Ok(s) = String::from_utf8(f.bytes.clone()) {
                secrets.push(s);
            }
        }
        if let Some(o) = &case.oracle {
            for p in &o.predicates {
                match p {
                    bt::Predicate::StdoutEquals { bytes }
                    | bt::Predicate::StderrEquals { bytes }
                    | bt::Predicate::StdoutNotContains { bytes }
                    | bt::Predicate::StderrNotContains { bytes } => {
                        if let Ok(s) = String::from_utf8(bytes.clone()) {
                            secrets.push(s);
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    if text.len() > 1024
        || text.chars().any(char::is_control)
        || text.contains("BLINDTEST_PRIVATE")
        || text.contains('/')
        || text.contains('\\')
        || secrets.iter().any(|s| !s.is_empty() && text.contains(s))
    {
        fallback.into()
    } else {
        text.into()
    }
}
fn failures(r: &bt::BlindTestRunResult) -> Vec<PublicFailure> {
    let mut seen = std::collections::BTreeSet::new();
    r.violations.iter().filter(|v|seen.insert((&v.invariant_id,&v.failure_kind))).take(16).map(|v| {
        let i=r.suite.invariants.iter().find(|i|i.invariant_id==v.invariant_id).expect("verified invariant");
        let case=r.suite.cases.iter().find(|c|c.case_id==v.case_id).expect("verified case");
        let x=r.executions.iter().find(|x|x.case_id==v.case_id).expect("verified execution");
        let leakage=v.failure_kind=="isolation_leakage";
        PublicFailure { invariant:if leakage {"Private artifact isolation boundary".into()} else {safe_text(&i.public_summary,"An approved CLI invariant was violated",r)},failure_kind:v.failure_kind.clone(),
            expected:if leakage {"Private artifacts remain inaccessible".into()} else {safe_text(&i.expected_semantic,"The approved CLI contract is satisfied",r)},
            observed:if leakage {"Protected marker detected in target output".into()} else {match x.capture.exit_code { Some(0)=>"Command completed successfully; the observed behavior violated the contract",Some(_)=>"Command rejected the operation; the observed behavior violated the contract",None=>"Observed output violated the contract"}.into()},
            reproduction:if leakage {vec!["Run the target with the approved Docker isolation policy.".into(),"Inspect the private leakage evidence in the trusted human view.".into()]} else {case.reproduction_template.iter().map(|s|safe_text(s,"Repeat the public action described by this invariant.",r)).collect()},
            evidence_refs:vec![v.evidence_ref.clone()], isolation_attestation_ref:v.evidence_ref.clone(), }
    }).collect()
}
fn title(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "Hidden verification passed",
        Verdict::Fail => "Hidden verification found a contract violation",
        Verdict::Inconclusive => "Hidden verification could not obtain\nall required evidence",
        Verdict::Error => "Blind verifier could not complete",
    }
}
pub(super) fn load(store: &EvidenceStore, id: &str) -> io::Result<VerifiedReport> {
    let r = bt::load(store, id)?;
    let raw = serde_json::to_value(&r)?;
    let source = reference(id, &r)?;
    let failures = failures(&r);
    let isolation = if r.executions.is_empty() {
        "Not attested"
    } else {
        "Docker verified"
    }
    .to_string();
    let (_, items) = store.load(id)?;
    let evidence_summaries = items
        .iter()
        .map(|e| {
            Ok(EvidenceRef {
                run_id: id.into(),
                evidence_id: e.evidence_id.clone(),
                evidence_hash: canonical_hash(e)?,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let blind=BlindTestReport{hidden_checks:r.suite.cases.len(),complete_checks:r.complete_cases,isolation,quality:r.quality.clone(),target_image:r.image.as_ref().map(|i|i.image_id.clone()),failures:failures.clone(),
        isolation_attestations:json!(r.executions.iter().map(|x|&x.attestation).collect::<Vec<_>>()),
        runs:json!(r.executions.iter().enumerate().map(|(n,x)|json!({"evidence":format!("execution-{n}"),"image":x.image_id,"capture_complete":x.capture.runner_failure.is_none()&&!x.capture.timed_out,"started_at":x.capture.started_at,"ended_at":x.capture.ended_at})).collect::<Vec<_>>()),
        hidden_details:json!({"suite":r.suite,"executions":r.executions,"violations":r.violations})};
    let reason = match r.verdict {
        Verdict::Pass => "No failures detected\nwithin the executed hidden suite".into(),
        Verdict::Fail => failures
            .first()
            .map(|f| f.invariant.clone())
            .unwrap_or_else(|| "Hidden contract violation".into()),
        Verdict::Inconclusive => {
            "Required hidden case completion or process capture is incomplete.".into()
        }
        Verdict::Error => {
            "The controller could not initialize or complete the declared Docker experiment.".into()
        }
    };
    let document=ReportDocument{schema_version:"3".into(),projection_version:"3".into(),report_id:format!("report-{}",source.integrity_hash.trim_start_matches("sha256:")),product:"blindtest".into(),kind:ReportKind::BlindTest,source:source.clone(),verdict:r.verdict,outcome:"BLINDTEST".into(),headline:title(r.verdict).into(),reason,expected:failures.first().map(|f|json!(f.expected)),observed:failures.first().map(|f|json!(f.observed)),primary_failure:None,other_failures:vec![],coverage:json!({"required_hidden_cases":blind.hidden_checks,"completed_hidden_cases":blind.complete_checks,"case_budget":r.config.max_cases,"isolation":blind.isolation,"quality":blind.quality}),reproduction:None,evidence_summaries,run_references:vec![],limitations:vec!["Tests your coding agent can't see. Scope: the executed CLI hidden suite and the Docker target boundary.".into(),"DOCKER_ISOLATION only. Host users, Docker VM compromise, kernel exploits and side channels are outside this boundary.".into(),bt::QUALITY_LIMIT.into()],replayability:Replayability::Unavailable{reason:"Exact single-case inputs and immutable target identity are retained in the trusted human evidence. Automatic replay CLI is unavailable.".into()},related_artifacts:vec![],raw_artifact:source,sideeffect:None,blindtest:Some(blind)};
    Ok(VerifiedReport { document, raw })
}
pub(super) fn agent(report: &VerifiedReport) -> AgentReport {
    let d = &report.document;
    let b = d.blindtest.as_ref().expect("blindtest projection");
    // Run names are caller-chosen. Export an opaque alias even if a caller embeds
    // a secret in the original name. Evidence IDs themselves are controller-owned.
    let opaque = format!(
        "blindtest-{}",
        canonical_hash(&d.source.artifact_id)
            .expect("hash")
            .trim_start_matches("sha256:")
    );
    let source = ArtifactRef {
        artifact_id: opaque.clone(),
        integrity_hash: d.source.integrity_hash.clone(),
    };
    AgentReport{schema_version:"3".into(),disclosure_policy:"blindtest.public-semantics.v1".into(),sideeffect:None,blindtest:Some(BlindTestAgent{hidden_checks:b.hidden_checks,complete_checks:b.complete_checks,isolation:b.isolation.clone(),quality:b.quality.clone(),target_image:b.target_image.clone(),failures:b.failures.clone()}),source,verdict:d.verdict,kind:d.kind.clone(),outcome:d.outcome.clone(),summary:d.headline.clone(),expected:d.expected.clone(),observed:d.observed.clone(),reproduction:None,reduction_status:None,
        evidence_refs:d.evidence_summaries.iter().filter(|e|e.evidence_id.starts_with("execution-")).take(8).map(|e|EvidenceRef{run_id:opaque.clone(),evidence_id:e.evidence_id.clone(),evidence_hash:e.evidence_hash.clone()}).collect(),evidence_count:d.evidence_summaries.len(),limitations:vec!["Bounded hidden CLI verification; only sanitized public semantics are disclosed.".into(),bt::QUALITY_LIMIT.into()],replayability:Replayability::Unavailable{reason:"Sanitized public reproduction steps are supplied; exact hidden inputs remain in trusted controller evidence.".into()}}
}
pub(super) fn human(b: &BlindTestReport) -> String {
    let mut out = format!(
        "\nHidden checks · {}\nCompleted · {}\nIsolation · {}\nQuality · {}\n",
        b.hidden_checks, b.complete_checks, b.isolation, b.quality
    );
    for f in &b.failures {
        out.push_str(&format!(
            "\n{}\n\nExpected\n{}\n\nObserved\n{}\n\nReproduction\n{}\n",
            f.invariant,
            f.expected,
            f.observed,
            f.reproduction.join("\n")
        ));
    }
    out
}
