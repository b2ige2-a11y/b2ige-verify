//! P6 trusted controller. Hidden artifacts and raw evidence stay outside the agent
//! workspace. Verdicts are recomputed from approved predicates and runtime evidence.
pub mod docker;
mod model;
use crate::{behavior, Verdict};
pub use model::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, fs, io, path::Path};
use verify_evidence::{canonical_hash, store::EvidenceStore, Evidence, Observation, TrustClass};

pub const CHECKER: &str = "b2ige.blindtest.cli-bytes-and.v1";
pub const QUALITY_LIMIT: &str =
    "Bounded self-validation only; no exhaustive mutation adequacy or perfect secrecy claim.";
pub(crate) fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn nonempty(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.contains('\0')
}
fn approved(p: &Provenance) -> bool {
    matches!(
        p.status,
        ProvenanceStatus::Approved | ProvenanceStatus::Authoritative
    ) && nonempty(&p.source)
        && p.approved_by.as_ref().is_some_and(|s| nonempty(s))
}
pub fn requirement_hash(r: &RequirementArtifact) -> io::Result<String> {
    Ok(canonical_hash(&(
        &r.schema_version,
        &r.requirement_id,
        &r.text,
        &r.source,
        &r.version,
    ))?)
}
pub fn invariant_binding(i: &InvariantArtifact) -> io::Result<ApprovedInvariantRef> {
    let artifact_hash = canonical_hash(i)?;
    Ok(ApprovedInvariantRef {
        invariant_id: i.invariant_id.clone(),
        requirement_hash: i.requirement_hash.clone(),
        checker_binding_hash: canonical_hash(&(
            CHECKER,
            &artifact_hash,
            &i.requirement_id,
            &i.requirement_hash,
        ))?,
        artifact_hash,
    })
}
fn valid_predicate(p: &Predicate) -> bool {
    match p {
        Predicate::ExitEquals { value } | Predicate::ExitNotEquals { value } => {
            (0..=255).contains(value)
        }
        Predicate::StdoutEquals { bytes } | Predicate::StderrEquals { bytes } => {
            bytes.len() <= 65536
        }
        Predicate::StdoutNotContains { bytes } | Predicate::StderrNotContains { bytes } => {
            !bytes.is_empty() && bytes.len() <= 65536
        }
    }
}
pub fn validate_suite(s: &SealedSuite, c: &BlindTestConfig) -> io::Result<()> {
    if s.schema_version != "1"
        || s.manifest.schema_version != "1"
        || !identifier(&s.manifest.suite_id)
        || !nonempty(&s.manifest.version)
        || !approved(&s.manifest.provenance)
        || canonical_hash(s)? != c.suite_hash
        || s.manifest.approved_invariants != c.approved_invariants
        || s.cases.is_empty()
        || s.cases.len() > 128
        || s.cases.len() != s.manifest.case_hashes.len()
        || s.invariants.is_empty()
        || s.invariants.len() > 64
        || s.requirements.is_empty()
        || s.requirements.len() > 64
        || s.private_canary.len() < 32
        || s.private_canary.len() > 256
        || !s.private_canary.starts_with("BLINDTEST_PRIVATE_CANARY_")
        || !s.private_metadata.contains(&s.private_canary)
        || s.invariants.len() != c.approved_invariants.len()
    {
        return Err(invalid("sealed suite integrity/approval invalid"));
    }
    if std::iter::once(&c.target.command)
        .chain(c.target.args.iter())
        .chain(c.target.environment.keys())
        .chain(c.target.environment.values())
        .any(|v| v.contains(&s.private_canary))
    {
        return Err(invalid("private canary overlaps fixed target input"));
    }
    let mut requirements = BTreeSet::new();
    for r in &s.requirements {
        if r.schema_version != "1"
            || !identifier(&r.requirement_id)
            || !requirements.insert(&r.requirement_id)
            || !nonempty(&r.text)
            || !nonempty(&r.source)
            || !nonempty(&r.version)
            || requirement_hash(r)? != r.content_hash
        {
            return Err(invalid("requirement identity/hash invalid"));
        }
    }
    let mut invariants = BTreeSet::new();
    for i in &s.invariants {
        if i.schema_version != "2"
            || !identifier(&i.invariant_id)
            || !invariants.insert(&i.invariant_id)
            || !approved(&i.provenance)
            || !nonempty(&i.public_summary)
            || !nonempty(&i.expected_semantic)
            || i.predicates.is_empty()
            || i.predicates.len() > 16
            || !i.predicates.iter().all(valid_predicate)
            || !s.requirements.iter().any(|r| {
                r.requirement_id == i.requirement_id && r.content_hash == i.requirement_hash
            })
            || c.approved_invariants
                .iter()
                .filter(|a| a.invariant_id == i.invariant_id)
                .count()
                != 1
            || !c.approved_invariants.contains(&invariant_binding(i)?)
        {
            return Err(invalid(
                "approved invariant/checker/requirement binding invalid",
            ));
        }
    }
    let mut cases = BTreeSet::new();
    for case in &s.cases {
        let i = s
            .invariants
            .iter()
            .find(|i| i.invariant_id == case.invariant_id)
            .ok_or_else(|| invalid("hidden case invariant missing"))?;
        if case.schema_version != "1"
            || !identifier(&case.case_id)
            || !cases.insert(&case.case_id)
            || s.manifest.case_hashes.get(&case.case_id) != Some(&canonical_hash(case)?)
            || case.oracle.as_ref().is_none_or(|o| {
                o.schema_version != "1"
                    || o.checker != CHECKER
                    || o.invariant_hash != canonical_hash(i).unwrap_or_default()
                    || o.predicates != i.predicates
            })
            || !nonempty(&case.public_failure_label)
            || case.reproduction_template.is_empty()
            || case.reproduction_template.len() > 8
            || !case.reproduction_template.iter().all(|v| nonempty(v))
            || case.args.len() > c.target.max_case_args
            || !case
                .args
                .iter()
                .all(|v| v.len() <= 8192 && !v.contains('\0'))
            || case.environment.len() > 32
            || case
                .environment
                .iter()
                .any(|(k, v)| !c.target.allowed_case_env.contains(k) || !nonempty(v))
            || case
                .fixture
                .as_ref()
                .is_some_and(|f| !identifier(&f.name) || f.bytes.len() > 16384)
        {
            return Err(invalid(
                "hidden case/oracle integrity or input policy invalid",
            ));
        }
        // A canary must never be supplied as current input, including encoded fixture bytes.
        if contains_canary(
            s,
            &serde_json::to_vec(&(&case.args, &case.environment, &case.fixture))?,
        ) || case
            .args
            .iter()
            .chain(case.environment.values())
            .any(|v| v.contains(&s.private_canary))
            || case
                .fixture
                .as_ref()
                .is_some_and(|f| contains_canary(s, &f.bytes))
        {
            return Err(invalid("private canary overlaps target input"));
        }
    }
    if s.invariants.iter().any(|i| {
        !s.cases
            .iter()
            .any(|case| case.invariant_id == i.invariant_id)
    }) {
        return Err(invalid("approved invariant has no required hidden case"));
    }
    Ok(())
}
impl BlindTestConfig {
    pub fn validate(&self) -> io::Result<()> {
        let t = &self.target;
        let b = &t.bounds;
        if self.schema_version != "1"
            || !verify_evidence::valid_hash(&self.suite_hash)
            || self.required_isolation != IsolationLevel::DockerIsolation
            || self.max_cases > 128
            || self.approved_invariants.is_empty()
            || !nonempty(&t.image)
            || t.image.starts_with('-')
            || t.image.chars().any(char::is_whitespace)
            || !t.command.starts_with('/')
            || !nonempty(&t.command)
            || t.args.len() > 64
            || t.args.iter().any(|v| v.len() > 8192 || v.contains('\0'))
            || t.max_case_args > 64
            || !t.workspace.is_absolute()
            || !verify_evidence::valid_hash(&t.workspace_hash)
            || !nonempty(&t.build_identity)
            || t.environment.len() > 32
            || t.allowed_case_env.len() > 32
            || t.allowed_case_env.iter().any(|k| !env_key(k))
            || t.environment
                .iter()
                .any(|(k, v)| !env_key(k) || !nonempty(v))
            || t.allowed_case_env.iter().collect::<BTreeSet<_>>().len() != t.allowed_case_env.len()
            || t.allowed_case_env
                .iter()
                .any(|k| t.environment.contains_key(k))
            || b.timeout_ms == 0
            || b.timeout_ms > 60_000
            || !(16 * 1024 * 1024..=1024 * 1024 * 1024).contains(&b.memory_bytes)
            || b.nano_cpus == 0
            || b.nano_cpus > 2_000_000_000
            || b.pids == 0
            || b.pids > 256
            || b.tmpfs_bytes == 0
            || b.tmpfs_bytes > 64 * 1024 * 1024
            || self
                .validation_receipt
                .as_ref()
                .is_some_and(|id| !identifier(id))
        {
            return Err(invalid("invalid or unsupported BlindTest config/isolation"));
        }
        Ok(())
    }
}
fn env_key(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 64
        && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !k.starts_with("B2IGE_FIXTURE_")
}
/// Canonicalize both directions; reject aliases and ancestors, not just lexical prefixes.
/// Requiring an existing private root avoids creating secrets through untrusted symlinks.
pub fn check_paths(workspace: &Path, sealed: &Path, store: &Path) -> io::Result<()> {
    let w = fs::canonicalize(workspace)?;
    let s = fs::canonicalize(sealed)?;
    if !w.is_dir() || !s.is_dir() || w.starts_with(&s) || s.starts_with(&w) {
        return Err(invalid("sealed/workspace paths overlap"));
    }
    let existing = if store.exists() {
        store
    } else {
        store
            .parent()
            .ok_or_else(|| invalid("private store parent missing"))?
    };
    let p = fs::canonicalize(existing)?;
    if p.starts_with(&w) || w.starts_with(&p) {
        return Err(invalid("private evidence store overlaps workspace"));
    }
    Ok(())
}
pub fn read_suite(root: &Path) -> io::Result<SealedSuite> {
    let file = root.join("suite.json");
    let meta = fs::symlink_metadata(&file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(invalid("sealed suite must not have hard-link aliases"));
        }
    }
    if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
        return Err(invalid("sealed suite must be a bounded regular file"));
    }
    Ok(serde_json::from_slice(&fs::read(file)?)?)
}
pub fn contains_canary(s: &SealedSuite, bytes: &[u8]) -> bool {
    bytes
        .windows(s.private_canary.len())
        .any(|w| w == s.private_canary.as_bytes())
}
fn predicate_holds(p: &Predicate, exit: i32, stdout: &[u8], stderr: &[u8]) -> bool {
    match p {
        Predicate::ExitEquals { value } => exit == *value,
        Predicate::ExitNotEquals { value } => exit != *value,
        Predicate::StdoutEquals { bytes } => stdout == bytes,
        Predicate::StderrEquals { bytes } => stderr == bytes,
        Predicate::StdoutNotContains { bytes } => !stdout.windows(bytes.len()).any(|w| w == bytes),
        Predicate::StderrNotContains { bytes } => !stderr.windows(bytes.len()).any(|w| w == bytes),
    }
}
fn evidence<T: Serialize>(
    run: &str,
    name: &str,
    source: &str,
    trust: TrustClass,
    value: &T,
) -> io::Result<Evidence> {
    let observation = Observation::Value {
        value: serde_json::to_value(value)?,
    };
    Ok(Evidence {
        evidence_id: name.into(),
        run_id: run.into(),
        source: source.into(),
        trust_class: trust,
        order: 0,
        integrity_hash: canonical_hash(&observation)?,
        observation,
        related_claim_ids: vec!["blindtest".into()],
    })
}
fn require<T: Serialize>(
    items: &[Evidence],
    id: &str,
    name: &str,
    source: &str,
    trust: TrustClass,
    value: &T,
) -> io::Result<()> {
    if items.iter().find(|e| e.evidence_id == name)
        != Some(&evidence(id, name, source, trust, value)?)
    {
        return Err(invalid(
            "hidden result contradicts required source evidence",
        ));
    }
    Ok(())
}
fn recompute(r: &mut BlindTestRunResult) -> io::Result<()> {
    r.config.validate()?;
    validate_suite(&r.suite, &r.config)?;
    r.verdict = Verdict::Inconclusive;
    r.violations.clear();
    r.complete_cases = 0;
    r.leakage_detected = false;
    if r.schema_version != "1"
        || !identifier(&r.blindtest_result_id)
        || r.executions.len() > r.config.max_cases
        || r.executions.len() > r.suite.cases.len()
    {
        return Err(invalid("invalid hidden run coverage"));
    }
    if r.controller_error.as_ref().is_some_and(|e| {
        ![
            "docker_initialization_failed",
            "image_resolution_failed",
            "isolation_or_runtime_failed",
            "workspace_identity_changed",
        ]
        .contains(&e.as_str())
    }) {
        return Err(invalid("unknown controller error"));
    }
    let Some(image) = &r.image else {
        if !r.executions.is_empty() || r.controller_error.is_none() {
            return Err(invalid("image evidence missing"));
        }
        r.verdict = Verdict::Error;
        return Ok(());
    };
    docker::validate_image(image)?;
    if (r.config.target.image.starts_with("sha256:") && r.config.target.image != image.image_id)
        || (r.config.target.image.contains("@sha256:")
            && !image.inspect["RepoDigests"]
                .as_array()
                .is_some_and(|v| v.contains(&serde_json::json!(r.config.target.image))))
    {
        return Err(invalid(
            "resolved image differs from requested immutable identity",
        ));
    }
    for (index, x) in r.executions.iter().enumerate() {
        let case = &r.suite.cases[index];
        if x.case_id != case.case_id || x.image_id != image.image_id {
            return Err(invalid("hidden execution plan order/identity mismatch"));
        }
        let a = &x.attestation;
        docker::validate_attestation(a, &r.config, image, case)?;
        if contains_canary(&r.suite, &x.capture.stdout)
            || contains_canary(&r.suite, &x.capture.stderr)
        {
            r.leakage_detected = true;
            r.violations.push(PrivateViolation {
                case_id: case.case_id.clone(),
                invariant_id: case.invariant_id.clone(),
                predicate_index: 0,
                failure_kind: "isolation_leakage".into(),
                evidence_ref: format!("execution-{index}"),
            });
        }
        let capture = &x.capture;
        if capture.stdout.len() > verify_runner::process::CAPTURE_LIMIT
            || capture.stderr.len() > verify_runner::process::CAPTURE_LIMIT
            || [&capture.started_at, &capture.ended_at].iter().any(|s| {
                !s.strip_prefix("unix-ns:")
                    .is_some_and(|t| t.parse::<u128>().is_ok_and(|n| n > 0))
            })
        {
            return Err(invalid("invalid bounded process acquisition metadata"));
        }
        if !capture.started
            || capture.timed_out
            || capture.runner_failure.is_some()
            || capture.signal.is_some()
            || !x.cleanup_confirmed
        {
            continue;
        }
        let state = &a.after["State"];
        if state["OOMKilled"] != false {
            continue;
        }
        let exit = state["ExitCode"]
            .as_i64()
            .ok_or_else(|| invalid("target exit missing"))? as i32;
        if capture.exit_code != Some(exit) {
            return Err(invalid("Docker attach exit disagrees with target state"));
        }
        r.complete_cases += 1;
        let oracle = case
            .oracle
            .as_ref()
            .ok_or_else(|| invalid("oracle missing"))?;
        for (p_index, p) in oracle.predicates.iter().enumerate() {
            if !predicate_holds(p, exit, &capture.stdout, &capture.stderr) {
                r.violations.push(PrivateViolation {
                    case_id: case.case_id.clone(),
                    invariant_id: case.invariant_id.clone(),
                    predicate_index: p_index,
                    failure_kind: "contract_violation".into(),
                    evidence_ref: format!("execution-{index}"),
                });
            }
        }
    }
    r.verdict = if r.controller_error.is_some() {
        Verdict::Error
    } else if !r.violations.is_empty() {
        Verdict::Fail
    } else if r.complete_cases == r.suite.cases.len() {
        Verdict::Pass
    } else {
        Verdict::Inconclusive
    };
    Ok(())
}
/// All secret-bearing outputs use the private store; the public config has no locator.
pub fn execute(
    c: &BlindTestConfig,
    sealed: &Path,
    store: &EvidenceStore,
    id: &str,
) -> io::Result<BlindTestRunResult> {
    c.validate()?;
    check_paths(&c.target.workspace, sealed, store.root())?;
    if behavior::snapshot_identity(Some(&c.target.workspace))? != c.target.workspace_hash {
        return Err(invalid("public workspace identity mismatch"));
    }
    let suite = read_suite(sealed)?;
    validate_suite(&suite, c)?;
    let quality = match &c.validation_receipt {
        Some(receipt) => {
            let q = load_validation(store, receipt)?;
            if q.suite_hash != c.suite_hash {
                return Err(invalid("quality receipt suite mismatch"));
            }
            if q.matched {
                "self-validation supplied; bounded corpus matched"
            } else {
                "self-validation supplied; corpus mismatch"
            }
        }
        None => "self-validation not supplied",
    }
    .into();
    let run = store.reserve(id)?;
    let mut r = BlindTestRunResult {
        schema_version: "1".into(),
        blindtest_result_id: id.into(),
        config: c.clone(),
        suite,
        image: None,
        executions: vec![],
        controller_error: None,
        verdict: Verdict::Error,
        violations: vec![],
        complete_cases: 0,
        leakage_detected: false,
        quality,
    };
    let mut items = vec![evidence(
        id,
        "plan",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &(&r.config, &r.suite),
    )?];
    match docker::Docker::discover() {
        Err(_) => r.controller_error = Some("docker_initialization_failed".into()),
        Ok(docker) => match docker.resolve_image(&c.target.image) {
            Err(_) => r.controller_error = Some("image_resolution_failed".into()),
            Ok(image) => {
                r.image = Some(image.clone());
                items.push(evidence(
                    id,
                    "image",
                    "docker_image_inspect",
                    TrustClass::DirectRuntime,
                    &image,
                )?);
                for (index, case) in r.suite.cases.iter().take(c.max_cases).enumerate() {
                    match docker.execute(c, &image, case) {
                        Ok(x) => {
                            items.push(evidence(
                                id,
                                &format!("execution-{index}"),
                                "docker_runtime",
                                TrustClass::DirectRuntime,
                                &x,
                            )?);
                            r.executions.push(x);
                        }
                        Err(_) => {
                            r.controller_error = Some("isolation_or_runtime_failed".into());
                            break;
                        }
                    }
                }
            }
        },
    }
    if !behavior::snapshot_identity(Some(&c.target.workspace))
        .is_ok_and(|h| h == c.target.workspace_hash)
    {
        r.controller_error = Some("workspace_identity_changed".into());
    }
    items.push(evidence(
        id,
        "controller",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &r.controller_error,
    )?);
    recompute(&mut r)?;
    for e in &items {
        run.write_evidence(e)?;
    }
    run.complete(
        &r,
        &items
            .iter()
            .map(|e| e.evidence_id.clone())
            .collect::<Vec<_>>(),
    )?;
    load(store, id)
}
pub fn load(store: &EvidenceStore, id: &str) -> io::Result<BlindTestRunResult> {
    load_inner(store, id, true)
}
fn load_inner(store: &EvidenceStore, id: &str, quality: bool) -> io::Result<BlindTestRunResult> {
    let (v, items) = store.load(id)?;
    let mut r: BlindTestRunResult = serde_json::from_value(v)?;
    // A copied raw store in the recorded agent workspace cannot retain a secrecy
    // claim. Historical reports otherwise do not require the live source tree.
    let store_path = fs::canonicalize(store.root())?;
    let workspace = fs::canonicalize(&r.config.target.workspace)
        .unwrap_or_else(|_| r.config.target.workspace.clone());
    if store_path.starts_with(&workspace) || workspace.starts_with(&store_path) {
        return Err(invalid("private evidence stored in agent workspace"));
    }
    if r.blindtest_result_id != id {
        return Err(invalid("hidden result run identity mismatch"));
    }
    require(
        &items,
        id,
        "plan",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &(&r.config, &r.suite),
    )?;
    require(
        &items,
        id,
        "controller",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &r.controller_error,
    )?;
    if let Some(image) = &r.image {
        require(
            &items,
            id,
            "image",
            "docker_image_inspect",
            TrustClass::DirectRuntime,
            image,
        )?;
    }
    for (index, x) in r.executions.iter().enumerate() {
        require(
            &items,
            id,
            &format!("execution-{index}"),
            "docker_runtime",
            TrustClass::DirectRuntime,
            x,
        )?;
    }
    if items.len() != 2 + usize::from(r.image.is_some()) + r.executions.len() {
        return Err(invalid("hidden evidence inventory differs from executions"));
    }
    r.quality = match &r.config.validation_receipt {
        Some(receipt) if quality => {
            let q = load_validation(store, receipt)?;
            if q.suite_hash != r.config.suite_hash {
                return Err(invalid("quality receipt suite mismatch"));
            }
            if q.matched {
                "self-validation supplied; bounded corpus matched"
            } else {
                "self-validation supplied; corpus mismatch"
            }
        }
        Some(_) => return Err(invalid("recursive validation receipt forbidden")),
        None => "self-validation not supplied",
    }
    .into();
    recompute(&mut r)?;
    Ok(r)
}
pub fn execute_validation(
    c: &SuiteValidationConfig,
    sealed: &Path,
    store: &EvidenceStore,
    id: &str,
) -> io::Result<BlindTestSuiteValidationReceipt> {
    if c.schema_version != "1"
        || c.targets.is_empty()
        || c.targets.len() > 16
        || !identifier(id)
        || c.targets
            .iter()
            .map(|t| &t.label)
            .collect::<BTreeSet<_>>()
            .len()
            != c.targets.len()
        || c.targets.iter().any(|t| {
            !identifier(&t.label)
                || t.config.validation_receipt.is_some()
                || t.config.suite_hash != c.targets[0].config.suite_hash
        })
    {
        return Err(invalid("invalid suite self-validation config"));
    }
    let run = store.reserve(id)?;
    let mut r = BlindTestSuiteValidationReceipt {
        schema_version: "1".into(),
        blindtest_validation_id: id.into(),
        suite_hash: c.targets[0].config.suite_hash.clone(),
        checker: CHECKER.into(),
        runs: vec![],
        matched: false,
        limitation: QUALITY_LIMIT.into(),
    };
    for (index, t) in c.targets.iter().enumerate() {
        let child = execute(&t.config, sealed, store, &format!("{id}-v{index}"))?;
        r.runs.push(ValidationRun {
            label: t.label.clone(),
            run_id: child.blindtest_result_id.clone(),
            run_hash: canonical_hash(&child)?,
            expected: t.expected,
            observed: child.verdict,
        });
    }
    r.matched = r.runs.iter().all(|r| r.expected == r.observed);
    run.write_evidence(&evidence(
        id,
        "validation",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &r.runs,
    )?)?;
    run.complete(&r, &["validation".into()])?;
    load_validation(store, id)
}
pub fn load_validation(
    store: &EvidenceStore,
    id: &str,
) -> io::Result<BlindTestSuiteValidationReceipt> {
    let (v, items) = store.load(id)?;
    let mut r: BlindTestSuiteValidationReceipt = serde_json::from_value(v)?;
    if r.schema_version != "1"
        || r.blindtest_validation_id != id
        || r.checker != CHECKER
        || r.runs.is_empty()
        || r.runs.len() > 16
        || items.len() != 1
        || r.runs
            .iter()
            .map(|v| &v.run_id)
            .collect::<BTreeSet<_>>()
            .len()
            != r.runs.len()
    {
        return Err(invalid("invalid self-validation receipt"));
    }
    require(
        &items,
        id,
        "validation",
        "sealed_controller",
        TrustClass::DirectRuntime,
        &r.runs,
    )?;
    for v in &mut r.runs {
        let child = load_inner(store, &v.run_id, false)?;
        if canonical_hash(&child)? != v.run_hash || child.config.suite_hash != r.suite_hash {
            return Err(invalid("self-validation actual child evidence mismatch"));
        }
        v.observed = child.verdict;
    }
    r.matched = r.runs.iter().all(|r| r.expected == r.observed);
    r.limitation = QUALITY_LIMIT.into();
    Ok(r)
}
pub fn read_config<T: DeserializeOwned>(p: &Path) -> io::Result<T> {
    if fs::metadata(p)?.len() > 8 * 1024 * 1024 {
        return Err(invalid("config size limit"));
    }
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
pub fn schema<T: schemars::JsonSchema>(name: &str, version: &str) -> serde_json::Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    v["$id"] = format!("https://b2ige.dev/schemas/verify/{name}.v{version}.json").into();
    v["properties"]["schema_version"]["const"] = version.into();
    v
}
