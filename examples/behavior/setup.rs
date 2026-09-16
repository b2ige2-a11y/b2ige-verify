use super::*;
use std::collections::{BTreeMap, BTreeSet};
use verify_core::{
    behavior::*, checker_binding_hash, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator,
};
pub fn setup(root: &Path, changed: bool) -> io::Result<()> {
    fs::create_dir(root)?;
    for (name, body) in [
        ("before", "printf hello"),
        (
            "after",
            if changed {
                "printf goodbye"
            } else {
                "printf hello"
            },
        ),
    ] {
        let p = root.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n"))?;
        make_executable(&p)?;
    }
    let case = BehaviorCase {
        schema_version: "1".into(),
        case_id: "hello".into(),
        args: vec!["base".into()],
        environment: BTreeMap::new(),
        timeout_ms: 5000,
        fixture: LocalFixture {
            source: None,
            snapshot_identity: snapshot_identity(None)?,
        },
        comparison_policy: ComparisonPolicy::ProcessByteExactV1,
        required_observers: vec!["cli_process".into()],
    };
    let baseline = Baseline {
        baseline_id: "public-example-v1".into(),
        target_revision: executable_identity(&root.join("before"))?,
        created_by: BaselineCreator::Human,
        approval: BaselineApproval {
            status: ApprovalStatus::Approved,
            actor: Some("example operator".into()),
            reason: Some("Explicit reference shell program; never candidate output".into()),
        },
        observation_contract_hash: case.observation_contract_hash()?,
        stability_runs: Some(3),
        notes: None,
    };
    let auth = BehaviorAuthorization {
        approved_baselines: BTreeSet::from([canonical_hash(&baseline)?]),
        approved_checker_bindings: BTreeSet::from([checker_binding_hash(
            &baseline,
            &case.checker_claim()?,
        )?]),
        baseline_stable: true,
    };
    let target = |name: &str| -> io::Result<BehaviorTarget> {
        Ok(BehaviorTarget {
            executable: root.join(name),
            identity: executable_identity(&root.join(name))?,
            input_identity: case.input_identity()?,
        })
    };
    let e = BehaviorExperiment {
        schema_version: "1".into(),
        before: target("before")?,
        after: target("after")?,
        case,
        baseline,
        seed: 42,
    };
    write_json(&root.join("authorization.json"), &auth)?;
    write_json(&root.join("experiment.json"), &e)?;
    Ok(())
}

fn make_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    {
        // Windows has no Unix executable mode. The public shell fixture is a
        // source/build fixture only on that platform; no verifier PASS is
        // inferred from creating it.
        let _ = path;
    }
    Ok(())
}
