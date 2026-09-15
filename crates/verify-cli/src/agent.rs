//! Agent Protocol v1: projection only; verified loaders retain verdict authority.
use crate::{ArtifactRef, EvidenceRef, VerifiedReport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use verify_core::Verdict;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Product {
    Behavior,
    Sideeffect,
    Blindtest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Verify,
    Report,
    Doctor,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol_version: Version,
    pub product: Product,
    pub operation: Operation,
    /// Trusted registry key, never an arbitrary path or command.
    pub identity: String,
    pub output: AgentMode,
    /// Unsupported overrides are rejected; product configs own execution budgets.
    pub execution_budget: Option<u64>,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "1")]
    V1,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum AgentMode {
    #[serde(rename = "agent")]
    Agent,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScopeSummary {
    pub description: String,
    pub coverage: BTreeMap<String, u64>,
    pub budget: BTreeMap<String, u64>,
}
fn scope(r: &VerifiedReport, product: Product) -> ScopeSummary {
    let d = r.document();
    let mut s = ScopeSummary {
        description: match product {
            Product::Behavior => "Recorded Behavior process comparisons only",
            Product::Sideeffect => {
                "Configured local SQLite committed effects and fault schedules only"
            }
            Product::Blindtest => {
                "Approved hidden CLI invariants under the Docker target boundary only"
            }
        }
        .into(),
        coverage: BTreeMap::new(),
        budget: BTreeMap::new(),
    };
    // Only known numeric public metadata; never arbitrary human evidence or strings.
    for key in [
        "cases",
        "baseline_repetitions",
        "required_schedules",
        "executed_schedules",
        "required_hidden_cases",
        "completed_hidden_cases",
        "reduction_executions",
    ] {
        if let Some(n) = d.coverage[key].as_u64() {
            s.coverage.insert(key.into(), n);
        }
    }
    for key in ["case_budget", "execution_budget", "exploration_budget"] {
        if let Some(n) = d.coverage[key].as_u64() {
            s.budget.insert(key.into(), n);
        }
    }
    for key in ["max_schedules", "max_attempts", "reduction_executions"] {
        if let Some(n) = d.coverage["budget"][key].as_u64() {
            s.budget.insert(key.into(), n);
        }
    }
    if let Some(n) = d.coverage["profiling_budget"]["repetitions"].as_u64() {
        s.budget.insert("repetitions".into(), n);
    }
    if let Some(repro) = &d.reproduction {
        s.budget
            .insert("case_timeout_ms".into(), repro.experiment.case.timeout_ms);
    }
    if let Some(n) = d.coverage["cases"].as_u64() {
        s.budget.insert("explicit_cases".into(), n);
    }
    s.coverage.insert(
        "verified_evidence_count".into(),
        r.agent().evidence_count as u64,
    );
    s
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol_version: Version,
    pub product: Product,
    pub operation: Operation,
    pub verdict: Verdict,
    pub kind: String,
    pub summary: String,
    pub expected: Option<Value>,
    pub observed: Option<Value>,
    pub reproduction: Value,
    pub evidence_refs: Vec<EvidenceRef>,
    pub source: Option<ArtifactRef>,
    pub scope: Option<ScopeSummary>,
    pub limitations: Vec<String>,
    pub next_action: String,
    pub other_failure_count: usize,
}
impl Response {
    pub fn error(product: Product, operation: Operation) -> Self {
        Self {
            protocol_version: Version::V1,
            product,
            operation,
            verdict: Verdict::Error,
            kind: "infrastructure_error".into(),
            summary: "Configuration, execution, or required evidence could not be verified".into(),
            expected: None,
            observed: None,
            reproduction: json!({"replayability":"unavailable","reason":"No verified result available"}),
            evidence_refs: vec![],
            source: None,
            scope: None,
            limitations: vec!["No verification success established".into()],
            next_action: action(Verdict::Error).into(),
            other_failure_count: 0,
        }
    }
    pub fn from_verified(r: &VerifiedReport, operation: Operation) -> Self {
        // Mandatory existing sanitation boundary, including all BlindTest text.
        let a = r.agent();
        let product = if a.blindtest.is_some() {
            Product::Blindtest
        } else if a.sideeffect.is_some() {
            Product::Sideeffect
        } else {
            Product::Behavior
        };
        let steps = a
            .blindtest
            .as_ref()
            .and_then(|b| b.failures.first())
            .map(|f| f.reproduction.clone());
        let count = a.blindtest.as_ref().map_or_else(
            || {
                r.document()
                    .sideeffect
                    .as_ref()
                    .map_or(r.document().other_failures.len(), |s| {
                        s.violations.len().saturating_sub(1)
                    })
            },
            |b| b.failures.len().saturating_sub(1),
        );
        Self {
            protocol_version: Version::V1,
            product,
            operation,
            verdict: a.verdict,
            kind: serde_json::to_value(a.kind)
                .expect("enum")
                .as_str()
                .expect("string")
                .into(),
            summary: a
                .blindtest
                .as_ref()
                .and_then(|b| b.failures.first())
                .map_or(a.summary, |f| f.invariant.clone()),
            expected: a.expected,
            observed: a.observed,
            reproduction: json!({"artifact":a.reproduction,"steps":steps,"replayability":a.replayability}),
            evidence_refs: a.evidence_refs.into_iter().take(8).collect(),
            source: Some(a.source),
            scope: Some(scope(r, product)),
            limitations: a.limitations,
            next_action: action(a.verdict).into(),
            other_failure_count: count,
        }
    }
}
fn action(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "Verification complete only within the declared tested scope; assess remaining change scope",
        Verdict::Fail => "Use the public reproduction and evidence references to repair the violated contract, then verify again",
        Verdict::Inconclusive => "Obtain missing required evidence and rerun; do not claim completion",
        Verdict::Error => "Repair verifier configuration or infrastructure and rerun; do not treat this as a product failure or completion",
    }
}
pub fn response_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Response)).expect("schema")
}
pub fn request_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Request)).expect("schema")
}

/// CI validates transport shape and exit agreement, never upgrades a verdict.
pub fn ci_check(bytes: &[u8], child_exit: i32) -> Response {
    let parsed = serde_json::from_slice::<Response>(bytes);
    match parsed {
        Ok(r)
            if r.operation == Operation::Verify
                && i32::from(r.verdict.exit_code()) == child_exit
                && (r.verdict == Verdict::Error || (r.source.is_some() && r.scope.is_some()))
                && r.kind != "readiness" =>
        {
            r
        }
        _ => Response::error(Product::Behavior, Operation::Verify),
    }
}
