fn merge_response_outcomes(primary: &str, secondary: &str, max_len: usize) -> String {
    let left = clean_text(primary, max_len.max(1));
    let right = clean_text(secondary, max_len.max(1));
    if left.is_empty() || left == "unchanged" {
        return if right.is_empty() {
            "unchanged".to_string()
        } else {
            right
        };
    }
    if right.is_empty() || right == "unchanged" {
        return left;
    }
    if left == right {
        return left;
    }
    clean_text(&format!("{left}+{right}"), max_len.max(1))
}

fn enforce_user_facing_finalization_contract(
    output: String,
    response_tools: &[Value],
) -> (String, Value, String) {
    let findings = response_tools_summary_for_user(response_tools, 4);
    let findings = if findings.is_empty() {
        None
    } else {
        Some(findings)
    };
    let (prefinalized, pre_outcome, _) =
        finalize_user_facing_response_with_outcome(output, findings);
    let (finalized, report) = enforce_tool_completion_contract(prefinalized, response_tools);
    let contract_outcome = clean_text(
        report
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or("unchanged"),
        200,
    );
    let merged_outcome = merge_response_outcomes(&pre_outcome, &contract_outcome, 220);
    (finalized, report, merged_outcome)
}

fn available_model_count(root: &Path, snapshot: &Value) -> usize {
    crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("available")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn no_models_available_payload(agent_id: &str) -> Value {
    json!({
        "ok": false,
        "error": "no_models_available",
        "error_code": "no_models_available",
        "agent_id": clean_agent_id(agent_id),
        "hint": "No usable LLMs are available yet. Install Ollama or add an API key.",
        "setup": {
            "steps": [
                "Install Ollama: https://ollama.com/download",
                "Start Ollama: ollama serve",
                "Pull at least one model: ollama pull qwen2.5:3b-instruct",
                "Or add API keys in Settings or via /apikey <key>"
            ]
        },
        "links": [
            {"label": "Ollama Download", "url": "https://ollama.com/download"},
            {"label": "Ollama Library", "url": "https://ollama.com/library"},
            {"label": "OpenRouter Keys", "url": "https://openrouter.ai/keys"},
            {"label": "OpenAI API Keys", "url": "https://platform.openai.com/api-keys"},
            {"label": "Anthropic API Keys", "url": "https://console.anthropic.com/settings/keys"},
            {"label": "Google AI Studio Keys", "url": "https://aistudio.google.com/app/apikey"}
        ]
    })
}

fn response_tools_summary_for_user(response_tools: &[Value], max_items: usize) -> String {
    let limit = max_items.clamp(1, 8);
    let mut lines = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools {
        let name = clean_text(
            tool.get("name").and_then(Value::as_str).unwrap_or("tool"),
            80,
        )
        .to_ascii_lowercase();
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        if tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let raw_result = clean_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            2_000,
        );
        if raw_result.is_empty() {
            continue;
        }
        let lowered = raw_result.to_ascii_lowercase();
        if lowered.contains("model attempted this call as text") {
            continue;
        }
        if response_looks_like_tool_ack_without_findings(&raw_result) {
            continue;
        }
        if response_looks_like_unsynthesized_web_snippet_dump(&raw_result)
            || response_looks_like_raw_web_artifact_dump(&raw_result)
            || response_contains_tool_telemetry_dump(&raw_result)
        {
            continue;
        }
        if looks_like_search_engine_chrome_summary(&lowered) {
            continue;
        }
        let snippet = first_sentence(&raw_result, 220);
        if snippet.is_empty() {
            continue;
        }
        let pretty_name = name.replace('_', " ");
        let line = format!("- {}: {}", clean_text(&pretty_name, 60), snippet);
        let key = line.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        lines.push(line);
        if lines.len() >= limit {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!("Here's what I found:\n{}", lines.join("\n")),
        32_000,
    )
}

fn parse_tool_input_payload(raw_input: &str) -> Value {
    let cleaned = clean_text(raw_input, 12_000);
    if cleaned.is_empty() {
        return Value::Null;
    }
    serde_json::from_str::<Value>(&cleaned).unwrap_or_else(|_| Value::String(cleaned))
}

fn tool_payload_count(payload: &Value, keys: &[&str]) -> usize {
    for key in keys {
        let Some(value) = payload.get(*key) else {
            continue;
        };
        match value {
            Value::Array(rows) => {
                if !rows.is_empty() {
                    return rows.len().min(99);
                }
            }
            Value::Number(number) => {
                if let Some(raw) = number.as_u64() {
                    let bounded = raw.min(99) as usize;
                    if bounded > 0 {
                        return bounded;
                    }
                }
            }
            Value::String(text) => {
                if !text.trim().is_empty() {
                    return 1;
                }
            }
            Value::Object(map) => {
                if !map.is_empty() {
                    return 1;
                }
            }
            Value::Bool(flag) => {
                if *flag {
                    return 1;
                }
            }
            _ => {}
        }
    }
    0
}

fn tool_completion_status_for_tool(tool_name: &str, tool_input: &str) -> String {
    let normalized = normalize_tool_name(tool_name);
    if normalized == "thought_process" {
        return "Thinking".to_string();
    }
    let payload = parse_tool_input_payload(tool_input);
    let status = match normalized.as_str() {
        "batch_query" | "web_search" | "search_web" | "search" | "web_query" => {
            "Searching internet".to_string()
        }
        "web_fetch" | "browse" | "web_conduit_fetch" => "Reading web pages".to_string(),
        "file_read" | "read_file" | "file" => {
            let count = tool_payload_count(
                &payload,
                &["paths", "files", "file_paths", "targets", "path", "file"],
            );
            if count > 1 {
                format!("Scanning {count} files")
            } else if count == 1 {
                "Scanning 1 file".to_string()
            } else {
                "Scanning files".to_string()
            }
        }
        "file_read_many" => {
            let count = tool_payload_count(&payload, &["paths", "files", "file_paths", "targets"]);
            if count > 1 {
                format!("Scanning {count} files")
            } else if count == 1 {
                "Scanning 1 file".to_string()
            } else {
                "Scanning files".to_string()
            }
        }
        "folder_export" | "list_folder" | "folder_tree" | "folder" => {
            let count =
                tool_payload_count(&payload, &["folders", "paths", "targets", "path", "folder"]);
            if count > 1 {
                format!("Scanning {count} folders")
            } else if count == 1 {
                "Scanning 1 folder".to_string()
            } else {
                "Scanning folders".to_string()
            }
        }
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => {
            "Running terminal command".to_string()
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let count =
                tool_payload_count(&payload, &["count", "agent_count", "num_agents", "agents"]);
            if count > 0 {
                format!("Summoning {count} agents")
            } else {
                "Summoning agents".to_string()
            }
        }
        "memory_semantic_query" => "Searching memory".to_string(),
        "cron_schedule" => "Scheduling follow-up work".to_string(),
        "cron_run" => "Running scheduled work".to_string(),
        "cron_list" => "Checking schedules".to_string(),
        "session_rollback_last_turn" => "Rewinding the last turn".to_string(),
        _ => {
            let cleaned = normalized.replace('_', " ");
            if cleaned.is_empty() {
                "Running tool".to_string()
            } else {
                format!("Running {cleaned}")
            }
        }
    };
    clean_text(&status, 180)
}

fn tool_completion_live_steps(response_tools: &[Value]) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for tool in response_tools {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        let input = clean_text(
            tool.get("input").and_then(Value::as_str).unwrap_or(""),
            12_000,
        );
        let status = tool_completion_status_for_tool(&name, &input);
        if status.is_empty() {
            continue;
        }
        out.push(json!({
            "tool": name,
            "status": status,
            "is_error": tool
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        }));
        if out.len() >= 16 {
            break;
        }
    }
    out
}

