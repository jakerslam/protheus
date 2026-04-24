const CONVERSATION_BYPASS_MAX_TURNS: u64 = 3;

fn workflow_turn_contains_any(lowered: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| lowered.contains(marker))
}

fn message_requests_conversation_bypass(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "break the workflow",
            "bypass the workflow",
            "workflow bypass",
            "respond directly",
            "direct mode",
            "talk freely",
            "no workflow",
            "skip workflow",
        ],
    )
}

fn message_requests_conversation_bypass_disable(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "resume workflow",
            "restore workflow",
            "turn workflow back on",
            "re-enable workflow",
            "enable workflow",
            "use normal workflow",
        ],
    )
}

fn message_requests_high_risk_external_action(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "send email",
            "send an email",
            "tweet",
            "post publicly",
            "publish",
            "deploy to production",
            "drop database",
            "delete production",
            "exfiltrate",
            "leak secrets",
        ],
    )
}

fn value_as_u64_like(value: Option<&Value>) -> u64 {
    value
        .and_then(|row| {
            row.as_u64()
                .or_else(|| row.as_i64().map(|v| v.max(0) as u64))
        })
        .unwrap_or(0)
}

fn latest_assistant_conversation_bypass_remaining_turns(active_messages: &[Value]) -> u64 {
    for row in active_messages.iter().rev() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
            .to_ascii_lowercase();
        if role != "assistant" && role != "agent" {
            continue;
        }
        let from_finalization = value_as_u64_like(row.pointer(
            "/response_finalization/workflow_control/conversation_bypass/remaining_turns_after",
        ));
        if from_finalization > 0 {
            return from_finalization;
        }
        let from_workflow = value_as_u64_like(row.pointer(
            "/response_workflow/workflow_control/conversation_bypass/remaining_turns_after",
        ));
        if from_workflow > 0 {
            return from_workflow;
        }
    }
    0
}

fn workflow_conversation_bypass_control_for_turn(
    message: &str,
    active_messages: &[Value],
    inline_tools_allowed: bool,
) -> Value {
    let requested_enable = message_requests_conversation_bypass(message);
    let requested_disable = message_requests_conversation_bypass_disable(message);
    let previous_remaining = latest_assistant_conversation_bypass_remaining_turns(active_messages);
    let retired_sticky_state_seen = previous_remaining > 0;
    let explicit_tool_request = inline_tool_calls_allowed_for_user_message(message)
        && !message_explicitly_disallows_tool_calls(message);
    let high_risk_external_action = message_requests_high_risk_external_action(message);

    json!({
        "enabled": false,
        "source": "retired",
        "reason": "direct_response_uses_gate_1_false",
        "blocked": false,
        "block_reason": "",
        "requested_enable": requested_enable,
        "requested_disable": requested_disable,
        "sticky_requested": retired_sticky_state_seen,
        "explicit_tool_request": explicit_tool_request,
        "gate_is_advisory": false,
        "inline_tools_allowed": inline_tools_allowed,
        "high_risk_external_action": high_risk_external_action,
        "requested_ttl_turns": CONVERSATION_BYPASS_MAX_TURNS,
        "remaining_turns_before": previous_remaining,
        "remaining_turns_after": 0,
        "workflow_mode_override": "",
        "should_emit_event": false
    })
}

fn workflow_turn_is_meta_control_message(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "that was just a test",
            "just a test",
            "just testing",
            "test only",
            "ignore that",
            "never mind",
            "nm",
            "thanks",
            "thank you",
            "cool",
            "sounds good",
            "did you try it",
            "did you do it",
            "what happened",
        ],
    ) && !workflow_turn_contains_any(
        &lowered,
        &[
            "search", "web", "online", "internet", "file", "patch", "edit", "update", "create",
            "read", "memory", "repo", "codebase",
        ],
    )
}

