//! Local canonical store v1. A run directory is reserved before execution;
//! result.json is the commit marker. Hashes detect corruption, not hostile rewriting.
use crate::{canonical_bytes, canonical_hash, Evidence};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct EvidenceStore {
    root: PathBuf,
    reads: Option<Arc<Mutex<BTreeMap<String, String>>>>,
}
pub struct RunStore {
    path: PathBuf,
    run_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    store_schema_version: String,
    run_id: String,
    evidence_hashes: BTreeMap<String, String>,
    result: serde_json::Value,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    manifest: Manifest,
    integrity_hash: String,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn component(value: &str) -> io::Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("invalid store identity"));
    }
    Ok(())
}
fn real_directory(path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(invalid("expected real directory"));
    }
    Ok(())
}
fn read_canonical<T: serde::de::DeserializeOwned + Serialize>(path: &Path) -> io::Result<T> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("expected regular artifact"));
    }
    let bytes = fs::read(path)?;
    let value: T = serde_json::from_slice(&bytes)?;
    if canonical_bytes(&value)? != bytes {
        return Err(invalid("noncanonical or corrupt artifact"));
    }
    Ok(value)
}
fn atomic_write<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = canonical_bytes(value)?;
    let temporary = path.with_extension("pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        // hard_link publishes an already complete inode without replacing a name.
        fs::hard_link(&temporary, path)?;
        fs::remove_file(&temporary)?;
        File::open(path.parent().ok_or_else(|| invalid("missing parent"))?)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
impl EvidenceStore {
    /// Trusted callers may enforce stronger product-specific storage boundaries.
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Root is normally `.b2ige/runs`; caller owns this trusted local directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            reads: None,
        }
    }
    /// Capture the exact transitive source inventory read by a verified loader.
    /// Repeated reads must agree; this is content binding, not filesystem isolation.
    pub fn record_reads<T>(
        &self,
        load: impl FnOnce(&Self) -> io::Result<T>,
    ) -> io::Result<(T, BTreeMap<String, String>)> {
        let reads = Arc::new(Mutex::new(BTreeMap::new()));
        let scoped = Self {
            root: self.root.clone(),
            reads: Some(reads.clone()),
        };
        let result = load(&scoped)?;
        let inventory = reads
            .lock()
            .map_err(|_| invalid("poisoned read inventory"))?
            .clone();
        if inventory.values().any(String::is_empty) {
            return Err(invalid("source changed during verified load"));
        }
        Ok((result, inventory))
    }
    pub fn reserve(&self, run_id: &str) -> io::Result<RunStore> {
        component(run_id)?;
        fs::create_dir_all(&self.root)?;
        real_directory(&self.root)?;
        let path = self.root.join(run_id);
        fs::create_dir(&path)?; // existing and interrupted runs are never reused
        fs::create_dir(path.join("evidence"))?;
        File::open(&self.root)?.sync_all()?;
        Ok(RunStore {
            path,
            run_id: run_id.into(),
        })
    }
    /// Rejects partial runs, altered payloads, wrong run links and noncanonical JSON.
    pub fn load(&self, run_id: &str) -> io::Result<(serde_json::Value, Vec<Evidence>)> {
        component(run_id)?;
        real_directory(&self.root)?;
        let path = self.root.join(run_id);
        real_directory(&path)?;
        real_directory(&path.join("evidence"))?;
        let commit: Commit = read_canonical(&path.join("result.json"))?;
        let manifest = commit.manifest;
        if manifest.store_schema_version != "1"
            || manifest.run_id != run_id
            || canonical_hash(&manifest)? != commit.integrity_hash
            || manifest.evidence_hashes.is_empty()
        {
            return Err(invalid("invalid run commit"));
        }
        let mut evidence = vec![];
        for (id, hash) in manifest.evidence_hashes {
            component(&id)?;
            let item: Evidence = read_canonical(&path.join("evidence").join(format!("{id}.json")))?;
            if !item.valid()
                || item.evidence_id != id
                || item.run_id != run_id
                || canonical_hash(&item)? != hash
            {
                return Err(invalid("invalid evidence integrity or run linkage"));
            }
            evidence.push(item);
        }
        if let Some(reads) = &self.reads {
            let identity = canonical_hash(&serde_json::json!({
                "domain": "b2ige.verify.source-read.v1",
                "run_id": run_id, "result": manifest.result, "evidence": evidence,
            }))?;
            let mut reads = reads
                .lock()
                .map_err(|_| invalid("poisoned read inventory"))?;
            if reads.get(run_id).is_some_and(|old| old != &identity) {
                // Sticky failure even if a caller handles an individual read error.
                reads.insert(run_id.into(), String::new());
                return Err(invalid("source changed during verified load"));
            }
            reads.insert(run_id.into(), identity);
        }
        Ok((manifest.result, evidence))
    }
}
impl RunStore {
    pub fn write_evidence(&self, item: &Evidence) -> io::Result<()> {
        component(&item.evidence_id)?;
        if item.run_id != self.run_id || !item.valid() {
            return Err(invalid("invalid evidence"));
        }
        if self.path.join("result.json").exists() {
            return Err(invalid("run already committed"));
        }
        atomic_write(
            &self
                .path
                .join("evidence")
                .join(format!("{}.json", item.evidence_id)),
            item,
        )
    }
    /// Commits only evidence reread and verified from disk. Result is a canonical
    /// acquisition artifact, not a product verdict supplied by the target.
    pub fn complete<T: Serialize>(&self, result: &T, evidence_ids: &[String]) -> io::Result<()> {
        if evidence_ids.is_empty() {
            return Err(invalid("missing evidence"));
        }
        let mut evidence_hashes = BTreeMap::new();
        for id in evidence_ids {
            component(id)?;
            let item: Evidence =
                read_canonical(&self.path.join("evidence").join(format!("{id}.json")))?;
            if !item.valid()
                || item.evidence_id != *id
                || item.run_id != self.run_id
                || evidence_hashes
                    .insert(id.clone(), canonical_hash(&item)?)
                    .is_some()
            {
                return Err(invalid("invalid or duplicate evidence"));
            }
        }
        let manifest = Manifest {
            store_schema_version: "1".into(),
            run_id: self.run_id.clone(),
            evidence_hashes,
            result: serde_json::from_slice(&canonical_bytes(result)?)?,
        };
        let integrity_hash = canonical_hash(&manifest)?;
        atomic_write(
            &self.path.join("result.json"),
            &Commit {
                manifest,
                integrity_hash,
            },
        )
    }
}
