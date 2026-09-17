//! Benchmark-only mechanical relocation of existing reviewed fixtures.
//! No candidate is executed here; all authority comes from the included pre-V110 code.
#![cfg_attr(not(unix), allow(unused))]
#[cfg(unix)]
#[path = "../../verify-mcp/tests/support/behavior.rs"]
mod behavior_fixture;
#[cfg(unix)]
#[path = "../../../benchmarks/corpus/blindtest.rs"]
mod blind_fixture;
#[cfg(unix)]
#[path = "../../verify-mcp/tests/support/sideeffect.rs"]
mod effect_fixture;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf};
    use verify_core::behavior::{executable_identity, snapshot_identity};
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("benchmark fixture: PRODUCT MODE PROJECT CONTROLLER TARGET".into());
    }
    let project = PathBuf::from(&args[3]);
    let controller = PathBuf::from(&args[4]);
    let target = PathBuf::from(&args[5]);
    if !project.is_absolute()
        || !controller.is_absolute()
        || project.starts_with(&controller)
        || controller.starts_with(&project)
        || controller.join("input.json").exists()
    {
        return Err("fresh separate benchmark lanes required".into());
    }
    let input = controller.join("input.json");
    match args[1].as_str() {
        "behavior" => {
            // Keep the original reference, baseline, stability assertion and checker pins.
            let c = behavior_fixture::Case::new("printf same");
            let mut experiment = c.experiment.clone();
            let reference = controller.join("reference");
            fs::copy(&experiment.before.executable, &reference)?;
            experiment.before.executable = reference;
            experiment.after.executable = target;
            experiment.after.identity = executable_identity(&experiment.after.executable)?;
            fs::write(
                controller.join("authorization.json"),
                serde_json::to_vec(&c.auth)?,
            )?;
            fs::write(input, serde_json::to_vec(&experiment)?)?;
            fs::remove_dir_all(c.dir)?;
        }
        "sideeffect" => {
            let c = effect_fixture::Case::new(if args[2] == "unsafe_retry" {
                "unsafe"
            } else {
                "safe"
            });
            let fixture = project.join("fixture");
            fs::create_dir(&fixture)?;
            fs::copy(c.dir.join("fixture/ledger.db"), fixture.join("ledger.db"))?;
            // Relocate the exact reviewed script into a conventional source tree and
            // the reset snapshot. Snapshot identity binds its bytes, not an external path.
            fs::create_dir(project.join("src"))?;
            fs::write(project.join("src/checkout.py"), &c.contract.trigger.args[1])?;
            fs::copy(project.join("src/checkout.py"), fixture.join("checkout.py"))?;
            let mut contract = c.contract.clone();
            contract.trigger.args = vec!["checkout.py".into()];
            contract.trigger.fixture.source = Some(fixture.clone());
            contract.trigger.fixture.snapshot_identity = snapshot_identity(Some(&fixture))?;
            fs::write(input, serde_json::to_vec(&contract)?)?;
        }
        "blindtest" => {
            // P8 constructor's approvals are fixed independently of candidate execution.
            let c = blind_fixture::Corpus::new(controller.join("material"));
            let mut config = c.config.clone();
            config.target.workspace = project.clone();
            config.target.workspace_hash = snapshot_identity(Some(&project))?;
            config.target.image = args[5].clone();
            if args[2] == "partial" {
                config.max_cases = 1;
            }
            fs::write(input, serde_json::to_vec(&config)?)?;
        }
        _ => return Err("unsupported benchmark product".into()),
    }
    Ok(())
}
#[cfg(not(unix))]
fn main() {
    eprintln!("Adoption benchmark requires Unix PTYs and existing Unix fixtures");
    std::process::exit(3);
}
