#[path = "support/sealed.rs"]
mod sealed;
#[path = "support/blindtest.rs"]
mod support;
use std::fs;
use verify_core::sealed_run::{
    query::{Ledger, Policy},
    Execution,
};
use verify_evidence::{canonical_bytes, canonical_hash};

fn setup(limit: u32) -> (support::Corpus, Policy, Execution) {
    let mut c = support::Corpus::temporary();
    c.config.target.image = format!("sha256:{}", "0".repeat(64));
    let p = Policy {
        schema_version: "1".into(),
        seal_commitment: sealed::seal().commitment().unwrap(),
        suite_hash: c.config.suite_hash.clone(),
        max_attempts: limit,
    };
    let e = Execution::Blindtest {
        config: Box::new(c.config.clone()),
    };
    (c, p, e)
}

#[test]
fn adversarial_query_crash_restart_new_candidate_and_run_id_do_not_reset_budget() {
    let (c, p, mut e) = setup(2);
    let root = c.sealed.join("queries");
    let ledger = Ledger::initialize(&root, &c.workspace, &c.sealed, &p).unwrap();
    // Interrupted admission counts even without a commit marker.
    fs::create_dir(root.join("attempt-0")).unwrap();
    // Attack the real sealed execution path: no suite means no target can run.
    fs::remove_file(c.sealed.join("suite.json")).unwrap();
    let seal = sealed::seal();
    assert!(ledger
        .execute(
            &c.store,
            "first",
            &seal,
            &sealed::approve(&seal, &e),
            &e,
            &c.sealed
        )
        .is_err());
    assert!(root.join("attempt-1/result.json").is_file());
    if let Execution::Blindtest { config } = &mut e {
        config.target.build_identity = "another candidate".into();
    }
    let reopened = Ledger::open(&root, &p.commitment().unwrap()).unwrap();
    let error = reopened
        .execute(
            &c.store,
            "new-id",
            &seal,
            &sealed::approve(&seal, &e),
            &e,
            &c.sealed,
        )
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "hidden query budget exhausted");
    assert!(!c.store.root().join("new-id").exists());
    assert!(Ledger::initialize(&root, &c.workspace, &c.sealed, &p).is_err());
}

#[test]
fn adversarial_query_concurrent_admissions_never_exceed_limit() {
    let (c, p, e) = setup(3);
    let root = c.sealed.join("queries");
    Ledger::initialize(&root, &c.workspace, &c.sealed, &p).unwrap();
    fs::remove_file(c.sealed.join("suite.json")).unwrap();
    let outcomes = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..12)
            .map(|n| {
                let root = &root;
                let c = &c;
                let p = &p;
                let e = &e;
                scope.spawn(move || {
                    let ledger = Ledger::open(root, &p.commitment().unwrap()).unwrap();
                    let seal = sealed::seal();
                    ledger
                        .execute(
                            &c.store,
                            &format!("run-{n}"),
                            &seal,
                            &sealed::approve(&seal, e),
                            e,
                            &c.sealed,
                        )
                        .err()
                        .unwrap()
                        .to_string()
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|e| *e == "hidden query budget exhausted")
            .count(),
        9
    );
    for n in 0..3 {
        assert!(root.join(format!("attempt-{n}/result.json")).is_file());
    }
    assert!(!root.join("attempt-3").exists());
}

#[test]
fn adversarial_query_policy_rehash_missing_evidence_and_wrong_pin_fail_closed() {
    let (c, p, e) = setup(1);
    let root = c.sealed.join("queries");
    let ledger = Ledger::initialize(&root, &c.workspace, &c.sealed, &p).unwrap();
    assert!(Ledger::open(&root, &canonical_hash(&"forged pin").unwrap()).is_err());
    let path = root.join("policy/result.json");
    let original = fs::read(&path).unwrap();
    let mut raw: serde_json::Value = serde_json::from_slice(&original).unwrap();
    raw["manifest"]["result"]["max_attempts"] = 1024.into();
    // Rewrite both result and evidence with internally valid hashes. Only the
    // independently retained policy pin distinguishes the substituted policy.
    let evidence_path = root.join("policy/evidence/admission.json");
    let mut evidence: verify_evidence::Evidence =
        serde_json::from_slice(&fs::read(&evidence_path).unwrap()).unwrap();
    evidence.observation = verify_evidence::Observation::Value {
        value: raw["manifest"]["result"].clone(),
    };
    evidence.integrity_hash = canonical_hash(&evidence.observation).unwrap();
    raw["manifest"]["evidence_hashes"]["admission"] = canonical_hash(&evidence).unwrap().into();
    fs::write(&evidence_path, canonical_bytes(&evidence).unwrap()).unwrap();
    raw["integrity_hash"] = canonical_hash(&raw["manifest"]).unwrap().into();
    fs::write(&path, canonical_bytes(&raw).unwrap()).unwrap();
    assert!(verify_evidence::store::EvidenceStore::new(&root)
        .load("policy")
        .is_ok());
    assert!(Ledger::open(&root, &p.commitment().unwrap()).is_err());
    let seal = sealed::seal();
    assert!(ledger
        .execute(
            &c.store,
            "forgery",
            &seal,
            &sealed::approve(&seal, &e),
            &e,
            &c.sealed
        )
        .is_err());
    assert!(!root.join("attempt-0").exists());
    fs::write(path, original).unwrap();
    fs::remove_file(root.join("policy/evidence/admission.json")).unwrap();
    assert!(Ledger::open(&root, &p.commitment().unwrap()).is_err());
}

#[test]
fn adversarial_query_policy_wire_and_commitment_are_strict() {
    let (_c, p, _) = setup(1);
    let raw = serde_json::to_value(&p).unwrap();
    for key in raw.as_object().unwrap().keys() {
        let mut missing = raw.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Policy>(missing).is_err());
    }
    let mut extra = raw.clone();
    extra["verdict"] = "PASS".into();
    assert!(serde_json::from_value::<Policy>(extra).is_err());
    let encoded = serde_json::to_string(&p).unwrap();
    assert!(
        serde_json::from_str::<Policy>(&encoded.replacen("{", "{\"max_attempts\":1,", 1)).is_err()
    );
    for key in raw.as_object().unwrap().keys() {
        let mut changed = raw.clone();
        changed[key] = if key == "max_attempts" {
            2.into()
        } else {
            canonical_hash(&"replacement").unwrap().into()
        };
        let changed: Policy = serde_json::from_value(changed).unwrap();
        assert!(
            changed.commitment().is_err()
                || changed.commitment().unwrap() != p.commitment().unwrap()
        );
    }
    let mut invalid = p;
    invalid.max_attempts = 1025;
    assert!(invalid.commitment().is_err());
}

