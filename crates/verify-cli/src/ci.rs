//! Read-only CI bootstrap unless --write is explicit. No approval or verdict API.
use crate::{adoption, integration::Entry};
use serde::{de, Deserialize, Deserializer};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::Path,
};

pub const HELP: &str = "b2ige ci init --identity ID --provider github-actions --registry FILE --verifier-repo OWNER/REPO --verifier-ref FULL_SHA [--root DIRECTORY] [--workflow .github/workflows/NAME.yml] [--write]";
const TEMPLATE: &str = include_str!("ci-workflow.yml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    #[serde(deserialize_with = "unique_entries")]
    entries: BTreeMap<String, Entry>,
}
fn unique_entries<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, Entry>, D::Error> {
    struct Unique;
    impl<'de> de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, Entry>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("unique registered identities")
        }
        fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut entries = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, Entry>()? {
                if entries.insert(key, value).is_some() {
                    return Err(de::Error::custom("duplicate identity"));
                }
            }
            Ok(entries)
        }
    }
    d.deserialize_map(Unique)
}
fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
fn token(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn run(args: &[String]) -> io::Result<()> {
    let mut options = BTreeMap::new();
    let mut write = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--write" && !write {
            write = true;
            i += 1;
            continue;
        }
        let key = args[i].as_str();
        if ![
            "--identity",
            "--provider",
            "--registry",
            "--verifier-repo",
            "--verifier-ref",
            "--root",
            "--workflow",
        ]
        .contains(&key)
            || i + 1 == args.len()
            || options.insert(key, args[i + 1].as_str()).is_some()
        {
            return Err(invalid(
                "unknown, missing or duplicate CI option; see ci --help",
            ));
        }
        i += 2;
    }
    let get = |key| {
        options.get(key).copied().ok_or_else(|| {
            invalid("identity, provider, registry and immutable verifier source are required")
        })
    };
    let id = get("--identity")?;
    if !token(id, 64) {
        return Err(invalid("unsafe identity"));
    }
    if get("--provider")? != "github-actions" {
        return Err(invalid("only github-actions is supported"));
    }
    let repo = get("--verifier-repo")?;
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2 || !parts.iter().all(|s| token(s, 100)) {
        return Err(invalid(
            "repository must be OWNER/REPO using letters, digits, underscores or hyphens",
        ));
    }
    let pin = get("--verifier-ref")?;
    if pin.len() != 40
        || !pin
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "verifier-ref requires a full lowercase 40-hex Git commit; mutable refs refused",
        ));
    }
    let registry_path = adoption::path(Path::new(get("--registry")?))?;
    let registry: Registry =
        serde_json::from_slice(&crate::integration::read_bytes(&registry_path)?)?;
    if registry.schema_version != "1"
        || registry.entries.is_empty()
        || registry.entries.iter().any(|(id, e)| {
            !token(id, 64)
                || e.config.as_os_str().is_empty()
                || e.store.as_os_str().is_empty()
                || match e.product {
                    crate::agent::Product::Behavior => e
                        .authorization
                        .as_ref()
                        .is_none_or(|p| p.as_os_str().is_empty()),
                    _ => e.authorization.is_some(),
                }
        })
    {
        return Err(invalid("malformed CI registry"));
    }
    let entry = registry
        .entries
        .get(id)
        .ok_or_else(|| invalid("unknown registered identity"))?;
    let root = adoption::path(Path::new(options.get("--root").copied().unwrap_or(".")))?;
    if !root.is_dir() {
        return Err(invalid("project root must already exist"));
    }
    let destination = options
        .get("--workflow")
        .copied()
        .unwrap_or(".github/workflows/b2ige-verify.yml");
    let name = destination
        .strip_prefix(".github/workflows/")
        .and_then(|s| s.strip_suffix(".yml"));
    if !name.is_some_and(|s| token(s, 64)) {
        return Err(invalid(
            "destination must be .github/workflows/NAME.yml without traversal",
        ));
    }
    let path = adoption::path(&root.join(destination))?;
    if path.try_exists()? {
        return Err(invalid("workflow exists; review preview in a disposable root and compare manually; no overwrite"));
    }
    // Substitute only original template tokens; user values are never re-expanded.
    let yaml: String = TEMPLATE
        .split("__")
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 0 {
                return part;
            }
            match part {
                "IDENTITY" => id,
                "REPO" => repo,
                "PIN" => pin,
                _ => unreachable!("reviewed template token"),
            }
        })
        .collect();
    if write {
        fs::create_dir_all(path.parent().expect("workflow parent"))?;
        adoption::path(&path)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(yaml.as_bytes())?;
        file.sync_all()?;
    }
    println!("CI bootstrap only; verification_performed: false\nIdentity: {id}\nProduct: {:?}\nProvider: github-actions\nWorkflow: {destination}\nVerifier repository: {repo}\nVerifier commit: {pin}\nController: trusted pull_request_target base SHA; independent pinned verifier\nCandidate: exact event head SHA, Git objects only; separate isolated build provisioning REQUIRED\nExecution: trusted CI permits existing BlindTest DOCKER_ISOLATION only; native Behavior/SideEffect BLOCKED pending reviewed isolated runtime; build and approval do not provide isolation\nProtected inputs: reviewed base .b2ige/project.json (all contracts), configs, authorizations, sealed root, candidate-bound approval and approved runtime targets\nLocal --registry is inspection input; deploy its reviewed inventory at base .b2ige/project.json\nRetention: sanitized Agent JSON only, 14 days\nOnly PASS/0 is green; FAIL/1, INCONCLUSIVE/2, ERROR/3 remain non-green\nNo approval generation; no MCP/Agent approval path\nUnsupported: automatic candidate provisioning, non-GitHub providers, same-user secrecy\nMode: {}\n--- workflow ---\n{yaml}", entry.product, if write { "created" } else { "preview (no writes)" });
    Ok(())
}
pub fn cli(args: &[String]) -> u8 {
    if args == ["--help"] || args == ["init", "--help"] {
        println!("{HELP}");
        return 0;
    }
    if args.first().map(String::as_str) != Some("init") {
        eprintln!("{HELP}");
        return 64;
    }
    match run(&args[1..]) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("CI bootstrap refused: {e}");
            3
        }
    }
}
