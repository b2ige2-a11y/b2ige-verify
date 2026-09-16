//! P8 trusted, bounded benchmark controller. Product execution/verified loaders
//! own verdicts; independently authored labels own benchmark expectations.
#[cfg(unix)]
mod behavior;
#[cfg(not(unix))]
mod behavior {
    use super::{invalid, BenchmarkCase, BenchmarkCaseResult};
    use std::{io, path::Path};

    pub fn run(
        _case: &BenchmarkCase,
        _root: &Path,
        _result: &mut BenchmarkCaseResult,
    ) -> io::Result<()> {
        Err(invalid("Behavior benchmark runtime requires Unix"))
    }
}
#[cfg(unix)]
mod blindtest;
#[cfg(not(unix))]
mod blindtest {
    use super::{invalid, BenchmarkCase, BenchmarkCaseResult};
    use std::{io, path::Path};

    pub struct DockerCorpus;

    impl DockerCorpus {
        pub fn build(_root: &Path) -> io::Result<Self> {
            Err(invalid(
                "BlindTest benchmark runtime requires Unix and Docker",
            ))
        }
    }

    pub fn run(
        _case: &BenchmarkCase,
        _root: &Path,
        _images: &DockerCorpus,
        _result: &mut BenchmarkCaseResult,
    ) -> io::Result<()> {
        Err(invalid(
            "BlindTest benchmark runtime requires Unix and Docker",
        ))
    }
}
mod model;
mod sideeffect;
pub use model::*;
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use verify_core::{behavior::BehaviorAuthorization, Verdict};
use verify_evidence::{canonical_hash, store::EvidenceStore};
const DEFINITIONS: &str = include_str!("../../../../benchmarks/corpus/cases.v1.json");
const VERSION: &str = "b2ige-bench-v1";
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn ms(t: Instant) -> u64 {
    t.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}
fn write(p: &Path, v: &impl Serialize) -> io::Result<()> {
    fs::write(p, verify_evidence::canonical_bytes(v)?)
}
fn reference(store: &EvidenceStore, id: &str) -> String {
    store
        .root()
        .join(id)
        .join("result.json")
        .display()
        .to_string()
}
fn empty_auth() -> BehaviorAuthorization {
    BehaviorAuthorization {
        approved_baselines: Default::default(),
        approved_checker_bindings: Default::default(),
        baseline_stable: false,
    }
}
fn validate(schema: &serde_json::Value, value: &impl Serialize) -> io::Result<()> {
    let v = serde_json::to_value(value)?;
    let validator = jsonschema::validator_for(schema).map_err(io::Error::other)?;
    validator
        .validate(&v)
        .map_err(|e| io::Error::other(e.to_string()))
}
// A loader's successful verdict must remain observable even in a negative
// fixture. Only an actual loader error maps to the ERROR boundary.
fn loader_observation(result: io::Result<Verdict>) -> (Verdict, bool) {
    match result {
        Ok(verdict) => (verdict, false),
        Err(_) => (Verdict::Error, true),
    }
}
fn harness_failure(r: &mut BenchmarkCaseResult, error: io::Error) {
    // A partially observed result is not a completed, verified observation.
    // Keep its references for diagnosis, never count its provisional verdict.
    r.actual_verdict = None;
    r.harness_error = Some(error.to_string());
    r.evidence_valid = None;
    r.verified_reload = None;
}
fn corrupt_evidence(store: &EvidenceStore, id: &str) -> io::Result<()> {
    let evidence = store.root().join(id).join("evidence");
    let mut files: Vec<_> = fs::read_dir(evidence)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<_>>()?;
    files.sort();
    let file = files
        .into_iter()
        .next()
        .ok_or_else(|| invalid("missing evidence to corrupt"))?;
    fs::remove_file(file)
}
pub fn catalog() -> io::Result<Vec<BenchmarkCase>> {
    let mut cases: Vec<BenchmarkCase> = serde_json::from_str(DEFINITIONS)?;
    let mut ids = BTreeSet::new();
    for c in &mut cases {
        if c.schema_version != "1"
            || c.benchmark_case_id.is_empty()
            || !ids.insert(c.benchmark_case_id.clone())
            || c.allowed_verdicts.is_empty()
            || !["behavior", "sideeffect", "blindtest"].contains(&c.product.as_str())
        {
            return Err(invalid("invalid or duplicate benchmark case definition"));
        }
        c.config_identity = canonical_hash(c)?;
        c.fixture_identity = canonical_hash(&match c.product.as_str() {
            "behavior" => include_str!("behavior.rs").to_string(),
            "sideeffect" => format!(
                "{}{}",
                include_str!("sideeffect.rs"),
                include_str!("../../../../tests/fixtures/sideeffect.rs")
            ),
            "blindtest" => format!(
                "{}{}",
                include_str!("blindtest.rs"),
                include_str!("../../../../benchmarks/corpus/blindtest.rs")
            ),
            _ => return Err(invalid("unsupported corpus product")),
        })?;
        if !verify_evidence::valid_hash(&c.config_identity)
            || !verify_evidence::valid_hash(&c.fixture_identity)
        {
            return Err(invalid("benchmark case identity is not a content hash"));
        }
        validate(&schema::<BenchmarkCase>(), c)?;
    }
    Ok(cases)
}