#[test]
fn adversarial_query_runtime_error_receipt_never_refunds_or_becomes_pass() {
    let (c, p, e) = setup(1);
    let root = c.sealed.join("queries");
    let ledger = Ledger::initialize(&root, &c.workspace, &c.sealed, &p).unwrap();
    let seal = sealed::seal();
    let auth = sealed::approve(&seal, &e);
    // A nonexistent immutable image produces a real verifier ERROR, including
    // when Docker initialization itself is unavailable. No mock verdict source.
    let result = ledger
        .execute(&c.store, "runtime-error", &seal, &auth, &e, &c.sealed)
        .unwrap();
    assert_eq!(result.verdict(), verify_core::Verdict::Error);
    assert!(result.receipt().identities.runtime_identity.is_none());
    let reloaded = verify_core::sealed_run::load(
        &c.store,
        "runtime-error",
        &auth,
        &result.commitment().unwrap(),
    )
    .unwrap();
    assert_eq!(reloaded.verdict(), verify_core::Verdict::Error);
    let reopened = Ledger::open(&root, &p.commitment().unwrap()).unwrap();
    assert_eq!(
        reopened
            .execute(&c.store, "retry", &seal, &auth, &e, &c.sealed)
            .err()
            .unwrap()
            .to_string(),
        "hidden query budget exhausted"
    );
    assert!(!c.store.root().join("retry").exists());
}

#[test]
fn adversarial_query_zero_wrong_scope_and_workspace_ledger_are_refused() {
    let (c, p, mut e) = setup(0);
    assert!(Ledger::initialize(&c.workspace.join("queries"), &c.workspace, &c.sealed, &p).is_err());
    let root = c.sealed.join("queries");
    let ledger = Ledger::initialize(&root, &c.workspace, &c.sealed, &p).unwrap();
    let seal = sealed::seal();
    assert_eq!(
        ledger
            .execute(
                &c.store,
                "zero",
                &seal,
                &sealed::approve(&seal, &e),
                &e,
                &c.sealed
            )
            .err()
            .unwrap()
            .to_string(),
        "hidden query budget exhausted"
    );
    if let Execution::Blindtest { config } = &mut e {
        config.suite_hash = canonical_hash(&"other suite").unwrap();
    }
    assert_eq!(
        ledger
            .execute(
                &c.store,
                "scope",
                &seal,
                &sealed::approve(&seal, &e),
                &e,
                &c.sealed
            )
            .err()
            .unwrap()
            .to_string(),
        "hidden query scope mismatch"
    );
    assert!(!root.join("attempt-0").exists());
}
