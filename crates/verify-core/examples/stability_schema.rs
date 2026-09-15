fn main() {
    let schema = match std::env::args().nth(1).as_deref() {
        Some("comparison") => verify_core::behavior::stability::comparison_schema(),
        Some("profile") => verify_core::behavior::stability::profile_schema(),
        _ => panic!("expected profile or comparison"),
    };
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
