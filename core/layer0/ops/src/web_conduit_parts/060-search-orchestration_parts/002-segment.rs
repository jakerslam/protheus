fn search_failure_is_challenge_like(out: &Value, provider_errors: &[Value]) -> bool {
    !out.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !provider_errors.is_empty()
        && provider_errors.iter().all(|row| {
            row.get("challenge").and_then(Value::as_bool).unwrap_or(false)
                || row
                    .get("low_signal")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
}
