fn response_is_actionable_tool_diagnostic(text: &str) -> bool {
    let cleaned = clean_text(text, 1_400);
    if cleaned.is_empty() {
        return false;
    }
    if response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
        || response_contains_tool_telemetry_dump(&cleaned)
    {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    lowered.contains("low-signal snippets without synthesis")
        || lowered.contains("low-signal web snippets")
        || lowered.contains("raw web output")
        || lowered.contains("search returned no useful comparison findings")
        || lowered.contains("retrieval-quality miss")
        || lowered.contains("retrieval/synthesis miss")
        || lowered.contains("tooling is partially working")
        || lowered.contains("needs a query before it can run")
        || lowered.contains("query before it can run")
        || lowered.contains("fit safely in context")
        || lowered.contains("doctor --json")
}

fn eval_agent_feedback_prompt_context(root: &Path, agent_id: &str, max_items: usize) -> String {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return String::new();
    }
    let path = root
        .join("local/state/ops/eval_agent_feedback")
        .join(format!("{id}.json"));
    let Some(state) = read_json(&path) else {
        return String::new();
    };
    let rows = state
        .get("visible_feedback_items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    for row in rows.iter().take(max_items.max(1)) {
        let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 180);
        if title.is_empty() {
            continue;
        }
        let related_agent = clean_text(
            row.get("related_agent_id").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let severity = clean_text(row.get("severity").and_then(Value::as_str).unwrap_or(""), 40);
        let issue_class = clean_text(
            row.get("issue_class").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let expected_fix = clean_text(
            row.get("expected_fix").and_then(Value::as_str).unwrap_or(""),
            260,
        );
        let suggested_test = clean_text(
            row.get("suggested_test").and_then(Value::as_str).unwrap_or(""),
            180,
        );
        let relationship = if related_agent == id {
            "self".to_string()
        } else {
            format!("child:{related_agent}")
        };
        let mut line = format!("- [{severity}] {title} ({relationship}; class: {issue_class})");
        if !expected_fix.is_empty() {
            line.push_str(&format!(" Fix target: {expected_fix}"));
        }
        if !suggested_test.is_empty() {
            line.push_str(&format!(" Regression: {suggested_test}"));
        }
        lines.push(clean_text(&line, 700));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "Scoped eval feedback attention (visible only for this agent and descendants):\n{}",
        lines.join("\n")
    )
}

fn live_eval_monitor_enabled(root: &Path) -> bool {
    if let Ok(raw) = std::env::var("INFRING_LIVE_EVAL_MONITOR") {
        let v = raw.trim().to_ascii_lowercase();
        if matches!(v.as_str(), "0" | "false" | "off" | "disabled") {
            return false;
        }
        if matches!(v.as_str(), "1" | "true" | "on" | "enabled") {
            return true;
        }
    }
    for rel in [
        "local/state/ops/eval_live_monitor/config.json",
        "core/layer0/ops/config/live_eval_monitor_policy.json",
    ] {
        if let Some(config) = read_json(&root.join(rel)) {
            if let Some(enabled) = config.get("enabled").and_then(Value::as_bool) {
                return enabled;
            }
        }
    }
    true
}

fn live_eval_issue_event(
    agent_id: &str,
    issue_class: &str,
    severity: &str,
    summary: &str,
    message: &str,
    response: &str,
) -> Value {
    let issue_id = format!("{issue_class}_live");
    let seed = json!({"agent_id": agent_id, "issue": issue_id, "message": message, "response": response});
    json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "live_eval_turn_issue",
        "severity": severity,
        "summary": clean_text(summary, 260),
        "attention_key": format!("agent:{agent_id}:live_eval:{issue_class}:{}", crate::deterministic_receipt_hash(&seed).chars().take(20).collect::<String>()),
        "raw_event": {
            "agent_id": agent_id,
            "issue_class": issue_class,
            "issue_id": issue_id,
            "user_text": clean_text(message, 700),
            "assistant_text": clean_text(response, 700),
            "expected_fix": "Preserve LLM-authored chat output and record diagnostics without injecting system text."
        }
    })
}

