//! Non-authoritative adoption adapters. Execution and verdicts stay in Prepared/load.
use crate::{
    agent::{Operation, Product, Response},
    integration::{self, Config, Entry, Prepared, Project},
    pretty,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, IsTerminal, Write},
    path::{Component, Path, PathBuf},
};
use verify_core::{
    behavior::{
        self, BehaviorCase, BehaviorExperiment, BehaviorTarget, ComparisonPolicy, LocalFixture,
    },
    blindtest, sideeffect, ApprovalStatus, Baseline, BaselineApproval, BaselineCreator,
};
use verify_evidence::canonical_hash;

#[derive(Debug)]
struct InputError(String);
impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for InputError {}
fn bad(message: &str) -> io::Error {
    io::Error::other(InputError(message.into()))
}

// Linux distributions use /bin -> usr/bin and executable aliases such as
// /usr/bin/python3 -> python3.10. Accept only direct aliases within the system
// binary directories, with root-owned, non-writable parents and destinations.
// A project link to a system executable is still not a system alias.
#[cfg(target_os = "linux")]
fn linux_system_alias(p: &Path, link: &fs::Metadata) -> io::Result<Option<PathBuf>> {
    use std::os::unix::fs::MetadataExt;
    let directory_target = match p.to_str() {
        Some("/bin") => Some(Path::new("/usr/bin")),
        Some("/sbin") => Some(Path::new("/usr/sbin")),
        _ => None,
    };
    let system_bin = |p: &Path| p == Path::new("/usr/bin") || p == Path::new("/usr/sbin");
    if link.uid() != 0 || (directory_target.is_none() && !p.parent().is_some_and(system_bin)) {
        return Ok(None);
    }
    let target = p
        .parent()
        .expect("system alias parent")
        .join(fs::read_link(p)?);
    if target.components().any(|p| p == Component::ParentDir)
        || match directory_target {
            Some(expected) => target != expected,
            None => !target.parent().is_some_and(system_bin),
        }
    {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&target)?;
    if !(if directory_target.is_some() {
        metadata.is_dir()
    } else {
        metadata.is_file() && metadata.mode() & 0o111 != 0
    }) {
        return Ok(None);
    }
    // Check the complete source parent and destination chains. Never follow a
    // second link, a writable system directory, or a link into a user's tree.
    for path in p.ancestors().skip(1).chain(target.ancestors()) {
        let m = fs::symlink_metadata(path)?;
        if m.uid() != 0 || m.mode() & 0o022 != 0 || m.file_type().is_symlink() {
            return Ok(None);
        }
    }
    Ok(Some(target))
}

