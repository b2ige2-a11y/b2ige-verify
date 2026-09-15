use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use verify_evidence::TrustClass;

// P0 optional fields may be absent, but explicit null is not a valid artifact.
fn non_null_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub baseline_id: String,
    pub target_revision: String,
    pub created_by: BaselineCreator,
    pub approval: BaselineApproval,
    pub observation_contract_hash: String,
    #[serde(
        default,
        deserialize_with = "non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u64", range(min = 1))]
    pub stability_runs: Option<u64>,
    #[serde(
        default,
        deserialize_with = "non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Vec<String>")]
    pub notes: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BaselineCreator {
    Human,
    Policy,
    Imported,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineApproval {
    pub status: ApprovalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Approved,
    Unapproved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Invariant {
    pub invariant_id: String,
    pub statement: String,
    pub origin: InvariantOrigin,
    pub authority: InvariantAuthority,
    pub required_observations: Vec<String>,
    #[serde(
        default,
        deserialize_with = "non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "bool")]
    pub hidden: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvariantOrigin {
    MachineContract,
    HumanAuthored,
    LlmCandidate,
    GeneratedMutation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvariantAuthority {
    pub status: InvariantStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvariantStatus {
    Authoritative,
    Candidate,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectContract {
    pub effect_id: String,
    pub semantics: EffectSemantics,
    pub identity: IdentityRule,
    pub commit_observation: CommitObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_effect: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectSemantics {
    ExactlyOnce,
    AtMostOnce,
    AtLeastOnce,
    Never,
    EventuallyWithin,
    OrderedAfter,
    AtomicWith,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdentityRule {
    pub correlation_fields: Vec<String>,
    pub dedupe_domain: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitObservation {
    pub source: String,
    pub minimum_trust_class: TrustClass,
    #[serde(
        default,
        deserialize_with = "non_null_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u64")]
    pub eventual_window_ms: Option<u64>,
}
