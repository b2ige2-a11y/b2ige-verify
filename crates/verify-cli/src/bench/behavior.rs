use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::fs::PermissionsExt,
};
use verify_core::{
    behavior::{self, generation as gen, reduction as red, stability as stab, *},
    checker_binding_hash, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator,
};

fn setup(root: &Path, mode: &str) -> io::Result<(BehaviorExperiment, BehaviorAuthorization)> {
    let counter = root.join("counter");
    // Explicit deterministic evolving state, unique to this case, outside the reset snapshot.
    let noisy=format!("n=0; if [ -f '{}' ]; then read n < '{}'; fi; n=$((n+1)); printf '%s\\n' \"$n\" > '{}'; printf '%s' \"$n\"",counter.display(),counter.display(),counter.display());
    let before = if matches!(mode, "noise" | "mixed") {
        noisy.clone()
    } else {
        "printf same".into()
    };
    let after = match mode {
        "preserving" => "v=same; printf '%s' \"$v\"".into(),
        "stdout" => "printf changed".into(),
        "stderr" => "printf same; printf changed >&2".into(),
        "exit" => "printf same; exit 7".into(),
        "signal" => "printf same; kill -TERM $$".into(),
        "noise" => noisy.clone(),
        "mixed" => format!("{noisy}; printf changed >&2"),
        "generated" | "reducible" => {
            "if [ \"$1\" = trigger ]; then printf changed; else printf same; fi".into()
        }
        _ => return Err(invalid("unknown behavior case")),
    };
    for (name, body) in [("before", before), ("after", after)] {
        let p = root.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n"))?;
        fs::set_permissions(p, fs::Permissions::from_mode(0o700))?;
    }
    let case = BehaviorCase {
        schema_version: "1".into(),
        case_id: "benchmark".into(),
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
        baseline_id: "human-authored-bench-v1".into(),
        target_revision: executable_identity(&root.join("before"))?,
        created_by: BaselineCreator::Human,
        approval: BaselineApproval {
            status: ApprovalStatus::Approved,
            actor: Some("benchmark corpus maintainer".into()),
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
    write(&root.join("authorization.json"), &auth)?;
    write(&root.join("experiment.json"), &e)?;
    Ok((e, auth))
}
pub fn run(case: &BenchmarkCase, root: &Path, r: &mut BenchmarkCaseResult) -> io::Result<()> {
    let t = Instant::now();
    let mode = case.benchmark_case_id.split('.').next_back().unwrap();
    let (e, auth) = setup(root, mode)?;
    r.actual_config_hash = Some(canonical_hash(&e)?);
    r.actual_fixture_hash = Some(canonical_hash(&(&e.before.identity, &e.after.identity))?);
    let store = EvidenceStore::new(root.join("runs"));
    let work = root.join("work");
    r.setup_ms = ms(t);
    let t = Instant::now();
    if matches!(mode, "noise" | "mixed") {
        let p = stab::profile(
            &store,
            &work,
            "profile",
            &e,
            &auth,
            &stab::ProfilingConfig { repetitions: 3 },
        )?;
        let raw = stab::compare(&store, &work, "result", &e, &auth, &p.reference()?)?;
        let loaded = stab::load_comparison(&store, "result", &auth)?;
        validate(&stab::comparison_schema(), &loaded)?;
        r.actual_verdict = Some(loaded.verdict);
        r.product_outcome = Some(format!("{:?}", loaded.outcome));
        r.evidence_valid = Some(true);
        r.verified_reload = Some(raw == loaded);
        r.observation_coverage = Rate::new(
            p.observables
                .iter()
                .filter(|o| o.stability != stab::Stability::Incomplete)
                .count(),
            p.observables.len(),
        );
        r.metrics.insert(
            "noise_handling".into(),
            Rate::new(
                usize::from(if mode == "noise" {
                    loaded.verdict == Verdict::Inconclusive
                } else {
                    loaded.verdict == Verdict::Fail
                }),
                1,
            ),
        );
        r.result_refs.push(reference(&store, "result"));
        if loaded.verdict == Verdict::Fail {
            let rerun = stab::compare(&store, &work, "reproduction", &e, &auth, &p.reference()?)?;
            let rerun = stab::load_comparison(&store, &rerun.comparison_id, &auth)?;
            r.reproduction=Reproduction{semantics:"Fresh candidate against recorded baseline stability profile; divergence observable signature".into(),exists:true,verified:Some(rerun.divergences.iter().map(|x|&x.observable).eq(loaded.divergences.iter().map(|x|&x.observable))),refs:vec![reference(&store,"reproduction")],..Default::default()};
        }
    } else if matches!(mode, "generated" | "reducible") {
        let base = behavior::execute(&store, &work, "base", &e, &auth)?;
        if behavior::load(&store, "base", &auth)?.verdict != Verdict::Pass
            || base.verdict != Verdict::Pass
        {
            return Err(invalid("generated-only bug also fails base input"));
        }
        let spec = gen::GenerationSpec {
            generation_id: "bounded-benchmark".into(),
            schema_version: "1".into(),
            base: e,
            mode: gen::GenerationMode::Cartesian,
            dimensions: vec![
                gen::Dimension {
                    target: gen::DimensionTarget::Argument { index: 0 },
                    candidates: vec!["trigger".into()],
                },
                gen::Dimension {
                    target: gen::DimensionTarget::Environment {
                        name: "IRRELEVANT".into(),
                    },
                    candidates: vec!["x".into()],
                },
            ],
            max_cases: 4,
        };
        let deterministic = gen::generate(&spec)? == gen::generate(&spec)?;
        let raw = gen::execute(&store, &work, "result", &spec, &auth)?;
        let suite = gen::load(&store, "result", &auth)?;
        validate(&gen::suite_schema(), &suite)?;
        r.actual_verdict = Some(suite.aggregate_verdict);
        r.product_outcome = Some("GENERATED_SUITE".into());
        r.evidence_valid = Some(true);
        r.verified_reload = Some(raw == suite);
        r.observation_coverage = Rate::new(
            suite.coverage.completed_pairs as usize,
            suite.coverage.expected_cases as usize,
        );
        r.metrics.insert(
            "deterministic_generation".into(),
            Rate::new(usize::from(deterministic), 1),
        );
        r.metrics.insert(
            "generator_bug_discovery".into(),
            Rate::new(usize::from(suite.aggregate_verdict == Verdict::Fail), 1),
        );
        r.result_refs.push(reference(&store, "result"));
        let source = suite
            .comparisons
            .iter()
            .find(|c| {
                behavior::load(&store, &c.comparison_id, &auth)
                    .is_ok_and(|v| v.verdict == Verdict::Fail)
            })
            .ok_or_else(|| invalid("generator did not discover known bug"))?;
        let failing = behavior::load(&store, &source.comparison_id, &auth)?;
        if mode == "reducible" {
            let identity = suite
                .generated_cases
                .iter()
                .find(|c| c.case_identity == failing.case_identity)
                .ok_or_else(|| invalid("missing generated identity"))?;
            let input = red::ReductionInput {
                source_suite_id: suite.suite_id.clone(),
                source_suite_hash: canonical_hash(&suite)?,
                source_comparison: source.clone(),
                failing_case: identity.clone(),
                generation_spec: spec,
                expected_signature: red::signature(&failing)?,
                execution_budget: 16,
            };
            red::execute(&store, &work, "reduction", &input, &auth)?;
            let reduced = red::load(&store, "reduction", &auth)?;
            validate(&red::result_schema(), &reduced)?;
            let final_run = behavior::load(&store, &reduced.final_comparison.comparison_id, &auth)?;
            r.reproduction = Reproduction {
                semantics:
                    "P3 deterministic assignment-deletion neighborhood; proven observable signature"
                        .into(),
                exists: true,
                verified: Some(red::signature(&final_run)? == input.expected_signature),
                reduction_available: true,
                locally_minimized: Some(reduced.minimal),
                original_size: Some(reduced.original_assignment.len()),
                reduced_size: Some(reduced.reduced_assignment.len()),
                refs: vec![
                    reference(&store, "reduction"),
                    reference(&store, &reduced.final_comparison.comparison_id),
                ],
                ..Default::default()
            };
            r.metrics.insert(
                "reduction_success".into(),
                Rate::new(
                    usize::from(
                        reduced.minimal
                            && reduced.reduced_assignment.len() < reduced.original_assignment.len(),
                    ),
                    1,
                ),
            );
        } else {
            reproduce(&store, &work, &failing, &auth, r)?;
        }
    } else {
        let raw = behavior::execute(&store, &work, "result", &e, &auth)?;
        let loaded = behavior::load(&store, "result", &auth)?;
        validate(&behavior::comparison_schema(), &loaded)?;
        r.actual_verdict = Some(loaded.verdict);
        r.product_outcome = Some(format!("{:?}", loaded.outcome));
        r.evidence_valid = Some(true);
        r.verified_reload = Some(raw == loaded);
        r.observation_coverage = Rate::new(
            loaded
                .coverage
                .values()
                .filter(|c| serde_json::to_value(c).is_ok_and(|v| v == "complete"))
                .count(),
            loaded.coverage.len(),
        );
        r.result_refs.push(reference(&store, "result"));
        if loaded.verdict == Verdict::Fail {
            reproduce(&store, &work, &loaded, &auth, r)?;
        }
    }
    let report = crate::load(&store, "result", &auth)?;
    if Some(report.document().verdict) != r.actual_verdict {
        return Err(invalid("verified report verdict mismatch"));
    }
    r.execution_ms = ms(t);
    Ok(())
}
fn reproduce(
    store: &EvidenceStore,
    work: &Path,
    original: &BehaviorComparisonResult,
    auth: &BehaviorAuthorization,
    r: &mut BenchmarkCaseResult,
) -> io::Result<()> {
    behavior::execute(store, work, "reproduction", &original.experiment, auth)?;
    let new = behavior::load(store, "reproduction", auth)?;
    r.reproduction = Reproduction {
        semantics:
            "Fresh P2 pair with recorded controllable experiment inputs; exact divergence signature"
                .into(),
        exists: true,
        verified: Some(red::signature(&new)? == red::signature(original)?),
        refs: vec![reference(store, "reproduction")],
        ..Default::default()
    };
    let mut replayed = true;
    for (i, id) in original
        .before_run_id
        .iter()
        .chain(original.after_run_id.iter())
        .enumerate()
    {
        let replay = verify_core::acquisition::replay::execute(store, id, &format!("replay-{i}"));
        replayed &= serde_json::to_value(&replay)?["status"] == "REPRODUCED";
        r.reproduction
            .refs
            .push(reference(store, &format!("replay-{i}")));
    }
    r.reproduction.replay_available = true;
    r.reproduction.replay_reproduced = Some(replayed);
    r.reproduction.unavailable_reason = None;
    Ok(())
}