/// Resolve paths once for persisted adoption inputs. Only documented OS aliases
/// are normalized; reject other symlinks, special files and parent traversal.
/// No filesystem writes.
pub fn path(input: &Path) -> io::Result<PathBuf> {
    let absolute = if input.is_absolute() {
        input.to_owned()
    } else {
        std::env::current_dir()?.join(input)
    };
    // The OS temporary-directory prefix can itself be a platform alias (/var on macOS).
    let temp = std::env::temp_dir();
    let absolute = if let Ok(tail) = absolute.strip_prefix(&temp) {
        fs::canonicalize(&temp)?.join(tail)
    } else {
        absolute
    };
    let mut resolved = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::ParentDir => {
                return Err(bad(
                    "parent traversal is not accepted; use a resolved absolute path",
                ))
            }
            Component::CurDir => continue,
            _ => resolved.push(part),
        }
        match fs::symlink_metadata(&resolved) {
            Ok(m) if m.file_type().is_symlink() => {
                #[cfg(target_os = "linux")]
                if let Some(target) = linux_system_alias(&resolved, &m)? {
                    resolved = target;
                    continue;
                }
                return Err(bad("symlink path is not accepted"));
            }
            Ok(m) if !m.is_file() && !m.is_dir() => {
                return Err(bad("path must be a regular file or directory"))
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    Ok(resolved)
}
fn existing(input: &Path) -> io::Result<PathBuf> {
    let p = path(input)?;
    fs::metadata(&p)?;
    Ok(p)
}
fn write_new(p: &Path, bytes: &[u8]) -> io::Result<()> {
    path(p)?;
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(p)?;
    f.write_all(bytes)?;
    f.sync_all()
}
fn write_json(p: &Path, value: &impl Serialize) -> io::Result<()> {
    write_new(p, &serde_json::to_vec_pretty(value)?)
}
fn id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn registry(p: &Path) -> io::Result<Project> {
    Ok(registry_snapshot(p)?.0)
}
fn registry_snapshot(p: &Path) -> io::Result<(Project, Vec<u8>)> {
    let p = existing(p)?;
    let bytes = integration::read_bytes(&p)?;
    let project: Project = serde_json::from_slice(&bytes)?;
    if project.schema_version != "1" {
        return Err(bad("invalid registry version"));
    }
    Ok((project, bytes))
}

pub fn inspect(root: &Path) -> io::Result<String> {
    let root = public_directory(root)?;
    let mut lines = vec!["Factual discovery only; verification_performed: false".to_owned()];
    let mut found = 0;
    for name in [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "Makefile",
        "CMakeLists.txt",
        "Dockerfile",
    ] {
        let status = match fs::symlink_metadata(root.join(name)) {
            Ok(m) if m.is_file() => {
                found += usize::from(name != "Dockerfile");
                "detected"
            }
            Ok(_) => "unsupported (not a regular public metadata file)",
            Err(e) if e.kind() == io::ErrorKind::NotFound => "absent",
            Err(_) => "unsupported (metadata unavailable)",
        };
        lines.push(format!("{name}: {status}"));
    }
    let p = root.join(".b2ige/project.json");
    let status = if fs::symlink_metadata(&p).is_ok() {
        if registry(&p).is_ok() {
            "configured (contents and product files not displayed)"
        } else {
            "unsupported (unsafe or malformed registry)"
        }
    } else {
        "absent"
    };
    lines.push(format!("registry: {status}"));
    lines.push(format!(
        "project stack: {}",
        match found {
            0 => "unsupported (no recognized build marker)",
            1 => "detected",
            _ => "ambiguous (multiple build markers)",
        }
    ));
    let docker_present = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .take(128)
        .any(|p| p.join("docker").is_file());
    lines.push(format!(
        "Docker executable: {}; engine availability needs operator decision (not executed)",
        if docker_present { "detected" } else { "absent" }
    ));
    lines.push(
        "product: needs operator decision — explicitly choose behavior, sideeffect, or blindtest"
            .into(),
    );
    lines.push("Next: b2ige init; b2ige prepare --product PRODUCT --help. No scripts/builds/images/hidden material were executed or opened. A local registry is not a secrecy boundary.".into());
    Ok(lines.join("\n"))
}

struct Args(BTreeMap<String, String>);
impl Args {
    fn parse(args: &[String], allowed: &[&str]) -> io::Result<Self> {
        let mut map = BTreeMap::new();
        for pair in args.chunks(2) {
            if pair.len() != 2
                || !allowed.contains(&pair[0].as_str())
                || pair[1].starts_with("--")
                || map.insert(pair[0].clone(), pair[1].clone()).is_some()
            {
                return Err(bad("unknown, duplicate, or missing option; see --help"));
            }
        }
        Ok(Self(map))
    }
    fn get(&self, key: &str) -> io::Result<&str> {
        self.0
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| bad(&format!("missing {key}; see --help")))
    }
    fn optional(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
    fn file(&self, key: &str) -> io::Result<PathBuf> {
        public_file(Path::new(self.get(key)?))
    }
    fn number(&self, key: &str) -> io::Result<u64> {
        self.get(key)?
            .parse()
            .map_err(|_| bad("invalid numeric budget"))
    }
    fn json<T: serde::de::DeserializeOwned>(&self, key: &str, default: &str) -> io::Result<T> {
        Ok(serde_json::from_str(self.optional(key).unwrap_or(default))?)
    }
}
fn product(s: &str) -> io::Result<Product> {
    match s {
        "behavior" => Ok(Product::Behavior),
        "sideeffect" => Ok(Product::Sideeffect),
        "blindtest" => Ok(Product::Blindtest),
        _ => Err(bad(
            "explicit --product behavior|sideeffect|blindtest required",
        )),
    }
}
fn product_name(p: Product) -> &'static str {
    match p {
        Product::Behavior => "behavior",
        Product::Sideeffect => "sideeffect",
        Product::Blindtest => "blindtest",
    }
}
fn public_file(p: &Path) -> io::Result<PathBuf> {
    let p = existing(p)?;
    if !p.is_file() {
        return Err(bad("public input must be a regular file"));
    }
    if let Some(sealed) = std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT") {
        if p.starts_with(path(Path::new(&sealed))?) {
            return Err(bad("public input overlaps sealed root"));
        }
    }
    Ok(p)
}
fn public_directory(p: &Path) -> io::Result<PathBuf> {
    let p = existing(p)?;
    if !p.is_dir() {
        return Err(bad("public fixture/workspace must be a directory"));
    }
    if let Some(sealed) = std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT") {
        let sealed = path(Path::new(&sealed))?;
        if !disjoint(&p, &sealed) {
            return Err(bad("public input overlaps sealed root"));
        }
    }
    Ok(p)
}
fn normalize_fixture(f: &mut LocalFixture) -> io::Result<()> {
    f.source = f.source.as_deref().map(public_directory).transpose()?;
    f.snapshot_identity = behavior::snapshot_identity(f.source.as_deref())?;
    Ok(())
}

fn behavior_draft(a: &Args) -> io::Result<BehaviorExperiment> {
    let mut c: BehaviorExperiment = if let Some(p) = a.optional("--config") {
        integration::read(&public_file(Path::new(p))?)?
    } else {
        let fixture = LocalFixture {
            source: a.optional("--fixture").map(PathBuf::from),
            snapshot_identity: String::new(),
        };
        let case = BehaviorCase {
            schema_version: "1".into(),
            case_id: a.get("--case-id")?.into(),
            args: a.json("--args-json", "[]")?,
            environment: a.json("--env-json", "{}")?,
            timeout_ms: a.number("--timeout-ms")?,
            fixture,
            comparison_policy: ComparisonPolicy::ProcessByteExactV1,
            required_observers: vec!["cli_process".into()],
        };
        BehaviorExperiment {
            schema_version: "1".into(),
            before: BehaviorTarget {
                executable: a.file("--reference")?,
                identity: String::new(),
                input_identity: String::new(),
            },
            after: BehaviorTarget {
                executable: a.file("--candidate")?,
                identity: String::new(),
                input_identity: String::new(),
            },
            baseline: Baseline {
                baseline_id: a.get("--case-id")?.into(),
                target_revision: String::new(),
                created_by: BaselineCreator::Imported,
                approval: BaselineApproval {
                    status: ApprovalStatus::Unapproved,
                    actor: None,
                    reason: None,
                },
                observation_contract_hash: String::new(),
                stability_runs: None,
                notes: None,
            },
            case,
            seed: a.number("--seed")?,
        }
    };
    normalize_fixture(&mut c.case.fixture)?;
    let input = c.case.input_identity()?;
    for t in [&mut c.before, &mut c.after] {
        t.executable = public_file(&t.executable)?;
        t.identity = behavior::executable_identity(&t.executable)?;
        t.input_identity = input.clone();
    }
    if let Some(b) = a.optional("--baseline") {
        c.baseline = integration::read(&public_file(Path::new(b))?)?;
    }
    // Only an unapproved draft may receive freshly computed reference pins.
    if c.baseline.approval.status == ApprovalStatus::Unapproved {
        c.baseline.target_revision = c.before.identity.clone();
        c.baseline.observation_contract_hash = c.case.observation_contract_hash()?;
    }
    if c.schema_version != "1"
        || c.case.schema_version != "1"
        || c.case.case_id.trim().is_empty()
        || c.case.timeout_ms == 0
        || c.case.timeout_ms > verify_evidence::MAX_SAFE_INTEGER
        || c.seed > verify_evidence::MAX_SAFE_INTEGER
        || c.case.required_observers != ["cli_process"]
        || c.baseline.target_revision != c.before.identity
        || c.baseline.observation_contract_hash != c.case.observation_contract_hash()?
    {
        return Err(bad(
            "Behavior case/reference/baseline fields mismatch or unsupported",
        ));
    }
    Ok(c)
}

