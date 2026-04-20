fn fetch_truthy_value(value: &Value) -> bool {
    value.as_bool().unwrap_or_else(|| {
        value
            .as_str()
            .map(|raw| {
                let lowered = clean_text(raw, 24).to_ascii_lowercase();
                matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
            })
            .or_else(|| value.as_i64().map(|raw| raw != 0))
            .unwrap_or(false)
    })
}
