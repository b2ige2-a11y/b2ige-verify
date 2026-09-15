use crate::{behavior::LocalFixture, Verdict};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use verify_evidence::ObservationCoverage;
use verify_runner::process::{ControlledProcessObservation, ProcessSpec};

macro_rules! model {
    ($($item:item)*) => {$ (#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)] #[serde(deny_unknown_fields)] $item)*};
}
model! {
    pub struct OperationIdentity {
        pub operation_id: String,
        pub idempotency_identity: String,
        pub correlation_identity: String,
    }
    pub struct Trigger {
        pub executable: PathBuf,
        pub executable_hash: String,
        pub args: Vec<String>,
        #[serde(deserialize_with = "verify_evidence::unique_map")]
        pub environment: BTreeMap<String, String>,
        pub fixture: LocalFixture,
        pub timeout_ms: u64,
    }
    /// Declares a durable, append-only committed ledger, never a request log.
    pub struct SQLiteEffectObserver {
        pub observer_id: String,
        pub db_path: PathBuf,
        pub table: String,
        pub external_effect_id_column: String,
        pub idempotency_column: String,
        pub correlation_column: String,
        pub operation_column: String,
        /// Integer, unique, increasing commit sequence in this table.
        pub commit_order_column: String,
        pub authoritative_source: LedgerAuthority,
    }
    pub struct EffectContract {
        pub effect_id: String,
        pub provider: String,
        pub adapter: Adapter,
        pub operation: String,
        pub identity: IdentityContract,
        pub expectation: Expectation,
        pub authoritative_observer: String,
    }
    pub struct IdentityContract {
        pub external_identity: ExternalIdentity,
        pub idempotency_identity: String,
        pub correlation_identity: String,
    }
    pub struct FaultSchedule {
        pub schedule_id: String,
        pub primitives: Vec<FaultPrimitive>,
    }
    pub struct ExplorationBudget {
        pub max_schedules: usize,
        pub max_attempts: usize,
        pub reduction_executions: usize,
    }
    pub struct SideEffectContract {
        pub contract_id: String,
        pub schema_version: String,
        pub operation: OperationIdentity,
        pub trigger: Trigger,
        pub effects: Vec<EffectContract>,
        pub relations: Vec<Relation>,
        pub fault_schedules: Vec<FaultSchedule>,
        pub exploration_budget: ExplorationBudget,
        pub required_observers: Vec<SQLiteEffectObserver>,
    }
    pub struct AttemptIdentity {
        pub attempt_id: String,
        pub run_id: String,
        pub operation_id: String,
    }
    pub struct CommittedEffectIdentity {
        pub provider: String,
        pub operation: String,
        pub external_effect_id: String,
        pub idempotency_identity: String,
        pub correlation_identity: String,
        pub commit_order: u64,
    }
    pub struct CommittedRow {
        pub effect_id: String,
        pub identity: CommittedEffectIdentity,
    }
    pub struct SQLiteSnapshot {
        pub observer_id: String,
        pub coverage: ObservationCoverage,
        pub reason: String,
        /// Online-backup database bytes; re-queried when loading, not a request log.
        pub database: Option<Vec<u8>>,
        pub rows: Vec<CommittedRow>,
    }
    pub struct AttemptRecord {
        pub schema_version: String,
        pub identity: AttemptIdentity,
        pub contract_hash: String,
        pub schedule_id: String,
        pub action_index: usize,
        pub requested: FaultPrimitive,
        pub process: ProcessSpec,
        pub runtime: ControlledProcessObservation,
        pub commit_before_kill: Vec<SQLiteSnapshot>,
        pub after: Vec<SQLiteSnapshot>,
    }
    pub struct ArtifactRef {
        pub artifact_id: String,
        pub integrity_hash: String,
    }
    pub struct HistoryEvent {
        pub event_id: String,
        pub logical_order: u64,
        pub run_id: String,
        pub attempt_id: Option<String>,
        pub kind: HistoryKind,
        pub related_effect: Option<CommittedEffectIdentity>,
        pub evidence_refs: Vec<String>,
    }
    pub struct SideEffectHistory {
        pub schema_version: String,
        pub run_id: String,
        pub events: Vec<HistoryEvent>,
    }
    pub struct FaultScheduleResult {
        pub schema_version: String,
        pub schedule: FaultSchedule,
        pub initial: Vec<SQLiteSnapshot>,
        pub attempts: Vec<ArtifactRef>,
        pub executed_faults: Vec<HistoryEvent>,
        pub complete: bool,
        pub committed: Vec<CommittedRow>,
        pub history: SideEffectHistory,
    }
    pub struct Violation {
        pub schedule_id: String,
        pub effect_ids: Vec<String>,
        pub kind: ViolationKind,
        pub expected: String,
        pub observed: String,
        pub evidence_refs: Vec<String>,
    }
    pub struct ReductionTrial {
        pub schedule: FaultSchedule,
        pub result: ArtifactRef,
        pub preserved: bool,
    }
    pub struct SideEffectCounterexample {
        pub original_schedule: FaultSchedule,
        pub retained_schedule: FaultSchedule,
        pub signature: ViolationSignature,
        pub status: ReductionStatus,
        pub trials: Vec<ReductionTrial>,
        pub reproduction: Option<ArtifactRef>,
        pub steps: Vec<String>,
        pub replayability: String,
    }
    pub struct ViolationSignature {
        pub effect_ids: Vec<String>,
        pub kind: ViolationKind,
    }
    pub struct SideEffectExperimentResult {
        pub schema_version: String,
        pub sideeffect_result_id: String,
        pub contract: SideEffectContract,
        pub schedules: Vec<FaultScheduleResult>,
        pub verdict: Verdict,
        pub violations: Vec<Violation>,
        pub reasons: Vec<String>,
        pub counterexample: Option<SideEffectCounterexample>,
        pub limitations: Vec<String>,
        pub replayability: verify_replay::Replayability,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LedgerAuthority {
    DurableAppendOnlyCommittedState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    Sqlite,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExternalIdentity {
    ProviderOperationExternalId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Expectation {
    ExactlyOnce,
    AtMostOnce,
    AtLeastOnce,
    Never,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "semantics", rename_all = "snake_case", deny_unknown_fields)]
pub enum Relation {
    OrderedAfter { effect: String, after: String },
    AtomicWith { effect: String, other: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaultPrimitive {
    None,
    Retry,
    DuplicateDelivery,
    KillAfterCommit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    OperationStarted,
    AttemptStarted,
    EffectObservedCommitted,
    TargetCompleted,
    TargetKilled,
    RetryStarted,
    DuplicateDeliveryStarted,
    ObserverFailure,
    ExperimentCompleted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViolationKind {
    DuplicateCommittedEffect,
    MissingCommittedEffect,
    ForbiddenCommittedEffect,
    CommitOrder,
    HalfCommit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReductionStatus {
    LocallyMinimized,
    BudgetExhausted,
    Unreproduced,
}
