//! Trusted local invocation and readiness. This module does not check verdicts.
use crate::{
    agent::{Operation, Product, Response},
    load, VerifiedReport,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use verify_core::{
    behavior::{self, BehaviorAuthorization, BehaviorExperiment},
    blindtest, sideeffect, Verdict,
};
use verify_evidence::store::EvidenceStore;

pub fn read<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    Ok(serde_json::from_slice(&read_bytes(path)?)?)
}
pub(crate) fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(io::Error::other("config too large"));
    }
    Ok(bytes)
}
pub fn nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "p7-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn empty_auth() -> BehaviorAuthorization {
    BehaviorAuthorization {
        approved_baselines: Default::default(),
        approved_checker_bindings: Default::default(),
        baseline_stable: false,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub product: Product,
    pub config: PathBuf,
    pub store: PathBuf,
    pub authorization: Option<PathBuf>,
}
/// Loaded once by the trusted controller. Caller cannot substitute a config at tool time.
pub enum Config {
    Behavior(Box<BehaviorExperiment>, BehaviorAuthorization),
    Sideeffect(Box<sideeffect::SideEffectContract>),
    Blindtest(Box<blindtest::BlindTestConfig>, PathBuf),
}
pub struct Prepared {
    pub config: Config,
    pub store: PathBuf,
}
impl Prepared {
    pub fn open(e: &Entry) -> io::Result<Self> {
        let config = match e.product {
            Product::Behavior => Config::Behavior(
                Box::new(read(&e.config)?),
                read(
                    e.authorization
                        .as_deref()
                        .ok_or_else(|| io::Error::other("authorization required"))?,
                )?,
            ),
            Product::Sideeffect => Config::Sideeffect(Box::new(read(&e.config)?)),
            Product::Blindtest => Config::Blindtest(
                Box::new(read(&e.config)?),
                std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT")
                    .map(PathBuf::from)
                    .ok_or_else(|| io::Error::other("sealed root required"))?,
            ),
        };
        Ok(Self {
            config,
            store: e.store.clone(),
        })
    }
    pub fn execute(&self) -> io::Result<(String, VerifiedReport)> {
        let store = EvidenceStore::new(&self.store);
        let id = nonce();
        let auth = empty_auth();
        let auth = match &self.config {
            Config::Behavior(_, a) => a,
            _ => &auth,
        };
        match &self.config {
            Config::Behavior(c, a) => {
                behavior::execute(&store, &self.store.join("work"), &id, c, a)?;
            }
            Config::Sideeffect(c) => {
                sideeffect::execute(c, &store, &id)?;
            }
            Config::Blindtest(c, sealed) => {
                blindtest::execute(c, sealed, &store, &id)?;
            }
        }
        Ok((id.clone(), load(&store, &id, auth)?))
    }
    pub fn report(&self, id: &str) -> io::Result<VerifiedReport> {
        let auth = empty_auth();
        let auth = match &self.config {
            Config::Behavior(_, a) => a,
            _ => &auth,
        };
        load(&EvidenceStore::new(&self.store), id, auth)
    }
    pub fn doctor(&self, product: Product) -> Response {
        let mut r = Response::error(product, Operation::Doctor);
        r.kind = "readiness".into();
        r.summary = "Readiness checks failed; verification was not performed".into();
        r.limitations =
            vec!["Readiness only; no verification verdict or per-run isolation attestation".into()];
        if self.check_ready().is_ok() {
            r.verdict = Verdict::Pass;
            r.summary = "Configured prerequisites are ready; verification was not performed".into();
            r.next_action =
                "Run the selected verifier; readiness cannot establish completion".into();
        }
        r
    }
    fn check_ready(&self) -> io::Result<()> {
        if crate::adoption::readiness_fields(self)
            .values()
            .all(|v| v["ready"] == true)
        {
            Ok(())
        } else {
            Err(io::Error::other("readiness prerequisites unavailable"))
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: String,
    #[serde(deserialize_with = "verify_evidence::unique_map")]
    pub entries: BTreeMap<String, Entry>,
}
fn registry_location(root: &Path) -> io::Result<(PathBuf, bool, bool)> {
    let path = root.join(".b2ige/project.json");
    let parent = path.parent().expect("registry parent");
    let parent_exists = match fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => true,
        Ok(_) => return Err(io::Error::other(".b2ige must be a real directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    let existing = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => true,
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(io::Error::other(
                "project registry symlinks are not accepted",
            ))
        }
        Ok(_) => return Err(io::Error::other("project registry must be a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    Ok((path, parent_exists, existing))
}
pub fn init(root: &Path, dry_run: bool) -> io::Result<Value> {
    let root = crate::adoption::path(root)?;
    let (path, parent_exists, existing) = registry_location(&root)?;
    if existing {
        crate::adoption::registry(&path)?;
    }
    let parent = path.parent().expect("registry parent");
    let d = blindtest::docker::doctor();
    let result = json!({"schema_version":"1","rust":root.join("Cargo.toml").is_file(),"node":root.join("package.json").is_file(),"docker_available":d.docker_available,"existing_config":existing,"dry_run":dry_run,"kind":"init","verification_performed":false,"surfaces":["behavior: approved executable comparison","sideeffect: configured local SQLite ledger","blindtest: approved sealed Docker suite"],"next_action":"Register reviewed product configs; stack detection does not create evidence or approve baselines"});
    if !dry_run && !existing {
        if !parent_exists {
            fs::create_dir(parent)?;
        }
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        f.write_all(
            serde_json::to_string_pretty(&Project {
                schema_version: "1".into(),
                entries: BTreeMap::new(),
            })?
            .as_bytes(),
        )?;
        f.sync_all()?;
    }
    Ok(result)
}
pub fn setup(root: &Path, dry_run: bool) -> io::Result<Value> {
    let mut result = init(root, dry_run)?;
    result["kind"] = "setup".into();
    result["verification_performed"] = false.into();
    Ok(result)
}
pub fn project_doctor(path: &Path) -> Value {
    let project = crate::adoption::registry(path);
    let docker = blindtest::docker::doctor();
    let mut checks = BTreeMap::new();
    let mut fields = BTreeMap::new();
    let valid = match project {
        Ok(p) if !p.entries.is_empty() => {
            for (key, e) in p.entries {
                let prepared = Prepared::open(&e);
                let details = match &prepared {
                    Ok(p) => crate::adoption::readiness_fields(p),
                    Err(_) => BTreeMap::from([(
                        "config_or_authorization",
                        json!({"ready":false,"next_action":"Supply a valid typed config, Behavior authorization or controller sealed root; review registered paths (legacy relative paths use the current directory)"}),
                    )]),
                };
                let ready = details.values().all(|v| v["ready"] == true);
                let mut response = Response::error(e.product, Operation::Doctor);
                response.kind = "readiness".into();
                response.summary = "Readiness only; verification was not performed".into();
                response.next_action =
                    "Review field blockers, then run registered verification".into();
                if ready {
                    response.verdict = Verdict::Pass;
                }
                fields.insert(key.clone(), details);
                checks.insert(key, response);
            }
            checks.values().all(|r| r.verdict == Verdict::Pass)
        }
        _ => false,
    };
    json!({"schema_version":"1","kind":"readiness","tool_version":env!("CARGO_PKG_VERSION"),"ready":valid,"verification_performed":false,"docker_available":docker.docker_available,"checks":checks,"fields":fields,"next_action":if fields.is_empty() {"Initialize a safe v1 registry, prepare explicit public inputs, then have a trusted operator approve an identity"} else {"Resolve field blockers and run b2ige verify ID --registry FILE; doctor success is readiness only"}})
}