fn workflow_turn_tool_decision_tree(message: &str) -> Value {
    let canonical_gate = crate::app_plane::chat_ui_turn_tool_decision_tree(message);
    let requires_file_mutation = canonical_gate
        .get("requires_file_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_local_lookup = canonical_gate
        .get("requires_local_lookup")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_live_web = canonical_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let explicit_web_intent = canonical_gate
        .get("explicit_web_intent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_sufficient_information = canonical_gate
        .get("has_sufficient_information")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let should_call_tools = canonical_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let gate_decision_mode = clean_text(
        canonical_gate
            .get("gate_decision_mode")
            .and_then(Value::as_str)
            .unwrap_or("manual_need_tool_access"),
        40,
    );
    let reason_code = clean_text(
        canonical_gate
            .get("reason_code")
            .and_then(Value::as_str)
            .unwrap_or("manual_menu_presented"),
        80,
    );
    let info_source = clean_text(
        canonical_gate
            .get("info_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        24,
    );
    let selected_tool_family = clean_text(
        canonical_gate
            .get("selected_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("unselected"),
        40,
    );
    let meta_control = canonical_gate
        .get("meta_control_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status_check = canonical_gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let meta_diagnostic_request = canonical_gate
        .get("meta_diagnostic_request")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let llm_should_answer_directly = canonical_gate
        .get("llm_should_answer_directly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let automatic_tool_calls_allowed = canonical_gate
        .get("automatic_tool_calls_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tool_selection_authority = clean_text(
        canonical_gate
            .get("tool_selection_authority")
            .and_then(Value::as_str)
            .unwrap_or("llm_submitted_menu_or_text_input"),
        32,
    );
    let decision_authority_mode = clean_text(
        canonical_gate
            .get("decision_authority_mode")
            .and_then(Value::as_str)
            .unwrap_or("llm_manual_only_v1"),
        40,
    );
    let gate_enforcement_mode = clean_text(
        canonical_gate
            .get("gate_enforcement_mode")
            .and_then(Value::as_str)
            .unwrap_or("disabled"),
        32,
    );
    let gate_is_advisory = canonical_gate
        .get("gate_is_advisory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let workflow_retry_limit = canonical_gate
        .get("workflow_retry_limit")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let needs_tool_access = canonical_gate
        .get("needs_tool_access")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let gate_prompt = clean_text(
        canonical_gate
            .get("gate_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Need tool access for this query? T/F"),
        120,
    );
    let tool_family_menu = canonical_gate
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tool_menu = canonical_gate
        .get("tool_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tool_menu_by_family = canonical_gate
        .get("tool_menu_by_family")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let manual_tool_selection = canonical_gate
        .get("manual_tool_selection")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let auto_decisions_disabled = canonical_gate
        .get("auto_decisions_disabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let manual_gate_mode = clean_text(
        canonical_gate
            .get("manual_gate_mode")
            .and_then(Value::as_str)
            .unwrap_or("llm_only_multiple_choice_v1"),
        60,
    );
    json!({
        "contract": "tool_decision_tree_v3",
        "workflow_gate_contract": "tool_menu_interface_v1",
        "gate_decision_mode": gate_decision_mode,
        "reason_code": reason_code,
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "llm_should_answer_directly": llm_should_answer_directly,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "gate_prompt": gate_prompt,
        "info_source": info_source,
        "selected_tool_family": selected_tool_family,
        "decision_authority_mode": decision_authority_mode,
        "gate_enforcement_mode": gate_enforcement_mode,
        "gate_is_advisory": gate_is_advisory,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "tool_menu_by_family": tool_menu_by_family,
        "manual_tool_selection": manual_tool_selection,
        "auto_decisions_disabled": auto_decisions_disabled,
        "manual_gate_mode": manual_gate_mode,
        "meta_control_message": meta_control,
        "status_check_message": status_check,
        "meta_diagnostic_request": meta_diagnostic_request,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit,
        "gates": {
            "gate_1": {
                "name": "needs_tool_access",
                "question": gate_prompt,
                "required": false,
                "selection_mode": "multiple_choice",
                "options": [
                    {"option": "F", "key": "no_tools", "label": "No tools; answer directly"},
                    {"option": "T", "key": "use_tool", "label": "Use a tool"}
                ],
                "reason_code": reason_code
            },
            "gate_2": {
                "name": "tool_family_selection",
                "tooling_default": "disabled",
                "selected_family": selected_tool_family,
                "selection_source": "llm_submission_only",
                "selection_mode": "multiple_choice",
                "family_menu": tool_family_menu
            },
            "gate_3": {
                "name": "tool_selection",
                "wait_for_tools": needs_tool_access,
                "skip_when_no_tools": !needs_tool_access,
                "selection_mode": "multiple_choice",
                "tool_menu_by_family": tool_menu_by_family
            },
            "gate_4": {
                "name": "request_payload_entry",
                "selection_mode": "text_input",
                "request_format_source": "selected_tool.request_format"
            },
            "gate_5": {
                "name": "post_tool_decision",
                "selection_mode": "multiple_choice",
                "options": [
                    {"option": 1, "key": "finish", "label": "Finish and synthesize"},
                    {"option": 2, "key": "another_tool", "label": "Run another tool"}
                ]
            },
            "gate_6": {
                "name": "final_output",
                "output_contract": "llm_authored_final_answer_only",
                "retry_limit": workflow_retry_limit
            }
        }
    })
}

fn workflow_library_prompt_context(message: &str, latent_tool_candidates: &[Value]) -> String {
    let _ = latent_tool_candidates;
    let tool_gate = workflow_turn_tool_decision_tree(message);
    let gate_prompt = clean_text(
        tool_gate
            .get("gate_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Need tool access for this query? T/F"),
        120,
    );
    let tool_family_menu = clean_text(
        &tool_gate
            .get("tool_family_menu")
            .cloned()
            .unwrap_or_else(|| json!([]))
            .to_string(),
        1_400,
    );
    let tool_menu_by_family = clean_text(
        &tool_gate
            .get("tool_menu_by_family")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string(),
        2_600,
    );
    clean_text(
        &format!(
            "Workflow interface only: present exactly one gate at a time; do not recommend, infer, classify, explain, or inject final chat text. Gate 1 multiple choice: `{gate_prompt}` options `F) no tools, answer directly` and `T) use a tool`. If F, answer normally. If T, present only the numbered family menu: {tool_family_menu}. Then present only the selected family's numbered tool menu from: {tool_menu_by_family}. Then present only the selected tool's request_format as a data-entry field. After tool results, present only `1) finish` or `2) another tool`; final synthesis is authored by the model only.",
        ),
        3_600,
    )
}

fn turn_workflow_requires_final_llm(
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> bool {
    if !response_tools.is_empty() || !workflow_events.is_empty() {
        return true;
    }
    let cleaned_draft = clean_text(draft_response, 4_000);
    if cleaned_draft.is_empty() {
        return true;
    }
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(&cleaned_draft, 6);
    if !inline_calls.is_empty()
        || without_inline_calls
            .to_ascii_lowercase()
            .contains("<function=")
    {
        return true;
    }
    if response_is_no_findings_placeholder(&cleaned_draft)
        || response_looks_like_tool_ack_without_findings(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        return true;
    }
    false
}

fn turn_workflow_stage_rows(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Vec<Value> {
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let _ = workflow_mode;
    let cleaned_draft = clean_text(draft_response, 2_000);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    vec![
        json!({
            "stage": "workflow_gate",
            "status": "enforced"
        }),
        json!({
            "stage": "initial_model_interpretation",
            "status": if cleaned_draft.is_empty() {
                "completed_empty"
            } else {
                "completed"
            },
            "draft_response_state": if cleaned_draft.is_empty() {
                "empty"
            } else if response_is_no_findings_placeholder(&cleaned_draft) {
                "no_findings"
            } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
                "ack_only"
            } else {
                "present"
            }
        }),
        json!({
            "stage": "tool_and_system_collection",
            "status": if response_tools.is_empty() && workflow_events.is_empty() {
                "no_external_events"
            } else {
                "collected"
            },
            "tool_count": response_tools.len(),
            "system_event_count": workflow_events.len()
        }),
        json!({
            "stage": "final_llm_response",
            "required": requires_final_llm,
            "status": final_stage_status
        }),
    ]
}

fn turn_workflow_metadata(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    message: &str,
) -> Value {
    let cleaned_draft = clean_text(draft_response, 4_000);
    let draft_response_state = if cleaned_draft.is_empty() {
        "empty"
    } else if response_is_no_findings_placeholder(&cleaned_draft) {
        "no_findings"
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        "ack_only"
    } else {
        "present"
    };
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let tool_gate = workflow_turn_tool_decision_tree(message);
    json!({
        "contract": "agent_workflow_library_v1",
        "workflow_gate": {
            "required": true,
            "status": "enforced"
        },
        "tool_gate": tool_gate,
        "library": {
            "default_workflow": default_turn_workflow_name(),
            "available_workflows": turn_workflow_library_catalog()
        },
        "selected_workflow": selected_turn_workflow(workflow_mode),
        "tool_count": response_tools.len(),
        "system_event_count": workflow_events.len(),
        "draft_response_state": draft_response_state,
        "findings_summary": clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000),
        "failure_summary": clean_text(&response_tools_failure_reason_for_user(response_tools, 4), 2_000),
        "workflow_control": {
            "mode": "tool_menu_interface_v1",
            "direct_response_path": "gate_1_false"
        },
        "system_events": workflow_events,
        "stage_statuses": turn_workflow_stage_rows(workflow_mode, response_tools, workflow_events, draft_response),
        "final_llm_response": {
            "required": requires_final_llm,
            "source": "workflow_post_synthesis"
        }
    })
}

fn set_turn_workflow_final_stage_status(workflow: &mut Value, status: &str) {
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response")
                .unwrap_or(false)
            {
                row["status"] = Value::String(clean_text(status, 80));
            }
        }
    }
}

fn workflow_response_requests_more_tooling(response: &str) -> bool {
    let lowered = clean_text(response, 800).to_ascii_lowercase();
    !lowered.is_empty()
        && [
            "i'll get you an update",
            "i will get you an update",
            "let me get you an update",
            "i'll look into",
            "i will look into",
            "let me look into",
            "i'll check",
            "i will check",
            "let me check",
            "working on it",
            "one moment",
            "stand by",
            "i'll report back",
            "i will report back",
            "let me search",
            "i'll search",
            "i will search",
            "would you like me to search",
            "would you like me to fetch",
            "search for more",
            "rerun with",
            "retry with",
            "narrower query",
            "specific source url",
            "need to search",
            "need targeted web research",
            "need more specific",
            "let me try",
            "i'll try",
            "i will try",
            "if you'd like, i can search",
            "if you would like, i can search",
            "if you'd like, i can fetch",
            "if you would like, i can fetch",
            "if you'd like, i can look deeper",
            "if you would like, i can look deeper",
            "more targeted approach",
            "another search",
            "technical documentation",
            "architecture details to enable",
        ]
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn strip_dangling_inline_tool_markup(text: &str) -> String {
    let mut cleaned = text.to_string();
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let Some(start) = lowered.find("<function=") else {
            break;
        };
        let tail = &cleaned[start..];
        let end_rel = tail
            .find("</function>")
            .map(|idx| idx + "</function>".len())
            .or_else(|| tail.find('\n'))
            .unwrap_or(tail.len());
        let end = start.saturating_add(end_rel).min(cleaned.len());
        if end <= start {
            break;
        }
        cleaned.replace_range(start..end, "");
    }
    cleaned.replace("</function>", "")
}

fn sanitize_workflow_final_response_candidate(response: &str) -> String {
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(response, 6);
    let candidate = if inline_calls.is_empty() {
        response
    } else {
        without_inline_calls.trim()
    };
    let mut cleaned = clean_chat_text(strip_dangling_inline_tool_markup(candidate).trim(), 32_000);
    let lowered = cleaned.to_ascii_lowercase();
    let cutoff = [
        "let me try",
        "i'll try",
        "i will try",
        "let me search",
        "i'll search",
        "i will search",
        "would you like me to search",
        "would you like me to fetch",
        "if you'd like, i can search",
        "if you would like, i can search",
        "if you'd like, i can fetch",
        "if you would like, i can fetch",
        "if you'd like, i can look deeper",
        "if you would like, i can look deeper",
    ]
    .iter()
    .filter_map(|marker| lowered.find(marker))
    .min();
    if let Some(idx) = cutoff {
        cleaned = cleaned[..idx]
            .trim()
            .trim_end_matches(&['\n', ' ', '-', ':'][..])
            .to_string();
    }
    clean_chat_text(cleaned.trim(), 32_000)
}

#[cfg(test)]
mod workflow_control_tests {
    use super::*;

    #[test]
    fn conversation_bypass_control_enables_for_direct_override_phrase() {
        let control = workflow_conversation_bypass_control_for_turn(
            "break the workflow and respond directly",
            &[],
            false,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(
            control.get("source").and_then(Value::as_str),
            Some("retired")
        );
        assert_eq!(
            control
                .get("workflow_mode_override")
                .and_then(Value::as_str),
            Some("")
        );
    }

    #[test]
    fn conversation_bypass_control_blocks_when_tooling_is_required() {
        let control = workflow_conversation_bypass_control_for_turn(
            "break the workflow and respond directly",
            &[],
            true,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(control.get("blocked").and_then(Value::as_bool), Some(false));
        assert_eq!(
            control.get("block_reason").and_then(Value::as_str),
            Some("")
        );
    }

    #[test]
    fn conversation_bypass_control_continues_sticky_state() {
        let active_messages = vec![json!({
            "role": "assistant",
            "response_finalization": {
                "workflow_control": {
                    "conversation_bypass": {
                        "remaining_turns_after": 2
                    }
                }
            }
        })];
        let control =
            workflow_conversation_bypass_control_for_turn("status?", &active_messages, false);
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(
            control.get("source").and_then(Value::as_str),
            Some("retired")
        );
        assert_eq!(
            control
                .get("remaining_turns_before")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            control.get("remaining_turns_after").and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn conversation_bypass_control_disables_when_user_requests_resume() {
        let active_messages = vec![json!({
            "role": "assistant",
            "response_finalization": {
                "workflow_control": {
                    "conversation_bypass": {
                        "remaining_turns_after": 2
                    }
                }
            }
        })];
        let control = workflow_conversation_bypass_control_for_turn(
            "resume workflow now",
            &active_messages,
            false,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(
            control.get("source").and_then(Value::as_str),
            Some("retired")
        );
        assert_eq!(
            control.get("remaining_turns_after").and_then(Value::as_u64),
            Some(0)
        );
    }
}
