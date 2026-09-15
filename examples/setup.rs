//! Sets up reviewed example configs; never approves a user's candidate baseline.
use serde_json::json;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use verify_evidence::canonical_hash;
#[path = "behavior/setup.rs"]
mod behavior;
#[path = "blindtest/controller.rs"]
mod blindtest;
#[path = "sideeffect/setup.rs"]
mod sideeffect;
fn write_json(path: &Path, value: &impl serde::Serialize) -> io::Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "usage: b2ige-demo behavior|sideeffect|blindtest NEW_ABSOLUTE_DIRECTORY".into(),
        );
    }
    let root = PathBuf::from(&args[1]);
    if !root.is_absolute() || root.exists() {
        return Err("use a new absolute directory; no overwrite".into());
    }
    match args[0].as_str() {
        "behavior" => {
            fs::create_dir(&root)?;
            behavior::setup(&root.join("pass"), false)?;
            behavior::setup(&root.join("fail"), true)?;
        }
        "sideeffect" => sideeffect::setup(&root)?,
        "blindtest" => {
            let mut c = blindtest::Corpus::new(root.clone());
            c.build_images();
            for mode in ["correct", "mutant_a", "probe"] {
                write_json(&root.join(format!("{mode}.json")), &c.config_for(mode))?;
            }
        }
        _ => return Err("unknown product".into()),
    }
    Ok(())
}