fn sideeffect_draft(a: &Args) -> io::Result<sideeffect::SideEffectContract> {
    use sideeffect::*;
    // A disposable fixture is always an explicit operator input, including --config mode.
    let fixture = public_directory(Path::new(a.get("--fixture")?))?;
    let mut c: SideEffectContract = if let Some(p) = a.optional("--config") {
        integration::read(&public_file(Path::new(p))?)?
    } else {
        let op = OperationIdentity {
            operation_id: a.get("--operation-id")?.into(),
            idempotency_identity: a.get("--idempotency")?.into(),
            correlation_identity: a.get("--correlation")?.into(),
        };
        let expectation = match a.get("--expectation")? {
            "exactly-once" => Expectation::ExactlyOnce,
            "at-most-once" => Expectation::AtMostOnce,
            _ => return Err(bad("expectation must be exactly-once or at-most-once")),
        };
        SideEffectContract {
            schema_version: "1".into(),
            contract_id: a.get("--contract-id")?.into(),
            operation: op.clone(),
            trigger: Trigger {
                executable: a.file("--trigger")?,
                executable_hash: String::new(),
                args: a.json("--args-json", "[]")?,
                environment: a.json("--env-json", "{}")?,
                fixture: LocalFixture {
                    source: Some(fixture.clone()),
                    snapshot_identity: String::new(),
                },
                timeout_ms: a.number("--timeout-ms")?,
            },
            effects: vec![EffectContract {
                effect_id: "effect".into(),
                provider: a.get("--provider")?.into(),
                adapter: Adapter::Sqlite,
                operation: a.get("--operation")?.into(),
                identity: IdentityContract {
                    external_identity: ExternalIdentity::ProviderOperationExternalId,
                    idempotency_identity: op.idempotency_identity,
                    correlation_identity: op.correlation_identity,
                },
                expectation,
                authoritative_observer: "ledger".into(),
            }],
            relations: vec![],
            fault_schedules: a.json("--schedules-json", "[]")?,
            exploration_budget: a.json("--budget-json", "{}")?,
            required_observers: vec![SQLiteEffectObserver {
                observer_id: "ledger".into(),
                db_path: a.get("--db")?.into(),
                table: a.get("--table")?.into(),
                external_effect_id_column: a.get("--external-id-column")?.into(),
                idempotency_column: a.get("--idempotency-column")?.into(),
                correlation_column: a.get("--correlation-column")?.into(),
                operation_column: a.get("--operation-column")?.into(),
                commit_order_column: a.get("--commit-order-column")?.into(),
                authoritative_source: LedgerAuthority::DurableAppendOnlyCommittedState,
            }],
        }
    };
    c.trigger.fixture.source = Some(fixture);
    normalize_fixture(&mut c.trigger.fixture)?;
    c.trigger.executable = public_file(&c.trigger.executable)?;
    c.trigger.executable_hash = behavior::executable_identity(&c.trigger.executable)?;
    c.validate()?;
    // Readiness observes only the supplied disposable snapshot; never a production DB.
    sideeffect::check_prerequisites(&c)?;
    Ok(c)
}
fn blindtest_draft(a: &Args) -> io::Result<blindtest::BlindTestConfig> {
    // Existing PUBLIC typed references only. Never open or manufacture a sealed suite.
    let workspace = public_directory(Path::new(a.get("--workspace")?))?;
    let mut c: blindtest::BlindTestConfig = if let Some(config) = a.optional("--config") {
        integration::read(&public_file(Path::new(config))?)?
    } else {
        blindtest::BlindTestConfig {
            schema_version: "1".into(),
            suite_hash: a.get("--suite-hash")?.into(),
            approved_invariants: a.json("--approved-invariants-json", "[]")?,
            target: blindtest::TargetSpec {
                image: a.get("--image")?.into(),
                command: a.get("--command")?.into(),
                args: a.json("--args-json", "[]")?,
                environment: a.json("--env-json", "{}")?,
                allowed_case_env: a.json("--allowed-case-env-json", "[]")?,
                max_case_args: a
                    .number("--max-case-args")?
                    .try_into()
                    .map_err(|_| bad("invalid max-case-args"))?,
                workspace: workspace.clone(),
                workspace_hash: String::new(),
                build_identity: a.get("--build-identity")?.into(),
                bounds: a.json("--bounds-json", "{}")?,
            },
            required_isolation: blindtest::IsolationLevel::DockerIsolation,
            max_cases: a
                .number("--max-cases")?
                .try_into()
                .map_err(|_| bad("invalid max-cases"))?,
            validation_receipt: None,
        }
    };
    c.target.workspace = workspace;
    c.target.workspace_hash = behavior::snapshot_identity(Some(&c.target.workspace))?;
    c.validate()?;
    Ok(c)
}