/// Validate the non-runtime portion of a saved result before comparing it.
/// Stored summaries and semantic hashes remain derived and are recomputed by
/// callers; corpus and case identities are the independent comparison scope.
fn validate_snapshot_shape(run: &BenchmarkRunResult) -> io::Result<()> {
    validate(&schema::<BenchmarkRunResult>(), run)?;
    if run.schema_version != "1" || run.benchmark_version != VERSION {
        return Err(invalid("unsupported benchmark result version"));
    }
    let all = catalog()?;
    if run.corpus_hash != canonical_hash(&all)? {
        return Err(invalid("benchmark corpus hash mismatch"));
    }
    let all_ids: BTreeSet<_> = all.iter().map(|c| c.benchmark_case_id.clone()).collect();
    let requested: BTreeSet<_> = run.requested_case_ids.iter().cloned().collect();
    if requested.len() != run.requested_case_ids.len()
        || requested.is_empty() && !run.cases.is_empty()
        || !requested.is_subset(&all_ids)
        || run.complete_release_corpus != (requested == all_ids)
    {
        return Err(invalid("invalid benchmark selection"));
    }
    let seen: BTreeSet<_> = run
        .cases
        .iter()
        .map(|r| r.case.benchmark_case_id.clone())
        .collect();
    if seen.len() != run.cases.len() || seen != requested {
        return Err(invalid("benchmark result inventory mismatch"));
    }
    for r in &run.cases {
        let expected = all
            .iter()
            .find(|c| c.benchmark_case_id == r.case.benchmark_case_id)
            .ok_or_else(|| invalid("benchmark result contains unknown case"))?;
        if &r.case != expected {
            return Err(invalid("benchmark result case definition mismatch"));
        }
        if r.actual_verdict.is_some() {
            if r.harness_error.is_some()
                || r.actual_config_hash
                    .as_deref()
                    .is_none_or(|h| !verify_evidence::valid_hash(h))
                || r.actual_fixture_hash
                    .as_deref()
                    .is_none_or(|h| !verify_evidence::valid_hash(h))
            {
                return Err(invalid(
                    "completed benchmark result lacks identity evidence",
                ));
            }
        } else if r.harness_error.is_none() {
            return Err(invalid("benchmark result lacks verdict or harness error"));
        }
    }
    Ok(())
}
#[derive(Default)]
pub struct Options {
    pub product: Option<String>,
    pub case: Option<String>,
    pub reverse: bool,
    pub save: Option<PathBuf>,
}
fn version(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().into())
}
pub fn execute(options: &Options) -> io::Result<BenchmarkRunResult> {
    let total = Instant::now();
    if options.save.as_ref().is_some_and(|p| p.exists()) {
        return Err(invalid(
            "snapshot destination already exists; automatic overwrite is forbidden",
        ));
    }
    let all = catalog()?;
    let mut selected: Vec<_> = all
        .iter()
        .filter(|c| {
            options.product.as_ref().is_none_or(|p| p == &c.product)
                && options
                    .case
                    .as_ref()
                    .is_none_or(|id| id == &c.benchmark_case_id)
        })
        .cloned()
        .collect();
    if options.reverse {
        selected.reverse();
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    // Every raw BlindTest artifact stays in a private controller root outside the repo.
    let root = std::env::temp_dir().join(format!("b2ige-bench-{}-{nonce}", std::process::id()));
    fs::create_dir(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    }
    let build = Instant::now();
    let images = if selected.iter().any(|c| c.product == "blindtest") {
        Some(
            std::panic::catch_unwind(|| {
                let docker = verify_core::blindtest::docker::Docker::discover().map_err(|e| {
                    io::Error::other(format!("Docker corpus preflight: local engine unavailable: {e}"))
                })?;
                let base = std::env::var("B2IGE_P6_BASE_IMAGE")
                    .unwrap_or_else(|_| "node:24.18.1-bookworm-slim".into());
                let image = docker.resolve_image(&base).map_err(|e| {
                    io::Error::other(format!("Docker corpus preflight: local base image unavailable or invalid; check B2IGE_P6_BASE_IMAGE: {e}"))
                })?;
                if !image.inspect["RepoDigests"][0]
                    .as_str()
                    .is_some_and(|digest| digest.contains("@sha256:"))
                {
                    return Err(invalid("Docker corpus preflight: local base image lacks a repository digest"));
                }
                blindtest::DockerCorpus::build(&root.join("image-controller"))
            })
            .unwrap_or_else(|_| {
                Err(invalid(
                    "Docker corpus setup failed: local pinned Node base image and Docker required",
                ))
            }),
        )
    } else {
        None
    };
    let build_ms = ms(build);
    let mut rows = vec![];
    for c in &selected {
        let dir = root.join(&c.benchmark_case_id);
        fs::create_dir(&dir)?;
        let mut r = BenchmarkCaseResult::empty(c);
        let start = Instant::now();
        let result = match c.product.as_str() {
            "behavior" => behavior::run(c, &dir, &mut r),
            "sideeffect" => sideeffect::run(c, &dir, &mut r),
            "blindtest" => match images.as_ref().unwrap() {
                Ok(i) => blindtest::run(c, &dir, i, &mut r),
                Err(e) => Err(io::Error::other(e.to_string())),
            },
            _ => Err(invalid("unknown product")),
        };
        let result = result.and_then(|()| verify_recorded_verdict(&r));
        if let Err(e) = result {
            harness_failure(&mut r, e);
            r.execution_ms = ms(start);
        }
        validate(&schema::<BenchmarkCaseResult>(), &r)?;
        // Persist after every case. A crash leaves a partial journal, never a PASS run.
        write(&dir.join("benchmark-case.json"), &r)?;
        rows.push(r);
    }
    if let Some(r) = rows.iter_mut().find(|r| r.case.product == "blindtest") {
        r.setup_ms += build_ms;
    }
    let summary = summarize(&selected, &rows);
    let mut products = BTreeMap::new();
    for p in ["behavior", "sideeffect", "blindtest"] {
        let expected: Vec<_> = selected
            .iter()
            .filter(|c| c.product == p)
            .cloned()
            .collect();
        if !expected.is_empty() {
            let rr: Vec<_> = rows
                .iter()
                .filter(|r| r.case.product == p)
                .cloned()
                .collect();
            products.insert(p.into(), summarize(&expected, &rr));
        }
    }
    let mut run=BenchmarkRunResult{schema_version:"1".into(),benchmark_version:VERSION.into(),tool_version:env!("CARGO_PKG_VERSION").into(),platform:format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH),rust_version:version("rustc",&["--version"]).unwrap_or_else(||"unavailable".into()),docker_version:if images.is_some(){version("docker",&["version","--format","{{.Server.Version}}"])}else{None},corpus_hash:canonical_hash(&all)?,requested_case_ids:selected.iter().map(|c|c.benchmark_case_id.clone()).collect(),complete_release_corpus:selected.len()==all.len(),elapsed_ms:ms(total),summary,products,semantic_hash:semantic_hash(&rows)?,cases:rows,limitations:vec!["Small explicit bounded corpus; no exhaustive proof, generalized mutation score, or performance release threshold".into(),"Trusted local controller; same-user host access is outside Docker secrecy boundary".into(),"Rates with denominator zero are null / N/A. Expected corrupted artifacts are measured as loader rejections; intact pre-corruption artifacts are schema/reload checked".into(),"Use --reverse in a separate fresh run to measure order dependence; immutable Docker image layers may be reused, never run evidence".into()]};
    if !run.complete_release_corpus {
        run.summary.gate_pass = false;
        run.summary.blockers.push(
            "Full release gate requires all corpus cases; product summaries are scoped gates"
                .into(),
        );
    }
    validate_snapshot_shape(&run)?;
    validate(&schema::<BenchmarkRunResult>(), &run)?;
    write(&root.join("benchmark-run.json"), &run)?;
    if let Some(save) = &options.save {
        fs::create_dir(save)?;
        write(&save.join("result.json"), &run)?;
        fs::write(save.join("report.txt"), human(&run))?;
        write(
            &save.join("benchmark-run-result.v1.schema.json"),
            &schema::<BenchmarkRunResult>(),
        )?;
        fs::write(save.join("canonical.sha256"), canonical_hash(&run)?)?;
    }
    Ok(run)
}
fn source(r: &BenchmarkCaseResult) -> io::Result<(EvidenceStore, String)> {
    let path = Path::new(
        r.result_refs
            .first()
            .ok_or_else(|| invalid("missing product result reference"))?,
    );
    let run = path
        .parent()
        .ok_or_else(|| invalid("invalid result reference"))?;
    let id = run
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("invalid result id"))?
        .to_string();
    Ok((
        EvidenceStore::new(run.parent().ok_or_else(|| invalid("invalid store"))?),
        id,
    ))
}
pub fn verify_recorded_verdict(r: &BenchmarkCaseResult) -> io::Result<()> {
    let expected = catalog()?
        .into_iter()
        .find(|c| c.benchmark_case_id == r.case.benchmark_case_id)
        .ok_or_else(|| invalid("benchmark result contains unknown case"))?;
    if r.case != expected {
        return Err(invalid("benchmark case definition mismatch"));
    }
    if r.harness_error.is_some() || r.actual_verdict.is_none() || r.result_refs.is_empty() {
        return Err(invalid(
            "benchmark result is not a completed product observation",
        ));
    }
    if r.actual_config_hash
        .as_deref()
        .is_none_or(|h| !verify_evidence::valid_hash(h))
        || r.actual_fixture_hash
            .as_deref()
            .is_none_or(|h| !verify_evidence::valid_hash(h))
    {
        return Err(invalid("benchmark result lacks identity evidence"));
    }
    let (store, id) = source(r)?;
    let auth = if r.case.product == "behavior" {
        serde_json::from_slice(&fs::read(
            store
                .root()
                .parent()
                .ok_or_else(|| invalid("missing auth directory"))?
                .join("authorization.json"),
        )?)?
    } else {
        empty_auth()
    };
    let report = crate::load(&store, &id, &auth);
    if matches!(
        r.case.benchmark_case_id.as_str(),
        "sideeffect.corrupt" | "blindtest.missing"
    ) && r.negative_evidence_rejected == Some(true)
        && r.actual_verdict == Some(Verdict::Error)
    {
        if report.is_err() {
            return Ok(());
        }
        return Err(invalid("expected missing evidence was accepted"));
    }
    if Some(report?.document().verdict) != r.actual_verdict {
        return Err(invalid(
            "benchmark verdict differs from verified product loader",
        ));
    }
    Ok(())
}
pub fn human(r: &BenchmarkRunResult) -> String {
    let mut out=format!("B2IGE VERIFY BENCH\n{} · {}\n\nOverall\nCases                 {}\nKnown bugs detected   {}\nFalse PASS            {}\nFalse FAIL            {}\nRelease gate          {}\nElapsed               {} ms\n",r.benchmark_version,r.platform,r.summary.total_cases,r.summary.rates["true_bug_detection"].human(),r.summary.false_pass,r.summary.false_fail,if r.summary.gate_pass{"PASS"}else{"BLOCKED"},r.elapsed_ms);
    for (p, s) in &r.products {
        out.push_str(&format!(
            "\n{p} — {}\n",
            if s.gate_pass { "PASS" } else { "BLOCKED" }
        ));
        for (k, v) in &s.rates {
            out.push_str(&format!("{k}: {}\n", v.human()));
        }
        out.push_str(&format!(
            "Hidden leakage: {}\nAgent leakage: {}\n",
            s.hidden_leakage, s.agent_leakage
        ));
    }
    for c in &r.cases {
        out.push_str(&format!(
            "\n{}: {:?} → {:?}{}",
            c.case.benchmark_case_id,
            c.case.expected_classification,
            c.actual_verdict,
            c.harness_error
                .as_ref()
                .map(|e| format!("; HARNESS ERROR: {e}"))
                .unwrap_or_default()
        ));
    }
    for b in &r.summary.blockers {
        out.push_str(&format!("\nBLOCKER: {b}"));
    }
    out.push_str("\n\nLimitations\n");
    for l in &r.limitations {
        out.push_str(&format!("- {l}\n"));
    }
    out
}
/// Comparisons require the same independently versioned corpus; time and random
/// hidden values are excluded. Mixed changes are reported per case, not averaged.
pub fn compare(
    before: &BenchmarkRunResult,
    after: &BenchmarkRunResult,
) -> io::Result<serde_json::Value> {
    validate_snapshot_shape(before)?;
    validate_snapshot_shape(after)?;
    if before.requested_case_ids.is_empty() || after.requested_case_ids.is_empty() {
        return Err(invalid("zero-case benchmark comparison is not evidence"));
    }
    if before.benchmark_version != after.benchmark_version
        || before.schema_version != after.schema_version
        || before.corpus_hash != after.corpus_hash
        || before
            .requested_case_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            != after.requested_case_ids.iter().collect()
    {
        return Err(invalid("incompatible corpus or selection"));
    }
    let mut changes = BTreeMap::new();
    for a in &after.cases {
        let b = before
            .cases
            .iter()
            .find(|b| b.case.benchmark_case_id == a.case.benchmark_case_id)
            .ok_or_else(|| invalid("missing comparison case"))?;
        let label = if semantic_hash(std::slice::from_ref(a))?
            == semantic_hash(std::slice::from_ref(b))?
        {
            "unchanged"
        } else {
            let a_ok = summarize(std::slice::from_ref(&a.case), std::slice::from_ref(a)).gate_pass;
            let b_ok = summarize(std::slice::from_ref(&b.case), std::slice::from_ref(b)).gate_pass;
            if a_ok && !b_ok {
                "improvement"
            } else if !a_ok && b_ok {
                "regression"
            } else {
                measurement_change(b, a)
            }
        };
        changes.insert(a.case.benchmark_case_id.clone(), label);
    }
    Ok(
        json!({"corpus_hash":after.corpus_hash,"semantic_equal":semantic_hash(&before.cases)?==semantic_hash(&after.cases)?,"cases":changes}),
    )
}
fn measurement_change(before: &BenchmarkCaseResult, after: &BenchmarkCaseResult) -> &'static str {
    let mut better = false;
    let mut worse = false;
    let mut compare = |a: f64, b: f64| {
        better |= a > b;
        worse |= a < b;
    };
    for (name, a) in &after.metrics {
        if let Some(b) = before.metrics.get(name) {
            if let (Some(a), Some(b)) = (a.fraction, b.fraction) {
                compare(a, b);
            }
        }
    }
    if let (Some(a), Some(b)) = (
        after.observation_coverage.fraction,
        before.observation_coverage.fraction,
    ) {
        compare(a, b);
    }
    for (a, b) in [
        (after.reproduction.verified, before.reproduction.verified),
        (
            after.reproduction.replay_reproduced,
            before.reproduction.replay_reproduced,
        ),
        (
            after.reproduction.locally_minimized,
            before.reproduction.locally_minimized,
        ),
    ] {
        if let (Some(a), Some(b)) = (a, b) {
            compare(f64::from(u8::from(a)), f64::from(u8::from(b)));
        }
    }
    if let (Some(a), Some(b)) = (
        after.reproduction.reduced_size,
        before.reproduction.reduced_size,
    ) {
        compare(-(a as f64), -(b as f64));
    }
    if better && !worse {
        "improvement"
    } else {
        "regression"
    }
}
pub fn cli(args: &[String]) -> u8 {
    let usage="b2ige bench [behavior|sideeffect|blindtest] [--output human|json] [--save NEW_DIRECTORY] [--case ID] [--reverse]\nb2ige bench compare BASELINE.json CURRENT.json\nb2ige bench schemas NEW_DIRECTORY";
    if args == ["--help"] {
        println!("{usage}");
        return 0;
    }
    if args.first().is_some_and(|s| s == "schemas") {
        let result = (|| -> io::Result<()> {
            if args.len() != 2 {
                return Err(invalid(usage));
            }
            let p = Path::new(&args[1]);
            fs::create_dir(p)?;
            for (name, v) in [
                ("benchmark-case", schema::<BenchmarkCase>()),
                ("benchmark-case-result", schema::<BenchmarkCaseResult>()),
                ("benchmark-summary", schema::<BenchmarkSummary>()),
                ("benchmark-run-result", schema::<BenchmarkRunResult>()),
            ] {
                write(&p.join(format!("{name}.v1.schema.json")), &v)?;
            }
            Ok(())
        })();
        return match result {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                3
            }
        };
    }
    if args.first().is_some_and(|s| s == "compare") {
        let result = (|| -> io::Result<serde_json::Value> {
            if args.len() != 3 {
                return Err(invalid(usage));
            }
            let a = serde_json::from_slice(&fs::read(&args[1])?)?;
            let b = serde_json::from_slice(&fs::read(&args[2])?)?;
            compare(&a, &b)
        })();
        return match result {
            Ok(v) => {
                println!("{}", crate::pretty(&v));
                0
            }
            Err(e) => {
                eprintln!("{e}");
                3
            }
        };
    }
    let mut o = Options::default();
    let mut output = "human";
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "behavior" | "sideeffect" | "blindtest" if o.product.is_none() => {
                o.product = Some(args[i].clone())
            }
            "--reverse" => o.reverse = true,
            "--case" | "--save" | "--output" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("{usage}");
                    return 64;
                };
                match args[i].as_str() {
                    "--case" => o.case = Some(v.clone()),
                    "--save" => o.save = Some(PathBuf::from(v)),
                    _ => output = v,
                }
                i += 1;
            }
            _ => {
                eprintln!("{usage}");
                return 64;
            }
        }
        i += 1;
    }
    if !["human", "json"].contains(&output) {
        eprintln!("{usage}");
        return 64;
    }
    match execute(&o) {
        Ok(r) => {
            let success = if o.product.is_some() && o.case.is_none() {
                r.products.values().all(|s| s.gate_pass) && !r.products.is_empty()
            } else {
                r.summary.gate_pass
            };
            println!(
                "{}",
                if output == "json" {
                    crate::pretty(&r)
                } else {
                    human(&r)
                }
            );
            if success {
                0
            } else {
                1
            }
        }
        Err(e) => {
            if output == "json" {
                println!(
                    "{}",
                    json!({"schema_version":"1","harness_error":e.to_string(),"release_gate_pass":false})
                );
            } else {
                eprintln!("Benchmark ERROR: {e}");
            }
            3
        }
    }
}
#[cfg(test)]
mod tests;
