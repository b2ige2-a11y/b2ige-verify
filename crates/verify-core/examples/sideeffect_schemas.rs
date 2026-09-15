fn main() {
    for (name, mut value) in [
        (
            "sideeffect-contract",
            verify_core::sideeffect::contract_schema(),
        ),
        (
            "sideeffect-experiment-result",
            verify_core::sideeffect::result_schema(),
        ),
        (
            "sideeffect-history",
            verify_core::sideeffect::history_schema(),
        ),
        (
            "fault-schedule-result",
            serde_json::to_value(schemars::schema_for!(
                verify_core::sideeffect::FaultScheduleResult
            ))
            .unwrap(),
        ),
    ] {
        value["$id"] = format!("https://b2ige.dev/schemas/verify/{name}.v1.json").into();
        value["properties"]["schema_version"]["const"] = "1".into();
        std::fs::write(
            format!("schemas/{name}.schema.json"),
            format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        )
        .unwrap();
    }
}
