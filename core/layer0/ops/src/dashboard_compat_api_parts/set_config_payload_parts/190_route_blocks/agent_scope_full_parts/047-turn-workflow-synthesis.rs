fn turn_workflow_event(kind: &str, detail: Value) -> Value {
    json!({
        "kind": clean_text(kind, 80),
        "detail": detail
    })
}

fn bump_workflow_quality_counter(workflow: &mut Value, key: &str) {
    let pointer = format!("/quality_telemetry/{key}");
    let current = workflow
        .pointer(&pointer)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    workflow["quality_telemetry"][key] = json!(current + 1);
}

fn response_tool_workflow_events(response_tools: &[Value]) -> Vec<Value> {
    let mut events = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools.iter().take(8) {
        let tool_name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if tool_name.is_empty() {
            continue;
        }
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let blocked = tool.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let is_error = tool.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let result = clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 600);
        let attempt_reason = clean_text(
            tool.pointer("/tool_attempt_receipt/reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let attempt_backend = clean_text(
            tool.pointer("/tool_attempt_receipt/backend")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let low_signal = !result.is_empty()
            && (response_looks_like_tool_ack_without_findings(&result)
                || response_is_no_findings_placeholder(&result)
                || response_looks_like_unsynthesized_web_snippet_dump(&result)
                || response_looks_like_raw_web_artifact_dump(&result));
        let event_kind = if blocked || matches!(status.as_str(), "blocked" | "policy_denied") {
            "tool_blocked"
        } else if matches!(status.as_str(), "timeout") {
            "tool_timeout"
        } else if is_error || matches!(status.as_str(), "error" | "failed" | "execution_error") {
            "tool_failed"
        } else if low_signal || matches!(status.as_str(), "no_results") {
            "tool_low_signal"
        } else {
            "tool_completed"
        };
        let key = format!("{tool_name}:{event_kind}:{status}:{attempt_reason}");
        if !seen.insert(key) {
            continue;
        }
        events.push(turn_workflow_event(
            event_kind,
            json!({
                "tool_name": tool_name,
                "status": status,
                "blocked": blocked,
                "is_error": is_error,
                "reason": attempt_reason,
                "backend": attempt_backend,
                "result_excerpt": first_sentence(&result, 220)
            }),
        ));
    }
    events
}

fn build_turn_workflow_events(
    response_tools: &[Value],
    pending_confirmation: Option<&Value>,
    replayed_pending_confirmation: bool,
) -> Vec<Value> {
    let mut events = response_tool_workflow_events(response_tools);
    if let Some(pending) = pending_confirmation {
        let tool_name = clean_text(
            pending
                .get("tool_name")
                .or_else(|| pending.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let source = clean_text(
            pending.get("source").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        events.push(turn_workflow_event(
            "pending_confirmation_required",
            json!({
                "tool_name": tool_name,
                "source": source
            }),
        ));
    }
    if replayed_pending_confirmation {
        events.push(turn_workflow_event(
            "pending_confirmation_replayed",
            json!({"ok": true}),
        ));
    }
    events
}

fn workflow_final_response_status(workflow: &Value) -> String {
    clean_text(
        workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
}

fn workflow_final_response_used(workflow: &Value) -> bool {
    workflow
        .pointer("/final_llm_response/used")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && workflow
            .get("response")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn workflow_final_response_allows_system_fallback(workflow: &Value) -> bool {
    matches!(
        workflow_final_response_status(workflow).as_str(),
        "invoke_failed"
            | "synthesis_failed"
            | "skipped_missing_model"
            | "skipped_test"
            | "skipped_not_required"
    )
}

fn workflow_response_repetition_breaker_active(latest_assistant_text: &str) -> bool {
    let lowered = latest_assistant_text.to_ascii_lowercase();
    let macro_signals = workflow_retry_macro_signal_count(&lowered);
    lowered.contains("i completed the workflow gate, but the final workflow state was unexpected")
        || lowered.contains("i completed the run, but the final reply did not render")
        || lowered.contains("please retry so i can rerun the chain cleanly")
        || lowered.contains("ask me to continue and i will synthesize")
        || (lowered.contains("next actions:")
            && lowered.contains("run one targeted tool call")
            && lowered.contains("return a concise answer from current context"))
        || (macro_signals >= 3
            && (lowered.contains("workflow gate")
                || lowered.contains("next actions")
                || lowered.contains("final reply did not render")))
}

fn recent_assistant_retry_loop_detected(active_messages: &[Value]) -> bool {
    let mut assistant_turns_scanned = 0usize;
    let mut retry_boilerplate_turns = 0usize;
    for row in active_messages.iter().rev() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
            .to_ascii_lowercase();
        if role != "assistant" && role != "agent" {
            continue;
        }
        let text = clean_chat_text(
            row.get("text")
                .or_else(|| row.get("content"))
                .or_else(|| row.get("message"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        if text.is_empty() {
            continue;
        }
        assistant_turns_scanned += 1;
        if response_contains_unexpected_state_retry_boilerplate(&text)
            || workflow_response_repetition_breaker_active(&text)
        {
            retry_boilerplate_turns += 1;
        }
        if assistant_turns_scanned >= 3 {
            break;
        }
    }
    retry_boilerplate_turns >= 2
}

fn workflow_retry_macro_signal_count(lowered: &str) -> usize {
    let macro_signals = [
        "workflow gate",
        "final workflow state was unexpected",
        "final reply did not render",
        "please retry",
        "rerun the chain",
        "ask me to continue",
        "synthesize from the recorded workflow state",
        "next actions",
        "targeted tool call",
        "concise answer from current context",
    ];
    macro_signals
        .iter()
        .filter(|token| lowered.contains(**token))
        .count()
}

fn response_contains_unexpected_state_retry_boilerplate(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let macro_signals = workflow_retry_macro_signal_count(&lowered);
    let workflow_gate_template = lowered.contains("workflow gate")
        && lowered.contains("unexpected")
        && (lowered.contains("retry") || lowered.contains("rerun"));
    let next_actions_template = lowered.contains("next actions:")
        && lowered.contains("clarify the exact outcome you want")
        && lowered.contains("run one targeted tool call")
        && lowered.contains("return a concise answer from current context");
    lowered.contains("final workflow state was unexpected")
        || lowered.contains("final reply did not render")
        || lowered.contains("please retry so i can rerun the chain cleanly")
        || lowered.contains("ask me to continue and i will synthesize")
        || workflow_gate_template
        || next_actions_template
        || (macro_signals >= 3
            && (lowered.contains("workflow gate")
                || lowered.contains("next actions")
                || lowered.contains("final reply did not render")))
}

fn message_requests_plain_direct_reply(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("just answer")
        || lowered.contains("regular response")
        || lowered.contains("talk to me")
        || lowered == "hello"
}

fn message_is_minimal_conversational_ping(message: &str) -> bool {
    let lowered = clean_text(message, 200).trim().to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "hi" | "hello" | "hey" | "hey there" | "hello there" | "yo"
    )
}

fn message_requests_diagnostic_explanation(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("what do you think is happening")
        || lowered.contains("what do you think happened")
        || lowered.contains("what happened")
        || lowered.contains("what's going on")
        || lowered.contains("whats going on")
        || lowered.contains("why is this happening")
        || lowered.contains("why do you think")
        || lowered.contains("is it too strict")
        || lowered.contains("hardlocked")
        || lowered.contains("hard-locked")
        || lowered.contains("hard coded")
        || lowered.contains("hard-coded")
        || lowered.contains("system response")
        || lowered.contains("improve the system")
}

fn message_mentions_tool_routing_authority(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_authority = lowered.contains("automatic tool")
        || lowered.contains("auto tool")
        || lowered.contains("tool routing")
        || lowered.contains("tool selection")
        || lowered.contains("llm should")
        || lowered.contains("llm-controlled")
        || lowered.contains("llm controlled");
    let mentions_tooling_surface =
        lowered.contains("tool") || lowered.contains("routing") || lowered.contains("selection");
    mentions_authority && mentions_tooling_surface
}

fn message_checks_hardcoded_response(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let hardcoded_signal = lowered.contains("hard coded")
        || lowered.contains("hard-coded")
        || lowered.contains("hardlocked")
        || lowered.contains("hard-locked");
    let system_response_signal =
        lowered.contains("system response") || lowered.contains("coded response");
    hardcoded_signal || system_response_signal
}

fn message_requests_workspace_file_action(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_workspace_surface = lowered.contains("file")
        || lowered.contains("directory")
        || lowered.contains("local dir")
        || lowered.contains("workspace")
        || lowered.contains("path");
    let mentions_action = lowered.contains("look at")
        || lowered.contains("check")
        || lowered.contains("read")
        || lowered.contains("list")
        || lowered.contains("show")
        || lowered.contains("open");
    mentions_workspace_surface && mentions_action
}

fn message_requests_system_improvement_plan(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("improve the system")
        || lowered.contains("make infring better")
        || lowered.contains("make the system better")
        || lowered.contains("what would make")
        || lowered.contains("what do we need to do")
}

fn message_requests_patch_effectiveness_check(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("are the patches working")
        || lowered.contains("patches we've done are working")
        || lowered.contains("notice any improvements")
        || lowered.contains("did the patches work")
        || lowered.contains("is it working now")
        || lowered.contains("on your end")
}

fn workflow_policy_block_summary(response_tools: &[Value]) -> String {
    for row in response_tools {
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 240);
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 240);
        let is_policy_like = blocked
            || status.contains("lease_denied")
            || status.contains("policy_denied")
            || result.to_ascii_lowercase().contains("lease_denied")
            || error.to_ascii_lowercase().contains("lease_denied");
        if !is_policy_like {
            continue;
        }
        let tool_name =
            normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or("tool"));
        let reason = if result.is_empty() {
            if error.is_empty() {
                "policy gate denied tool execution".to_string()
            } else {
                first_sentence(&error, 140)
            }
        } else {
            first_sentence(&result, 140)
        };
        if tool_name.is_empty() {
            return reason;
        }
        return format!("{tool_name}: {reason}");
    }
    String::new()
}

fn workflow_turn_has_policy_block(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || row
                .get("status")
                .and_then(Value::as_str)
                .map(|raw| raw.to_lowercase().contains("lease_denied"))
                .unwrap_or(false)
            || row
                .get("result")
                .and_then(Value::as_str)
                .map(|raw| raw.to_lowercase().contains("lease_denied"))
                .unwrap_or(false)
    })
}

fn normalized_response_similarity_key(text: &str) -> String {
    let lowered = clean_text(text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(lowered.len());
    let mut previous_space = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { ' ' };
        if mapped == ' ' {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
            continue;
        }
        out.push(mapped);
        previous_space = false;
    }
    out.trim().to_string()
}

fn response_repeats_latest_assistant_copy(response_text: &str, latest_assistant_text: &str) -> bool {
    let cleaned_response = sanitize_workflow_final_response_candidate(
        &strip_internal_cache_control_markup(&strip_internal_context_metadata_prefix(
            response_text,
        )),
    );
    let cleaned_latest = sanitize_workflow_final_response_candidate(
        &strip_internal_cache_control_markup(&strip_internal_context_metadata_prefix(
            latest_assistant_text,
        )),
    );
    let normalized_response = normalized_response_similarity_key(&cleaned_response);
    let normalized_latest = normalized_response_similarity_key(&cleaned_latest);
    let response_first_sentence = first_sentence(&cleaned_response, 200);
    let latest_first_sentence = first_sentence(&cleaned_latest, 200);
    let normalized_contains = normalized_response.len() >= 48
        && normalized_latest.len() >= 48
        && (normalized_response.contains(&normalized_latest)
            || normalized_latest.contains(&normalized_response));
    let first_sentence_match = response_first_sentence.len() >= 40
        && latest_first_sentence.len() >= 40
        && response_first_sentence.eq_ignore_ascii_case(&latest_first_sentence);
    !cleaned_response.is_empty()
        && !cleaned_latest.is_empty()
        && !normalized_response.is_empty()
        && !normalized_latest.is_empty()
        && cleaned_response.len() >= 24
        && (cleaned_response.eq_ignore_ascii_case(&cleaned_latest)
            || normalized_response == normalized_latest
            || normalized_contains
            || first_sentence_match)
}

fn workflow_repeat_safe_direct_reply(message: &str, response_tools: &[Value]) -> String {
    let policy_blocked = workflow_turn_has_policy_block(response_tools);
    if message_requests_diagnostic_explanation(message) {
        if policy_blocked {
            return "The last tool attempt was blocked by a local policy gate, so I’m switching to a direct-answer path from current context and keeping tools off unless you explicitly request one.".to_string();
        }
        return "The previous fallback text repeated, so I’m switching to a direct-answer path from current context and keeping tools off unless you explicitly request one.".to_string();
    }
    if policy_blocked {
        return "Answering directly from current context now. The last tool attempt was policy-blocked, and I’ll keep tools off unless you explicitly request one.".to_string();
    }
    "Answering directly from current context now, with tools off unless you explicitly request one."
        .to_string()
}

fn fallback_reply_variant_seed(message: &str, latest_assistant_text: &str) -> usize {
    let mut acc: usize = 0;
    for byte in clean_text(message, 2_000).bytes() {
        acc = acc.wrapping_mul(131).wrapping_add(byte as usize);
    }
    for byte in clean_text(latest_assistant_text, 2_000).bytes() {
        acc = acc.wrapping_mul(131).wrapping_add(byte as usize);
    }
    acc
}

fn pick_non_repeating_reply_variant(
    candidates: &[&str],
    message: &str,
    latest_assistant_text: &str,
) -> String {
    if candidates.is_empty() {
        return String::new();
    }
    let seed = fallback_reply_variant_seed(message, latest_assistant_text);
    let start = seed % candidates.len();
    for offset in 0..candidates.len() {
        let idx = (start + offset) % candidates.len();
        let candidate = candidates[idx];
        if !response_repeats_latest_assistant_copy(candidate, latest_assistant_text) {
            return candidate.to_string();
        }
    }
    candidates[start].to_string()
}

fn workflow_non_repeating_last_resort_reply(
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) -> String {
    let policy_blocked = workflow_turn_has_policy_block(response_tools);
    if message_is_minimal_conversational_ping(message) {
        return pick_non_repeating_reply_variant(
            &[
                "Hey - I'm here and ready. Tell me what you want next, and I will answer directly from current context.",
                "Hi - I can answer directly from current context. Tell me what you want next.",
            ],
            message,
            latest_assistant_text,
        );
    }
    if message_requests_diagnostic_explanation(message) {
        if policy_blocked {
            return pick_non_repeating_reply_variant(
                &[
                    "Root cause is likely a policy gate on the prior tool attempt. I will keep this answer direct and continue without new tool calls unless you explicitly request one.",
                    "Likely cause is a local policy gate on the previous tool attempt. I will proceed with a direct answer from current context and keep tools off unless you explicitly request one.",
                ],
                message,
                latest_assistant_text,
            );
        }
        return pick_non_repeating_reply_variant(
            &[
                "Root cause is likely fallback-loop churn in finalization. I will keep this answer direct and continue without new tool calls unless you explicitly request one.",
                "Likely cause is fallback-loop churn during finalization. I will continue with a direct answer from current context and keep tools off unless you explicitly request one.",
            ],
            message,
            latest_assistant_text,
        );
    }
    if policy_blocked {
        return pick_non_repeating_reply_variant(
            &[
                "Direct answer mode is active. Prior tool execution was policy-blocked, and I will continue from current context unless you explicitly request a tool.",
                "Direct-answer path is active. The previous tool step was policy-blocked, so I will continue from current context unless you explicitly request a tool.",
            ],
            message,
            latest_assistant_text,
        );
    }
    pick_non_repeating_reply_variant(
        &[
            "Direct answer mode is active. I will continue from current context and avoid tool calls unless you explicitly request one.",
            "Direct-answer path is active. I will continue from current context and keep tools off unless you explicitly request one.",
        ],
        message,
        latest_assistant_text,
    )
}

fn enforce_non_repeating_fallback_text(
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    candidate: &str,
) -> String {
    if !response_repeats_latest_assistant_copy(candidate, latest_assistant_text) {
        return candidate.to_string();
    }
    let alternate = workflow_repeat_safe_direct_reply(message, response_tools);
    if !response_repeats_latest_assistant_copy(&alternate, latest_assistant_text) {
        return alternate;
    }
    workflow_non_repeating_last_resort_reply(message, latest_assistant_text, response_tools)
}

fn workflow_unexpected_state_user_fallback(
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) -> String {
    if message_is_minimal_conversational_ping(message) {
        return "Hey — I’m here. I can continue directly from current context without extra tool calls; tell me what you want next.".to_string();
    }
    let policy_blocked = workflow_turn_has_policy_block(response_tools);
    let policy_summary = workflow_policy_block_summary(response_tools);
    let repeated_fallback = workflow_response_repetition_breaker_active(latest_assistant_text);
    if repeated_fallback {
        if message_requests_diagnostic_explanation(message) {
            if message_requests_patch_effectiveness_check(message) {
                if policy_blocked {
                    if !policy_summary.is_empty() {
                        return format!(
                            "Partial improvement is active: the fallback guard prevented another repeated workflow template, but this turn still hit a policy gate ({policy_summary}) on tool execution. I can continue in direct-answer mode from current context."
                        );
                    }
                    return "Partial improvement is active: the fallback guard prevented another repeated workflow template, but this turn still hit a policy gate on tool execution. I can continue in direct-answer mode from current context.".to_string();
                }
                return "Yes—improvement is active here: the fallback guard caught the repeated workflow template and switched to stable direct-answer mode from current context.".to_string();
            }
            if message_requests_system_improvement_plan(message) {
                if policy_blocked {
                    if !policy_summary.is_empty() {
                        return format!(
                            "Top improvements: 1) keep tool routing LLM-controlled, 2) keep policy-denied turns fail-closed with explicit reason ({policy_summary}), and 3) continue direct-answer mode from current context when tool execution is blocked."
                        );
                    }
                    return "Top improvements: 1) keep tool routing LLM-controlled, 2) keep policy-denied turns fail-closed with explicit reason, and 3) continue direct-answer mode from current context when tool execution is blocked.".to_string();
                }
                return "Top improvements: 1) keep tool routing LLM-controlled, 2) aggressively suppress repeated workflow fallback boilerplate, and 3) keep direct-answer mode stable from current context unless a tool call is explicitly requested.".to_string();
            }
            if message_checks_hardcoded_response(message) {
                if policy_blocked {
                    if !policy_summary.is_empty() {
                        return format!(
                            "This is not a hard-coded response lock. The fallback guard activated because a local ingress policy gate denied the requested tool route ({policy_summary}), and I can continue with a normal direct answer from current context."
                        );
                    }
                    return "This is not a hard-coded response lock. The fallback guard activated because a local ingress policy gate denied the requested tool route, and I can continue with a normal direct answer from current context.".to_string();
                }
                return "This is not a hard-coded response lock. The workflow fallback guard triggered after repeated unexpected-state boilerplate, and I can continue with a normal direct answer from current context.".to_string();
            }
            if message_mentions_tool_routing_authority(message) {
                if policy_blocked {
                    if !policy_summary.is_empty() {
                        return format!(
                            "Tool selection should remain LLM-controlled, and this turn did not auto-fire a tool successfully: a local ingress policy gate denied the requested route ({policy_summary}). I can continue in direct-answer mode from current context unless you explicitly request another tool call."
                        );
                    }
                    return "Tool selection should remain LLM-controlled, and this turn did not auto-fire a tool successfully: a local ingress policy gate denied the requested route. I can continue in direct-answer mode from current context unless you explicitly request another tool call.".to_string();
                }
                return "Tool selection should remain LLM-controlled. This turn hit a workflow finalization fallback edge, so I am staying in stable direct-answer mode from current context unless you explicitly request a tool call.".to_string();
            }
            if policy_blocked {
                if !policy_summary.is_empty() {
                    return format!(
                        "Likely root cause: a local ingress policy gate denied the requested tool call ({policy_summary}). I can continue with a direct answer from current context without running another tool unless you explicitly request one."
                    );
                }
                return "Likely root cause: a local ingress policy gate denied the requested tool call. I can continue with a direct answer from current context without running another tool unless you explicitly request one.".to_string();
            }
            return "Likely root cause: the previous workflow fallback text repeated after a finalization edge. I am switching to a stable direct-answer path now and will avoid extra tool calls unless you explicitly request one.".to_string();
        }
        if message_requests_plain_direct_reply(message) {
            if policy_blocked {
                return "The prior tool step was blocked by a local policy gate. I can continue with a plain direct answer from current context right now without running another tool.".to_string();
            }
            return "Understood. I can continue with a plain direct answer from current context right now and avoid extra tool calls unless you explicitly request one.".to_string();
        }
        if policy_blocked {
            if message_requests_workspace_file_action(message) {
                return "I can’t execute that workspace/file tool call in this runtime lane because a local ingress policy gate denied the route. I can continue from current context here, or if you switch to a workspace-read-enabled lane I can run the directory/file check directly.".to_string();
            }
            return "I'm not hard-locked; the last turn was blocked by a local access policy gate. I can continue with a direct answer from current context right now, and I will avoid extra tool calls unless you explicitly request one.".to_string();
        }
        return "I'm not hard-locked. The previous fallback repeated, so I'm switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one.".to_string();
    }
    if policy_blocked {
        if message_requests_diagnostic_explanation(message) {
            if message_requests_patch_effectiveness_check(message) {
                if !policy_summary.is_empty() {
                    return format!(
                        "Partial improvement is active, but this tool turn remains blocked by runtime-lane policy ({policy_summary}). The fallback path stayed fail-closed and I can continue with a direct answer from current context."
                    );
                }
                return "Partial improvement is active, but this tool turn remains blocked by runtime-lane policy. The fallback path stayed fail-closed and I can continue with a direct answer from current context.".to_string();
            }
            if message_requests_system_improvement_plan(message) {
                if !policy_summary.is_empty() {
                    return format!(
                        "Top improvements: 1) keep tool routing LLM-controlled, 2) keep policy-denied turns explicit and fail-closed ({policy_summary}), and 3) continue direct-answer mode from current context when runtime-lane permissions block tool execution."
                    );
                }
                return "Top improvements: 1) keep tool routing LLM-controlled, 2) keep policy-denied turns explicit and fail-closed, and 3) continue direct-answer mode from current context when runtime-lane permissions block tool execution.".to_string();
            }
            if message_checks_hardcoded_response(message) {
                if !policy_summary.is_empty() {
                    return format!(
                        "This is not a hard-coded system response. A local ingress policy gate denied the requested tool route ({policy_summary}), and I can continue with a direct answer from current context."
                    );
                }
                return "This is not a hard-coded system response. A local ingress policy gate denied the requested tool route, and I can continue with a direct answer from current context.".to_string();
            }
            if message_mentions_tool_routing_authority(message) {
                if !policy_summary.is_empty() {
                    return format!(
                        "Tool selection should remain LLM-controlled. This turn did not auto-fire a successful tool call because a local ingress policy gate denied the requested route ({policy_summary}). I can still answer directly from current context."
                    );
                }
                return "Tool selection should remain LLM-controlled. This turn did not auto-fire a successful tool call because a local ingress policy gate denied the requested route. I can still answer directly from current context.".to_string();
            }
            if !policy_summary.is_empty() {
                return format!(
                    "Likely root cause: a local ingress policy gate denied the requested tool call ({policy_summary}). I can still answer directly from current context without another tool call."
                );
            }
            return "Likely root cause: a local ingress policy gate denied the requested tool call. I can still answer directly from current context without another tool call.".to_string();
        }
        if message_requests_plain_direct_reply(message) {
            return "Understood. I can answer directly from current context now. The prior tool step was blocked by a local policy gate, so I’m staying in direct-answer mode unless you explicitly request another tool call.".to_string();
        }
        if message_requests_workspace_file_action(message) {
            return "That workspace/file action is blocked in this runtime lane by an ingress policy gate. I can continue with a direct answer from current context, or you can retry from a lane that has workspace-read permission and I’ll run the check.".to_string();
        }
        return "The prior step was blocked by a local access policy gate. I can still answer directly from current context without another tool call.".to_string();
    }
    if message_requests_plain_direct_reply(message) {
        return "Understood. I can continue with a plain direct answer from current context right now.".to_string();
    }
    if message_requests_workspace_file_action(message) {
        return "I can run that workspace/file check directly from current context. If you want me to execute it now, give me the exact path or tell me to use the current workspace root.".to_string();
    }
    if message_requests_diagnostic_explanation(message) {
        if message_requests_system_improvement_plan(message) {
            return "Top improvements: 1) keep tool routing LLM-controlled, 2) keep repeated workflow fallback suppression active, and 3) stay in direct-answer mode from current context unless a tool call is explicitly requested.".to_string();
        }
        if message_checks_hardcoded_response(message) {
            return "This is not a hard-coded system response. This turn hit a workflow finalization edge without a policy denial, and I can continue with a direct answer from current context.".to_string();
        }
        if message_mentions_tool_routing_authority(message) {
            return "Tool routing remains LLM-controlled. This turn hit a workflow finalization edge without a policy-denied tool route, and I can continue in direct-answer mode from current context unless you explicitly request a tool call.".to_string();
        }
        return "Likely root cause: a workflow finalization edge occurred on this turn without a policy denial. I can continue with a direct answer from current context and avoid extra tool calls unless you explicitly request one.".to_string();
    }
    if message_requests_patch_effectiveness_check(message) {
        return "Yes—fallback hardening is active on this lane. I can continue in direct-answer mode from current context, and tool calls remain explicit opt-in unless you request one.".to_string();
    }
    if message.trim().is_empty() {
        return "I hit a workflow finalization edge, but I can continue with a direct response from current context.".to_string();
    }
    "I hit a workflow finalization edge on this turn. I can still answer directly from current context, and I'll avoid extra tool calls unless you explicitly request one.".to_string()
}

fn should_force_direct_workflow_fallback(
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    recent_retry_loop_detected: bool,
) -> bool {
    last_reject_reason == "unexpected_state_retry_boilerplate"
        || response_contains_unexpected_state_retry_boilerplate(last_invalid_excerpt)
        || workflow_response_repetition_breaker_active(latest_assistant_text)
        || recent_retry_loop_detected
        || workflow_turn_has_policy_block(response_tools)
}

fn sanitize_skipped_final_response_fallback_response(
    message: &str,
    draft_response: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) -> (String, bool, &'static str) {
    let draft_fallback = sanitize_workflow_final_response_candidate(
        &strip_internal_cache_control_markup(&strip_internal_context_metadata_prefix(
            draft_response,
        )),
    );
    let latest_assistant_fallback = sanitize_workflow_final_response_candidate(
        &strip_internal_cache_control_markup(&strip_internal_context_metadata_prefix(
            latest_assistant_text,
        )),
    );
    let mut fallback_source: &'static str = if !draft_fallback.is_empty() {
        "draft"
    } else if !latest_assistant_fallback.is_empty() {
        "latest"
    } else {
        "empty"
    };
    let mut fallback_response = if !draft_fallback.is_empty() {
        draft_fallback
    } else {
        latest_assistant_fallback
    };
    let mut sanitized_retry_loop = false;
    if response_contains_unexpected_state_retry_boilerplate(&fallback_response) {
        fallback_response =
            workflow_unexpected_state_user_fallback(message, latest_assistant_text, response_tools);
        sanitized_retry_loop = true;
        fallback_source = "generated";
    }
    if clean_text(&fallback_response, 2_000).is_empty() {
        fallback_response =
            workflow_unexpected_state_user_fallback(message, latest_assistant_text, response_tools);
        sanitized_retry_loop = true;
        fallback_source = "generated";
    }
    let guarded_response = ensure_no_retry_boilerplate_copy(
        message,
        latest_assistant_text,
        response_tools,
        &fallback_response,
    );
    if guarded_response != fallback_response {
        sanitized_retry_loop = true;
        fallback_response = guarded_response;
        fallback_source = "generated";
    }
    if fallback_response.is_empty() {
        fallback_source = "empty";
    }
    (fallback_response, sanitized_retry_loop, fallback_source)
}

fn apply_skipped_final_response_fallback(
    workflow: &mut Value,
    fallback_response: &str,
    sanitized_retry_loop: bool,
    fallback_source: &str,
) {
    if !fallback_response.is_empty() {
        workflow["response"] = Value::String(fallback_response.to_string());
        workflow["final_llm_response"]["fallback_from_existing_draft"] =
            Value::Bool(matches!(fallback_source, "draft" | "latest"));
        workflow["final_llm_response"]["fallback_source"] =
            Value::String(clean_text(fallback_source, 40));
        workflow["final_llm_response"]["used"] = Value::Bool(true);
    } else {
        workflow["final_llm_response"]["fallback_from_existing_draft"] = Value::Bool(false);
        workflow["final_llm_response"]["fallback_source"] =
            Value::String(clean_text(fallback_source, 40));
        workflow["final_llm_response"]["used"] = Value::Bool(false);
    }
    if sanitized_retry_loop {
        workflow["final_llm_response"]["fallback_sanitized_retry_loop"] = Value::Bool(true);
        mark_workflow_fallback_guard_reason(
            workflow,
            "skipped_fallback_retry_sanitized",
            "skipped_fallback",
        );
    }
}

fn mark_workflow_fallback_guard_reason(workflow: &mut Value, reason: &str, stage: &str) {
    let cleaned_reason = clean_text(reason, 80);
    let cleaned_stage = clean_text(stage, 80);
    if cleaned_reason.is_empty() {
        return;
    }
    let mut reason_history = workflow
        .pointer("/final_llm_response/fallback_guard_reasons")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !reason_history.iter().any(|entry| {
        entry
            .as_str()
            .map(|value| value == cleaned_reason)
            .unwrap_or(false)
    }) {
        reason_history.push(Value::String(cleaned_reason.clone()));
        if reason_history.len() > 8 {
            let overflow = reason_history.len() - 8;
            reason_history.drain(0..overflow);
        }
    }
    let mut stage_history = workflow
        .pointer("/final_llm_response/fallback_guard_stages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !cleaned_stage.is_empty()
        && !stage_history.iter().any(|entry| {
            entry
                .as_str()
                .map(|value| value == cleaned_stage)
                .unwrap_or(false)
        })
    {
        stage_history.push(Value::String(cleaned_stage.clone()));
        if stage_history.len() > 8 {
            let overflow = stage_history.len() - 8;
            stage_history.drain(0..overflow);
        }
    }
    let mut guard_events = workflow
        .pointer("/final_llm_response/fallback_guard_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    guard_events.push(json!({
        "reason": cleaned_reason,
        "stage": cleaned_stage
    }));
    if guard_events.len() > 16 {
        let overflow = guard_events.len() - 16;
        guard_events.drain(0..overflow);
    }
    workflow["final_llm_response"]["fallback_guard_reason"] = Value::String(cleaned_reason);
    workflow["final_llm_response"]["fallback_guard_reasons"] = Value::Array(reason_history);
    workflow["final_llm_response"]["fallback_guard_stages"] = Value::Array(stage_history.clone());
    let trigger_count = workflow
        .pointer("/quality_telemetry/fallback_guard_trigger_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    let distinct_reason_count = workflow
        .pointer("/final_llm_response/fallback_guard_reasons")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let distinct_stage_count = stage_history.len();
    let multi_stage = stage_history.len() > 1;
    let (severity, requires_operator_review, escalation_reason, recommended_action) =
        workflow_fallback_guard_summary_classification(trigger_count, distinct_stage_count);
    workflow["final_llm_response"]["fallback_guard_multi_stage"] = Value::Bool(multi_stage);
    workflow["final_llm_response"]["fallback_guard_events"] = Value::Array(guard_events);
    workflow["final_llm_response"]["fallback_guard_last_stage"] =
        Value::String(cleaned_stage);
    workflow["final_llm_response"]["fallback_guard_summary"] = json!({
        "trigger_count": trigger_count,
        "distinct_reason_count": distinct_reason_count,
        "distinct_stage_count": distinct_stage_count,
        "multi_stage": multi_stage,
        "severity": severity,
        "requires_operator_review": requires_operator_review,
        "escalation_reason": escalation_reason,
        "recommended_action": recommended_action
    });
    let stage_counter_key = workflow_fallback_guard_stage_counter_key(stage);
    let reason_counter_key = workflow_fallback_guard_reason_counter_key(reason);
    bump_workflow_quality_counter(workflow, &stage_counter_key);
    bump_workflow_quality_counter(workflow, &reason_counter_key);
    bump_workflow_quality_counter(workflow, "fallback_guard_trigger_count");
}

fn workflow_fallback_guard_stage_counter_key(stage: &str) -> String {
    let mut out = String::with_capacity(96);
    out.push_str("fallback_guard_stage_");
    let mut previous_underscore = false;
    for ch in clean_text(stage, 80).chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if !previous_underscore {
                out.push('_');
                previous_underscore = true;
            }
            continue;
        }
        out.push(mapped.to_ascii_lowercase());
        previous_underscore = false;
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out == "fallback_guard_stage" {
        "fallback_guard_stage_unknown".to_string()
    } else {
        out
    }
}

