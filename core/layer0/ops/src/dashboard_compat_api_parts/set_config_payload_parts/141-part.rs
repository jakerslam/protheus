fn safe_step_command_candidate(message: &str) -> String {
    let cleaned = clean_text(message, 800);
    if cleaned.is_empty() {
        return String::new();
    }
    if let Some(start) = cleaned.find('`') {
        if let Some(end_rel) = cleaned[start + 1..].find('`') {
            let candidate = clean_text(&cleaned[start + 1..start + 1 + end_rel], 400);
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    let lowered = cleaned.to_ascii_lowercase();
    if let Some(rest) = lowered.strip_prefix("run ") {
        if let Some(end_idx) = rest.find(" as the next safe step") {
            let prefix_len = cleaned.len().saturating_sub(rest.len());
            let candidate = clean_text(&cleaned[prefix_len..prefix_len + end_idx], 400);
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    String::new()
}

fn follow_up_suggestion_tool_intent_from_message(message: &str) -> Option<(String, Value)> {
    let candidate = safe_step_command_candidate(message);
    if candidate.is_empty() {
        return None;
    }
    let lowered = candidate.to_ascii_lowercase();
    let supported_prefixes = [
        ("infring web search", "web search"),
        ("infring batch-query", "batch-query"),
        ("infring batch query", "batch-query"),
    ];
    for (prefix, label) in supported_prefixes {
        if !lowered.starts_with(prefix) {
            continue;
        }
        let query = strip_wrapped_natural_web_query(candidate[prefix.len()..].trim(), 600);
        if query.is_empty() {
            return Some((
                "tool_command_router".to_string(),
                json!({
                    "ok": false,
                    "error": "tool_command_query_required",
                    "message": format!(
                        "`{}` needs a query before it can run. Ask me to {} for a specific topic, for example `try to web search \"top AI agent frameworks\"`.",
                        clean_text(prefix, 80),
                        label
                    )
                }),
            ));
        }
        return Some((
            "batch_query".to_string(),
            json!({"source": "web", "query": query, "aperture": "medium"}),
        ));
    }
    None
}

fn message_requests_tooling_failure_diagnosis(message: &str) -> bool {
    let lowered = clean_text(message, 500).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let asks_about_tooling = lowered.contains("tooling")
        || lowered.contains("tool")
        || lowered.contains("web search")
        || lowered.contains("web fetch")
        || lowered.contains("file tooling")
        || lowered.contains("file access")
        || lowered.contains("search");
    let asks_failure = lowered.contains("broken")
        || lowered.contains("failing")
        || lowered.contains("failed")
        || lowered.contains("not working")
        || lowered.contains("isn't working")
        || lowered.contains("isnt working")
        || lowered.contains("failure mode")
        || lowered.contains("root cause")
        || lowered.contains("why")
        || lowered.contains("fix")
        || lowered.contains("better")
        || lowered.contains("improved");
    asks_about_tooling && asks_failure
}

fn normalize_placeholder_signature(text: &str) -> String {
    clean_text(text, 800)
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn response_mentions_context_guard(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("context overflow: estimated context size exceeds safe threshold during tool loop")
        || lowered.contains("more characters truncated")
        || lowered.contains("middle content omitted")
        || lowered.contains("output exceeded safe context budget")
        || lowered.contains("safe context budget")
}

fn web_tool_context_guard_fallback(scope: &str) -> String {
    let label = clean_text(scope, 120);
    if label.is_empty() {
        return "The web tool returned more output than fit safely in context before a final answer was composed. Retry with a narrower query, one specific source URL, or ask me to continue from the partial result.".to_string();
    }
    format!(
        "{} returned more output than fit safely in context before a final answer was composed. Retry with a narrower query, one specific source URL, or ask me to continue from the partial result.",
        label
    )
}

fn tooling_failure_diagnostic_fallback() -> String {
    "Web/search tooling is partially working: retrieval ran, but this turn returned low-signal output (search-engine chrome or parse miss) instead of usable findings. This is usually extraction/parsing drift, not a total outage. Next step: rerun with `batch_query` and a narrower query (or give one source URL for `web_fetch`). If it keeps repeating, run `infringctl doctor --json` and share the output so I can pinpoint the failing lane."
        .to_string()
}

fn follow_up_suggestion_no_findings_fallback(message: &str) -> Option<String> {
    if let Some((tool_name, payload)) = follow_up_suggestion_tool_intent_from_message(message) {
        if tool_name == "tool_command_router" {
            let summary = clean_text(
                payload.get("message").and_then(Value::as_str).unwrap_or(""),
                320,
            );
            if !summary.is_empty() {
                return Some(summary);
            }
        }
    }
    let lowered = clean_text(message, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return None;
    }
    if lowered.contains("command-to-route mapping")
        && lowered.contains("supported tool hit rate")
    {
        return Some(
            "That suggestion is an implementation task, not a runnable command. The right next step is to patch the command-to-route mapping and add a regression for the missed prompt shape."
                .to_string(),
        );
    }
    if lowered.contains("supported rust route")
        && (lowered.contains("tool::spawn_subagents") || lowered.contains("spawn_subagents"))
    {
        return Some(
            "That is a runtime-route implementation task, not a live web query. The right next step is to patch the Rust route layer for `spawn_subagents` and add a regression proving the prompt resolves cleanly."
                .to_string(),
        );
    }
    if lowered.contains("tooling")
        && lowered.contains("better")
        && (lowered.contains("web") || lowered.contains("file"))
    {
        return Some(
            "The web/file tooling is better in some lanes, but this turn still fell into the no-findings fallback instead of a real status answer. That points to a routing/finalization miss, not a total outage. Next step: run one concrete `web_search`, `web_fetch`, or `file_read` probe and inspect the route that handled it."
                .to_string(),
        );
    }
    None
}

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
    if normalized == "workspace_analyze" {
        let stdout = clean_text(payload.get("stdout").and_then(Value::as_str).unwrap_or(""), 2_000);
        let tool_summary = clean_text(
            payload.get("tool_summary").and_then(Value::as_str).unwrap_or(""),
            400,
        );
        let stderr = clean_text(payload.get("stderr").and_then(Value::as_str).unwrap_or(""), 800);
        let stdout_lines = stdout
            .lines()
            .map(|line| clean_text(line, 220))
            .filter(|line| !line.is_empty())
            .take(3)
            .collect::<Vec<_>>();
        if !stdout_lines.is_empty() {
            return trim_text(
                &format!("Key findings: {}", stdout_lines.join(" | ")),
                1_200,
            );
        }
        if !tool_summary.is_empty() {
            return trim_text(&format!("Key findings: {tool_summary}"), 1_200);
        }
        if !stderr.is_empty() {
            return trim_text(
                &format!(
                    "Workspace analysis returned diagnostics: {}",
                    first_sentence(&stderr, 220)
                ),
                1_200,
            );
        }
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

fn summarize_tool_capability_payload(
    normalized: &str,
    tool_name: &str,
    payload: &Value,
) -> Option<String> {
    if normalized != "tool_capabilities"
        && normalized != "capabilities"
        && normalized != "capability_status"
        && normalized != "tools_status"
    {
        return None;
    }
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Some(user_facing_tool_failure_summary(tool_name, payload).unwrap_or_else(
            || "I couldn't inspect tool capabilities right now.".to_string(),
        ));
    }
    let mut lines = vec!["Tool capability status (governed router):".to_string()];
    if let Some(rows) = payload.get("tools").and_then(Value::as_array) {
        for row in rows.iter().take(8) {
            let name = clean_text(row.get("tool").and_then(Value::as_str).unwrap_or(""), 80);
            let tier = clean_text(row.get("tier").and_then(Value::as_str).unwrap_or(""), 40);
            if name.is_empty() {
                continue;
            }
            if tier.is_empty() {
                lines.push(format!("- {name}"));
            } else {
                lines.push(format!("- {name}: {tier}"));
            }
        }
    }
    let domains = payload
        .get("catalog_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let domain = clean_text(row.get("domain").and_then(Value::as_str).unwrap_or(""), 80);
            let count = row
                .get("tool_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if domain.is_empty() {
                None
            } else {
                Some(format!("{domain} ({count})"))
            }
        })
        .collect::<Vec<_>>();
    if !domains.is_empty() {
        lines.push(format!(
            "Catalog domains: {}.",
            clean_text(&domains.join(", "), 240)
        ));
    }
    let read_defaults = payload
        .get("read_surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        })
        .collect::<Vec<_>>();
    if !read_defaults.is_empty() {
        lines.push(format!(
            "Default read surfaces: {}.",
            clean_text(&read_defaults.join(", "), 240)
        ));
    }
    Some(trim_text(&lines.join("\n"), 24_000))
}
