//! Emit only the new P2 artifact schema; never rewrites P1 schemas or baselines.
fn main() {
    let schema = verify_core::behavior::comparison_schema();
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