fn review(c: &Config) -> io::Result<String> {
    let (details, requirements) = match c {
        Config::Behavior(c, _) => (json!({"schema_version":c.schema_version,"reference_baseline":{"executable":c.before.executable,"actual_identity":behavior::executable_identity(&c.before.executable)?,"baseline_identity":canonical_hash(&c.baseline)?,"approval_status":c.baseline.approval.status},"candidate":{"executable":c.after.executable,"actual_identity":behavior::executable_identity(&c.after.executable)?},"fixture_identity":behavior::snapshot_identity(c.case.fixture.source.as_deref())?,"input_identity":c.case.input_identity()?,"checker_binding":verify_core::checker_binding_hash(&c.baseline,&c.case.checker_claim()?)?,"required_observers":c.case.required_observers,"timeout_ms":c.case.timeout_ms,"seed":c.seed,"execution_budget":2}), "Required: complete cli_process/direct_runtime evidence, byte-exact exit/signal/stdout/stderr comparison. BLOCKED until independently approved reference baseline, checker binding and explicit baseline_stable authorization. Stability is never inferred. Candidate is not a baseline approval."),
        Config::Sideeffect(c) => (json!({"schema_version":c.schema_version,"reference_baseline":"not applicable","candidate":{"executable":c.trigger.executable,"actual_identity":behavior::executable_identity(&c.trigger.executable)?},"fixture_identity":behavior::snapshot_identity(c.trigger.fixture.source.as_deref())?,"required_observers":c.required_observers,"effects":c.effects,"relations":c.relations,"fault_schedules":c.fault_schedules,"budget":c.exploration_budget,"timeout_ms":c.trigger.timeout_ms}), "BLOCKED until operator confirms the disposable fixture and durable append-only committed SQLite state are authoritative. All required observers/schedules need complete authoritative_target_state evidence. Attempts/stdout/request logs are NOT committed effects."),
        Config::Blindtest(c, _) => (json!({"schema_version":c.schema_version,"reference_baseline":"not applicable; approved private artifacts remain controller-owned","candidate_image":c.target.image,"approved_suite_identity":c.suite_hash,"approved_binding_count":c.approved_invariants.len(),"workspace_identity":behavior::snapshot_identity(Some(&c.target.workspace))?,"build_identity":c.target.build_identity,"required_isolation":c.required_isolation,"bounds":c.target.bounds,"max_cases":c.max_cases}), "BLOCKED until trusted suite/invariant/checker approval, sealed-root ownership/separation and immutable local image are confirmed. Requires complete hidden coverage and actual before/after Docker isolation plus direct_runtime captures; runtime loader checks all private evidence. No hidden inventory is opened or displayed here."),
    };
    Ok(format!("NON-AUTHORITATIVE review; verification_performed: false\n{}\n{requirements}\nIdentities are rechecked at runtime. Hashes are integrity identities, not publisher authentication. Same-user host processes can access controller files; project-local registry is not a secrecy boundary.\n", pretty(&details)))
}

fn prepare(a: &Args) -> io::Result<()> {
    let p = product(a.get("--product")?)?;
    let allowed: &[&str] = match (p, a.optional("--config").is_some()) {
        (Product::Behavior, true) => &["--product", "--out", "--config", "--baseline"],
        (Product::Sideeffect, true) => &["--product", "--out", "--config", "--fixture"],
        (Product::Blindtest, true) => &["--product", "--out", "--config", "--workspace"],
        (Product::Behavior, false) => &[
            "--product",
            "--out",
            "--reference",
            "--candidate",
            "--case-id",
            "--timeout-ms",
            "--seed",
            "--fixture",
            "--args-json",
            "--env-json",
            "--baseline",
        ],
        (Product::Sideeffect, false) => &[
            "--product",
            "--out",
            "--trigger",
            "--fixture",
            "--contract-id",
            "--operation-id",
            "--idempotency",
            "--correlation",
            "--provider",
            "--operation",
            "--expectation",
            "--db",
            "--table",
            "--external-id-column",
            "--idempotency-column",
            "--correlation-column",
            "--operation-column",
            "--commit-order-column",
            "--schedules-json",
            "--budget-json",
            "--timeout-ms",
            "--args-json",
            "--env-json",
        ],
        (Product::Blindtest, false) => &[
            "--product",
            "--out",
            "--workspace",
            "--suite-hash",
            "--approved-invariants-json",
            "--image",
            "--command",
            "--build-identity",
            "--bounds-json",
            "--max-cases",
            "--max-case-args",
            "--allowed-case-env-json",
            "--args-json",
            "--env-json",
        ],
    };
    if a.0.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(bad(
            "conflicting config/input options or option belongs to another product",
        ));
    }
    let out = path(Path::new(a.get("--out")?))?;
    if let Some(sealed) = std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT") {
        if !disjoint(&out, &path(Path::new(&sealed))?) {
            return Err(bad("draft output overlaps sealed root"));
        }
    }
    if out.exists() {
        return Err(bad("draft output already exists; never overwritten"));
    }
    let c = match p {
        Product::Behavior => {
            Config::Behavior(Box::new(behavior_draft(a)?), integration::empty_auth())
        }
        Product::Sideeffect => Config::Sideeffect(Box::new(sideeffect_draft(a)?)),
        Product::Blindtest => Config::Blindtest(Box::new(blindtest_draft(a)?), PathBuf::new()),
    };
    let snapshot = match &c {
        Config::Behavior(c, _) => c.case.fixture.source.as_deref(),
        Config::Sideeffect(c) => c.trigger.fixture.source.as_deref(),
        Config::Blindtest(c, _) => Some(c.target.workspace.as_path()),
    };
    if snapshot.is_some_and(|p| !disjoint(p, &out)) {
        return Err(bad("draft directory must not overlap fixture/workspace"));
    }
    let summary = review(&c)?;
    let value = config_value(&c)?;
    // Exclusive output creation. No registry or authorization is written.
    fs::create_dir(&out)?;
    write_json(&out.join(format!("{}.json", product_name(p))), &value)?;
    write_new(&out.join("REVIEW.md"), summary.as_bytes())?;
    println!("{summary}\nDraft saved. Next: trusted operator runs b2ige trust approve DRAFT --product {} --identity ID --registry CONTROLLER_REGISTRY --controller CONTROLLER_DIR --project-root PROJECT --store STORE [--authorization EXISTING_AUTH].",product_name(p));
    Ok(())
}
fn config_value(c: &Config) -> io::Result<Value> {
    Ok(match c {
        Config::Behavior(c, _) => serde_json::to_value(c)?,
        Config::Sideeffect(c) => serde_json::to_value(c)?,
        Config::Blindtest(c, _) => serde_json::to_value(c)?,
    })
}

