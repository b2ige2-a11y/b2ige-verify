//! Validate the independent release manifest schema using the installed workspace validator.
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "expected schema and manifest paths");
    let read = |p: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
    };
    let schema = read(&args[0]);
    let manifest = read(&args[1]);
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&manifest)
        .unwrap();
    println!("Release manifest schema: PASS");
}
