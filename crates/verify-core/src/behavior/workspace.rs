//! Content/mode snapshot reset, not a filesystem observer or a sandbox.
use super::{invalid, raw_hash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, FileTimes, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use verify_evidence::canonical_hash;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;

#[derive(Serialize, Deserialize)]
struct Entry {
    mode: u32,
    // None is a directory; Some(empty) is an empty regular file.
    bytes: Option<Vec<u8>>,
}

pub(crate) struct Snapshot(BTreeMap<String, Entry>);

#[cfg(unix)]
fn mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn mode(_: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> io::Result<()> {
    Err(invalid("P2 workspace reset requires Unix"))
}

impl Snapshot {
    pub(crate) fn capture(source: Option<&Path>) -> io::Result<Self> {
        let mut snapshot = Self(BTreeMap::new());
        if let Some(source) = source {
            if !source.is_absolute() || !fs::symlink_metadata(source)?.is_dir() {
                return Err(invalid("fixture must be an absolute real directory"));
            }
            snapshot.read(source, source, &mut 0, 0)?;
        } else {
            snapshot.0.insert(
                String::new(),
                Entry {
                    mode: 0o700,
                    bytes: None,
                },
            );
        }
        Ok(snapshot)
    }

    fn read(
        &mut self,
        root: &Path,
        path: &Path,
        total: &mut usize,
        depth: usize,
    ) -> io::Result<()> {
        if self.0.len() >= MAX_ENTRIES || depth > 64 {
            return Err(invalid("fixture exceeds entry/depth limit"));
        }
        let metadata = fs::symlink_metadata(path)?;
        let name = path
            .strip_prefix(root)
            .map_err(io::Error::other)?
            .to_str()
            .ok_or_else(|| invalid("fixture paths must be UTF-8"))?
            .to_owned();
        let bytes = if metadata.is_file() {
            let mut bytes = Vec::new();
            File::open(path)?
                .take((MAX_BYTES - *total + 1) as u64)
                .read_to_end(&mut bytes)?;
            *total += bytes.len();
            if *total > MAX_BYTES {
                return Err(invalid("fixture exceeds 64 MiB limit"));
            }
            Some(bytes)
        } else if metadata.is_dir() {
            None
        } else {
            return Err(invalid(
                "fixture symlinks and special files are unsupported",
            ));
        };
        self.0.insert(
            name,
            Entry {
                mode: mode(&metadata),
                bytes,
            },
        );
        if metadata.is_dir() {
            for child in fs::read_dir(path)? {
                self.read(root, &child?.path(), total, depth + 1)?;
            }
        }
        Ok(())
    }

    pub(crate) fn identity(&self) -> io::Result<String> {
        // Source paths and host metadata are deliberately not input identity.
        // Both copies get the same fixed access/modified times and permission bits.
        let entries: BTreeMap<_, _> = self
            .0
            .iter()
            .map(|(path, entry)| {
                (
                    path,
                    serde_json::json!({
                        "mode": entry.mode,
                        "content_hash": entry.bytes.as_deref().map(raw_hash),
                    }),
                )
            })
            .collect();
        Ok(canonical_hash(&serde_json::json!({
            "snapshot_version": "1", "timestamps": "unix_epoch", "entries": entries,
        }))?)
    }

    pub(crate) fn materialize(&self, destination: &Path) -> io::Result<()> {
        // create_dir/create_new never reuse an old or interrupted workspace.
        for (name, entry) in &self.0 {
            let path = destination.join(name);
            if let Some(bytes) = &entry.bytes {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                file.write_all(bytes)?;
                file.sync_all()?;
            } else {
                fs::create_dir(&path)?;
            }
        }
        // Reverse order restores directory metadata after children are created.
        for (name, entry) in self.0.iter().rev() {
            let path = destination.join(name);
            File::open(&path)?.set_times(
                FileTimes::new()
                    .set_accessed(UNIX_EPOCH)
                    .set_modified(UNIX_EPOCH),
            )?;
            set_mode(&path, entry.mode)?;
        }
        Ok(())
    }
}

pub(crate) fn reserve(root: &Path, comparison_id: &str) -> io::Result<PathBuf> {
    if !root.is_absolute() {
        return Err(invalid("workspace root must be absolute"));
    }
    fs::create_dir_all(root)?;
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(invalid("workspace root must be a real directory"));
    }
    let path = fs::canonicalize(root)?.join(comparison_id);
    fs::create_dir(&path)?;
    Ok(path)
}
