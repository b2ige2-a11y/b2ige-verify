use crate::Verdict;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use verify_runner::process::ProcessObservation;

macro_rules! model {
    ($($item:item)*) => {$ (#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)] #[serde(deny_unknown_fields)] $item)*};
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IsolationLevel {
    None,
    WorkspaceSeparation,
    DockerIsolation,
    HardenedLinux,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceStatus {
    Candidate,
    Reviewed,
    Approved,
    Authoritative,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    ExitEquals { value: i32 },
    ExitNotEquals { value: i32 },
    StdoutEquals { bytes: Vec<u8> },
    StderrEquals { bytes: Vec<u8> },
    StdoutNotContains { bytes: Vec<u8> },
    StderrNotContains { bytes: Vec<u8> },
}
model! {
    pub struct Provenance {
        pub status: ProvenanceStatus,
        pub source: String,
        pub approved_by: Option<String>,
    }
    pub struct RequirementArtifact {
        pub schema_version: String,
        pub requirement_id: String,
        pub text: String,
        pub source: String,
        pub version: String,
        pub content_hash: String,
    }
    /// Separate v2 executable artifact; P0 invariant v1 remains unchanged.
    pub struct InvariantArtifact {
        pub schema_version: String,
        pub invariant_id: String,
        pub requirement_id: String,
        pub requirement_hash: String,
        pub public_summary: String,
        pub expected_semantic: String,
        /// A bounded AND. Exact bytes remain in trusted controller storage.
        pub predicates: Vec<Predicate>,
        pub provenance: Provenance,
    }
    pub struct ApprovedInvariantRef {
        pub invariant_id: String,
        pub artifact_hash: String,
        pub requirement_hash: String,
        pub checker_binding_hash: String,
    }
    pub struct HiddenOracle {
        pub schema_version: String,
        pub checker: String,
        pub invariant_hash: String,
        pub predicates: Vec<Predicate>,
    }
    /// Only the current fixture bytes may be delivered. No suite file is copied.
    pub struct BoundedFixture {
        pub name: String,
        pub bytes: Vec<u8>,
    }
    pub struct BlindTestHiddenCase {
        pub schema_version: String,
        pub case_id: String,
        pub invariant_id: String,
        pub args: Vec<String>,
        #[serde(deserialize_with = "verify_evidence::unique_map")]
        pub environment: BTreeMap<String, String>,
        pub fixture: Option<BoundedFixture>,
        pub oracle: Option<HiddenOracle>,
        pub public_failure_label: String,
        pub reproduction_template: Vec<String>,
    }
    pub struct BlindTestHiddenSuiteManifest {
        pub schema_version: String,
        pub suite_id: String,
        pub version: String,
        pub provenance: Provenance,
        pub approved_invariants: Vec<ApprovedInvariantRef>,
        #[serde(deserialize_with = "verify_evidence::unique_map")]
        pub case_hashes: BTreeMap<String, String>,
    }
    pub struct SealedSuite {
        pub schema_version: String,
        pub manifest: BlindTestHiddenSuiteManifest,
        pub requirements: Vec<RequirementArtifact>,
        pub invariants: Vec<InvariantArtifact>,
        pub cases: Vec<BlindTestHiddenCase>,
        /// Never transmitted to the target or the agent projection.
        pub private_canary: String,
        pub private_metadata: String,
    }
    pub struct ResourceBounds {
        pub timeout_ms: u64,
        pub memory_bytes: u64,
        pub nano_cpus: u64,
        pub pids: u64,
        pub tmpfs_bytes: u64,
    }
    pub struct TargetSpec {
        /// A tag is resolved once to an immutable local content identity before create.
        pub image: String,
        pub command: String,
        pub args: Vec<String>,
        #[serde(deserialize_with = "verify_evidence::unique_map")]
        pub environment: BTreeMap<String, String>,
        pub allowed_case_env: Vec<String>,
        pub max_case_args: usize,
        pub workspace: PathBuf,
        pub workspace_hash: String,
        pub build_identity: String,
        pub bounds: ResourceBounds,
    }
    /// Safe in the agent workspace: no private filesystem locator or case inventory.
    pub struct BlindTestConfig {
        pub schema_version: String,
        pub suite_hash: String,
        pub approved_invariants: Vec<ApprovedInvariantRef>,
        pub target: TargetSpec,
        pub required_isolation: IsolationLevel,
        /// Must cover the complete manifest for PASS; a smaller budget is incomplete.
        pub max_cases: usize,
        pub validation_receipt: Option<String>,
    }
    pub struct ImageIdentity {
        pub image_id: String,
        pub platform: String,
        pub inspect: serde_json::Value,
    }
    pub struct BlindTestIsolationAttestation {
        pub schema_version: String,
        pub required: IsolationLevel,
        pub observed: IsolationLevel,
        pub controller_platform: String,
        pub container_id: String,
        pub before: serde_json::Value,
        pub after: serde_json::Value,
    }
    pub struct HiddenCaseExecution {
        pub case_id: String,
        pub image_id: String,
        pub attestation: BlindTestIsolationAttestation,
        pub capture: ProcessObservation,
        pub cleanup_confirmed: bool,
    }
    pub struct PrivateViolation {
        pub case_id: String,
        pub invariant_id: String,
        pub predicate_index: usize,
        pub failure_kind: String,
        pub evidence_ref: String,
    }
    /// Only controller errors from a fixed vocabulary, never Docker stderr/paths.
    pub struct BlindTestRunResult {
        pub schema_version: String,
        pub blindtest_result_id: String,
        pub config: BlindTestConfig,
        pub suite: SealedSuite,
        pub image: Option<ImageIdentity>,
        pub executions: Vec<HiddenCaseExecution>,
        pub controller_error: Option<String>,
        pub verdict: Verdict,
        pub violations: Vec<PrivateViolation>,
        pub complete_cases: usize,
        pub leakage_detected: bool,
        pub quality: String,
    }
    pub struct ValidationTarget {
        pub label: String,
        pub config: BlindTestConfig,
        pub expected: Verdict,
    }
    pub struct SuiteValidationConfig {
        pub schema_version: String,
        pub targets: Vec<ValidationTarget>,
    }
    pub struct ValidationRun {
        pub label: String,
        pub run_id: String,
        pub run_hash: String,
        pub expected: Verdict,
        pub observed: Verdict,
    }
    pub struct BlindTestSuiteValidationReceipt {
        pub schema_version: String,
        pub blindtest_validation_id: String,
        pub suite_hash: String,
        pub checker: String,
        pub runs: Vec<ValidationRun>,
        pub matched: bool,
        pub limitation: String,
    }
    pub struct DoctorReport {
        pub schema_version: String,
        pub docker_available: bool,
        pub docker_version: Option<String>,
        pub target_platform: Option<String>,
        pub supported_isolation: IsolationLevel,
        pub path_rules: String,
        pub required_capabilities: Vec<String>,
        pub reported_security_options: Vec<String>,
        pub cgroup_version: Option<String>,
        pub runtime_capabilities_available: bool,
        pub attestation_required_per_run: bool,
    }
}