fn live_eval_monitor_turn(
    root: &Path,
    agent_id: &str,
    message: &str,
    response: &str,
    previous_assistant: &str,
    response_finalization: &Value,
) -> Value {
    let id = clean_agent_id(agent_id);
    let enabled = live_eval_monitor_enabled(root);
    if !enabled {
        return json!({
            "ok": true,
            "enabled": false,
            "type": "live_eval_turn_monitor",
            "agent_id": id,
            "generated_at": crate::now_iso(),
            "issue_count": 0,
            "issues": [],
            "stream_path": "local/state/ops/eval_live_monitor/events.jsonl",
            "queue_receipts": [],
            "chat_injection_allowed": false
        });
    }
    let final_text = clean_text(response, 2_400);
    let prev_sig = normalize_placeholder_signature(previous_assistant);
    let final_sig = normalize_placeholder_signature(&final_text);
    let final_ack_only = response_finalization
        .get("final_ack_only")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| response_looks_like_tool_ack_without_findings(&final_text));
    let system_fallback = response_finalization
        .get("workflow_system_fallback_used")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let route_failure_code = clean_text(
        response_finalization
            .pointer("/route_failure/error_code")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let mut events = Vec::<Value>::new();
    if final_text.is_empty() && !route_failure_code.is_empty() {
        events.push(live_eval_issue_event(&id, "message_route_error", "warn", &format!("Live eval saw a structured message route error: {route_failure_code}."), message, response));
    } else if final_text.is_empty() {
        events.push(live_eval_issue_event(&id, "no_response", "high", "Live eval saw an empty finalized assistant response.", message, response));
    } else if !prev_sig.is_empty() && prev_sig == final_sig {
        events.push(live_eval_issue_event(&id, "repeated_response_loop", "high", "Live eval saw a repeated assistant response.", message, response));
    } else if final_ack_only {
        events.push(live_eval_issue_event(&id, "ack_only_final_response", "warn", "Live eval saw an ack-only final response.", message, response));
    }
    if visible_response_looks_like_internal_deliberation(&final_text) {
        events.push(live_eval_issue_event(
            &id,
            "visible_internal_deliberation",
            "high",
            "Live eval saw internal deliberation exposed in the visible assistant response.",
            message,
            response,
        ));
    }
    if visible_response_looks_like_json_response_wrapper(&final_text) {
        events.push(live_eval_issue_event(
            &id,
            "visible_json_response_wrapper",
            "warn",
            "Live eval saw a JSON response wrapper exposed in the visible assistant response.",
            message,
            response,
        ));
    }
    if system_fallback {
        events.push(live_eval_issue_event(&id, "system_fallback_in_chat", "high", "Live eval saw system fallback usage in finalization.", message, response));
    }
    let stream_path = root.join("local/state/ops/eval_live_monitor/events.jsonl");
    let latest_path = root.join("local/state/ops/eval_live_monitor/latest.json");
    let mut queue_receipts = Vec::<Value>::new();
    for event in &events {
        append_jsonl_row(&stream_path, event);
        append_jsonl_row(
            &root.join("local/state/ops/eval_agent_feedback").join(format!("{id}.attention.jsonl")),
            event,
        );
        queue_receipts.push(enqueue_attention_event_best_effort(root, "dashboard_live_eval_monitor", event));
    }
    let latest = json!({
        "ok": true,
        "enabled": true,
        "type": "live_eval_turn_monitor",
        "agent_id": id,
        "generated_at": crate::now_iso(),
        "issue_count": events.len(),
        "issues": events,
        "stream_path": "local/state/ops/eval_live_monitor/events.jsonl",
        "queue_receipts": queue_receipts,
        "chat_injection_allowed": false
    });
    write_json_pretty(&latest_path, &latest);
    latest
}

fn visible_response_looks_like_internal_deliberation(response: &str) -> bool {
    let lowered = response.to_ascii_lowercase();
    let markers = [
        "i'm trying to craft a response",
        "i am trying to craft a response",
        "i need to ensure my answer",
        "given the original question",
        "considering the constraints",
        "my approach should be",
        "first, i recognize the importance",
        "to do this, i need to",
        "i should provide a",
    ];
    let marker_hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    marker_hits >= 2 || (marker_hits >= 1 && response.split_whitespace().count() > 80)
}

fn visible_response_looks_like_json_response_wrapper(response: &str) -> bool {
    normalize_response_field_json_wrapper(response).is_some()
}

fn strip_redundant_key_findings_prefix(raw: &str) -> String {
    let mut cleaned = clean_text(raw, 2_400);
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        if lowered.starts_with("key findings:") {
            cleaned = clean_text(cleaned["key findings:".len()..].trim(), 2_400);
            continue;
        }
        break;
    }
    cleaned
}

fn rewrite_workspace_analyze_result_for_user_summary(raw_result: &str) -> Option<String> {
    let payload = parse_json_payload_dump(&strip_redundant_key_findings_prefix(raw_result))?;
    let stdout = clean_text(payload.get("stdout").and_then(Value::as_str).unwrap_or(""), 2_000);
    let stderr = clean_text(payload.get("stderr").and_then(Value::as_str).unwrap_or(""), 800);
    let tool_summary = clean_text(
        payload.get("tool_summary").and_then(Value::as_str).unwrap_or(""),
        400,
    );
    let stdout_lines = stdout
        .lines()
        .map(|line| clean_text(line, 220))
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>();
    if !stdout_lines.is_empty() {
        return Some(trim_text(
            &format!("Local workspace evidence: {}", stdout_lines.join(" | ")),
            420,
        ));
    }
    if !tool_summary.is_empty() {
        return Some(trim_text(
            &format!("Local workspace evidence: {tool_summary}"),
            420,
        ));
    }
    if !stderr.is_empty() {
        return Some(trim_text(
            &format!(
                "Workspace analysis returned diagnostics: {}",
                first_sentence(&stderr, 220)
            ),
            420,
        ));
    }
    None
}

