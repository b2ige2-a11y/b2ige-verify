//! P1C: trusted local replay of a persisted P1B acquisition. Store hashes establish
//! integrity, not process provenance. Only the shared acquisition path generates
//! replay evidence; raw Store writes can never establish executor success.
use super::{acquire_with_completion, assemble, validate_inputs, AcquisitionResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};
use verify_evidence::{canonical_hash, store::EvidenceStore};
use verify_replay::{immutable_target, ReplayStatus, Replayability};
use verify_runner::process::ProcessObservation;

/// New replay envelope v1. The embedded acquisition remains unmodified v1.
/// No product checker or failure reproduction claim is made by this envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayResult {
    pub replay_schema_version: String,
    pub original_run_id: String,
    /// Requested fresh run identity. None when preflight rejected execution.
    pub replay_run_id: Option<String>,
    pub status: ReplayStatus,
    pub reason: String,
    pub original_result_hash: Option<String>,
    pub original_evidence_hashes: BTreeMap<String, String>,
    pub acquisition: Option<AcquisitionResult>,
}

fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

/// Byte hash, deliberately distinct from canonical JSON hashing. Only a sha256
/// executable identity can be verified by this backend. Git revisions alone do
/// not bind an arbitrary compiled executable; they remain unavailable in P1C.
fn verify_target(original: &AcquisitionResult) -> io::Result<()> {
    let revision = &original.context.target_revision;
    if !immutable_target(revision) || !revision.starts_with("sha256:") {
        return Err(invalid("requires a sha256 identity of the executable bytes; mutable, missing and unbound Git targets are unavailable"));
    }
    let mut file = File::open(&original.process.executable)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("target must be a regular executable file"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("sha256:{:x}", hash.finalize()) != *revision {
        return Err(invalid(
            "executable content differs from recorded target identity",
        ));
    }
    Ok(())
}

fn load_original(store: &EvidenceStore, run_id: &str) -> io::Result<AcquisitionResult> {
    let (value, evidence) = store.load(run_id)?;
    let original: AcquisitionResult = serde_json::from_value(value)?;
    validate_inputs(&original.plan, &original.process)?;
    if !original.context.valid() {
        return Err(invalid("invalid original run context"));
    }
    // Reconstruct the P1B envelope from its recorded observation. This checks all
    // plan/config/seed/version/run/source/bundle/coverage/hash links, rather than
    // trusting the store's generic (and deliberately untyped) result payload.
    let (expected, expected_evidence) = assemble(
        &original.plan,
        &original.process,
        run_id,
        original.observation.clone(),
    )?;
    if original != expected || evidence != [expected_evidence] {
        return Err(invalid(
            "inconsistent P1B identity, version, observation or evidence linkage",
        ));
    }
    verify_target(&original)?;
    Ok(original)
}

fn reproduced(original: &ProcessObservation, replay: &ProcessObservation) -> bool {
    original.started == replay.started
        && original.exit_code == replay.exit_code
        && original.signal == replay.signal
        && original.stdout == replay.stdout
        && original.stderr == replay.stderr
        && original.timed_out == replay.timed_out
        && original.runner_failure == replay.runner_failure
}

/// Load and validate before reserving/executing. The caller supplies a fresh run
/// ID; acquisition reserves it exclusively before spawn. Preflight rejection is
/// returned as UNAVAILABLE without a new run. A storage or runner failure is ERROR.
/// A committed replay result embeds fresh acquisition plus original hash links.
/// Identical inputs do not promise identical external state or output bytes.
pub fn execute(store: &EvidenceStore, original_run_id: &str, replay_run_id: &str) -> ReplayResult {
    let mut report = ReplayResult {
        replay_schema_version: "1".into(),
        original_run_id: original_run_id.into(),
        replay_run_id: None,
        status: ReplayStatus::Unavailable,
        reason: String::new(),
        original_result_hash: None,
        original_evidence_hashes: BTreeMap::new(),
        acquisition: None,
    };
    if original_run_id == replay_run_id {
        report.reason = "replay requires a different run_id".into();
        return report;
    }
    let original = match load_original(store, original_run_id) {
        Ok(original) => original,
        Err(error) => {
            report.reason = format!("replay preflight rejected: {error}");
            return report;
        }
    };
    match canonical_hash(&original) {
        Ok(hash) => report.original_result_hash = Some(hash),
        Err(error) => {
            report.reason = format!("original result cannot be hashed: {error}");
            return report;
        }
    }
    report.original_evidence_hashes = original.bundle.evidence_hashes.clone();
    report.replay_run_id = Some(replay_run_id.into());
    let execution = acquire_with_completion(
        &original.plan,
        &original.process,
        store,
        replay_run_id,
        |acquisition, run| {
            (report.status, report.reason) = if let Some(error) =
                &acquisition.observation.runner_failure
            {
                (
                    ReplayStatus::Error,
                    format!("replay runner failure: {error}"),
                )
            } else if reproduced(&original.observation, &acquisition.observation) {
                (ReplayStatus::Reproduced, "Recorded process result reproduced, excluding timestamps; no product checker ran.".into())
            } else {
                (ReplayStatus::ExecutedButDiverged, "Controllable inputs reused; process result differs. This is not a product FAIL.".into())
            };
            let mut recorded = acquisition.clone();
            recorded.bundle.replayability = Replayability::Unavailable {
                reason: "P1C accepts P1B original artifacts only; replay-of-replay is unsupported."
                    .into(),
            };
            recorded.bundle.replay_instructions = format!(
                "Repeat execute with original run {} and a fresh run_id; preflight is required on every attempt.",
                original_run_id
            );
            report.acquisition = Some(recorded);
            run.complete(&report, &acquisition.bundle.evidence_refs)
        },
    );
    if let Err(error) = execution {
        report.status = ReplayStatus::Error;
        report.reason = format!("replay acquisition/commit failed: {error}");
        // Never return an uncommitted acquisition as a successful durable result.
        report.acquisition = None;
    }
    report
}
