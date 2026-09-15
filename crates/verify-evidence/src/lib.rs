//! Evidence contracts, canonical hashing and local persistence.
pub mod store;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Exact counters/seeds use the interoperable integer range. Arbitrary JSON
/// observation numbers instead have RFC 8785 binary64 semantics.
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    AuthoritativeExternal,
    AuthoritativeTargetState,
    DirectRuntime,
    Derived,
    Advisory,
}
impl TrustClass {
    pub fn may_decide(self) -> bool {
        !matches!(self, Self::Derived | Self::Advisory)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationCoverage {
    Complete,
    Partial,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub status: ObservationCoverage,
    pub verdict_critical: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectIdentity {
    pub provider: String,
    pub operation: String,
    pub external_effect_id: String,
    pub idempotency_identity: String,
}
impl EffectIdentity {
    pub fn valid(&self) -> bool {
        [
            &self.provider,
            &self.operation,
            &self.external_effect_id,
            &self.idempotency_identity,
        ]
        .iter()
        .all(|s| !s.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Value {
        value: serde_json::Value,
    },
    Attempt {
        request_id: String,
    },
    /// Complete committed-state snapshot; empty means observed zero, not missing evidence.
    CommittedEffects {
        effects: Vec<EffectIdentity>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub evidence_id: String,
    pub run_id: String,
    pub source: String,
    pub trust_class: TrustClass,
    pub order: u64,
    pub observation: Observation,
    pub integrity_hash: String,
    pub related_claim_ids: Vec<String>,
}
impl Evidence {
    pub fn valid(&self) -> bool {
        self.order <= MAX_SAFE_INTEGER
            && !self.evidence_id.trim().is_empty()
            && !self.run_id.trim().is_empty()
            && !self.source.trim().is_empty()
            && !self.related_claim_ids.is_empty()
            && canonical_hash(&self.observation).is_ok_and(|h| h == self.integrity_hash)
            && match &self.observation {
                Observation::CommittedEffects { effects } => {
                    effects.iter().all(EffectIdentity::valid)
                }
                Observation::Attempt { request_id } => !request_id.trim().is_empty(),
                Observation::Value { .. } => true,
            }
    }
}

pub fn valid_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// One canonical representation for hashes and checker equality. Preserve float
/// types until validation: serde_json::to_value would silently turn NaN into null.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    use serde::ser::Error;
    let value = serde_value::to_value(value).map_err(serde_json::Error::custom)?;
    fn validate(value: &serde_value::Value) -> bool {
        use serde_value::Value::*;
        match value {
            F32(n) => n.is_finite(),
            F64(n) => n.is_finite(),
            Option(Some(v)) | Newtype(v) => validate(v),
            Seq(values) => values.iter().all(validate),
            Map(values) => values
                .iter()
                .all(|(k, v)| matches!(k, String(_)) && validate(v)),
            _ => true,
        }
    }
    if !validate(&value) {
        return Err(serde_json::Error::custom(
            "JCS requires finite binary64 numbers and string keys",
        ));
    }
    serde_json_canonicalizer::to_vec(&value)
}

/// RFC 8785 canonical JSON, SHA-256; never an authentication mechanism.
pub fn canonical_hash<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_bytes(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub fn canonical_equal(
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Result<bool, serde_json::Error> {
    Ok(canonical_bytes(a)? == canonical_bytes(b)?)
}

/// Reject ambiguity before a map can overwrite a verdict-critical record.
pub fn unique_map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an object with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, T>()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate object key"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Visitor(std::marker::PhantomData))
}
