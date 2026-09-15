use super::*;
use verify_core::blindtest::*;
use verify_core::blindtest::{execute, schema};
#[path = "../../../../benchmarks/corpus/blindtest.rs"]
mod corpus;
pub struct DockerCorpus {
    pub corpus: corpus::Corpus,
}
impl DockerCorpus {
    pub fn build(root: &Path) -> io::Result<Self> {
        docker::Docker::discover()?;
        // Existing P6 explicit known-mutant images. Constructor only writes trusted
        // controller material outside every agent workspace.
        let mut corpus = corpus::Corpus::new(root.to_path_buf());
        corpus.build_images();
        Ok(Self { corpus })
    }
}
pub fn run(
    case: &BenchmarkCase,
    root: &Path,
    images: &DockerCorpus,
    r: &mut BenchmarkCaseResult,
) -> io::Result<()> {
    let t = Instant::now();
    let mode = case.benchmark_case_id.split('.').next_back().unwrap();
    let fixture = corpus::Corpus::new(root.join("private-controller"));
    let target_mode = if matches!(mode, "partial" | "isolation" | "missing") {
        "correct"
    } else {
        mode
    };
    let workspace = &fixture.workspace;
    let public_source = images
        .corpus
        .root
        .join(format!("public-build-{target_mode}"));
    for name in ["target.js", "Dockerfile"] {
        fs::copy(public_source.join(name), workspace.join(name))?;
    }
    let mut c = fixture.config.clone();
    c.target.image = images.corpus.images[target_mode].clone();
    c.target.workspace_hash = verify_core::behavior::snapshot_identity(Some(workspace))?;
    if mode == "partial" {
        c.max_cases = 1;
    }
    if mode == "timeout" {
        c.target.bounds.timeout_ms = 200;
    }
    if mode == "isolation" {
        c.target.image = format!("sha256:{}", "0".repeat(64));
    }
    r.actual_config_hash = Some(canonical_hash(&c)?);
    r.actual_fixture_hash = Some(c.target.image.clone());
    write(&root.join("config.json"), &c)?;
    r.setup_ms = ms(t);
    let t = Instant::now();
    let raw = execute(&c, &fixture.sealed, &fixture.store, "result")?;
    let loaded = load(&fixture.store, "result")?;
    validate(
        &schema::<BlindTestRunResult>("blindtest-run-result", "1"),
        &loaded,
    )?;
    r.actual_verdict = Some(loaded.verdict);
    r.evidence_valid = Some(true);
    r.verified_reload = Some(raw == loaded);
    r.observation_coverage = Rate::new(loaded.complete_cases, loaded.suite.cases.len());
    r.result_refs.push(reference(&fixture.store, "result"));
    r.hidden_leakage = Some(usize::from(loaded.leakage_detected));
    let report = crate::load(&fixture.store, "result", &empty_auth())?;
    let agent = serde_json::to_string(&report.agent())?;
    let mut secrets = vec![
        loaded.suite.private_canary.clone(),
        loaded.suite.private_metadata.clone(),
        fixture.sealed.display().to_string(),
    ];
    for c in &loaded.suite.cases {
        secrets.push(c.case_id.clone());
        secrets.extend(c.args.clone());
        secrets.extend(c.environment.values().cloned());
    }
    r.agent_leakage = Some(
        secrets
            .iter()
            .filter(|s| !s.is_empty() && agent.contains(s.as_str()))
            .count(),
    );
    r.metrics.insert(
        "attested_case_coverage".into(),
        Rate::new(loaded.executions.len(), loaded.suite.cases.len()),
    );
    if case.category == "known_mutant" {
        r.metrics.insert(
            "Mutation adequacy on benchmark corpus".into(),
            Rate::new(usize::from(loaded.verdict == Verdict::Fail), 1),
        );
    }
    if loaded.verdict == Verdict::Fail {
        let raw_repro = execute(&c, &fixture.sealed, &fixture.store, "reproduction")?;
        let reproduced = load(&fixture.store, "reproduction")?;
        let signature = |v: &BlindTestRunResult| {
            v.violations
                .iter()
                .map(|v| (v.case_id.clone(), v.invariant_id.clone(), v.predicate_index))
                .collect::<Vec<_>>()
        };
        r.reproduction=Reproduction{semantics:"Trusted rerun of identical sealed suite and immutable Docker image; matching hidden predicate violations. No automatic replay CLI or reducer".into(),exists:!loaded.violations.is_empty(),verified:Some(raw_repro==reproduced && reproduced.verdict==Verdict::Fail && signature(&loaded)==signature(&reproduced)),refs:vec![reference(&fixture.store,"reproduction")],..Default::default()};
    }
    if mode == "correct" {
        let mut targets = vec![];
        for (name, expected) in [
            ("correct", Verdict::Pass),
            ("mutant_a", Verdict::Fail),
            ("mutant_b", Verdict::Fail),
            ("noop", Verdict::Fail),
        ] {
            let public = root.join(format!("quality-target-{name}"));
            fs::create_dir(&public)?;
            for file in ["target.js", "Dockerfile"] {
                fs::copy(
                    images
                        .corpus
                        .root
                        .join(format!("public-build-{name}"))
                        .join(file),
                    public.join(file),
                )?;
            }
            let mut vc = c.clone();
            vc.validation_receipt = None;
            vc.target.image = images.corpus.images[name].clone();
            vc.target.workspace_hash = verify_core::behavior::snapshot_identity(Some(&public))?;
            vc.target.workspace = public;
            targets.push(ValidationTarget {
                label: name.into(),
                config: vc,
                expected,
            });
        }
        let config = SuiteValidationConfig {
            schema_version: "1".into(),
            targets,
        };
        execute_validation(&config, &fixture.sealed, &fixture.store, "quality")?;
        let receipt = load_validation(&fixture.store, "quality")?;
        validate(
            &schema::<BlindTestSuiteValidationReceipt>("blindtest-suite-validation-receipt", "1"),
            &receipt,
        )?;
        let mut accurate = 0;
        for child in &receipt.runs {
            let observed = load(&fixture.store, &child.run_id)?.verdict;
            if child.observed == observed && child.expected == observed {
                accurate += 1;
            }
        }
        if !receipt.matched {
            accurate = 0;
        }
        r.metrics.insert(
            "suite_quality_receipt_accuracy".into(),
            Rate::new(accurate, config.targets.len()),
        );
        r.result_refs.push(reference(&fixture.store, "quality"));
    }
    if mode == "missing" {
        corrupt_evidence(&fixture.store, "result")?;
        let (observed, rejected) =
            loader_observation(load(&fixture.store, "result").map(|v| v.verdict));
        r.negative_evidence_rejected =
            Some(rejected && crate::load(&fixture.store, "result", &empty_auth()).is_err());
        r.actual_verdict = Some(observed);
        r.product_outcome = Some("VERIFIED_LOADER_AFTER_EVIDENCE_REMOVAL".into());
        r.limitations.push("Intact run schema/reload validated before removing required hidden evidence; ERROR denotes verified-load refusal".into());
    }
    r.execution_ms = ms(t);
    Ok(())
}
