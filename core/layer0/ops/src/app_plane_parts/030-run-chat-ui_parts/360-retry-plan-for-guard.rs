fn chat_ui_retry_plan_for_guard(
    retry_recommended: bool,
    retry_strategy: &str,
    retry_lane: &str,
) -> Value {
    if !retry_recommended {
        return json!({
            "auto": false,
            "attempts": 0,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        });
    }
    match (retry_strategy, retry_lane) {
        ("retry_with_backoff", "delayed") => json!({
            "auto": true,
            "attempts": 2,
            "min_delay_ms": 400,
            "max_delay_ms": 30000,
            "jitter": 0.1
        }),
        ("rerun_with_tool_call", "immediate") => json!({
            "auto": true,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
        ("narrow_query", "immediate") => json!({
            "auto": false,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
        _ => json!({
            "auto": false,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
    }
}

#[cfg(test)]
#[test]
