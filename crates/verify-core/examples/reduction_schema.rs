fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&verify_core::behavior::reduction::result_schema()).unwrap()
    );
}
