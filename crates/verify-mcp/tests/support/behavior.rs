#![allow(dead_code)]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use verify_core::{
    behavior::*, checker_binding_hash, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator,
};
use verify_evidence::canonical_hash;
pub struct Case {
    pub dir: PathBuf,
    pub experiment: BehaviorExperiment,
    pub auth: BehaviorAuthorization,
}
impl Case {
    pub fn new(after: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "b2ige-p4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        for (name, body) in [("before", "printf same"), ("after", after)] {
            let path = dir.join(name);
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let case = BehaviorCase {
            schema_version: "1".into(),
            case_id: "case".into(),
            args: vec!["base".into()],
            environment: BTreeMap::new(),
            timeout_ms: 15000,
            fixture: LocalFixture {
                source: None,
                snapshot_identity: snapshot_identity(None).unwrap(),
            },
            comparison_policy: ComparisonPolicy::ProcessByteExactV1,
            required_observers: vec!["cli_process".into()],
        };
        let baseline = Baseline {
            baseline_id: "baseline".into(),
            target_revision: executable_identity(&dir.join("before")).unwrap(),
            created_by: BaselineCreator::Human,
            approval: BaselineApproval {
                status: ApprovalStatus::Approved,
                actor: Some("test-author".into()),
                reason: Some("test fixture approval".into()),
            },
            observation_contract_hash: case.observation_contract_hash().unwrap(),
            stability_runs: Some(1),
            notes: None,
        };
        let auth = BehaviorAuthorization {
            approved_baselines: BTreeSet::from([canonical_hash(&baseline).unwrap()]),
            approved_checker_bindings: BTreeSet::from([checker_binding_hash(
                &baseline,
                &case.checker_claim().unwrap(),
            )
            .unwrap()]),
            baseline_stable: true,
        };
        let target = |name: &str| BehaviorTarget {
            identity: executable_identity(&dir.join(name)).unwrap(),
            executable: dir.join(name),
            input_identity: case.input_identity().unwrap(),
        };
        let experiment = BehaviorExperiment {
            schema_version: "1".into(),
            before: target("before"),
            after: target("after"),
            case,
            baseline,
            seed: 42,
        };
        fs::write(dir.join("auth.json"), serde_json::to_vec(&auth).unwrap()).unwrap();
        Self {
            dir,
            experiment,
            auth,
        }
    }
}