fn tool_terminal_transcript(response_tools: &[Value]) -> Vec<Value> {
    let mut rows = Vec::<Value>::new();
    for tool in response_tools {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or(""));
        if !is_terminal_tool_name(&name) {
            continue;
        }
        let parsed_input =
            serde_json::from_str::<Value>(tool.get("input").and_then(Value::as_str).unwrap_or(""))
                .unwrap_or_else(|_| json!({}));
        let command = clean_text(
            parsed_input
                .get("command")
                .or_else(|| parsed_input.get("cmd"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            12_000,
        );
        let output = trim_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            24_000,
        );
        let cwd = clean_text(
            parsed_input
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or(""),
            4_000,
        );
        let is_error = tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if command.is_empty() && output.trim().is_empty() {
            continue;
        }
        rows.push(json!({
            "tool": name,
            "command": command,
            "output": output,
            "cwd": cwd,
            "is_error": is_error
        }));
    }
    rows
}

fn enrich_tool_completion_receipt(tool_completion: Value, response_tools: &[Value]) -> Value {
    let mut enriched = if tool_completion.is_object() {
        tool_completion
    } else {
        json!({})
    };
    let steps = tool_completion_live_steps(response_tools);
    let tool_attempts = response_tools
        .iter()
        .filter_map(|row| {
            row.get("tool_attempt_receipt")
                .cloned()
                .or_else(|| row.pointer("/tool_attempt/attempt").cloned())
        })
        .take(16)
        .collect::<Vec<_>>();
    let live_tool_status = steps
        .first()
        .and_then(|row| row.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    enriched["live_tool_status"] = json!(clean_text(&live_tool_status, 180));
    enriched["live_tool_steps"] = Value::Array(steps);
    enriched["tool_attempts"] = Value::Array(tool_attempts);
    enriched["live_status_source"] = json!("tool_completion_receipt_v1");
    enriched
}

#[cfg(test)]
mod tool_completion_live_status_tests {
    use super::*;

    #[test]
    fn builds_live_status_for_known_tools() {
        let tools = vec![json!({
            "name": "web_search",
            "input": "{\"query\":\"latest stack\"}",
            "result": "ok",
            "is_error": false
        })];
        let enriched =
            enrich_tool_completion_receipt(json!({"completion_state":"reported_findings"}), &tools);
        assert_eq!(
            enriched
                .get("live_tool_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "Searching internet"
        );
        let steps = enriched
            .get("live_tool_steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn skips_thought_process_for_live_status() {
        let tools = vec![json!({
            "name": "thought_process",
            "input": "Thinking about next step.",
            "result": "",
            "is_error": false
        })];
        let enriched =
            enrich_tool_completion_receipt(json!({"completion_state":"reported_reason"}), &tools);
        let steps = enriched
            .get("live_tool_steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(steps.is_empty());
        assert_eq!(
            enriched
                .get("live_tool_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            ""
        );
    }

    #[test]
    fn builds_terminal_transcript_rows_from_terminal_tools() {
        let rows = tool_terminal_transcript(&[json!({
            "name": "terminal_exec",
            "input": "{\"command\":\"printf 'ok'\",\"cwd\":\"/tmp\"}",
            "result": "ok",
            "is_error": false
        })]);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("command").and_then(Value::as_str),
            Some("printf 'ok'")
        );
        assert_eq!(rows[0].get("output").and_then(Value::as_str), Some("ok"));
        assert_eq!(rows[0].get("cwd").and_then(Value::as_str), Some("/tmp"));
    }

    #[test]
    fn carries_tool_attempt_receipts_into_tool_completion() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "terminal_exec",
                "input": "{\"command\":\"ls\"}",
                "result": "permission denied",
                "is_error": true,
                "tool_attempt_receipt": {
                    "tool_name": "terminal_exec",
                    "status": "blocked",
                    "outcome": "blocked",
                    "reason_code": "caller_not_authorized",
                    "reason": "caller_not_authorized",
                    "backend": "governed_terminal",
                    "required_args": ["command"],
                    "discoverable": true
                }
            })],
        );
        assert_eq!(
            enriched
                .get("tool_attempts")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }
}
