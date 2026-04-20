fn web_tool_name_for_invariant(name: &str) -> bool {
    matches!(
        normalize_tool_name(name).as_str(),
        "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "batch_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
            | "web_tooling_health_probe"
    )
}

fn response_tools_include_web_attempt(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        web_tool_name_for_invariant(&name)
    })
}

fn response_tools_web_blocked(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            return false;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || error.contains("nexus_delivery_denied")
            || error.contains("permission_denied")
    })
}

fn response_tools_web_low_signal(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            return false;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2000);
        matches!(status.as_str(), "low_signal" | "no_results")
            || response_looks_like_tool_ack_without_findings(&result)
            || response_is_no_findings_placeholder(&result)
            || response_looks_like_unsynthesized_web_snippet_dump(&result)
            || response_looks_like_raw_web_artifact_dump(&result)
    })
}

fn web_failure_code_from_response_tools(response_tools: &[Value]) -> String {
    for row in response_tools {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            continue;
        }
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        if error.is_empty() {
            let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
                .to_ascii_lowercase();
            if matches!(status.as_str(), "blocked" | "policy_denied") {
                return "web_tool_policy_blocked".to_string();
            }
            if status == "timeout" {
                return "web_tool_timeout".to_string();
            }
            if matches!(status.as_str(), "low_signal" | "no_results") {
                return "web_tool_low_signal".to_string();
            }
            continue;
        }
        if error.contains("nexus_delivery_denied") || error.contains("permission_denied") {
            return "web_tool_policy_blocked".to_string();
        }
        if error.contains("invalid_response_attempt") {
            return "web_tool_invalid_response".to_string();
        }
        if error.contains("timeout") {
            return "web_tool_timeout".to_string();
        }
        if error.contains("401") {
            return "web_tool_http_401".to_string();
        }
        if error.contains("403") {
            return "web_tool_http_403".to_string();
        }
        if error.contains("404") {
            return "web_tool_http_404".to_string();
        }
        if error.contains("422") {
            return "web_tool_http_422".to_string();
        }
        if error.contains("429") {
            return "web_tool_http_429".to_string();
        }
        if error.contains("500")
            || error.contains("502")
            || error.contains("503")
            || error.contains("504")
        {
            return "web_tool_http_5xx".to_string();
        }
        return "web_tool_error".to_string();
    }
    String::new()
}

fn classify_web_turn_state(
    requires_live_web: bool,
    tool_attempted: bool,
    blocked: bool,
    low_signal: bool,
) -> String {
    if !requires_live_web {
        return "not_requested".to_string();
    }
    if !tool_attempted {
        return "parse_failed".to_string();
    }
    if blocked {
        return "policy_blocked".to_string();
    }
    if low_signal {
        return "provider_low_signal".to_string();
    }
    "healthy".to_string()
}

fn response_tools_any_blocked(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || error.contains("nexus_delivery_denied")
            || error.contains("permission_denied")
    })
}

fn response_tools_any_low_signal(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        matches!(status.as_str(), "low_signal" | "no_results" | "partial_no_results")
            || response_looks_like_tool_ack_without_findings(&result)
            || response_is_no_findings_placeholder(&result)
    })
}
