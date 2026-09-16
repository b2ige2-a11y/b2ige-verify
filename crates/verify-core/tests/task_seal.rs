use serde_json::json;
use sha2::{Digest, Sha256};
use verify_core::task_seal::TaskSeal;
use verify_evidence::{canonical_hash, valid_hash};

fn seal() -> TaskSeal {
    TaskSeal {
        schema_version: "1".into(),
        task_identity: canonical_hash(&json!({"task": "example"})).unwrap(),
        verifier_identity: canonical_hash(&json!({"checker": "example-v1"})).unwrap(),
        environment_contract_identity: canonical_hash(&json!({"network": "none"})).unwrap(),
    }
}

const IDENTITIES: [&str; 3] = [
    "task_identity",
    "verifier_identity",
    "environment_contract_identity",
];

#[test]
fn valid_seal_validates() {
    let seal = seal();
    assert_eq!(seal.validate(), Ok(()));
    assert!(valid_hash(&seal.commitment().unwrap()));
}

#[test]
fn stable_domain_separated_commitment() {
    let seal = seal();
    // Independently spell out the canonical wire preimage, including key order.
    let preimage = format!(
        "{{\"domain\":\"b2ige.verify.task-seal.v1\",\"seal\":{{\"environment_contract_identity\":\"{}\",\"schema_version\":\"1\",\"task_identity\":\"{}\",\"verifier_identity\":\"{}\"}}}}",
        seal.environment_contract_identity, seal.task_identity, seal.verifier_identity
    );
    let expected = format!("sha256:{:x}", Sha256::digest(preimage.as_bytes()));
    assert_eq!(seal.commitment().unwrap(), expected);
    assert_eq!(seal.commitment(), seal.commitment());
    assert_ne!(seal.commitment().unwrap(), canonical_hash(&seal).unwrap());
    let reordered = format!(
        "{{\"verifier_identity\":\"{}\",\"task_identity\":\"{}\",\"schema_version\":\"1\",\"environment_contract_identity\":\"{}\"}}",
        seal.verifier_identity, seal.task_identity, seal.environment_contract_identity
    );
    let decoded: TaskSeal = serde_json::from_str(&reordered).unwrap();
    assert_eq!(decoded.commitment().unwrap(), expected);
}

#[test]
fn each_bound_identity_changes_commitment() {
    let original = seal();
    for field in IDENTITIES {
        let mut value = serde_json::to_value(&original).unwrap();
        value[field] = json!(canonical_hash(&json!({"changed": true})).unwrap());
        let changed: TaskSeal = serde_json::from_value(value).unwrap();
        assert_eq!(changed.validate(), Ok(()));
        assert_ne!(original.commitment(), changed.commitment(), "{field}");
    }
}

#[test]
fn malformed_hashes_rejected_in_every_identity() {
    for malformed in [
        String::new(),
        "a".repeat(64),
        format!("sha256:{}", "a".repeat(63)),
        format!("sha256:{}", "a".repeat(65)),
        format!("sha256:{}", "A".repeat(64)),
        format!("sha256:{}", "g".repeat(64)),
        format!(" sha256:{}", "a".repeat(64)),
    ] {
        for field in IDENTITIES {
            let mut value = serde_json::to_value(seal()).unwrap();
            value[field] = json!(malformed);
            let invalid: TaskSeal = serde_json::from_value(value).unwrap();
            assert!(invalid.validate().is_err(), "{field}: {malformed}");
            assert!(invalid.commitment().is_err());
        }
    }
}

#[test]
fn unsupported_versions_rejected() {
    for version in ["", "0", "2", "01", "1 "] {
        let mut invalid = seal();
        invalid.schema_version = version.into();
        assert!(invalid.validate().is_err());
        assert!(invalid.commitment().is_err());
    }
}

#[test]
fn missing_bound_fields_rejected() {
    for field in IDENTITIES.into_iter().chain(["schema_version"]) {
        let mut value = serde_json::to_value(seal()).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<TaskSeal>(value).is_err());
    }
}

#[test]
fn duplicate_fields_rejected() {
    let encoded = serde_json::to_string(&seal()).unwrap();
    let value = serde_json::to_value(seal()).unwrap();
    for field in IDENTITIES.into_iter().chain(["schema_version"]) {
        let duplicated = format!("{{\"{field}\":{},{}", value[field], &encoded[1..]);
        assert!(serde_json::from_str::<TaskSeal>(&duplicated).is_err());
    }
}

#[test]
fn candidate_identity_absent_by_construction() {
    let value = serde_json::to_value(seal()).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 4);
    assert!(value.get("candidate_identity").is_none());
    let mut injected = value;
    injected["candidate_identity"] = json!(canonical_hash(&"candidate").unwrap());
    assert!(serde_json::from_value::<TaskSeal>(injected).is_err());
}

#[test]
fn no_chronology_or_security_claims_in_artifact() {
    for field in [
        "created_at",
        "pre_candidate_proven",
        "authenticated",
        "environment_attested",
        "verdict",
    ] {
        let mut injected = serde_json::to_value(seal()).unwrap();
        injected[field] = json!(true);
        assert!(serde_json::from_value::<TaskSeal>(injected).is_err());
    }
    // A later local reconstruction is indistinguishable: this binds content only.
    let first = seal();
    let reconstructed: TaskSeal =
        serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    assert_eq!(first.commitment(), reconstructed.commitment());
}
