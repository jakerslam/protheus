fn user_message_allows_code_context(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "show me the code",
        "source code",
        "code snippet",
        "read file",
        "open file",
        "inspect file",
        "file content",
        "patch",
        "diff",
        "implementation",
        "function",
        "class",
        "php",
        "laravel",
        "typescript",
        "javascript",
        "rust",
        "python",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle))
}

fn response_code_context_marker_count(response_text: &str) -> usize {
    response_text
        .lines()
        .filter(|line| {
            let lowered = line.trim_start().to_ascii_lowercase();
            lowered.starts_with("<?php")
                || lowered.starts_with("namespace ")
                || lowered.starts_with("use ")
                || lowered.starts_with("import ")
                || lowered.starts_with("from ")
                || lowered.starts_with("export ")
                || lowered.starts_with("def ")
                || lowered.starts_with("class ")
                || lowered.starts_with("public class ")
                || lowered.starts_with("public function ")
                || lowered.starts_with("private function ")
                || lowered.starts_with("protected function ")
                || lowered.starts_with("fn ")
                || lowered.starts_with("impl ")
                || lowered.starts_with("const ")
                || lowered.starts_with("let ")
                || lowered.starts_with("if (")
                || lowered.starts_with("$this->")
                || lowered.starts_with("return ")
        })
        .count()
}

fn response_contains_stale_code_context_dump(user_message: &str, response_text: &str) -> bool {
    let cleaned = clean_text(response_text, 32_000);
    if cleaned.len() < 280 || user_message_allows_code_context(user_message) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let fenced_code = lowered.contains("```php")
        || lowered.contains("```ts")
        || lowered.contains("```js")
        || lowered.contains("```rust")
        || lowered.contains("```python");
    let php_dump = lowered.contains("<?php")
        && (lowered.contains("namespace ") || lowered.contains("extends serviceprovider"));
    let framework_dump = lowered.contains("class ")
        && lowered.contains("public function ")
        && (lowered.contains("namespace ") || lowered.contains("use "));
    php_dump
        || framework_dump
        || (fenced_code && response_code_context_marker_count(&cleaned) >= 2)
        || response_code_context_marker_count(&cleaned) >= 4
}

fn user_message_explicitly_requests_memory_context(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    memory_recall_requested(&lowered)
        || lowered.contains("memory")
        || lowered.contains("context")
        || lowered.contains("earlier")
        || lowered.contains("previous")
        || lowered.contains("what did we")
        || lowered.contains("what was decided")
        || lowered.contains("remember")
        || lowered.contains("recall")
}

fn simple_direct_chat_suppresses_passive_context(message: &str, inline_tools_allowed: bool) -> bool {
    (!inline_tools_allowed || message_explicitly_disallows_tool_calls(message))
        && !user_message_explicitly_requests_memory_context(message)
}

fn response_has_current_turn_tool_evidence(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or(""));
        !name.is_empty() && name != "thought_process"
    })
}

