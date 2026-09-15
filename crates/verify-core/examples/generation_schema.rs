//! Regenerate the independent P3B v1 schemas; existing P2/P3A schemas stay v1.
fn main() {
    let schema = match std::env::args().nth(1).as_deref() {
        Some("spec") => verify_core::behavior::generation::spec_schema(),
        Some("suite") => verify_core::behavior::generation::suite_schema(),
        _ => panic!("usage: generation_schema spec|suite"),
    };
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