struct DraftInputs {
    prepared: Prepared,
    config_bytes: Vec<u8>,
    authorization_bytes: Option<Vec<u8>>,
}
fn open_draft(
    draft: &Path,
    p: Product,
    auth: Option<&Path>,
    store: &Path,
) -> io::Result<DraftInputs> {
    let file = existing(&draft.join(format!("{}.json", product_name(p))))?;
    // Exactly one typed product file; no product inference or ambiguous draft directory.
    for other in [Product::Behavior, Product::Sideeffect, Product::Blindtest] {
        if other != p && draft.join(format!("{}.json", product_name(other))).exists() {
            return Err(bad("ambiguous product draft"));
        }
    }
    let config_bytes = integration::read_bytes(&file)?;
    let authorization_bytes = auth
        .map(|p| integration::read_bytes(&existing(p)?))
        .transpose()?;
    let config = match p {
        Product::Behavior => Config::Behavior(
            Box::new(serde_json::from_slice(&config_bytes)?),
            serde_json::from_slice(
                authorization_bytes
                    .as_deref()
                    .ok_or_else(|| bad("independent Behavior authorization required"))?,
            )?,
        ),
        Product::Sideeffect => Config::Sideeffect(Box::new(serde_json::from_slice(&config_bytes)?)),
        Product::Blindtest => Config::Blindtest(
            Box::new(serde_json::from_slice(&config_bytes)?),
            std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT")
                .map(PathBuf::from)
                .ok_or_else(|| bad("controller sealed root required"))?,
        ),
    };
    Ok(DraftInputs {
        prepared: Prepared {
            config,
            store: store.to_owned(),
        },
        config_bytes,
        authorization_bytes,
    })
}
fn disjoint(a: &Path, b: &Path) -> bool {
    !a.starts_with(b) && !b.starts_with(a)
}
fn check_public_paths(config: &Config) -> io::Result<()> {
    let fixture = match config {
        Config::Behavior(c, _) => {
            public_file(&c.before.executable)?;
            public_file(&c.after.executable)?;
            c.case.fixture.source.as_deref()
        }
        Config::Sideeffect(c) => {
            public_file(&c.trigger.executable)?;
            c.trigger.fixture.source.as_deref()
        }
        Config::Blindtest(c, _) => Some(c.target.workspace.as_path()),
    };
    if let Some(p) = fixture {
        if !p.is_absolute() {
            return Err(bad("approved fixture/workspace must be absolute"));
        }
        public_directory(p)?;
    }
    Ok(())
}
fn check_trust(prepared: &Prepared) -> io::Result<()> {
    check_public_paths(&prepared.config)?;
    match &prepared.config {
        Config::Behavior(c, a) => {
            behavior::validate_configuration(c, a)?;
            // The authorization stays independently supplied; do not add any candidate pins.
        }
        Config::Sideeffect(c) => sideeffect::check_prerequisites(c)?,
        Config::Blindtest(c, sealed) => {
            c.validate()?;
            if !verify_evidence::valid_hash(&c.target.image) {
                return Err(bad("immutable sha256 image required"));
            }
            blindtest::check_paths(&c.target.workspace, sealed, &prepared.store)?;
            if behavior::snapshot_identity(Some(&c.target.workspace))? != c.target.workspace_hash {
                return Err(bad("workspace identity mismatch"));
            }
            // Validate existing approved artifacts without printing or copying private content.
            blindtest::validate_suite(&blindtest::read_suite(sealed)?, c).map_err(|_| {
                bad("approved private suite/invariant/checker inputs unavailable or mismatched")
            })?;
            let docker = blindtest::docker::doctor();
            if !docker.runtime_capabilities_available {
                return Err(bad("Docker isolation prerequisites unavailable"));
            }
            blindtest::docker::Docker::discover()?.resolve_image(&c.target.image)?;
        }
    }
    Ok(())
}
fn approve(draft: &Path, a: &Args) -> io::Result<()> {
    // No bypass flag, environment switch, MCP or Agent Protocol entry point.
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(bad("approval requires a trusted operator's interactive terminal; non-interactive approval refused"));
    }
    let p = product(a.get("--product")?)?;
    let id = a.get("--identity")?;
    if !id_valid(id) {
        return Err(bad("invalid identity name"));
    }
    let draft = existing(draft)?;
    let controller = existing(Path::new(a.get("--controller")?))?;
    let project_root = existing(Path::new(a.get("--project-root")?))?;
    let registry_path = path(Path::new(a.get("--registry")?))?;
    let store = path(Path::new(a.get("--store")?))?;
    if !controller.is_dir()
        || !project_root.is_dir()
        || !disjoint(&controller, &project_root)
        || !disjoint(&controller, &draft)
        || !registry_path.starts_with(&controller)
        || !store.starts_with(&controller)
    {
        return Err(bad("registry/store/controller must be outside project and draft, in an explicit existing controller directory"));
    }
    let auth = a
        .optional("--authorization")
        .map(|s| existing(Path::new(s)))
        .transpose()?;
    if p != Product::Behavior && auth.is_some() {
        return Err(bad("authorization is Behavior-only"));
    }
    if auth.as_ref().is_some_and(|p| !p.starts_with(&controller)) {
        return Err(bad(
            "Behavior authorization must be an independently supplied controller file",
        ));
    }
    let (mut project, registry_bytes) = if registry_path.exists() {
        let (project, bytes) = registry_snapshot(&registry_path)?;
        (project, Some(bytes))
    } else {
        (
            Project {
                schema_version: "1".into(),
                entries: BTreeMap::new(),
            },
            None,
        )
    };
    if project.entries.contains_key(id) {
        return Err(bad(
            "identity already registered; existing approval is never replaced",
        ));
    }
    let inputs = open_draft(&draft, p, auth.as_deref(), &store)?;
    let prepared = &inputs.prepared;
    if let Config::Blindtest(c, sealed) = &prepared.config {
        blindtest::check_paths(&c.target.workspace, sealed, &store)
            .map_err(|_| bad("unsafe private path separation"))?;
        blindtest::check_paths(&project_root, sealed, &store)
            .map_err(|_| bad("unsafe private path separation"))?;
    }
    check_public_paths(&prepared.config)?;
    let source_value = config_value(&prepared.config)?;
    let auth_bytes = &inputs.authorization_bytes;
    let dest = controller.join(id);
    if dest.exists() {
        return Err(bad("controller identity directory already exists"));
    }
    let workspace = match &prepared.config {
        Config::Behavior(c, _) => c.case.fixture.source.as_deref(),
        Config::Sideeffect(c) => c.trigger.fixture.source.as_deref(),
        Config::Blindtest(c, _) => Some(c.target.workspace.as_path()),
    };
    if workspace.is_some_and(|p| existing(p).is_ok_and(|p| !disjoint(&controller, &p))) {
        return Err(bad("controller overlaps candidate fixture/workspace"));
    }
    if !disjoint(&store, &dest)
        || !disjoint(&store, &registry_path)
        || !disjoint(&registry_path, &dest)
        || auth.as_ref().is_some_and(|p| !disjoint(&store, p))
    {
        return Err(bad(
            "store, registry and approved input destinations must not overlap",
        ));
    }
    println!(
        "Identity: {id}\nProduct: {}\nConfig identity: {}\n{}",
        product_name(p),
        canonical_hash(&source_value)?,
        review(&prepared.config)?
    );
    if p == Product::Blindtest {
        println!("Config/registry/store: private controller destinations outside the project (paths withheld)");
    } else {
        println!(
            "Controller config: {}\nRegistry: {}\nStore: {}\n",
            serde_json::to_string(&dest)?,
            serde_json::to_string(&registry_path)?,
            serde_json::to_string(&store)?
        );
    }
    if let Config::Behavior(_, authorization) = &prepared.config {
        println!(
            "Authorization identity: {}\nExplicit baseline_stable: {}",
            canonical_hash(authorization)?,
            authorization.baseline_stable
        );
    }
    check_trust(prepared).map_err(|_| bad("trust blockers remain: review reference/stability/checker approval, observer completeness, or sealed suite/image/isolation inputs"))?;
    let statement = match p {
        Product::Behavior => "I independently reviewed the reference baseline, explicit stability assertion, checker binding and candidate role; the coding agent did not supply this trust decision.",
        Product::Sideeffect => "I independently confirm this is a disposable fixture of authoritative durable append-only committed SQLite state, not attempts/stdout/request logs; the coding agent did not supply this trust decision.",
        Product::Blindtest => "I independently own the sealed root and approve the existing suite/invariant/checker bindings, public image/build and complete coverage budget; the coding agent did not supply this trust decision.",
    };
    println!("{statement}\nType APPROVE {id} to register these exact inputs, or anything else to cancel:");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim_end_matches(['\r', '\n']) != format!("APPROVE {id}") {
        return Err(bad("approval cancelled; no authoritative writes"));
    }
    // Re-read after operator review, not the REVIEW.md or a self-reported draft status.
    let reread = open_draft(&draft, p, auth.as_deref(), &store)?;
    if reread.config_bytes != inputs.config_bytes
        || reread.authorization_bytes != inputs.authorization_bytes
        || (if registry_path.exists() {
            Some(integration::read_bytes(&existing(&registry_path)?)?)
        } else {
            None
        }) != registry_bytes
    {
        return Err(bad("reviewed inputs changed; approval refused"));
    }
    check_trust(&reread.prepared).map_err(|_| bad("reviewed trust prerequisites changed"))?;
    path(&controller)?;
    path(&registry_path)?;
    path(&store)?;
    // A create-new lock rejects concurrent adoption writers. Registry is published last.
    // Lock the registry itself: nested controller choices can share one registry.
    let mut lock_name = registry_path
        .file_name()
        .ok_or_else(|| bad("registry filename required"))?
        .to_os_string();
    lock_name.push(".adoption-approval.lock");
    let lock = registry_path.with_file_name(lock_name);
    write_new(&lock, b"exclusive adoption registration")?;
    let result = (|| {
        if (if registry_path.exists() {
            Some(integration::read_bytes(&existing(&registry_path)?)?)
        } else {
            None
        }) != registry_bytes
        {
            return Err(bad("registry changed during review"));
        }
        fs::create_dir(&dest)?;
        let config = dest.join("config.json");
        write_json(&config, &source_value)?;
        let authorization = if let Some(bytes) = auth_bytes {
            let p = dest.join("authorization.json");
            write_new(&p, bytes)?;
            Some(p)
        } else {
            None
        };
        project.entries.insert(
            id.into(),
            Entry {
                product: p,
                config,
                store,
                authorization,
            },
        );
        let staged = controller.join(format!(".registry-{}.json", integration::nonce()));
        write_json(&staged, &project)?;
        // Existing registries retain all other entries; publish one complete v1 file.
        if registry_bytes.is_some() {
            fs::rename(&staged, &registry_path)?;
        } else {
            fs::hard_link(&staged, &registry_path)?;
            fs::remove_file(&staged)?;
        }
        Ok(())
    })();
    let _ = fs::remove_file(lock);
    result?;
    println!("Registered reviewed identity. Next: b2ige doctor --registry REGISTRY; b2ige verify {id} --registry REGISTRY");
    Ok(())
}