fn compact_web_finding_line(row: &Value) -> Option<String> {
    match row {
        Value::String(raw) => {
            let text = first_sentence(&clean_text(raw, 240), 200);
            if text.is_empty() { None } else { Some(text) }
        }
        Value::Object(obj) => {
            let title = clean_text(
                obj.get("title")
                    .or_else(|| obj.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let snippet = clean_text(
                obj.get("snippet")
                    .or_else(|| obj.get("summary"))
                    .or_else(|| obj.get("text"))
                    .or_else(|| obj.get("content"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                200,
            );
            let url = clean_text(
                obj.get("url")
                    .or_else(|| obj.get("link"))
                    .or_else(|| obj.get("source"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            let mut line = if !title.is_empty() && !snippet.is_empty() {
                format!("{title}: {snippet}")
            } else if !snippet.is_empty() {
                snippet
            } else if !title.is_empty() {
                title
            } else {
                clean_text(obj.get("message").and_then(Value::as_str).unwrap_or(""), 220)
            };
            if line.is_empty() {
                return None;
            }
            if !url.is_empty() && !line.contains("http://") && !line.contains("https://") {
                line = format!("{line} ({url})");
            }
            Some(trim_text(&line, 220))
        }
        _ => None,
    }
}

fn rewrite_web_tool_result_from_json_payload(raw_result: &str) -> Option<String> {
    let payload = parse_json_payload_dump(&strip_redundant_key_findings_prefix(raw_result))?;
    let key_findings = clean_text(
        payload
            .get("key_findings")
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    if !key_findings.is_empty() {
        return Some(trim_text(&format!("Web findings: {key_findings}"), 420));
    }
    let summary = clean_text(
        payload
            .get("summary")
            .or_else(|| payload.get("tool_summary"))
            .or_else(|| payload.get("message"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    if !summary.is_empty() {
        return Some(trim_text(&format!("Web findings: {summary}"), 420));
    }

    let findings = payload
        .get("key_findings")
        .or_else(|| payload.get("results"))
        .or_else(|| payload.get("items"))
        .or_else(|| payload.get("snippets"))
        .or_else(|| payload.get("sources"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(compact_web_finding_line)
                .take(3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !findings.is_empty() {
        return Some(trim_text(
            &format!("Web findings: {}", findings.join(" | ")),
            420,
        ));
    }
    None
}

fn rewrite_tool_result_for_user_summary(tool_name: &str, raw_result: &str) -> Option<String> {
    let cleaned = clean_text(raw_result, 2_400);
    if cleaned.is_empty() {
        return None;
    }
    let cleaned_without_prefix = strip_redundant_key_findings_prefix(&cleaned);
    let lowered = cleaned.to_ascii_lowercase();
    let lowered_without_prefix = cleaned_without_prefix.to_ascii_lowercase();
    if lowered.contains("search returned no useful comparison findings") {
        let base = cleaned_without_prefix
            .trim_end_matches(|ch| matches!(ch, '.' | '!' | '?'))
            .trim()
            .to_string();
        return Some(trim_text(
            &format!(
                "{}; this is a retrieval-quality miss, not proof that the systems are equivalent.",
                base
            ),
            420,
        ));
    }
    if response_is_actionable_tool_diagnostic(&cleaned) {
        return Some(trim_text(&cleaned_without_prefix, 420));
    }
    let normalized = normalize_tool_name(tool_name);
    if normalized == "workspace_analyze" {
        if let Some(rewritten) = rewrite_workspace_analyze_result_for_user_summary(raw_result) {
            return Some(rewritten);
        }
        if cleaned_without_prefix != cleaned {
            return Some(trim_text(&cleaned_without_prefix, 420));
        }
    }
    let is_web_tool = matches!(
        normalized.as_str(),
        "batch_query"
            | "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
    );
    if !is_web_tool {
        return None;
    }
    if response_mentions_context_guard(&cleaned) {
        return Some(web_tool_context_guard_fallback("Live web retrieval"));
    }
    if let Some(rewritten) = rewrite_web_tool_result_from_json_payload(&cleaned_without_prefix) {
        return Some(rewritten);
    }
    if let Some((rewritten, _)) =
        crate::tool_output_match_filter::rewrite_unsynthesized_web_dump(&cleaned_without_prefix)
    {
        return Some(trim_text(&rewritten, 420));
    }
    if response_looks_like_raw_web_artifact_dump(&cleaned) {
        return Some(
            "I only have raw web output (placeholder or page/search chrome), not synthesized findings yet. I can rerun with `batch_query` or a narrower query and return a concise answer with sources."
                .to_string(),
        );
    }
    if response_contains_tool_telemetry_dump(&cleaned) {
        return Some(
            "The tool emitted internal telemetry instead of a user-facing answer. I can retry the retrieval or diagnose the failing lane."
                .to_string(),
        );
    }
    if lowered_without_prefix.contains("search returned no useful information")
        || response_is_no_findings_placeholder(&cleaned)
    {
        return Some(
            "Web retrieval ran, but this turn still came back without usable findings. That is a retrieval/synthesis miss, not a silent success. Retry with a narrower query or one specific source URL."
                .to_string(),
        );
    }
    None
}