fn response_claims_tool_success_without_current_turn_evidence(
    _user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_has_current_turn_tool_evidence(response_tools) {
        return false;
    }
    let response = clean_chat_text(response_text, 32_000);
    let lowered = response.to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_tool_surface = [
        "tool",
        "web search",
        "workspace",
        "file search",
        "file tooling",
        "searched the files",
        "searched files",
        "read the file",
        "opened the file",
        "inspected the file",
        "scanned the repo",
        "searched the repo",
        "terminal",
        "command",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    let claims_execution = [
        "i searched",
        "i ran",
        "i used",
        "i called",
        "i executed",
        "i opened",
        "i read",
        "i inspected",
        "i scanned",
        "i found",
        "tool ran",
        "tool succeeded",
        "tool completed",
        "search returned",
        "search found",
        "returned no findings",
        "returned no results",
        "found no results",
        "found no findings",
        "returned these",
        "returned the following",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    let claims_empty_results = (lowered.contains("no findings")
        || lowered.contains("no results")
        || lowered.contains("didn't return")
        || lowered.contains("did not return")
        || lowered.contains("limited results"))
        && (lowered.contains("search") || lowered.contains("tool") || lowered.contains("workspace"));
    let claims_listings = [
        "files i found",
        "file i found",
        "found these files",
        "found the following files",
        "workspace results",
        "search results",
        "returned listings",
        "listing",
        "listings",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    let claims_tool_result = (mentions_tool_surface && (claims_execution || claims_empty_results))
        || claims_listings;
    let hypothetical = [
        "i would ",
        "i'd ",
        "i can ",
        "i could ",
        "i should ",
        "would use",
        "would choose",
        "would run",
        "would search",
        "would inspect",
        "would read",
        "next i would",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    if hypothetical && !claims_tool_result {
        return false;
    }
    claims_tool_result
}

fn response_has_gate_choice_prefix_leakage(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 2_000).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return false;
    }
    let starts_with_gate_token = trimmed.starts_with("yes,")
        || trimmed.starts_with("yes.")
        || trimmed.starts_with("yes ")
        || trimmed.starts_with("no,")
        || trimmed.starts_with("no.")
        || trimmed.starts_with("no ");
    let starts_with_directive_leak = trimmed.starts_with("answer directly")
        || trimmed.starts_with("direct answer mode")
        || trimmed.starts_with("direct-answer mode")
        || trimmed.starts_with("direct answer path")
        || trimmed.starts_with("direct-answer path");
    if starts_with_directive_leak {
        return true;
    }
    if !starts_with_gate_token {
        return false;
    }
    let after_token = trimmed
        .strip_prefix("yes,")
        .or_else(|| trimmed.strip_prefix("yes."))
        .or_else(|| trimmed.strip_prefix("yes "))
        .or_else(|| trimmed.strip_prefix("no,"))
        .or_else(|| trimmed.strip_prefix("no."))
        .or_else(|| trimmed.strip_prefix("no "))
        .unwrap_or(trimmed)
        .trim_start();
    [
        "tool family:",
        "tool:",
        "need tools:",
        "use workflow:",
        "selected tool",
        "selected_tool",
        "workflow gate",
        "manual toolbox",
    ]
    .iter()
    .any(|needle| after_token.starts_with(*needle))
        || after_token.contains("request payload:")
}

fn natural_tool_choice_pending_request(response_text: &str, message: &str) -> Option<Value> {
    let lowered = clean_text(response_text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return None;
    }
    let describes_choice = [
        "i would choose",
        "i would use",
        "i'd choose",
        "i'd use",
        "i should choose",
        "i should use",
        "would choose",
        "would use",
        "would run",
        "use the",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    let claims_execution = [
        "i searched",
        "i ran",
        "i used",
        "i called",
        "i executed",
        "returned",
        "found",
        "completed",
        "succeeded",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    if !describes_choice || claims_execution {
        return None;
    }
    let (tool_name, family, label, input) = if lowered.contains("web search")
        || lowered.contains("batch_query")
        || lowered.contains("web_search")
    {
        (
            "batch_query",
            "Web Search / Fetch",
            "Web search",
            json!({"source": "web", "query": clean_text(message, 600), "aperture": "medium"}),
        )
    } else if lowered.contains("workspace_search")
        || lowered.contains("workspace search")
        || lowered.contains("file search")
        || lowered.contains("file tooling")
    {
        (
            "workspace_search",
            "File / Workspace",
            "Search workspace",
            json!({"path": ".", "pattern": clean_text(message, 600)}),
        )
    } else {
        return None;
    };
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "source": "natural_tool_choice",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600)
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "natural_tool_choice",
        "tool_name": tool_name,
        "selected_tool_family": family,
        "selected_tool_label": label,
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn response_contains_unrequested_content_without_tool_evidence(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_has_current_turn_tool_evidence(response_tools)
        || user_message_allows_code_context(user_message)
    {
        return false;
    }
    let cleaned = clean_chat_text(response_text, 32_000);
    if response_contains_short_unrelated_project_title(user_message, &cleaned) {
        return true;
    }
    if cleaned.len() < 160 {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if response_claims_tool_success_without_current_turn_evidence(
        user_message,
        &cleaned,
        response_tools,
    ) {
        return true;
    }
    response_contains_stale_code_context_dump(user_message, &cleaned)
        || response_is_unrelated_context_dump(user_message, &cleaned)
        || response_contains_project_dump_sections(&cleaned)
        || response_contains_tool_telemetry_dump(&cleaned)
        || ((lowered.contains("```") || lowered.contains("<?php"))
            && response_code_context_marker_count(&cleaned) >= 1)
        || response_code_context_marker_count(&cleaned) >= 3
}

fn strict_current_turn_question_shape(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 1_000).to_ascii_lowercase();
    lowered.contains('?')
        || lowered.starts_with("what")
        || lowered.starts_with("why")
        || lowered.starts_with("how")
        || lowered.starts_with("did")
        || lowered.starts_with("can")
        || lowered.starts_with("could")
        || lowered.starts_with("would")
        || lowered.contains("status")
        || lowered.contains("compare")
}

fn current_turn_dominance_payload(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> Value {
    let response = clean_chat_text(response_text, 32_000);
    let message_terms = important_memory_terms(user_message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let response_terms = important_memory_terms(&response, 72)
        .into_iter()
        .collect::<HashSet<_>>();
    let overlap_count = message_terms.intersection(&response_terms).count();
    let no_tool_evidence = !response_has_current_turn_tool_evidence(response_tools);
    let contamination_detected =
        response_contains_unrequested_content_without_tool_evidence(user_message, &response, response_tools);
    let low_overlap = no_tool_evidence
        && response.len() > 180
        && message_terms.len() >= 2
        && !response_terms.is_empty()
        && overlap_count == 0;
    let strict_answer_failure = no_tool_evidence
        && response.len() > 120
        && strict_current_turn_question_shape(user_message)
        && !response_answers_user_early(user_message, &response);
    let violation = !response.is_empty()
        && (contamination_detected
            || response_is_unrelated_context_dump(user_message, &response)
            || low_overlap
            || strict_answer_failure);
    json!({
        "contract": "current_turn_dominance_v1",
        "dominant": !violation,
        "violation": violation,
        "message_term_count": message_terms.len(),
        "response_term_count": response_terms.len(),
        "overlap_count": overlap_count,
        "current_turn_tool_evidence": !no_tool_evidence,
        "contamination_detected": contamination_detected,
        "low_overlap": low_overlap,
        "strict_answer_failure": strict_answer_failure
    })
}

fn response_current_turn_dominance_violation(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    current_turn_dominance_payload(user_message, response_text, response_tools)
        .get("violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn response_guard_bool(report: &Value, key: &str) -> bool {
    report.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn final_response_guard_report(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
    repair_candidate_contamination: bool,
) -> Value {
    let current_turn_dominance =
        current_turn_dominance_payload(user_message, response_text, response_tools);
    let current_turn_dominance_violation =
        response_guard_bool(&current_turn_dominance, "violation");
    let unsupported_content_contamination =
        response_contains_unrequested_content_without_tool_evidence(
            user_message,
            response_text,
            response_tools,
        );
    let unsupported_tool_success_claim =
        response_claims_tool_success_without_current_turn_evidence(
            user_message,
            response_text,
            response_tools,
        );
    let final_contamination_violation = repair_candidate_contamination
        || response_contains_stale_code_context_dump(user_message, response_text)
        || response_is_unrelated_context_dump(user_message, response_text)
        || unsupported_content_contamination;
    let visible_gate_choice_leakage = response_is_visible_workflow_gate_choice(response_text)
        || response_has_gate_choice_prefix_leakage(response_text);
    let final_contract_violation = response_fails_base_final_answer_contract(response_text)
        || (workflow_response_requests_more_tooling(response_text)
            && !response_is_manual_toolbox_gate_choice(response_text))
        || response_contains_unexpected_state_retry_boilerplate(response_text)
        || visible_gate_choice_leakage
        || unsupported_tool_success_claim
        || final_contamination_violation
        || current_turn_dominance_violation;
    json!({
        "current_turn_dominance": current_turn_dominance,
        "current_turn_dominance_violation": current_turn_dominance_violation,
        "visible_gate_choice_leakage": visible_gate_choice_leakage,
        "contamination_guard": {
            "contract": "unrequested_content_without_tool_evidence_v1",
            "detected": final_contamination_violation,
            "unsupported_content_detected": unsupported_content_contamination,
            "unsupported_tool_success_claim": unsupported_tool_success_claim,
            "current_turn_tool_evidence": response_has_current_turn_tool_evidence(response_tools)
        },
        "unsupported_tool_success_claim": unsupported_tool_success_claim,
        "final_contamination_violation": final_contamination_violation,
        "final_contract_violation": final_contract_violation
    })
}

fn final_response_guard_outcome(report: &Value) -> &'static str {
    if response_guard_bool(report, "unsupported_tool_success_claim") {
        "unsupported_tool_success_claim_withheld"
    } else if response_guard_bool(report, "final_contamination_violation") {
        "visible_response_contamination_withheld"
    } else if response_guard_bool(report, "current_turn_dominance_violation") {
        "current_turn_dominance_withheld"
    } else {
        "deterministic_final_fallback_suppressed"
    }
}

fn apply_response_guard_payloads(response_finalization: &mut Value, report: &Value) {
    response_finalization["current_turn_dominance"] = report
        .get("current_turn_dominance")
        .cloned()
        .unwrap_or_else(|| json!({}));
    response_finalization["contamination_guard"] = report
        .get("contamination_guard")
        .cloned()
        .unwrap_or_else(|| json!({}));
}

fn visible_response_source_for_turn(
    response_text: &str,
    workflow_used: bool,
    visible_response_repaired: bool,
    finalization_outcome: &str,
) -> &'static str {
    if clean_text(response_text, 1_000).is_empty() {
        "none"
    } else if workflow_used {
        "llm_final"
    } else if visible_response_repaired
        || finalization_outcome.contains("repaired_with_initial_draft")
        || finalization_outcome.contains("workflow_no_system_fallback")
    {
        "llm_draft"
    } else {
        "llm_draft"
    }
}

fn apply_visible_response_provenance(
    response_workflow: &mut Value,
    response_finalization: &mut Value,
    visible_response_source: &str,
) {
    let source = clean_text(visible_response_source, 80);
    response_workflow["visible_response_source"] = json!(source.clone());
    response_workflow["system_chat_injection_used"] = json!(false);
    response_finalization["visible_response_source"] = json!(source);
    response_finalization["system_chat_injection_used"] = json!(false);
}

fn apply_visible_response_provenance_for_turn(
    response_workflow: &mut Value,
    response_finalization: &mut Value,
    response_text: &str,
    workflow_used: bool,
    visible_response_repaired: bool,
    finalization_outcome: &str,
) -> &'static str {
    let source = visible_response_source_for_turn(
        response_text,
        workflow_used,
        visible_response_repaired,
        finalization_outcome,
    );
    apply_visible_response_provenance(response_workflow, response_finalization, source);
    source
}

fn workflow_visibility_payload(response_workflow: &Value, response_finalization: &Value) -> Value {
    let visibility = response_workflow
        .get("visibility")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let read_visibility = |key: &str, fallback: &str, max_len: usize| -> String {
        let value = visibility
            .get(key)
            .or_else(|| response_workflow.get(key))
            .and_then(Value::as_str)
            .unwrap_or(fallback);
        clean_text(value, max_len)
    };
    let current_stage = {
        let stage = read_visibility("current_stage", "final_response", 80);
        if stage.is_empty() {
            "final_response".to_string()
        } else {
            stage
        }
    };
    let current_stage_status = read_visibility("current_stage_status", "complete", 80);
    let ui_status = read_visibility("ui_status", "Workflow status available.", 180);
    let agent_process_status = read_visibility(
        "agent_process_status",
        "Workflow diagnostics available in payload.",
        220,
    );
    let debug_status = read_visibility("debug_status", "", 320);
    let formats = visibility
        .get("formats")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "ui": ui_status,
                "agent_process": agent_process_status,
                "debug": debug_status
            })
        });
    let visible_response_source = clean_text(
        response_finalization
            .get("visible_response_source")
            .or_else(|| response_workflow.get("visible_response_source"))
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let system_chat_injection_used = response_finalization
        .get("system_chat_injection_used")
        .or_else(|| response_workflow.get("system_chat_injection_used"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    json!({
        "contract": "workflow_visibility_payload_v1",
        "current_stage": current_stage,
        "current_stage_status": current_stage_status,
        "ui_status": ui_status,
        "agent_process_status": agent_process_status,
        "debug_status": debug_status,
        "formats": formats,
        "selected_workflow_id": clean_text(response_workflow.pointer("/selected_workflow/name").and_then(Value::as_str).unwrap_or(""), 80),
        "stage_count": response_workflow.get("stage_statuses").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "finalization_status": clean_text(response_finalization.get("outcome").and_then(Value::as_str).unwrap_or(""), 180),
        "diagnostics_only": true,
        "final_chat_authority": "llm_only",
        "visible_chat_text_authority": "llm_only",
        "chat_injection_allowed": false,
        "system_injected_chat_text_allowed": false,
        "system_chat_injection_used": system_chat_injection_used,
        "visible_response_source": visible_response_source
    })
}

fn agent_control_plane_health_snapshot_path(root: &Path, agent_id: &str) -> PathBuf {
    root.join("local/state/ops/agent_health_snapshots")
        .join(format!("{}.json", clean_agent_id(agent_id)))
}

fn build_agent_control_plane_health_snapshot(
    agent_id: &str,
    message: &str,
    response_text: &str,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    live_eval_monitor: &Value,
) -> Value {
    let current_turn = response_finalization
        .get("current_turn_dominance")
        .cloned()
        .unwrap_or_else(|| current_turn_dominance_payload(message, response_text, &[]));
    let contamination = response_finalization
        .get("contamination_guard")
        .cloned()
        .unwrap_or_else(|| json!({ "detected": false }));
    let loop_issue = live_eval_monitor
        .get("issues")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("issue_type")
                    .or_else(|| row.get("reason"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("repeated")
            })
        })
        .unwrap_or(false)
        || response_workflow_quality_count(response_workflow, "repeated_fallback_loop_detected") > 0;
    let degraded = !current_turn
        .get("dominant")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        || contamination
            .get("detected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || loop_issue
        || response_finalization
            .get("system_chat_injection_used")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || live_eval_monitor
            .get("issue_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0;
    let dashboard_health_indicator = agent_dashboard_health_indicator(
        response_workflow,
        response_finalization,
        process_summary,
        live_eval_monitor,
    );
    json!({
        "contract": "agent_control_plane_health_snapshot_v1",
        "agent_id": clean_agent_id(agent_id),
        "generated_at": crate::now_iso(),
        "status": if degraded { "degraded" } else { "healthy" },
        "request_excerpt": clean_text(message, 240),
        "finalization": {
            "outcome": clean_text(response_finalization.get("outcome").and_then(Value::as_str).unwrap_or(""), 220),
            "visible_response_source": clean_text(response_finalization.get("visible_response_source").and_then(Value::as_str).unwrap_or(""), 80),
            "system_chat_injection_used": response_finalization.get("system_chat_injection_used").and_then(Value::as_bool).unwrap_or(false),
            "final_ack_only": response_finalization.get("final_ack_only").and_then(Value::as_bool).unwrap_or(false)
        },
        "telemetry": {
            "workflow_visibility_contract": process_summary.pointer("/workflow_visibility/contract").cloned().unwrap_or(Value::Null),
            "live_eval_enabled": live_eval_monitor.get("enabled").and_then(Value::as_bool).unwrap_or(false),
            "live_eval_issue_count": live_eval_monitor.get("issue_count").and_then(Value::as_u64).unwrap_or(0),
            "chat_injection_allowed": live_eval_monitor.get("chat_injection_allowed").and_then(Value::as_bool).unwrap_or(false)
        },
        "loop": {
            "repeated_response_loop_detected": loop_issue
        },
        "tool_gate": process_summary.get("tool_gate").cloned().unwrap_or_else(|| json!({})),
        "contamination": contamination,
        "current_turn_dominance": current_turn,
        "dashboard_health_indicator": dashboard_health_indicator
    })
}

fn persist_agent_control_plane_health_snapshot(root: &Path, agent_id: &str, snapshot: &Value) {
    write_json_pretty(&agent_control_plane_health_snapshot_path(root, agent_id), snapshot);
}

fn persist_agent_control_plane_health_snapshot_for_turn(
    root: &Path,
    agent_id: &str,
    message: &str,
    response_text: &str,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    turn_receipt: &Value,
) -> Value {
    let snapshot = build_agent_control_plane_health_snapshot(
        agent_id,
        message,
        response_text,
        response_workflow,
        response_finalization,
        process_summary,
        turn_receipt.get("live_eval_monitor").unwrap_or(&Value::Null),
    );
    persist_agent_control_plane_health_snapshot(root, agent_id, &snapshot);
    snapshot
}