fn workflow_fallback_guard_reason_counter_key(reason: &str) -> String {
    let mut out = String::with_capacity(96);
    out.push_str("fallback_guard_reason_");
    let mut previous_underscore = false;
    for ch in clean_text(reason, 80).chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if !previous_underscore {
                out.push('_');
                previous_underscore = true;
            }
            continue;
        }
        out.push(mapped.to_ascii_lowercase());
        previous_underscore = false;
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out == "fallback_guard_reason" {
        "fallback_guard_reason_unknown".to_string()
    } else {
        out
    }
}

fn workflow_fallback_guard_summary_classification(
    trigger_count: u64,
    distinct_stage_count: usize,
) -> (&'static str, bool, &'static str, &'static str) {
    if trigger_count >= 3 || distinct_stage_count >= 3 {
        (
            "high",
            true,
            "high_trigger_or_stage_diversity",
            "operator_review_recommended",
        )
    } else if trigger_count >= 2 || distinct_stage_count >= 2 {
        (
            "moderate",
            false,
            "repeated_or_multi_stage_guard_activity",
            "monitor_and_continue_direct_mode",
        )
    } else {
        (
            "low",
            false,
            "single_guard_activation",
            "continue_direct_mode",
        )
    }
}

fn ensure_no_retry_boilerplate_copy(
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    response_text: &str,
) -> String {
    let cleaned = sanitize_workflow_final_response_candidate(
        &strip_internal_cache_control_markup(&strip_internal_context_metadata_prefix(response_text)),
    );
    if response_repeats_latest_assistant_copy(&cleaned, latest_assistant_text) {
        return enforce_non_repeating_fallback_text(
            message,
            latest_assistant_text,
            response_tools,
            &cleaned,
        );
    }
    if !response_contains_unexpected_state_retry_boilerplate(&cleaned) {
        return cleaned;
    }
    let fallback = clean_text(
        &workflow_unexpected_state_user_fallback(message, latest_assistant_text, response_tools),
        2_000,
    );
    if response_repeats_latest_assistant_copy(&fallback, latest_assistant_text) {
        return enforce_non_repeating_fallback_text(
            message,
            latest_assistant_text,
            response_tools,
            &fallback,
        );
    }
    if fallback.is_empty() || response_contains_unexpected_state_retry_boilerplate(&fallback) {
        if workflow_turn_has_policy_block(response_tools) {
            return enforce_non_repeating_fallback_text(
                message,
                latest_assistant_text,
                response_tools,
                "The prior tool step was blocked by a local policy gate. I can continue with a direct answer from current context without another tool call.",
            );
        }
        return enforce_non_repeating_fallback_text(
            message,
            latest_assistant_text,
            response_tools,
            "I can continue with a direct answer from current context and avoid extra tool calls unless you explicitly request one.",
        );
    }
    enforce_non_repeating_fallback_text(message, latest_assistant_text, response_tools, &fallback)
}