fn verify(id: &str, a: &Args) -> io::Result<u8> {
    let output = a.optional("--output").unwrap_or("human");
    if !["human", "json", "agent"].contains(&output)
        || a.optional("--protocol")
            .is_some_and(|p| p != "1" || output != "agent")
    {
        return Err(bad("unsupported output/protocol"));
    }
    let result: io::Result<u8> = (|| {
        let registry = registry(Path::new(
            a.optional("--registry").unwrap_or(".b2ige/project.json"),
        ))?;
        let entry = registry
            .entries
            .get(id)
            .ok_or_else(|| bad("unknown registered identity"))?;
        // Legacy registry v1 relative paths retain CWD semantics. New approvals persist absolute paths.
        existing(&entry.config)?;
        let draft_name = ["behavior.json", "sideeffect.json", "blindtest.json"]
            .iter()
            .any(|name| entry.config.file_name().is_some_and(|n| n == *name));
        if draft_name
            && entry
                .config
                .parent()
                .is_some_and(|p| p.join("REVIEW.md").exists())
        {
            return Err(bad(
                "draft paths cannot be used by registered verification; request trusted approval",
            ));
        }
        if let Some(p) = &entry.authorization {
            existing(p)?;
        }
        path(&entry.store)?;
        let (_, report) = Prepared::open(entry)?.execute()?;
        match output {
            "agent" if a.optional("--protocol").is_some() => println!(
                "{}",
                pretty(&Response::from_verified(&report, Operation::Verify))
            ),
            "agent" => println!("{}", pretty(&report.agent())),
            "json" => println!("{}", pretty(report.document())),
            _ => print!("{}", report.human()),
        }
        Ok(report.document().verdict.exit_code())
    })();
    match result {
        Ok(code) => Ok(code),
        Err(_) => {
            // Product unknown cannot be fabricated as a Behavior protocol response.
            if output == "human" {
                eprintln!("ERROR: registered identity/config/authorization or required evidence unavailable. Review the trusted registry and run doctor.");
            } else {
                let entry_product = registry(Path::new(
                    a.optional("--registry").unwrap_or(".b2ige/project.json"),
                ))
                .ok()
                .and_then(|p| p.entries.get(id).map(|e| e.product));
                if let Some(p) = entry_product {
                    println!("{}", pretty(&Response::error(p, Operation::Verify)));
                } else {
                    eprintln!("ERROR: registered identity unavailable; no product can be routed");
                }
            }
            Ok(3)
        }
    }
}

