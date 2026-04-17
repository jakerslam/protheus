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

fn response_tool_receipt_id(row: &Value) -> String {
    clean_text(
        row.pointer("/tool_attempt_receipt/receipt_hash")
            .or_else(|| row.pointer("/tool_attempt_receipt/receipt_id"))
            .or_else(|| row.pointer("/tool_attempt_receipt/id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
}

fn response_fails_base_final_answer_contract(text: &str) -> bool {
    let cleaned = clean_text(text, 32_000);
    cleaned.trim().is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
}

fn response_workflow_quality_value<'a>(workflow: &'a Value, key: &str) -> Option<&'a Value> {
    workflow
        .get("quality_telemetry")
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get(key))
}

fn response_workflow_quality_rate(workflow: &Value, key: &str) -> f64 {
    match response_workflow_quality_value(workflow, key) {
        Some(Value::Number(number)) => number
            .as_f64()
            .or_else(|| number.as_u64().map(|value| value as f64))
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

fn response_workflow_quality_count(workflow: &Value, key: &str) -> u64 {
    match response_workflow_quality_value(workflow, key) {
        Some(Value::Number(number)) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn append_failure_status_line_if_missing(
    response_text: String,
    turn_classification: &str,
    failure_code: &str,
    status_label: &str,
) -> (String, bool) {
    if failure_code.is_empty() {
        return (response_text, false);
    }
    if response_text
        .to_ascii_lowercase()
        .contains(&failure_code.to_ascii_lowercase())
    {
        return (response_text, false);
    }
    (
        trim_text(
            &format!(
                "{}\n\n{}: {}\nerror_code: {}",
                response_text, status_label, turn_classification, failure_code
            ),
            32_000,
        ),
        true,
    )
}

fn build_deterministic_final_fallback_response(
    response_tools: &[Value],
    web_intent_detected: bool,
    web_turn_classification: &str,
    web_failure_code: &str,
    tooling_attempted: bool,
    tooling_turn_classification: &str,
    tooling_failure_code: &str,
    inline_tools_allowed: bool,
) -> String {
    let status_failure_fallback =
        |lane: &str, classification: &str, failure_code: &str, next_step: &str| {
            format!(
                "{} did not produce a usable final answer in this turn. {}: {}. error_code: {}. Next step: {}.",
                lane,
                if lane.eq_ignore_ascii_case("web retrieval") {
                    "web_status"
                } else {
                    "tool_status"
                },
                classification,
                failure_code,
                next_step
            )
        };
    let mut deterministic_fallback =
        clean_text(&response_tools_failure_reason_for_user(response_tools, 4), 4_000);
    if deterministic_fallback.is_empty() {
        deterministic_fallback = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    }
    if deterministic_fallback.is_empty() && web_intent_detected {
        let stable_error = if web_failure_code.is_empty() {
            "web_tool_error".to_string()
        } else {
            web_failure_code.to_string()
        };
        deterministic_fallback = status_failure_fallback(
            "Web retrieval",
            web_turn_classification,
            &stable_error,
            "retry with a narrower query or provide one trusted source URL",
        );
    }
    if deterministic_fallback.is_empty() && tooling_attempted && !tooling_failure_code.is_empty() {
        deterministic_fallback = status_failure_fallback(
            "Tool execution",
            tooling_turn_classification,
            tooling_failure_code,
            "run one targeted tool call with explicit scope",
        );
    }
    if deterministic_fallback.is_empty() && response_tools.is_empty() && !inline_tools_allowed {
        deterministic_fallback =
            "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
    }
    if deterministic_fallback.is_empty() {
        deterministic_fallback = "I completed the workflow, but synthesis could not produce a valid final response in this turn. Please retry and I’ll rerun the chain with explicit failure details.".to_string();
    }
    clean_chat_text(&deterministic_fallback, 32_000)
}

fn build_response_finalization_payload(
    finalization_outcome: &str,
    initial_ack_only: bool,
    final_ack_only: bool,
    tool_completion: &Value,
    tooling_fallback_used: bool,
    comparative_fallback_used: bool,
    workflow_system_fallback_used: bool,
    visible_response_repaired: bool,
    response_quality_telemetry: &Value,
    tooling_invariant: &Value,
    web_invariant: &Value,
) -> Value {
    json!({
        "applied": finalization_outcome != "unchanged",
        "outcome": finalization_outcome,
        "initial_ack_only": initial_ack_only,
        "final_ack_only": final_ack_only,
        "findings_available": tool_completion
            .get("findings_available")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "tool_completion": tool_completion,
        "final_answer_contract": tool_completion
            .get("final_answer_contract")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "retry_attempted": false,
        "retry_used": false,
        "tool_synthesis_retry_used": false,
        "synthesis_retry_used": false,
        "tooling_fallback_used": tooling_fallback_used,
        "comparative_fallback_used": comparative_fallback_used,
        "workflow_system_fallback_used": workflow_system_fallback_used,
        "visible_response_repaired": visible_response_repaired,
        "response_quality_telemetry": response_quality_telemetry,
        "tooling_invariant": tooling_invariant,
        "web_invariant": web_invariant
    })
}

fn build_response_quality_telemetry_payload(
    response_workflow: &Value,
    final_fallback_used: bool,
    tooling_invariant_repair_used: bool,
    tooling_failure_code: &str,
    direct_answer_rate: f64,
    retry_rate: f64,
    tool_overcall_rate: f64,
    off_topic_reject_rate: f64,
) -> Value {
    json!({
        "off_topic_reject": response_workflow_quality_count(response_workflow, "off_topic_reject"),
        "deferred_reply_reject": response_workflow_quality_count(response_workflow, "deferred_reply_reject"),
        "alignment_reject": response_workflow_quality_count(response_workflow, "alignment_reject"),
        "prompt_echo_reject": response_workflow_quality_count(response_workflow, "prompt_echo_reject"),
        "unsourced_claim_reject": response_workflow_quality_count(response_workflow, "unsourced_claim_reject"),
        "direct_answer_reject": response_workflow_quality_count(response_workflow, "direct_answer_reject"),
        "meta_control_tool_block": response_workflow_quality_value(response_workflow, "meta_control_tool_block")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "final_fallback_used": final_fallback_used,
        "tooling_contract_repair_used": tooling_invariant_repair_used,
        "tooling_failure_code_present": !tooling_failure_code.is_empty(),
        "direct_answer_rate": direct_answer_rate,
        "retry_rate": retry_rate,
        "tool_overcall_rate": tool_overcall_rate,
        "off_topic_reject_rate": off_topic_reject_rate
    })
}

fn claim_source_tags_for_report(response_tools: &[Value], max_items: usize) -> Vec<String> {
    let mut tags = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for row in response_tools.iter().take(max_items.clamp(1, 8)) {
        let receipt = response_tool_receipt_id(row);
        if receipt.is_empty() {
            continue;
        }
        let tag = format!("tool_receipt:{receipt}");
        if seen.insert(tag.clone()) {
            tags.push(tag);
        }
    }
    tags
}

fn enforce_user_facing_finalization_contract(
    user_message: &str,
    output: String,
    response_tools: &[Value],
) -> (String, Value, String) {
    let findings = response_tools_summary_for_user(response_tools, 4);
    let findings = if findings.is_empty() { None } else { Some(findings) };
    let failure_reason = response_tools_failure_reason_for_user(response_tools, 4);
    let (mut prefinalized, mut pre_outcome, _) = finalize_user_facing_response_with_outcome(output, findings);
    let prefinalized_cleaned = clean_text(&prefinalized, 32_000);
    if !failure_reason.is_empty()
        && (prefinalized_cleaned.is_empty()
            || response_looks_like_tool_ack_without_findings(&prefinalized_cleaned)
            || response_is_no_findings_placeholder(&prefinalized_cleaned))
    {
        prefinalized = failure_reason.clone();
        pre_outcome = merge_response_outcomes(&pre_outcome, "replaced_no_findings_with_tool_failure_reason", 220);
    }
    let (mut finalized, mut report) = enforce_tool_completion_contract(prefinalized, response_tools);
    if !failure_reason.is_empty()
        && report.get("completion_state").and_then(Value::as_str) == Some("reported_no_findings")
    {
        finalized = failure_reason;
        if let Some(obj) = report.as_object_mut() {
            obj.insert("completion_state".to_string(), Value::String("reported_reason".to_string()));
            obj.insert("final_ack_only".to_string(), Value::Bool(false));
            obj.insert("final_no_findings".to_string(), Value::Bool(false));
            obj.insert("reasoning".to_string(), Value::String(first_sentence(&finalized, 220)));
        }
    }
    let deferred_execution = response_is_deferred_execution_preamble(&finalized)
        || response_is_deferred_retry_prompt(&finalized);
    if deferred_execution
        && report
            .get("completion_state")
            .and_then(Value::as_str)
            != Some("reported_findings")
    {
        finalized = no_findings_user_facing_response();
        if let Some(obj) = report.as_object_mut() {
            obj.insert(
                "completion_state".to_string(),
                Value::String("reported_no_findings".to_string()),
            );
            obj.insert("final_ack_only".to_string(), Value::Bool(false));
            obj.insert("final_no_findings".to_string(), Value::Bool(true));
            obj.insert("final_deferred_execution".to_string(), Value::Bool(false));
            obj.insert(
                "reasoning".to_string(),
                Value::String(first_sentence(&finalized, 220)),
            );
            let prior = clean_text(obj.get("outcome").and_then(Value::as_str).unwrap_or(""), 200);
            obj.insert(
                "outcome".to_string(),
                Value::String(append_tool_completion_outcome(
                    &prior,
                    "tool_completion_replaced_deferred_execution",
                )),
            );
        }
    }
    let claim_sources = if response_tools.is_empty() {
        vec!["local_context".to_string()]
    } else {
        claim_source_tags_for_report(response_tools, 4)
    };
    let final_answer_contract = json!({
        "contract": "final_answer_contract_v1",
        "direct_answer_required": true,
        "direct_answer_in_first_two_sentences": response_answers_user_early(user_message, &finalized),
        "no_prompt_echo": !response_prompt_echo_detected(user_message, &finalized),
        "no_placeholder_copy": !response_is_no_findings_placeholder(&finalized)
            && !response_looks_like_tool_ack_without_findings(&finalized),
        "no_unsourced_claims": !claim_sources.is_empty() || response_has_evidence_tags(&finalized),
        "claim_sources": claim_sources
    });
    if let Some(obj) = report.as_object_mut() {
        obj.insert("final_answer_contract".to_string(), final_answer_contract);
    }
    let contract_outcome = clean_text(report.get("outcome").and_then(Value::as_str).unwrap_or("unchanged"), 200);
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

fn response_tool_summary_text_is_rejected(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("model attempted this call as text")
        || response_looks_like_tool_ack_without_findings(text)
        || response_is_no_findings_placeholder(text)
        || response_looks_like_unsynthesized_web_snippet_dump(text)
        || response_looks_like_raw_web_artifact_dump(text)
        || response_contains_tool_telemetry_dump(text)
        || looks_like_search_engine_chrome_summary(&lowered)
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
        if response_tool_summary_text_is_rejected(&raw_result) {
            continue;
        }
        let user_result =
            rewrite_tool_result_for_user_summary(&name, &raw_result).unwrap_or(raw_result);
        if response_tool_summary_text_is_rejected(&user_result) {
            continue;
        }
        let snippet = if user_result.starts_with("Key findings:") {
            trim_text(&strip_redundant_key_findings_prefix(&user_result), 220)
        } else {
            first_sentence(&user_result, 220)
        };
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

fn append_turn_receipt_with_metadata(
    root: &Path,
    agent_id: &str,
    message: &str,
    finalized_response: &str,
    tools_payload: Value,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    turn_transaction: &Value,
    terminal_transcript: &[Value],
) -> Value {
    let mut turn_receipt = append_turn_message(root, agent_id, message, finalized_response);
    turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
        root,
        agent_id,
        finalized_response,
        &json!({
            "tools": tools_payload,
            "response_workflow": response_workflow.clone(),
            "response_finalization": response_finalization.clone(),
            "process_summary": process_summary.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.to_vec()
        }),
    );
    turn_receipt["process_summary"] = process_summary.clone();
    turn_receipt["response_finalization"] = response_finalization.clone();
    turn_receipt
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
