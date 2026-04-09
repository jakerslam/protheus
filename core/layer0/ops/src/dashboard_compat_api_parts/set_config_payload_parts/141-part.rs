fn summary_excluded_key(key: &str) -> bool {
    matches!(
        key,
        "screenshotBase64"
            | "content_base64"
            | "raw_html"
            | "html"
            | "raw_content"
            | "payload"
            | "response_finalization"
            | "turn_loop_tracking"
            | "turn_transaction"
            | "workspace_hints"
            | "latent_tool_candidates"
            | "nexus_connection"
    )
}

fn scalar_summary_fragment(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => {
            let trimmed = clean_text(raw, 160);
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Value::Bool(raw) => Some(if *raw { "true" } else { "false" }.to_string()),
        Value::Number(raw) => Some(raw.to_string()),
        _ => None,
    }
}

fn summarize_unknown_tool_payload(normalized: &str, payload: &Value) -> String {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return user_facing_tool_failure_summary(normalized, payload)
            .unwrap_or_else(|| format!("I couldn't complete `{normalized}` right now."));
    }
    if let Some(response) = payload.get("response").and_then(Value::as_str) {
        let candidate = clean_text(response, 1_400);
        if !candidate.is_empty()
            && !response_looks_like_tool_ack_without_findings(&candidate)
            && !response_looks_like_raw_web_artifact_dump(&candidate)
        {
            if let Some(unwrapped) = normalize_raw_response_payload_dump(&candidate) {
                return trim_text(&unwrapped, 1_400);
            }
            return trim_text(&candidate, 1_400);
        }
    }
    if let Some(summary) = payload.get("summary").and_then(Value::as_str) {
        let candidate = clean_text(summary, 1_200);
        if !candidate.is_empty() && !response_looks_like_tool_ack_without_findings(&candidate) {
            return trim_text(&candidate, 1_200);
        }
    }
    let mut fields = Vec::<String>::new();
    if let Some(obj) = payload.as_object() {
        for (key, value) in obj {
            if key == "ok" || summary_excluded_key(key.as_str()) {
                continue;
            }
            if let Some(fragment) = scalar_summary_fragment(value) {
                fields.push(format!("{}={}", clean_text(key, 40), fragment));
            } else if let Some(rows) = value.as_array() {
                if !rows.is_empty() {
                    fields.push(format!("{} count={}", clean_text(key, 40), rows.len()));
                }
            }
            if fields.len() >= 3 {
                break;
            }
        }
    }
    if fields.is_empty() {
        return format!("`{normalized}` completed. See tool details for structured output.");
    }
    trim_text(
        &format!(
            "`{normalized}` completed with {}.",
            clean_text(&fields.join(", "), 220)
        ),
        1_000,
    )
}
