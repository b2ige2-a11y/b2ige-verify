//! Replay input contracts; execution is composed with acquisition in verify-core.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use verify_evidence::{canonical_hash, valid_hash};
use verify_runner::RunContext;

/// P1A accepts content-addressed targets only. A branch, tag, short hash or an
/// unresolved revision label is not an immutable replay identity.
pub fn immutable_target(revision: &str) -> bool {
    valid_hash(revision)
        || revision.strip_prefix("git:").is_some_and(|hash| {
            matches!(hash.len(), 40 | 64)
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayInputs {
    pub seed: Option<u64>,
    pub experiment_plan: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub config_hash: Option<String>,
    pub fault_schedule: Option<Vec<serde_json::Value>>,
    pub target_revision: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Replayability {
    Available,
    Unavailable { reason: String },
}
impl ReplayInputs {
    pub fn assess(&self, context: &RunContext, required_observers: &[String]) -> Replayability {
        let valid = context.valid()
            && !required_observers.is_empty()
            && required_observers
                .iter()
                .all(|o| context.observer_versions.contains_key(o))
            && self.seed == Some(context.seed)
            && self.target_revision.as_ref() == Some(&context.target_revision)
            && self.config_hash.as_ref() == Some(&context.config_hash)
            && self
                .experiment_plan
                .as_ref()
                .is_some_and(|p| canonical_hash(p).is_ok_and(|h| h == context.plan_hash))
            && self
                .experiment_plan
                .as_ref()
                .is_some_and(|p| p.as_object().is_some_and(|o| !o.is_empty()))
            && self.fault_schedule.is_some()
            && self.fault_schedule.as_ref().is_some_and(|s| {
                self.config
                    .as_ref()
                    .and_then(|c| c.get("fault_schedule"))
                    .is_some_and(|declared| {
                        verify_evidence::canonical_equal(&serde_json::json!(s), declared)
                            .unwrap_or(false)
                    })
            })
            && self
                .target_revision
                .as_ref()
                .is_some_and(|r| immutable_target(r))
            && self.config_hash.as_ref().is_some_and(|h| {
                valid_hash(h)
                    && self
                        .config
                        .as_ref()
                        .is_some_and(|c| canonical_hash(c).is_ok_and(|actual| actual == *h))
            });
        if valid {
            Replayability::Available
        } else {
            Replayability::Unavailable {
                reason: "missing or inconsistent controllable experiment inputs".into(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionOutcome {
    Reproduced,
    NotReproduced,
    NotAttempted,
}

/// Separate from product Verdict. REPRODUCED refers only to the recorded process
/// result (stream bytes, exit/signal and timeout), excluding wall timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplayStatus {
    Reproduced,
    ExecutedButDiverged,
    Unavailable,
    Error,
}
