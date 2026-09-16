//! Independent V100 pre-candidate content commitment, not a verification verdict.
//!
//! The trusted caller supplies canonical identities before candidate identity exists.
//! No candidate implementation may contribute to these identities. Local existence
//! proves neither creation time nor pre-candidate chronology, authentication, or
//! actual environment conformance. Hash validation cannot establish input provenance.

use serde::{Deserialize, Serialize};
use verify_evidence::{canonical_hash, valid_hash};

/// Task Seal v1 binds only task, verifier and declared environment-contract identities.
/// Each identity must be produced using `verify_evidence::canonical_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSeal {
    pub schema_version: String,
    pub task_identity: String,
    pub verifier_identity: String,
    pub environment_contract_identity: String,
}

impl TaskSeal {
    /// Validates version and hash syntax, not provenance, chronology or a verdict.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != "1" {
            return Err("unsupported Task Seal schema version");
        }
        if ![
            &self.task_identity,
            &self.verifier_identity,
            &self.environment_contract_identity,
        ]
        .iter()
        .all(|identity| valid_hash(identity))
        {
            return Err("invalid Task Seal identity");
        }
        Ok(())
    }

    /// Recomputes the domain-separated RFC 8785 / SHA-256 content commitment.
    /// Compare with an independently retained commitment to detect mutation;
    /// replacing both the seal and its commitment is not authenticated here.
    pub fn commitment(&self) -> Result<String, &'static str> {
        self.validate()?;
        canonical_hash(&serde_json::json!({
            "domain": "b2ige.verify.task-seal.v1",
            "seal": self,
        }))
        .map_err(|_| "cannot canonicalize Task Seal")
    }
}
