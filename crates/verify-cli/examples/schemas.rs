fn main() {
    for (name, schema) in [
        ("agent-protocol.v1", verify_cli::agent::response_schema()),
        ("agent-request.v1", verify_cli::agent::request_schema()),
        ("report-document", verify_cli::report_schema()),
        ("agent-report", verify_cli::agent_schema()),
    ] {
        std::fs::write(
            format!("schemas/{name}.schema.json"),
            format!("{}\n", verify_cli::pretty(&schema)),
        )
        .unwrap();
    }
}
