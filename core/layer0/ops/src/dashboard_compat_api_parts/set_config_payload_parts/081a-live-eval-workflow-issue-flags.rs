fn live_eval_json_u64_at(value: &Value, pointers: &[&str]) -> u64 {
    for pointer in pointers {
        if let Some(raw) = value.pointer(pointer) {
            if let Some(num) = raw.as_u64() {
                return num;
            }
            if let Some(text) = raw
                .as_str()
                .and_then(|text| text.trim().parse::<u64>().ok())
            {
                return text;
            }
        }
    }
    0
}

fn live_eval_pending_tool(response_finalization: &Value) -> bool {
    response_finalization
        .pointer("/pending_tool_request/status")
        .and_then(Value::as_str)
        == Some("pending_confirmation")
}

fn live_eval_has_tool_result(response_finalization: &Value) -> bool {
    response_finalization
        .pointer("/tool_completion/tool_attempts")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || response_finalization
            .pointer("/tool_completion/status")
            .and_then(Value::as_str)
            .map(|status| matches!(status, "ok" | "success" | "error" | "blocked"))
            .unwrap_or(false)
}

fn live_eval_response_has_gate_token_leakage(response: &str) -> bool {
    let normalized = normalize_placeholder_signature(response);
    let lowered = response.to_ascii_lowercase();
    normalized.starts_with("yes. tool family:")
        || normalized.starts_with("yes. tool:")
        || normalized.starts_with("category:")
        || normalized == "respond directly"
        || normalized == format!("direct answer {}", "conversation")
        || normalized == "planning from current context"
        || normalized == "web research"
        || normalized == "workspace files"
        || normalized.starts_with("no. answer directly")
        || normalized.starts_with("no. i would")
        || normalized.contains("need tools? yes/no")
        || normalized.contains("what kind of work is this")
        || lowered.contains("need tools? yes/no")
        || lowered.contains("what kind of work is this?")
}

