//! Preflight contracts and bounded local process acquisition; no security isolation.
pub mod process;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use verify_evidence::{unique_map, valid_hash, MAX_SAFE_INTEGER};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    None,
    WorkspaceSeparation,
    ContainerIsolation,
    HardenedLinux,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IsolationAssessment {
    pub available: IsolationLevel,
    pub linux_backend: bool,
    pub protected_artifacts_visible: bool,
}
impl IsolationAssessment {
    /// An assessment supplied by trusted runner preflight, never by target output.
    pub fn validate(&self, required: IsolationLevel) -> Result<IsolationLevel, &'static str> {
        if required > self.available {
            return Err("required isolation unavailable");
        }
        if required != IsolationLevel::None && self.protected_artifacts_visible {
            return Err("protected artifacts exposed");
        }
        if required == IsolationLevel::HardenedLinux && !self.linux_backend {
            return Err("hardened isolation requires Linux backend");
        }
        Ok(required)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
    RunnerCrash,
    InvalidAuthoritativeConfig,
    ObserverInitializationFailure,
    CorruptedEvidenceStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunContext {
    pub run_id: String,
    pub schema_version: String,
    pub tool_version: String,
    pub seed: u64,
    pub target_revision: String,
    pub config_hash: String,
    /// Existing v1 name retained; experiment_hash is an explicit equal alias.
    pub plan_hash: String,
    pub experiment_hash: String,
    pub platform_runtime: String,
    #[serde(deserialize_with = "unique_map")]
    pub observer_versions: BTreeMap<String, String>,
    pub started_at: String,
    pub ended_at: String,
}
impl RunContext {
    pub fn valid(&self) -> bool {
        self.schema_version == "1"
            && self.seed <= MAX_SAFE_INTEGER
            && self.plan_hash == self.experiment_hash
            && [
                &self.run_id,
                &self.tool_version,
                &self.target_revision,
                &self.platform_runtime,
                &self.started_at,
                &self.ended_at,
            ]
            .iter()
            .all(|s| !s.trim().is_empty())
            && valid_hash(&self.config_hash)
            && valid_hash(&self.plan_hash)
            && self
                .observer_versions
                .iter()
                .all(|(id, v)| !id.trim().is_empty() && !v.trim().is_empty())
    }
}