pub const HELP: &str = "V110-A adoption (no approval or verification during preparation):
  b2ige inspect [ROOT]
  b2ige init [ROOT] [--dry-run]
  b2ige prepare --product behavior|sideeffect|blindtest --out NEW_DIRECTORY [inputs]
  b2ige trust approve DRAFT_DIRECTORY --product PRODUCT --identity ID --registry FILE --controller EXISTING_DIRECTORY --project-root PROJECT --store PATH [--authorization TRUSTED_FILE]
  b2ige doctor [--registry FILE|--config FILE] [--output human|json]
  b2ige verify ID [--registry FILE] [--output human|json|agent] [--protocol 1]
Behavior: --reference FILE --candidate FILE --case-id ID --timeout-ms N --seed N
  [--fixture DIRECTORY] [--args-json ARRAY] [--env-json OBJECT] [--baseline APPROVED_BASELINE]
  or --config EXISTING_BEHAVIOR_JSON [--baseline APPROVED_BASELINE]. No authorization is generated.
SideEffect: --fixture DISPOSABLE_DIRECTORY --config EXISTING_CONTRACT_JSON
  or --fixture DIR --trigger FILE --contract-id ID --operation-id ID --idempotency ID
  --correlation ID --provider NAME --operation NAME --expectation exactly-once|at-most-once
  --db RELATIVE_FILE --table NAME --external-id-column NAME --idempotency-column NAME
  --correlation-column NAME --operation-column NAME --commit-order-column NAME
  --timeout-ms N --schedules-json ARRAY --budget-json OBJECT [--args-json ARRAY] [--env-json OBJECT]
BlindTest: --config APPROVED_PUBLIC_CONFIG --workspace PUBLIC_DIRECTORY.
  or --workspace DIR --image IMMUTABLE_SHA256 --command ABSOLUTE_CONTAINER_COMMAND
  --build-identity ID --suite-hash APPROVED_HASH --approved-invariants-json APPROVED_REFS
  --bounds-json OBJECT --max-cases N --max-case-args N [--allowed-case-env-json ARRAY]
  [--args-json ARRAY] [--env-json OBJECT]. Only public references; no hidden generation/display.
Paths in preparation inputs resolve against the invocation directory and are persisted absolute.
Preparation cannot approve stability/checkers/committed-state authority/hidden suites.
Approval is interactive, independently operated, and outside the candidate workspace.
Same-user host access is NOT isolated. Existing expert commands remain available.";

/// None means this is an existing expert command.
pub fn cli(args: &[String]) -> Option<u8> {
    let command = args.first()?.as_str();
    if !["inspect", "prepare", "trust", "verify"].contains(&command) {
        return None;
    }
    if args.iter().any(|s| s == "--help") {
        println!("{HELP}");
        return Some(0);
    }
    let result = match command {
        "inspect" if args.len() <= 2 => {
            inspect(Path::new(args.get(1).map(String::as_str).unwrap_or("."))).map(|s| {
                println!("{s}");
                0
            })
        }
        "prepare" => Args::parse(
            &args[1..],
            &[
                "--product",
                "--out",
                "--config",
                "--reference",
                "--candidate",
                "--case-id",
                "--timeout-ms",
                "--seed",
                "--fixture",
                "--args-json",
                "--env-json",
                "--baseline",
                "--trigger",
                "--contract-id",
                "--operation-id",
                "--idempotency",
                "--correlation",
                "--provider",
                "--operation",
                "--expectation",
                "--db",
                "--table",
                "--external-id-column",
                "--idempotency-column",
                "--correlation-column",
                "--operation-column",
                "--commit-order-column",
                "--schedules-json",
                "--budget-json",
                "--workspace",
                "--suite-hash",
                "--approved-invariants-json",
                "--image",
                "--command",
                "--build-identity",
                "--bounds-json",
                "--max-cases",
                "--max-case-args",
                "--allowed-case-env-json",
            ],
        )
        .and_then(|a| prepare(&a).map(|()| 0)),
        "trust" if args.len() >= 3 && args[1] == "approve" => Args::parse(
            &args[3..],
            &[
                "--product",
                "--identity",
                "--registry",
                "--controller",
                "--project-root",
                "--store",
                "--authorization",
            ],
        )
        .and_then(|a| approve(Path::new(&args[2]), &a).map(|()| 0)),
        "verify" if args.len() >= 2 && !args[1].starts_with('-') => {
            Args::parse(&args[2..], &["--registry", "--output", "--protocol"])
                .and_then(|a| verify(&args[1], &a))
        }
        _ => Err(bad("invalid adoption command; see --help")),
    };
    Some(result.unwrap_or_else(|e| { // Never echo parser excerpts, hidden paths, environment values, or Docker diagnostics.
        let safe = if command == "trust" && (!io::stdin().is_terminal() || !io::stdout().is_terminal()) { "approval requires a trusted operator's interactive terminal; non-interactive approval refused".to_owned() } else if let Some(message) = e.get_ref().and_then(|x| x.downcast_ref::<InputError>()) {
            message.to_string()
        } else if e.get_ref().is_some_and(|x| x.is::<serde_json::Error>()) { "invalid typed input; consult the existing product schema".into() } else if command == "prepare" || command == "trust" { "preparation/approval blocked: check explicit public inputs, approved reference/checker/stability, disposable fixture, controller paths, or suite/image/isolation prerequisites; see --help".into() } else { "adoption input unavailable or unsafe; see --help".into() };
        eprintln!("ERROR: {safe}"); 3
    }))
}