fn apply_final_retry_boilerplate_guard(
    workflow: &mut Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) {
    let response_text = clean_text(
        workflow.get("response").and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    if response_text.is_empty() || !response_contains_unexpected_state_retry_boilerplate(&response_text)
    {
        return;
    }
    let guarded = ensure_no_retry_boilerplate_copy(
        message,
        latest_assistant_text,
        response_tools,
        &response_text,
    );
    if guarded.is_empty() {
        return;
    }
    workflow["response"] = Value::String(guarded.clone());
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(true);
    bump_workflow_quality_counter(workflow, "legacy_retry_template_detected");
    workflow["final_llm_response"]["fallback_sanitized_retry_loop"] = Value::Bool(true);
    workflow["final_llm_response"]["fallback_source"] = Value::String("generated_guard".to_string());
    workflow["final_llm_response"]["fallback_from_existing_draft"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    workflow["final_llm_response"]["status"] = Value::String("fallback_direct_answer".to_string());
    workflow["final_llm_response"]["fallback_response"] = Value::String(guarded);
    workflow["final_llm_response"]["error"] = Value::Null;
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("fallback_applied_guard".to_string());
    mark_workflow_fallback_guard_reason(
        workflow,
        "retry_boilerplate_guard",
        "final_retry_guard",
    );
    set_turn_workflow_final_stage_status(workflow, "fallback_direct_answer");
}

fn apply_final_response_presence_guard(
    workflow: &mut Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) {
    let response_text = clean_text(
        workflow.get("response").and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    if !response_text.is_empty() {
        return;
    }
    let candidate = workflow_unexpected_state_user_fallback(
        message,
        latest_assistant_text,
        response_tools,
    );
    let guarded = ensure_no_retry_boilerplate_copy(
        message,
        latest_assistant_text,
        response_tools,
        &candidate,
    );
    if guarded.is_empty() {
        return;
    }
    workflow["response"] = Value::String(guarded.clone());
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(true);
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    workflow["final_llm_response"]["status"] = Value::String("fallback_direct_answer".to_string());
    workflow["final_llm_response"]["fallback_response"] = Value::String(guarded);
    workflow["final_llm_response"]["fallback_source"] =
        Value::String("generated_presence_guard".to_string());
    workflow["final_llm_response"]["fallback_from_existing_draft"] = Value::Bool(false);
    workflow["final_llm_response"]["error"] = Value::Null;
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("fallback_applied_presence_guard".to_string());
    mark_workflow_fallback_guard_reason(
        workflow,
        "empty_response_presence_guard",
        "final_presence_guard",
    );
    set_turn_workflow_final_stage_status(workflow, "fallback_direct_answer");
}

fn tool_completion_report_for_response(
    response_text: &str,
    response_tools: &[Value],
    outcome: &str,
) -> Value {
    let cleaned = clean_chat_text(response_text, 32_000);
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    let reasoning_source = if !cleaned.is_empty() {
        cleaned.clone()
    } else if !failure_reason.is_empty() {
        failure_reason.clone()
    } else {
        findings.clone()
    };
    let completion_state = if response_tools.is_empty() {
        "not_applicable"
    } else if !failure_reason.is_empty() {
        "reported_reason"
    } else if !findings.is_empty() {
        "reported_findings"
    } else {
        "reported_no_findings"
    };
    let deferred_execution = response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned);
    json!({
        "completion_state": completion_state,
        "findings_available": !findings.is_empty(),
        "final_ack_only": response_looks_like_tool_ack_without_findings(&cleaned),
        "final_no_findings": response_is_no_findings_placeholder(&cleaned),
        "final_deferred_execution": deferred_execution,
        "final_requests_more_tooling": workflow_response_requests_more_tooling(&cleaned),
        "reasoning": first_sentence(&reasoning_source, 220),
        "outcome": clean_text(outcome, 200)
    })
}

fn augment_turn_workflow_events_for_final_response(
    message: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Vec<Value> {
    let mut events = workflow_events.to_vec();
    let cleaned_draft = clean_text(draft_response, 4_000);
    if response_is_no_findings_placeholder(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "no_findings_placeholder",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_contains_unexpected_state_retry_boilerplate(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "unexpected_state_retry_boilerplate",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "ack_only",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_is_deferred_execution_preamble(&cleaned_draft)
        || response_is_deferred_retry_prompt(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "deferred_retry_prompt",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    }
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000);
    if !findings.is_empty() {
        events.push(turn_workflow_event(
            "tool_findings_summary",
            json!({
                "summary": findings
            }),
        ));
    }
    let failure_summary = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        2_000,
    );
    if !failure_summary.is_empty() {
        events.push(turn_workflow_event(
            "tool_failure_summary",
            json!({
                "summary": failure_summary
            }),
        ));
    }
    if !response_tools.is_empty() {
        let readability_hint = clean_text(
            &ensure_tool_turn_response_text(draft_response, response_tools),
            2_000,
        );
        if !readability_hint.is_empty() && readability_hint != cleaned_draft {
            events.push(turn_workflow_event(
                "tool_response_readability_guidance",
                json!({
                    "suggested_response": readability_hint
                }),
            ));
        }
    }
    if let Some(tooling_guidance) =
        maybe_tooling_failure_fallback(message, draft_response, latest_assistant_text)
    {
        events.push(turn_workflow_event(
            "tooling_failure_guidance",
            json!({
                "suggested_response": clean_text(&tooling_guidance, 2_000)
            }),
        ));
    }
    if message_requests_comparative_answer(message) {
        events.push(turn_workflow_event(
            "comparative_answer_requested",
            json!({
                "live_web_focus": message_requests_live_web_comparison(message)
            }),
        ));
        if response_is_no_findings_placeholder(&cleaned_draft) || !failure_summary.is_empty() {
            events.push(turn_workflow_event(
                "comparative_guidance",
                json!({
                    "suggested_response": clean_text(
                        &comparative_no_findings_fallback(message),
                        2_000,
                    )
                }),
            ));
        }
    }
    events
}

#[cfg(test)]
fn workflow_test_llm_enabled(root: &Path) -> bool {
    root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json")
        .exists()
        || matches!(
            std::env::var("INFRING_LIVE_WEB_TOOLING_SMOKE")
                .ok()
                .as_deref()
                .map(|value| value.trim().to_ascii_lowercase()),
            Some(ref value) if value == "1" || value == "true" || value == "yes"
        )
}

#[cfg(not(test))]
fn workflow_test_llm_enabled(_root: &Path) -> bool {
    false
}

fn workflow_response_template_label(message: &str) -> &'static str {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return "quick_qa";
    }
    if message_is_tooling_status_check(message) || lowered.starts_with("did you") {
        return "status_check";
    }
    if lowered.contains("debug")
        || lowered.contains("root cause")
        || lowered.contains("why")
        || lowered.contains("diagnos")
    {
        return "debug_diagnosis";
    }
    if message_requests_comparative_answer(message) || lowered.contains("compare") {
        return "compare";
    }
    if lowered.contains("implement")
        || lowered.contains("patch")
        || lowered.contains("fix")
        || lowered.contains("build")
        || lowered.contains("create")
        || lowered.contains("wire")
    {
        return "implement_request";
    }
    "quick_qa"
}

fn workflow_template_instruction_for_label(label: &str) -> &'static str {
    match label {
        "status_check" => {
            "Template: Start with a direct status line in the first sentence, then explain evidence in 1-3 concise bullets."
        }
        "debug_diagnosis" => {
            "Template: First sentence should state the likely root cause. Then provide the top 1-3 fixes in priority order."
        }
        "compare" => {
            "Template: Give a concise side-by-side comparison with 2-4 bullets and call out practical tradeoffs."
        }
        "implement_request" => {
            "Template: State what was done in sentence one, then summarize impact and any important caveats."
        }
        _ => {
            "Template: Answer directly in plain language first, then add only necessary supporting detail."
        }
    }
}

