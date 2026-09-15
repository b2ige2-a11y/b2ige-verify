#[path = "../tests/support/blindtest.rs"]
mod support;
use support::*;
use verify_core::blindtest::*;
fn main() {
    let root = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("b2ige-p6-corpus-{}", std::process::id()))
        });
    assert!(
        !root.exists(),
        "Use a new corpus root; sealed fixtures are never overwritten"
    );
    let mut c = Corpus::new(root);
    c.build_images();
    for mode in [
        "correct", "mutant_a", "mutant_b", "noop", "probe", "timeout", "flood",
    ] {
        write_json(&c.root.join(format!("{mode}.json")), &c.config_for(mode));
    }
    let validation = c.validation();
    write_json(&c.root.join("validation.json"), &validation);
    let receipt =
        execute_validation(&validation, &c.sealed, &c.store, "corpus-validation").unwrap();
    let mut quality = c.config_for("correct");
    quality.validation_receipt = Some(receipt.blindtest_validation_id.clone());
    write_json(&c.root.join("quality.json"), &quality);
    let run = execute(&quality, &c.sealed, &c.store, "quality-correct").unwrap();
    write_json(
        &c.root.join("summary.json"),
        &serde_json::json!({"root":c.root,"receipt":receipt,"quality_verdict":run.verdict,"isolation":"DOCKER_ISOLATION","controller_platform":std::env::consts::OS}),
    );
    println!("{}", c.root.display());
    assert!(receipt.matched);
}