fn live_eval_response_is_unresolved_tool_intent(response: &str) -> bool {
    let lowered = clean_text(response, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let says_would_choose = lowered.contains("i would choose")
        || lowered.contains("i would run")
        || lowered.contains("i would use")
        || lowered.contains("i would call");
    let names_tool = lowered.contains("batch_query")
        || lowered.contains("batch query")
        || lowered.contains("web search")
        || lowered.contains("file tool")
        || lowered.contains("workspace tool")
        || lowered.contains("agent management");
    says_would_choose && names_tool
}

fn live_eval_hidden_second_pass(response_finalization: &Value, system_fallback: bool) -> bool {
    system_fallback
        || response_finalization
            .pointer("/final_llm_response/attempt_count")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            > 1
        || response_finalization
            .pointer("/final_llm_response/fallback_guard_multi_stage")
            .and_then(Value::as_bool)
            == Some(true)
}

fn live_eval_workflow_issue_events(
    agent_id: &str,
    message: &str,
    response: &str,
    previous_assistant: &str,
    response_finalization: &Value,
    system_fallback: bool,
) -> Vec<Value> {
    let final_text = clean_text(response, 2_400);
    let normalized = normalize_placeholder_signature(&final_text);
    let pending_tool = live_eval_pending_tool(response_finalization);
    let mut events = Vec::<Value>::new();
    if normalized.is_empty() && !pending_tool {
        events.push(live_eval_issue_event(
            agent_id,
            "empty_direct_reply",
            "high",
            "Live eval saw an empty direct reply without a pending tool.",
            message,
            response,
        ));
    }
    let gate_prompt_count = final_text
        .to_ascii_lowercase()
        .matches("need tools? yes/no")
        .count()
        + final_text
            .to_ascii_lowercase()
            .matches("what kind of work is this?")
            .count()
        + normalized.matches("need tools yes no").count()
        + normalized.matches("what kind of work is this").count();
    if gate_prompt_count > 1
        || (!previous_assistant.trim().is_empty()
            && normalize_placeholder_signature(previous_assistant) == normalized
            && (normalized.contains("need tools yes no")
                || normalized.contains("what kind of work is this")
                || final_text
                    .to_ascii_lowercase()
                    .contains("need tools? yes/no")
                || final_text
                    .to_ascii_lowercase()
                    .contains("what kind of work is this?")))
    {
        events.push(live_eval_issue_event(
            agent_id,
            "repeated_gate_prompt",
            "high",
            "Live eval saw a repeated workflow gate prompt.",
            message,
            response,
        ));
    }
    if live_eval_response_has_gate_token_leakage(&final_text) {
        events.push(live_eval_issue_event(
            agent_id,
            "gate_token_leakage",
            "high",
            "Live eval saw workflow gate tokens leak into visible chat.",
            message,
            response,
        ));
    }
    if live_eval_response_is_unresolved_tool_intent(&final_text) {
        events.push(live_eval_issue_event(
            agent_id,
            "unresolved_tool_intent_final",
            "high",
            "Live eval saw a final response describe a tool choice instead of submitting or completing the workflow gate.",
            message,
            response,
        ));
    }
    if response_contains_stale_code_context_dump(message, &final_text) {
        events.push(live_eval_issue_event(
            agent_id,
            "visible_stale_code_context_dump",
            "high",
            "Live eval saw unrelated source-code context exposed in the visible assistant response.",
            message,
            response,
        ));
    }
    if system_fallback {
        events.push(live_eval_issue_event(
            agent_id,
            "system_fallback_in_chat",
            "high",
            "Live eval saw system fallback usage in finalization.",
            message,
            response,
        ));
    }
    if live_eval_hidden_second_pass(response_finalization, system_fallback) {
        events.push(live_eval_issue_event(
            agent_id,
            "hidden_second_pass_call",
            "high",
            "Live eval saw evidence of a hidden second-pass workflow call.",
            message,
            response,
        ));
    }
    let latency_ms = live_eval_json_u64_at(
        response_finalization,
        &[
            "/latency_ms",
            "/duration_ms",
            "/elapsed_ms",
            "/tool_completion/latency_ms",
        ],
    );
    if pending_tool && latency_ms > 30_000 {
        events.push(live_eval_issue_event(
            agent_id,
            "pending_tool_stuck_too_long",
            "warn",
            "Live eval saw a pending tool request remain stuck beyond the workflow budget.",
            message,
            response,
        ));
    }
    let final_status = response_finalization
        .pointer("/final_llm_response/status")
        .and_then(Value::as_str);
    let known_unsynthesized = final_status
        .map(|status| status != "synthesized")
        .unwrap_or(false);
    if live_eval_has_tool_result(response_finalization)
        && (known_unsynthesized || normalized.is_empty())
    {
        events.push(live_eval_issue_event(
            agent_id,
            "tool_result_without_synthesis",
            "high",
            "Live eval saw tool results without an LLM-authored synthesis.",
            message,
            response,
        ));
    }
    let web_required_without_attempt = response_finalization
        .pointer("/web_invariant/requires_live_web")
        .and_then(Value::as_bool)
        == Some(true)
        && response_finalization
            .pointer("/web_invariant/tool_attempted")
            .and_then(Value::as_bool)
            != Some(true)
        && !pending_tool;
    let tooling_required_without_attempt = response_finalization
        .pointer("/tooling_invariant/classification")
        .and_then(Value::as_str)
        == Some("parse_failed")
        && response_finalization
            .pointer("/tooling_invariant/tool_attempted")
            .and_then(Value::as_bool)
            != Some(true)
        && !pending_tool;
    if web_required_without_attempt || tooling_required_without_attempt {
        events.push(live_eval_issue_event(
            agent_id,
            "required_tool_without_attempt",
            "high",
            "Live eval saw a tool-required turn finish without a tool attempt or pending tool request.",
            message,
            response,
        ));
    }
    events
}