fn workflow_user_prefers_deep_dive(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("deep dive")
        || lowered.contains("in depth")
        || lowered.contains("in-depth")
        || lowered.contains("detailed")
        || lowered.contains("thorough")
        || lowered.contains("full analysis")
        || lowered.contains("step by step")
}

fn run_turn_workflow_final_response(
    root: &Path,
    provider: &str,
    model: &str,
    active_messages: &[Value],
    message: &str,
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Value {
    let enriched_workflow_events = augment_turn_workflow_events_for_final_response(
        message,
        response_tools,
        workflow_events,
        draft_response,
        latest_assistant_text,
    );
    let mut workflow = turn_workflow_metadata(
        workflow_mode,
        response_tools,
        &enriched_workflow_events,
        draft_response,
        message,
    );
    let required = workflow
        .pointer("/final_llm_response/required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !required {
        let (fallback_response, sanitized_retry_loop, fallback_source) =
            sanitize_skipped_final_response_fallback_response(
            message,
            draft_response,
            latest_assistant_text,
            response_tools,
        );
        apply_skipped_final_response_fallback(
            &mut workflow,
            &fallback_response,
            sanitized_retry_loop,
            fallback_source,
        );
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_not_required".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_not_required");
        return workflow;
    }
    if cfg!(test) && !workflow_test_llm_enabled(root) {
        let (fallback_response, sanitized_retry_loop, fallback_source) =
            sanitize_skipped_final_response_fallback_response(
            message,
            draft_response,
            latest_assistant_text,
            response_tools,
        );
        apply_skipped_final_response_fallback(
            &mut workflow,
            &fallback_response,
            sanitized_retry_loop,
            fallback_source,
        );
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_test".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_test");
        return workflow;
    }
    let cleaned_provider = clean_text(provider, 80);
    let cleaned_model = clean_text(model, 240);
    if cleaned_provider.is_empty() || cleaned_model.is_empty() {
        let (fallback_response, sanitized_retry_loop, fallback_source) =
            sanitize_skipped_final_response_fallback_response(
            message,
            draft_response,
            latest_assistant_text,
            response_tools,
        );
        apply_skipped_final_response_fallback(
            &mut workflow,
            &fallback_response,
            sanitized_retry_loop,
            fallback_source,
        );
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] =
            Value::String("skipped_missing_model".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_missing_model");
        return workflow;
    }
    let tool_rows_json = serde_json::to_string(&tool_rows_for_llm_recovery(response_tools, 6))
        .unwrap_or_else(|_| "[]".to_string());
    let workflow_events_json = serde_json::to_string(&enriched_workflow_events)
        .unwrap_or_else(|_| "[]".to_string());
    let workflow_metadata_json =
        serde_json::to_string(&workflow).unwrap_or_else(|_| "{}".to_string());
    let template_label = workflow_response_template_label(message);
    let template_instruction = workflow_template_instruction_for_label(template_label);
    let detail_style = if workflow_user_prefers_deep_dive(message) {
        "detailed"
    } else {
        "concise"
    };
    let system_prompt = clean_text(
        &format!(
            "{}\n\nHardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Use the recorded evidence. If a tool failed, timed out, was blocked, or returned low-signal output, say that plainly in your own words. Never emit raw telemetry, placeholder copy, inline `<function=...>` markup, or pretend a failed tool succeeded.\n\nFinal-answer contract (final_answer_contract_v1): (1) answer the user's request in the first 1-2 sentences, (2) do not echo/restate the user prompt as your response, (3) do not include placeholder copy, (4) include source tags for key claims using `[source:local_context]` or `[source:tool_receipt:<id>]`.\n\nResponse template class: {}. {} Style: {} by default unless user requested a deep dive.",
            AGENT_RUNTIME_SYSTEM_PROMPT, template_label, template_instruction, detail_style
        ),
        12_000,
    );
    let user_prompt = clean_text(
        &format!(
            "User request:\n{message}\n\nCurrent draft response:\n{}\n\nWorkflow metadata:\n{workflow_metadata_json}\n\nRecorded tool outcomes:\n{tool_rows_json}\n\nWorkflow events:\n{workflow_events_json}\n\nWrite the final assistant response now.",
            if clean_text(draft_response, 2_000).is_empty() {
                "(empty)"
            } else {
                draft_response
            }
        ),
        20_000,
    );
    let coherence_window_messages = 2usize;
    let recent_context = active_messages
        .iter()
        .rev()
        .take(coherence_window_messages)
        .filter_map(|row| {
            let text = clean_text(
                row.get("text")
                    .or_else(|| row.get("content"))
                    .or_else(|| row.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                320,
            );
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let max_attempts: u64 = 2;
    let mut last_error = String::new();
    let mut last_invalid_excerpt = String::new();
    let mut last_reject_reason = String::new();
    let has_structured_block_evidence = response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        let tool_type = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        blocked
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || tool_type == "tool_pre_gate_blocked"
            || error.contains("nexus_delivery_denied")
            || error.contains("tool_permission_denied")
            || row
                .get("status_code")
                .and_then(Value::as_i64)
                .or_else(|| row.get("http_status").and_then(Value::as_i64))
                .map(|code| matches!(code, 401 | 403 | 404 | 422 | 429))
                .unwrap_or(false)
    });
    workflow["quality_telemetry"] = json!({
        "off_topic_reject": 0,
        "deferred_reply_reject": 0,
        "alignment_reject": 0,
        "prompt_echo_reject": 0,
        "unsourced_claim_reject": 0,
        "direct_answer_reject": 0,
        "unexpected_state_loop_reject": 0,
        "legacy_retry_template_detected": 0,
        "repeated_fallback_loop_detected": 0,
        "meta_control_tool_block": workflow
            .pointer("/tool_gate/meta_control_message")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && response_tools.is_empty(),
        "final_fallback_used": false
    });
    if response_contains_unexpected_state_retry_boilerplate(draft_response)
        || response_contains_unexpected_state_retry_boilerplate(latest_assistant_text)
    {
        bump_workflow_quality_counter(&mut workflow, "legacy_retry_template_detected");
    }
    let recent_retry_loop_detected = recent_assistant_retry_loop_detected(active_messages);
    if recent_retry_loop_detected {
        bump_workflow_quality_counter(&mut workflow, "repeated_fallback_loop_detected");
    }
    workflow["final_llm_response"]["attempted"] = Value::Bool(true);
    workflow["final_llm_response"]["max_attempts"] = json!(max_attempts);
    workflow["final_llm_response"]["coherence_window_messages"] =
        json!(coherence_window_messages);
    for attempt in 1..=max_attempts {
        workflow["final_llm_response"]["attempt_count"] = json!(attempt);
        let attempt_user_prompt = if attempt > 1 {
            clean_text(
                &format!(
                    "{}\n\nCorrection for attempt {} of {}: your previous answer did not complete the workflow because it tried to start another search, deferred the answer, emitted inline tool markup, or drifted away from the latest user request. Do not ask to retry, rerun, narrow the query, fetch another source, or emit `<function=...>` calls. Keep high lexical/semantic alignment to the latest user request and recent conversation context. Using only the recorded tool outcomes and workflow events above, explain what happened in your own words and tell the user what the tool actually returned.",
                    user_prompt, attempt, max_attempts
                ),
                20_000,
            )
        } else {
            user_prompt.clone()
        };
        match crate::dashboard_provider_runtime::invoke_chat(
            root,
            &cleaned_provider,
            &cleaned_model,
            &system_prompt,
            active_messages,
            &attempt_user_prompt,
        ) {
            Ok(retried) => {
                let mut retried_text = clean_chat_text(
                    retried
                        .get("response")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    32_000,
                );
                retried_text = sanitize_workflow_final_response_candidate(
                    &strip_internal_cache_control_markup(
                        &strip_internal_context_metadata_prefix(&retried_text),
                    ),
                );
                if !user_requested_internal_runtime_details(message) {
                    retried_text = abstract_runtime_mechanics_terms(&retried_text);
                }
                let deferred_reply = response_is_deferred_execution_preamble(&retried_text)
                    || response_is_deferred_retry_prompt(&retried_text)
                    || workflow_response_requests_more_tooling(&retried_text);
                let off_topic_reply = response_is_unrelated_context_dump(message, &retried_text);
                let low_alignment_reply = response_low_alignment_with_turn_context(
                    message,
                    &recent_context,
                    &retried_text,
                );
                let prompt_echo_reply = response_prompt_echo_detected(message, &retried_text);
                let receipt_mapped_sources = response_tools
                    .iter()
                    .any(|row| !response_tool_receipt_id(row).is_empty());
                let missing_evidence_tags = !response_tools.is_empty()
                    && !receipt_mapped_sources
                    && !response_has_evidence_tags(&retried_text);
                let missing_direct_answer = !response_answers_user_early(message, &retried_text);
                let direct_answer_in_first_two_sentences = !missing_direct_answer;
                let rejects_base_contract = response_fails_base_final_answer_contract(&retried_text);
                let rejects_speculative_blocker =
                    response_contains_speculative_web_blocker_language(&retried_text)
                        && !has_structured_block_evidence;
                let reject_checks = [
                    (deferred_reply, "deferred_reply", "deferred_reply_reject"),
                    (off_topic_reply, "off_topic_reply", "off_topic_reject"),
                    (low_alignment_reply, "low_alignment_reply", "alignment_reject"),
                    (prompt_echo_reply, "prompt_echo_reply", "prompt_echo_reject"),
                    (
                        missing_direct_answer,
                        "missing_direct_answer_reply",
                        "direct_answer_reject",
                    ),
                    (retried_text.is_empty(), "empty_reply", ""),
                    (
                        response_is_no_findings_placeholder(&retried_text),
                        "placeholder_reply",
                        "",
                    ),
                    (
                        response_contains_unexpected_state_retry_boilerplate(&retried_text),
                        "unexpected_state_retry_boilerplate",
                        "unexpected_state_loop_reject",
                    ),
                    (
                        response_looks_like_tool_ack_without_findings(&retried_text),
                        "ack_only_reply",
                        "",
                    ),
                    (
                        rejects_speculative_blocker || rejects_base_contract,
                        "invalid_reply",
                        "",
                    ),
                ];
                let (reject_reason, reject_counter) = reject_checks
                    .into_iter()
                    .find(|(should_reject, _, _)| *should_reject)
                    .map(|(_, reason, counter)| (reason, counter))
                    .unwrap_or(("", ""));
                if !reject_reason.is_empty() {
                    if !reject_counter.is_empty() {
                        bump_workflow_quality_counter(&mut workflow, reject_counter);
                    }
                    last_reject_reason = reject_reason.to_string();
                    last_invalid_excerpt = first_sentence(&retried_text, 240);
                    continue;
                }
                workflow["final_llm_response"]["used"] = Value::Bool(true);
                workflow["final_llm_response"]["status"] =
                    Value::String("synthesized".to_string());
                set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
                workflow["final_llm_response"]["helpfulness"] = json!({
                    "direct_answer_in_first_two_sentences": direct_answer_in_first_two_sentences,
                    "prompt_echo_detected": response_prompt_echo_detected(message, &retried_text),
                    "has_evidence_tags": response_has_evidence_tags(&retried_text)
                        || receipt_mapped_sources
                        || response_tools.is_empty(),
                    "missing_evidence_mapping": missing_evidence_tags,
                    "template_label": template_label,
                    "detail_style": detail_style
                });
                let attempt_count = attempt as f64;
                let off_topic_reject =
                    response_workflow_quality_rate(&workflow, "off_topic_reject");
                let direct_answer_rate = if direct_answer_in_first_two_sentences {
                    1.0
                } else {
                    0.0
                };
                let retry_rate = if max_attempts > 1 {
                    ((attempt.saturating_sub(1)) as f64 / (max_attempts.saturating_sub(1)) as f64)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let off_topic_reject_rate = if attempt_count > 0.0 {
                    (off_topic_reject / attempt_count).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                workflow["quality_telemetry"]["direct_answer_rate"] = json!(direct_answer_rate);
                workflow["quality_telemetry"]["retry_rate"] = json!(retry_rate);
                workflow["quality_telemetry"]["off_topic_reject_rate"] = json!(off_topic_reject_rate);
                workflow["response"] = Value::String(retried_text);
                return workflow;
            }
            Err(err) => {
                last_error = clean_text(&err, 240);
            }
        }
    }
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    if !last_invalid_excerpt.is_empty() {
        workflow["final_llm_response"]["status"] = Value::String("synthesis_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "synthesis_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_invalid_excerpt.clone());
        if !last_reject_reason.is_empty() {
            workflow["final_llm_response"]["last_reject_reason"] =
                Value::String(last_reject_reason.clone());
        }
    } else {
        workflow["final_llm_response"]["status"] = Value::String("invoke_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_error);
    }
    if should_force_direct_workflow_fallback(
        &last_reject_reason,
        &last_invalid_excerpt,
        latest_assistant_text,
        response_tools,
        recent_retry_loop_detected,
    ) {
        let fallback_response =
            workflow_unexpected_state_user_fallback(message, latest_assistant_text, response_tools);
        if !fallback_response.is_empty() {
            let guarded_fallback = ensure_no_retry_boilerplate_copy(
                message,
                latest_assistant_text,
                response_tools,
                &fallback_response,
            );
            workflow["response"] = Value::String(guarded_fallback.clone());
            workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(true);
            workflow["final_llm_response"]["used"] = Value::Bool(true);
            workflow["final_llm_response"]["status"] =
                Value::String("fallback_direct_answer".to_string());
            workflow["final_llm_response"]["fallback_response"] = Value::String(guarded_fallback);
            workflow["final_llm_response"]["fallback_source"] =
                Value::String("generated".to_string());
            workflow["final_llm_response"]["fallback_from_existing_draft"] = Value::Bool(false);
            workflow["final_llm_response"]["error"] = Value::Null;
            workflow["final_llm_response"]["last_reject_reason"] =
                Value::String("fallback_applied".to_string());
            mark_workflow_fallback_guard_reason(
                &mut workflow,
                "forced_fallback_after_synthesis_failure",
                "forced_synthesis_failure_fallback",
            );
            set_turn_workflow_final_stage_status(&mut workflow, "fallback_direct_answer");
        }
    }
    apply_final_retry_boilerplate_guard(
        &mut workflow,
        message,
        latest_assistant_text,
        response_tools,
    );
    apply_final_response_presence_guard(
        &mut workflow,
        message,
        latest_assistant_text,
        response_tools,
    );
    workflow
}

#[cfg(test)]
mod workflow_fallback_tests {
    use super::*;

    #[test]
    fn workflow_fallback_allowlist_includes_skipped_not_required() {
        let workflow = json!({
            "final_llm_response": {
                "status": "skipped_not_required"
            }
        });
        assert!(workflow_final_response_allows_system_fallback(&workflow));
    }

    #[test]
    fn workflow_unexpected_state_fallback_mentions_policy_gate_for_blocked_tools() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let message = "So is it too strict or what?";
        let response = workflow_unexpected_state_user_fallback(message, "", &tools);
        assert!(response.to_lowercase().contains("policy"));
        assert!(response.to_lowercase().contains("direct"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_explains_tool_routing_authority_for_policy_blocked_turns() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "but the system shouldnt be doing automatic tool calls at all right?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("llm-controlled"));
        assert!(lowered.contains("policy gate"));
        assert!(lowered.contains("direct-answer"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_explains_tool_routing_authority_without_policy_block() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "the llm should choose tool routing, right?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("llm-controlled"));
        assert!(lowered.contains("direct-answer mode"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_policy_blocked_workspace_action_is_actionable() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "try looking at the local directory",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("workspace/file tool call"));
        assert!(lowered.contains("workspace-read-enabled lane"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_workspace_action_is_actionable() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = workflow_unexpected_state_user_fallback(
            "can you list files in the workspace path",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("workspace/file action"));
        assert!(lowered.contains("workspace-read permission"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_hardcoded_check_is_explicit_for_policy_blocked_turns() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "are you hardlocked to this kind of response? or is this a hard coded system response?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("not a hard-coded response lock"));
        assert!(lowered.contains("policy gate"));
        assert!(lowered.contains("direct answer"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_hardcoded_check_is_explicit_without_policy_block() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "is this a hard coded system response?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("not a hard-coded response lock"));
        assert!(lowered.contains("workflow fallback guard"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_improvement_plan_is_actionable_when_policy_blocked() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "what do we need to do to improve the system?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("top improvements"));
        assert!(lowered.contains("llm-controlled"));
        assert!(lowered.contains("policy-denied"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_improvement_plan_is_actionable_without_policy_block() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "what would make the system better?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("top improvements"));
        assert!(lowered.contains("llm-controlled"));
        assert!(lowered.contains("boilerplate"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_patch_effectiveness_is_actionable_when_policy_blocked() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "are the patches working?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("partial improvement"));
        assert!(lowered.contains("policy gate"));
        assert!(lowered.contains("direct-answer mode"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_patch_effectiveness_is_actionable_without_policy_block() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(
            "are you able to notice any improvements on your end?",
            latest,
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("improvement is active"));
        assert!(lowered.contains("fallback guard"));
        assert!(lowered.contains("direct-answer mode"));
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_policy_block_plain_direct_reply_is_explicit() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = workflow_unexpected_state_user_fallback(
            "just answer the question",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("answer directly from current context"));
        assert!(lowered.contains("policy gate"));
        assert!(lowered.contains("direct-answer mode"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_plain_direct_reply_without_policy_is_explicit()
     {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response =
            workflow_unexpected_state_user_fallback("just answer the question", "", &tools);
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("plain direct answer"));
        assert!(lowered.contains("current context"));
        assert!(!lowered.contains("workflow finalization edge"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_workspace_action_without_policy_is_actionable()
     {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response = workflow_unexpected_state_user_fallback(
            "try looking at the local directory",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("workspace/file check"));
        assert!(lowered.contains("exact path"));
        assert!(lowered.contains("workspace root"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_patch_effectiveness_is_explicit_without_policy_block() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response = workflow_unexpected_state_user_fallback(
            "is it working now on your end?",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("fallback hardening is active"));
        assert!(lowered.contains("direct-answer mode"));
        assert!(lowered.contains("explicit opt-in"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_diagnostic_without_policy_is_explicit() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response =
            workflow_unexpected_state_user_fallback("what do you think is happening?", "", &tools);
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("workflow finalization edge"));
        assert!(lowered.contains("without a policy denial"));
        assert!(lowered.contains("direct answer"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_tool_routing_diagnostic_without_policy_is_explicit()
     {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response = workflow_unexpected_state_user_fallback(
            "the llm should choose tool routing right?",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("llm-controlled"));
        assert!(lowered.contains("without a policy-denied tool route"));
        assert!(lowered.contains("direct-answer mode"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_non_repeated_hardcoded_probe_without_policy_is_explicit() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let response = workflow_unexpected_state_user_fallback(
            "is this a hard coded system response?",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("not a hard-coded system response"));
        assert!(lowered.contains("without a policy denial"));
        assert!(lowered.contains("direct answer"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_breaks_repeated_legacy_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let message = "are you hardlocked to this kind of response?";
        let latest = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        let response = workflow_unexpected_state_user_fallback(message, latest, &tools);
        assert!(!response.contains("Please retry so I can rerun the chain cleanly."));
        assert!(response.to_lowercase().contains("not hard-locked"));
    }

    #[test]
    fn workflow_unexpected_state_fallback_prefers_human_ping_reply_for_greeting_only_messages() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = workflow_unexpected_state_user_fallback("hello", "", &tools);
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("tell me what you want next"));
        assert!(!lowered.contains("policy gate"));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_legacy_copy() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_next_actions_template() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.\n\nNext actions: 1) clarify the exact outcome you want 2) run one targeted tool call 3) return a concise answer from current context";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_paraphrased_macro_bundle() {
        let retry_boilerplate = "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_does_not_flag_plain_retry_offer() {
        let normal_text = "I can retry the query if you want, or I can answer directly from current context.";
        assert!(!response_contains_unexpected_state_retry_boilerplate(
            normal_text
        ));
    }

    #[test]
    fn workflow_unexpected_state_fallback_prefers_plain_reply_when_user_requests_direct_answer() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = workflow_unexpected_state_user_fallback(
            "just answer the question",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
        );
        assert!(response.to_lowercase().contains("plain direct answer"));
        assert!(!response.to_lowercase().contains("retry so i can rerun"));
    }

    #[test]
    fn force_direct_workflow_fallback_when_retry_boilerplate_reject_was_seen() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_force_direct_workflow_fallback(
            "unexpected_state_retry_boilerplate",
            "",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn force_direct_workflow_fallback_when_policy_block_tool_is_present() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        assert!(should_force_direct_workflow_fallback("", "", "", &tools, false));
    }

    #[test]
    fn force_direct_workflow_fallback_when_latest_reply_is_repeated_legacy_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_force_direct_workflow_fallback(
            "",
            "",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
            false,
        ));
    }

    #[test]
    fn force_direct_workflow_fallback_when_invalid_excerpt_has_retry_boilerplate() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_force_direct_workflow_fallback(
            "",
            "final reply did not render; please retry so i can rerun the chain cleanly",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_triggers_on_two_of_last_three_assistant_turns() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "user", "text": "what?"}),
            json!({"role": "assistant", "text": "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context."}),
            json!({"role": "assistant", "text": "Normal answer now."}),
        ];
        assert!(recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_ignores_single_retry_like_turn() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "assistant", "text": "I can answer directly from current context."}),
            json!({"role": "assistant", "text": "Here is the direct answer."}),
        ];
        assert!(!recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn force_direct_workflow_fallback_when_recent_loop_detected() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_force_direct_workflow_fallback(
            "",
            "",
            "normal latest",
            &tools,
            true
        ));
    }

    #[test]
    fn workflow_policy_block_fallback_returns_diagnostic_copy_when_requested() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = workflow_unexpected_state_user_fallback(
            "what do you think is happening?",
            "",
            &tools,
        );
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("likely root cause"));
        assert!(lowered.contains("policy gate"));
        assert!(lowered.contains("file_list"));
    }

    #[test]
    fn skipped_not_required_fallback_sanitizes_retry_boilerplate() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let (response, sanitized, source) = sanitize_skipped_final_response_fallback_response(
            "hello",
            "",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
        );
        assert!(sanitized);
        assert_eq!(source, "generated");
        let lowered = response.to_ascii_lowercase();
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
        assert!(lowered.contains("direct answer"));
    }

    #[test]
    fn skipped_fallback_keeps_clean_text_when_no_retry_boilerplate_present() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let (response, sanitized, source) = sanitize_skipped_final_response_fallback_response(
            "status?",
            "All checks look healthy.",
            "",
            &tools,
        );
        assert!(!sanitized);
        assert_eq!(source, "draft");
        assert_eq!(response, "All checks look healthy.");
    }

    #[test]
    fn skipped_fallback_uses_direct_answer_when_draft_and_latest_are_empty() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let (response, sanitized, source) = sanitize_skipped_final_response_fallback_response(
            "hello",
            "",
            "",
            &tools,
        );
        assert!(sanitized);
        assert_eq!(source, "generated");
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("direct answer") || lowered.contains("direct response"));
    }

    #[test]
    fn ensure_no_retry_boilerplate_copy_rewrites_legacy_template() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let response = ensure_no_retry_boilerplate_copy(
            "hello",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
            "I completed the run, but the final reply did not render. Ask me to continue and I will synthesize from the recorded workflow state.",
        );
        let lowered = response.to_ascii_lowercase();
        assert!(!lowered.contains("please retry so i can rerun the chain cleanly"));
        assert!(!lowered.contains("ask me to continue and i will synthesize"));
        assert!(lowered.contains("direct answer") || lowered.contains("direct response"));
    }

    #[test]
    fn ensure_no_retry_boilerplate_copy_breaks_exact_repeat_of_latest_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "The prior tool step was blocked by a local policy gate. I can still answer directly from current context without another tool call.";
        let response = ensure_no_retry_boilerplate_copy("status?", latest, &tools, latest);
        assert!(!response.eq_ignore_ascii_case(latest));
        assert!(response.to_ascii_lowercase().contains("answering directly"));
    }

    #[test]
    fn response_repeat_detector_catches_near_duplicate_formatting_variants() {
        let latest = "I'm not hard-locked. The previous fallback repeated, so I'm switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one.";
        let response = "Im not hard locked - the previous fallback repeated so im switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one";
        assert!(response_repeats_latest_assistant_copy(response, latest));
    }

    #[test]
    fn ensure_no_retry_boilerplate_copy_breaks_near_duplicate_latest_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        let latest = "The previous fallback repeated, so I'm switching to a stable direct-answer path now and keeping tools off unless you explicitly request one.";
        let candidate = "The previous fallback repeated so Im switching to a stable direct answer path now and keeping tools off unless you explicitly request one";
        let response = ensure_no_retry_boilerplate_copy("status?", latest, &tools, candidate);
        assert!(!response.eq_ignore_ascii_case(candidate));
        assert!(response.to_ascii_lowercase().contains("answering directly"));
    }

    #[test]
    fn ensure_no_retry_boilerplate_copy_uses_last_resort_variant_when_alternate_still_repeats() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "Answering directly from current context now. The last tool attempt was policy-blocked, and I'll keep tools off unless you explicitly request one.";
        let response = ensure_no_retry_boilerplate_copy("status?", latest, &tools, latest);
        assert!(!response.eq_ignore_ascii_case(latest));
        let lowered = response.to_ascii_lowercase();
        assert!(
            lowered.contains("direct answer mode is active")
                || lowered.contains("direct-answer path is active")
        );
    }

    #[test]
    fn workflow_non_repeating_last_resort_reply_rotates_to_non_matching_variant() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        let latest = "Direct answer mode is active. Prior tool execution was policy-blocked, and I will continue from current context unless you explicitly request a tool.";
        let response = workflow_non_repeating_last_resort_reply(
            "what happened?",
            latest,
            &tools,
        );
        assert!(!response.eq_ignore_ascii_case(latest));
        assert!(response.to_ascii_lowercase().contains("direct"));
    }

    #[test]
    fn apply_skipped_fallback_marks_used_true_when_response_present() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        apply_skipped_final_response_fallback(&mut workflow, "Direct response", true, "generated");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_from_existing_draft")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("generated")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reason")
                .and_then(Value::as_str),
            Some("skipped_fallback_retry_sanitized")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_last_stage")
                .and_then(Value::as_str),
            Some("skipped_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reasons/0")
                .and_then(Value::as_str),
            Some("skipped_fallback_retry_sanitized")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/0/reason")
                .and_then(Value::as_str),
            Some("skipped_fallback_retry_sanitized")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/0/stage")
                .and_then(Value::as_str),
            Some("skipped_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/0")
                .and_then(Value::as_str),
            Some("skipped_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/distinct_reason_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/distinct_stage_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_skipped_fallback")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_reason_skipped_fallback_retry_sanitized")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn apply_skipped_fallback_marks_used_false_when_response_absent() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        apply_skipped_final_response_fallback(&mut workflow, "", false, "empty");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("empty")
        );
    }

    #[test]
    fn apply_skipped_fallback_marks_existing_draft_source_truthfully() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        apply_skipped_final_response_fallback(&mut workflow, "Draft response", false, "draft");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_from_existing_draft")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn final_retry_boilerplate_guard_rewrites_response_and_sets_metadata() {
        let mut workflow = json!({
            "response": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        apply_final_retry_boilerplate_guard(
            &mut workflow,
            "hello",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(!response.contains("please retry so i can rerun the chain cleanly"));
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("fallback_direct_answer")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("generated_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reason")
                .and_then(Value::as_str),
            Some("retry_boilerplate_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_last_stage")
                .and_then(Value::as_str),
            Some("final_retry_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_final_retry_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_reason_retry_boilerplate_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn final_response_presence_guard_fills_empty_response_and_sets_metadata() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        apply_final_response_presence_guard(&mut workflow, "hello", "", &tools);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty());
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("fallback_direct_answer")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("generated_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reason")
                .and_then(Value::as_str),
            Some("empty_response_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_last_stage")
                .and_then(Value::as_str),
            Some("final_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reasons/0")
                .and_then(Value::as_str),
            Some("empty_response_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/0/stage")
                .and_then(Value::as_str),
            Some("final_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/0")
                .and_then(Value::as_str),
            Some("final_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_final_presence_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_reason_empty_response_presence_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn mark_workflow_fallback_guard_reason_tracks_history_and_counter() {
        let mut workflow = json!({
            "final_llm_response": {},
            "quality_telemetry": {}
        });
        mark_workflow_fallback_guard_reason(
            &mut workflow,
            "retry_boilerplate_guard",
            "final_retry_guard",
        );
        mark_workflow_fallback_guard_reason(
            &mut workflow,
            "empty_response_presence_guard",
            "final_presence_guard",
        );
        mark_workflow_fallback_guard_reason(
            &mut workflow,
            "retry_boilerplate_guard",
            "forced_synthesis_failure_fallback",
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reason")
                .and_then(Value::as_str),
            Some("retry_boilerplate_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_last_stage")
                .and_then(Value::as_str),
            Some("forced_synthesis_failure_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_reasons/1")
                .and_then(Value::as_str),
            Some("empty_response_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/1/stage")
                .and_then(Value::as_str),
            Some("final_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_events/2/stage")
                .and_then(Value::as_str),
            Some("forced_synthesis_failure_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/1")
                .and_then(Value::as_str),
            Some("final_presence_guard")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_stages/2")
                .and_then(Value::as_str),
            Some("forced_synthesis_failure_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/distinct_reason_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/distinct_stage_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/severity")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("high_trigger_or_stage_diversity")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_guard_summary/recommended_action")
                .and_then(Value::as_str),
            Some("operator_review_recommended")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_final_retry_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_final_presence_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_stage_forced_synthesis_failure_fallback")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_reason_retry_boilerplate_guard")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/fallback_guard_reason_empty_response_presence_guard")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn workflow_fallback_guard_stage_counter_key_sanitizes_non_alnum_stage_tokens() {
        assert_eq!(
            workflow_fallback_guard_stage_counter_key("Final Presence Guard!!"),
            "fallback_guard_stage_final_presence_guard"
        );
        assert_eq!(
            workflow_fallback_guard_stage_counter_key("___"),
            "fallback_guard_stage_unknown"
        );
    }

    #[test]
    fn workflow_fallback_guard_reason_counter_key_sanitizes_non_alnum_reason_tokens() {
        assert_eq!(
            workflow_fallback_guard_reason_counter_key("Retry Boilerplate Guard!!"),
            "fallback_guard_reason_retry_boilerplate_guard"
        );
        assert_eq!(
            workflow_fallback_guard_reason_counter_key("___"),
            "fallback_guard_reason_unknown"
        );
    }

    #[test]
    fn workflow_fallback_guard_summary_classification_escalates_with_counts() {
        assert_eq!(
            workflow_fallback_guard_summary_classification(1, 1),
            ("low", false, "single_guard_activation", "continue_direct_mode")
        );
        assert_eq!(
            workflow_fallback_guard_summary_classification(2, 1),
            (
                "moderate",
                false,
                "repeated_or_multi_stage_guard_activity",
                "monitor_and_continue_direct_mode",
            )
        );
        assert_eq!(
            workflow_fallback_guard_summary_classification(1, 3),
            (
                "high",
                true,
                "high_trigger_or_stage_diversity",
                "operator_review_recommended",
            )
        );
    }

    #[test]
    fn final_response_presence_guard_does_not_override_non_empty_response() {
        let mut workflow = json!({
            "response": "Answer already present.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "synthesized"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        apply_final_response_presence_guard(&mut workflow, "hello", "", &tools);
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some("Answer already present.")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
    }
}