/// Tooling-only diagnostics, not a schema-bearing product/protocol artifact.
/// Fixed field names/messages prevent private parser, suite and Docker errors leaking.
pub fn readiness_fields(p: &Prepared) -> BTreeMap<&'static str, Value> {
    let mut fields = BTreeMap::new();
    let mut add = |name, ok, next| {
        fields.insert(name,json!({"ready":ok,"next_action":if ok { "Run verification; readiness is not a verdict" } else { next }}));
    };
    match &p.config {
        Config::Behavior(c, a) => {
            add("reference_identity",behavior::executable_identity(&c.before.executable).is_ok_and(|h| h == c.before.identity),"Restore the explicitly reviewed reference executable; never substitute candidate output");
            add(
                "candidate_identity",
                behavior::executable_identity(&c.after.executable)
                    .is_ok_and(|h| h == c.after.identity),
                "Prepare the current candidate and request independent trusted review",
            );
            add(
                "fixture_identity",
                behavior::snapshot_identity(c.case.fixture.source.as_deref())
                    .is_ok_and(|h| h == c.case.fixture.snapshot_identity),
                "Restore the reviewed disposable fixture or prepare and review a new identity",
            );
            add(
                "baseline_approval",
                c.baseline.approval.status == ApprovalStatus::Approved
                    && canonical_hash(&c.baseline).is_ok_and(|h| a.approved_baselines.contains(&h)),
                "Supply independently approved reference baseline and its existing authorization",
            );
            add(
                "baseline_stability",
                a.baseline_stable,
                "Trusted operator must supply explicit baseline stability; it is not inferred",
            );
            add(
                "checker_binding",
                c.case
                    .checker_claim()
                    .and_then(|claim| Ok(verify_core::checker_binding_hash(&c.baseline, &claim)?))
                    .is_ok_and(|h| a.approved_checker_bindings.contains(&h)),
                "Supply independently approved checker binding in existing authorization",
            );
            add(
                "configuration",
                behavior::validate_configuration(c, a).is_ok(),
                "Review schema, input roles, observers, byte-exact policy and bounded timeout",
            );
        }
        Config::Sideeffect(c) => {
            add(
                "configuration",
                c.validate().is_ok(),
                "Review effect identity, relation, observer fields and bounded schedules",
            );
            add("committed_sqlite_observers",sideeffect::check_prerequisites(c).is_ok(),"Supply an empty disposable SQLite fixture with every configured committed-state table/column; attempts and stdout cannot substitute");
        }
        Config::Blindtest(c, sealed) => {
            add("public_configuration",c.validate().is_ok(),"Supply the existing public config with approved suite/invariant references, image and Docker isolation");
            add(
                "workspace_identity",
                blindtest::check_paths(&c.target.workspace, sealed, &p.store).is_ok()
                    && behavior::snapshot_identity(Some(&c.target.workspace))
                        .is_ok_and(|h| h == c.target.workspace_hash),
                "Prepare and independently review the public workspace identity",
            );
            add("private_path_separation",blindtest::check_paths(&c.target.workspace,sealed,&p.store).is_ok(),"Trusted controller must supply a separated sealed root and private store; never put private material in the project");
            // Fixed boolean only: no private filenames, values, inventory or errors are projected.
            add("approved_private_inputs",blindtest::read_suite(sealed).and_then(|s|blindtest::validate_suite(&s,c)).is_ok(),"Trusted controller must provision existing approved suite/invariant/checker inputs outside the project");
            let d = blindtest::docker::doctor();
            add(
                "docker_isolation",
                d.docker_available && d.runtime_capabilities_available,
                "Start a supported local Docker engine; no per-run isolation is attested by doctor",
            );
            add("local_image",blindtest::docker::Docker::discover().and_then(|d|d.resolve_image(&c.target.image)).is_ok(),"Trusted operator must supply the reviewed local image; doctor does not build or pull it");
        }
    }
    let storage = path(&p.store)
        .ok()
        .and_then(|p| p.ancestors().find(|p| p.exists()).map(Path::to_owned))
        .and_then(|p| fs::metadata(p).ok())
        .is_some_and(|m| m.is_dir() && !m.permissions().readonly());
    add(
        "storage",
        storage,
        "Choose a controller-owned writable store; actual writes are tested only during execution",
    );
    fields
}
pub fn doctor_human(v: &Value) -> String {
    let mut text = format!(
        "Readiness only; verification_performed: false\n{}\n",
        if v["ready"] == true {
            "READY TO ATTEMPT VERIFICATION"
        } else {
            "NOT READY FOR VERIFICATION"
        }
    );
    if let Some(entries) = v["fields"].as_object() {
        for (id, fields) in entries {
            text.push_str(&format!(
                "Identity {}:\n",
                serde_json::to_string(id).unwrap_or_default()
            ));
            if let Some(fields) = fields.as_object() {
                for (name, field) in fields {
                    text.push_str(&format!(
                        "  {name}: {} — {}\n",
                        if field["ready"] == true {
                            "READY"
                        } else {
                            "BLOCKED"
                        },
                        field["next_action"]
                            .as_str()
                            .unwrap_or("Review trusted inputs")
                    ));
                }
            }
        }
    }
    text.push_str(
        v["next_action"]
            .as_str()
            .unwrap_or("Run verification after trusted review"),
    );
    text.push('\n');
    text
}
