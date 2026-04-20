
fn tool_failure_code_from_response_tools(response_tools: &[Value]) -> String {
    for row in response_tools {
        let normalized_name =
            normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if normalized_name.eq_ignore_ascii_case("thought_process") {
            continue;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied");
        let token = {
            let cleaned = clean_text(&normalized_name, 48);
            if cleaned.is_empty() {
                "tool".to_string()
            } else {
                cleaned
            }
        };
        if blocked {
            return format!("{token}_policy_blocked");
        }
        if status == "timeout" || error.contains("timeout") {
            return format!("{token}_timeout");
        }
        if matches!(status.as_str(), "low_signal" | "no_results" | "partial_no_results") {
            return format!("{token}_low_signal");
        }
        if error.contains("invalid_response_attempt") || error.contains("invalid_response") {
            return format!("{token}_invalid_response");
        }
        if error.contains("401") {
            return format!("{token}_http_401");
        }
        if error.contains("403") {
            return format!("{token}_http_403");
        }
        if error.contains("404") {
            return format!("{token}_http_404");
        }
        if error.contains("422") {
            return format!("{token}_http_422");
        }
        if error.contains("429") {
            return format!("{token}_http_429");
        }
        if error.contains("500")
            || error.contains("502")
            || error.contains("503")
            || error.contains("504")
        {
            return format!("{token}_http_5xx");
        }
        let errored = row.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        if errored || matches!(status.as_str(), "error" | "failed" | "execution_error") {
            return format!("{token}_error");
        }
    }
    String::new()
}

fn process_summary_tool_rows(response_tools: &[Value], limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    for row in response_tools.iter().take(limit.clamp(1, 8)) {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80);
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160);
        let result_excerpt = clean_text(
            &first_sentence(
                row.get("result").and_then(Value::as_str).unwrap_or(""),
                240,
            ),
            240,
        );
        rows.push(json!({
            "tool": if name.is_empty() { "tool" } else { &name },
            "status": status,
            "error": error,
            "is_error": row.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "blocked": row.get("blocked").and_then(Value::as_bool).unwrap_or(false),
            "result_excerpt": result_excerpt
        }));
    }
    Value::Array(rows)
}

fn build_turn_process_summary(
    message: &str,
    response_tools: &[Value],
    response_workflow: &Value,
    response_finalization: &Value,
) -> Value {
    json!({
        "contract": "turn_process_summary_v1",
        "generated_at": crate::now_iso(),
        "request_excerpt": clean_text(message, 240),
        "tool_gate": response_workflow
            .get("tool_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "final_llm_status": clean_text(
            response_workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80
        ),
        "finalization_outcome": clean_text(
            response_finalization
                .get("outcome")
                .and_then(Value::as_str)
                .unwrap_or(""),
            220
        ),
        "final_answer_contract": response_finalization
            .get("final_answer_contract")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "quality_telemetry": response_workflow
            .get("quality_telemetry")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "tooling_invariant": response_finalization
            .get("tooling_invariant")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "web_invariant": response_finalization
            .get("web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "tools": {
            "attempted_count": response_tools.len(),
            "attempts": process_summary_tool_rows(response_tools, 5)
        }
    })
}

fn response_message_is_actionable_for_next_steps(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("next step")
        || lowered.contains("what next")
        || lowered.contains("what should")
        || lowered.contains("how should")
        || lowered.contains("how can we")
        || lowered.contains("plan")
        || lowered.contains("improve")
        || lowered.contains("fix")
        || lowered.contains("implement")
        || lowered.contains("harden")
}

fn next_action_options_for_message(message: &str, response_tools: &[Value]) -> Vec<String> {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.contains("web")
        || response_tools.iter().any(|row| {
            let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
            web_tool_name_for_invariant(&name)
        })
    {
        return vec![
            "retry with a narrower web query".to_string(),
            "target one trusted source URL".to_string(),
            "switch to local/workspace evidence only".to_string(),
        ];
    }
    if lowered.contains("implement") || lowered.contains("patch") || lowered.contains("fix") {
        return vec![
            "confirm exact acceptance criteria".to_string(),
            "apply the minimal code patch".to_string(),
            "run a targeted regression check".to_string(),
        ];
    }
    vec![
        "clarify the exact outcome you want".to_string(),
        "run one targeted tool call".to_string(),
        "return a concise answer from current context".to_string(),
    ]
}

fn append_next_actions_line_if_actionable(
    message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if cleaned.is_empty() || !response_message_is_actionable_for_next_steps(message) {
        return cleaned;
    }
    if cleaned.to_ascii_lowercase().contains("next actions:") {
        return cleaned;
    }
    let options = next_action_options_for_message(message, response_tools);
    if options.is_empty() {
        return cleaned;
    }
    trim_text(
        &format!(
            "{}\n\nNext actions: 1) {} 2) {} 3) {}",
            cleaned,
            options.first().cloned().unwrap_or_default(),
            options.get(1).cloned().unwrap_or_default(),
            options.get(2).cloned().unwrap_or_default()
        ),
        32_000,
    )
}
