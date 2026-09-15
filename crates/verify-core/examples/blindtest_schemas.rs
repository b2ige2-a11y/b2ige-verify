use verify_core::blindtest::*;
fn main() {
    let schemas = [
        (
            "blindtest-requirement",
            schema::<RequirementArtifact>("blindtest-requirement", "1"),
        ),
        (
            "blindtest-invariant.v2",
            schema::<InvariantArtifact>("blindtest-invariant", "2"),
        ),
        (
            "blindtest-hidden-suite-manifest",
            schema::<BlindTestHiddenSuiteManifest>("blindtest-hidden-suite-manifest", "1"),
        ),
        (
            "blindtest-hidden-case",
            schema::<BlindTestHiddenCase>("blindtest-hidden-case", "1"),
        ),
        (
            "blindtest-hidden-oracle",
            schema::<HiddenOracle>("blindtest-hidden-oracle", "1"),
        ),
        (
            "blindtest-sealed-suite",
            schema::<SealedSuite>("blindtest-sealed-suite", "1"),
        ),
        (
            "blindtest-isolation-attestation",
            schema::<BlindTestIsolationAttestation>("blindtest-isolation-attestation", "1"),
        ),
        (
            "blindtest-suite-validation-receipt",
            schema::<BlindTestSuiteValidationReceipt>("blindtest-suite-validation-receipt", "1"),
        ),
        (
            "blindtest-run-result",
            schema::<BlindTestRunResult>("blindtest-run-result", "1"),
        ),
        (
            "blindtest-config",
            schema::<BlindTestConfig>("blindtest-config", "1"),
        ),
        (
            "blindtest-validation-config",
            schema::<SuiteValidationConfig>("blindtest-validation-config", "1"),
        ),
    ];
    for (name, value) in schemas {
        std::fs::write(
            format!("schemas/{name}.schema.json"),
            format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        )
        .unwrap();
    }
}
