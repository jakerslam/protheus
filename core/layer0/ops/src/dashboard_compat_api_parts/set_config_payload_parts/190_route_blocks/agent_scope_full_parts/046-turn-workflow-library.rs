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
        "reason": "direct_response_uses_gate_1_no",
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
            "what?",
            "why are you repeating",
            "repeating the same",
            "fallback text",
            "parroting",
            "hard coded response",
            "hard-coded response",
        ],
    ) && !workflow_turn_contains_any(
        &lowered,
        &[
            "search", "web", "online", "internet", "file", "patch", "edit", "update", "create",
            "read", "memory", "repo", "codebase",
        ],
    )
}

fn workflow_turn_is_simple_conversation_without_tool_intent(message: &str) -> bool {
    let lowered = clean_text(message, 240).to_ascii_lowercase();
    if lowered.is_empty()
        || lowered.contains('\n')
        || inline_tool_calls_allowed_for_user_message(&lowered)
        || message_explicitly_disallows_tool_calls(&lowered)
        || message_requires_information_search(&lowered)
    {
        return false;
    }
    if workflow_turn_contains_any(
        &lowered,
        &[
            "tool", "search", "web", "file", "repo", "workspace", "read", "write", "patch",
            "edit", "run", "execute", "compare", "latest", "current",
        ],
    ) {
        return false;
    }
    matches!(
        lowered.trim_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace()),
        "hey"
            | "hi"
            | "hello"
            | "yo"
            | "sup"
            | "hiya"
            | "good morning"
            | "good afternoon"
            | "good evening"
            | "are you there"
            | "you there"
    )
}

fn default_workflow_tool_menu_contract() -> Value {
    default_workflow_definition().tool_menu_interface_contract
}

fn workflow_contract_gate(contract: &Value, gate_id: &str) -> Value {
    contract
        .pointer(&format!("/gates/{gate_id}"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_gate_options(contract: &Value, gate_id: &str) -> Vec<Value> {
    workflow_contract_gate(contract, gate_id)
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn workflow_option_label(option: &Value) -> String {
    clean_text(option.get("label").and_then(Value::as_str).unwrap_or(""), 120)
}

fn workflow_option_key(option: &Value) -> String {
    clean_text(option.get("key").and_then(Value::as_str).unwrap_or(""), 120)
}

fn normalized_workflow_token(value: &str) -> String {
    clean_text(value, 240)
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_workflow_option_tokens(option: &Value) -> Vec<String> {
    let mut tokens = [workflow_option_key(option), workflow_option_label(option)]
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(aliases) = option.get("aliases").and_then(Value::as_array) {
        tokens.extend(
            aliases
                .iter()
                .filter_map(Value::as_str)
                .map(|alias| clean_text(alias, 120)),
        );
    }
    tokens
        .into_iter()
        .map(|row| normalized_workflow_token(&row))
        .filter(|row| !row.is_empty())
        .collect()
}

fn workflow_gate_option_labels(contract: &Value, has_tools: Option<bool>) -> Vec<String> {
    workflow_gate_options(contract, "gate_1_work_category_menu")
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .filter_map(|option| {
            let label = workflow_option_label(&option);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .collect()
}

fn workflow_gate_option_menu_entries(contract: &Value, has_tools: Option<bool>) -> Vec<String> {
    workflow_gate_options(contract, "gate_1_work_category_menu")
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .filter_map(|option| {
            let label = workflow_option_label(&option);
            if label.is_empty() {
                return None;
            }
            let leading_alias = option
                .get("aliases")
                .and_then(Value::as_array)
                .and_then(|aliases| aliases.iter().filter_map(Value::as_str).next())
                .map(|alias| clean_text(alias, 40))
                .filter(|alias| !alias.is_empty());
            Some(match leading_alias {
                Some(alias) => format!("{alias} = {label}"),
                None => label,
            })
        })
        .collect()
}

fn workflow_gate_1_allowed_outputs(contract: &Value) -> Value {
    workflow_contract_gate(contract, "gate_1_work_category_menu")
        .pointer("/submission_contract/accepted_outputs")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn workflow_tool_family_menu(contract: &Value, selected_family: &str) -> Value {
    let selected_family = clean_text(selected_family, 120);
    let rows = contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Value::Array(
        rows.into_iter()
            .map(|mut row| {
                let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
                row["selected"] = Value::Bool(!selected_family.is_empty() && key == selected_family);
                row
            })
            .collect(),
    )
}

fn workflow_tool_menu_by_family(contract: &Value) -> Value {
    contract
        .get("tool_menu_by_family")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_post_tool_options(contract: &Value) -> Value {
    workflow_contract_gate(contract, "gate_5_post_tool_menu")
        .get("options")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn workflow_final_output_contract(contract: &Value) -> Value {
    workflow_contract_gate(contract, "gate_6_llm_final_output")
        .get("final_output_contract")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_example_tool_key(contract: &Value) -> String {
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| {
            families
                .values()
                .filter_map(Value::as_array)
                .flat_map(|tools| tools.iter())
                .filter_map(|tool| tool.get("key").and_then(Value::as_str))
                .next()
        })
        .map(|key| clean_text(key, 80))
        .unwrap_or_default()
}

fn workflow_tool_submission_format(contract: &Value) -> String {
    workflow_contract_gate(contract, "gate_1_work_category_menu")
        .pointer("/submission_contract/accepted_outputs")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .find(|row| row.contains("Category:") && row.contains("Request payload:"))
        })
        .map(|row| clean_text(row, 240))
        .unwrap_or_default()
}

fn workflow_message_matches_contract_markers(contract: &Value, pointer: &str, message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    contract
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|markers| {
            markers
                .iter()
                .filter_map(Value::as_str)
                .map(|marker| clean_text(marker, 120).to_ascii_lowercase())
                .filter(|marker| !marker.is_empty())
                .any(|marker| lowered.contains(&marker))
        })
        .unwrap_or(false)
}

fn render_workflow_instruction_template(contract: &Value, template: &str) -> String {
    let gate_prompt = workflow_contract_gate(contract, "gate_1_work_category_menu")
        .get("question")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 120))
        .unwrap_or_default();
    let category_options = workflow_gate_option_menu_entries(contract, None).join("`, `");
    let no_tool_categories = workflow_gate_option_menu_entries(contract, Some(false)).join("`, `");
    let tool_bearing_categories = workflow_gate_option_menu_entries(contract, Some(true)).join("`, `");
    let tool_submission_format = workflow_tool_submission_format(contract);
    let example_tool_key = workflow_example_tool_key(contract);
    clean_text(
        &template
            .replace("{gate_question}", &gate_prompt)
            .replace("{category_options}", &format!("`{category_options}`"))
            .replace("{no_tool_categories}", &format!("`{no_tool_categories}`"))
            .replace(
                "{tool_bearing_categories}",
                &format!("`{tool_bearing_categories}`"),
            )
            .replace("{tool_submission_format}", &tool_submission_format)
            .replace("{example_tool_key}", &example_tool_key),
        1_400,
    )
}

fn workflow_category_token_matches(response: &str, has_tools: Option<bool>) -> bool {
    workflow_category_selection(&default_workflow_tool_menu_contract(), response, has_tools).is_some()
}

fn response_contains_no_tool_gate_token_fragment(response: &str) -> bool {
    let token = normalized_workflow_token(response);
    if token.is_empty() {
        return false;
    }
    let haystack = format!(" {token} ");
    workflow_gate_options(
        &default_workflow_tool_menu_contract(),
        "gate_1_work_category_menu",
    )
    .into_iter()
    .filter(|option| {
        !option
            .get("has_tools")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
    .flat_map(|option| normalized_workflow_option_tokens(&option))
    .any(|candidate| {
        candidate.split_whitespace().count() > 1
            && haystack.contains(&format!(" {candidate} "))
    })
}

fn workflow_category_selection(
    contract: &Value,
    response: &str,
    has_tools: Option<bool>,
) -> Option<(String, String)> {
    let token = normalized_workflow_token(response);
    if token.is_empty() {
        return None;
    }
    workflow_gate_options(contract, "gate_1_work_category_menu")
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .find_map(|option| {
            let key = workflow_option_key(&option);
            let label = workflow_option_label(&option);
            normalized_workflow_option_tokens(&option)
                .into_iter()
                .any(|candidate| token == candidate)
                .then_some((key, label))
        })
}

fn workflow_category_phrase_matches(response: &str, has_tools: Option<bool>) -> bool {
    let token = normalized_workflow_token(response);
    if token.is_empty() {
        return false;
    }
    workflow_gate_options(&default_workflow_tool_menu_contract(), "gate_1_work_category_menu")
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .any(|option| {
            normalized_workflow_option_tokens(&option)
                .into_iter()
                .any(|candidate| {
                    token == candidate
                        || token == format!("work category {candidate}")
                        || token == format!("this kind of work is {candidate}")
                        || token.starts_with(&format!("this kind of work is {candidate} "))
                })
        })
}

fn workflow_family_key_for_selection(contract: &Value, family: &str) -> String {
    let selected = normalized_workflow_token(family);
    if selected.is_empty() {
        return String::new();
    }
    contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .and_then(|families| {
            families.iter().find_map(|row| {
                let tokens = normalized_workflow_option_tokens(row);
                if tokens.into_iter().any(|token| token == selected) {
                    Some(workflow_option_key(row))
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

fn workflow_tool_key_for_selection(contract: &Value, family: &str, tool_label: &str) -> String {
    let selected = normalized_workflow_token(tool_label);
    if selected.is_empty() {
        return String::new();
    }
    let selected_family = workflow_family_key_for_selection(contract, family);
    let Some(tool_menus) = contract.get("tool_menu_by_family").and_then(Value::as_object) else {
        return String::new();
    };
    let families = if selected_family.is_empty() {
        tool_menus.values().collect::<Vec<_>>()
    } else {
        tool_menus.get(&selected_family).into_iter().collect::<Vec<_>>()
    };
    families
        .into_iter()
        .filter_map(Value::as_array)
        .flat_map(|tools| tools.iter())
        .find_map(|tool| {
            let key = workflow_option_key(tool);
            normalized_workflow_option_tokens(tool)
                .into_iter()
                .any(|candidate| selected == candidate)
                .then_some(key)
        })
        .unwrap_or_default()
}

fn workflow_turn_tool_decision_tree(message: &str) -> Value {
    let contract = default_workflow_tool_menu_contract();
    let gate_1 = workflow_contract_gate(&contract, "gate_1_work_category_menu");
    let requires_file_mutation = false;
    let requires_local_lookup = false;
    let requires_live_web = false;
    let explicit_web_intent = false;
    let has_sufficient_information = false;
    let should_call_tools = false;
    let gate_decision_mode = "manual_work_category_v1";
    let reason_code = "manual_menu_presented";
    let info_source = "menu_only";
    let selected_tool_family = "unselected";
    let meta_control = workflow_turn_is_meta_control_message(message);
    let status_check =
        workflow_message_matches_contract_markers(&contract, "/diagnostic_markers/status_check", message);
    let meta_diagnostic_request =
        clean_text(message, 1_200).to_ascii_lowercase().contains("diagnostic");
    let llm_should_answer_directly = false;
    let automatic_tool_calls_allowed = false;
    let tool_selection_authority = "llm_submitted_menu_or_text_input";
    let decision_authority_mode = "llm_manual_only_v1";
    let gate_enforcement_mode = "disabled";
    let gate_is_advisory = false;
    let workflow_retry_limit = 1;
    let needs_tool_access = None;
    let selected_work_category = Value::Null;
    let gate_1_allowed_outputs = workflow_gate_1_allowed_outputs(&contract);
    let gate_1_submission_status = "awaiting_llm_submission";
    let gate_1_decision_source = "pending_llm_submission";
    let gate_prompt = clean_text(gate_1.get("question").and_then(Value::as_str).unwrap_or(""), 120);
    let gate_submission = json!({
        "gate_id": "gate_1_work_category_menu",
        "input_shape": {
            "type": gate_1.get("input_kind").and_then(Value::as_str).unwrap_or("multiple_choice"),
            "allowed_outputs": gate_1_allowed_outputs.clone()
        },
        "llm_submission": selected_work_category,
        "accepted": false,
        "resume_token": "gate_1_work_category_menu.awaiting_llm_submission"
    });
    let tool_family_menu = workflow_tool_family_menu(&contract, selected_tool_family);
    let tool_menu = json!([]);
    let tool_menu_by_family = workflow_tool_menu_by_family(&contract);
    let manual_tool_selection = true;
    let auto_decisions_disabled = true;
    let manual_gate_mode = "llm_only_multiple_choice_v1";
    let gate_1_options = workflow_gate_options(&contract, "gate_1_work_category_menu")
        .into_iter()
        .enumerate()
        .map(|(idx, option)| {
            json!({
                "option": idx + 1,
                "key": workflow_option_key(&option),
                "label": workflow_option_label(&option),
                "has_tools": option.get("has_tools").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();
    let gate_5_options = workflow_post_tool_options(&contract);
    let gate_6_contract = workflow_final_output_contract(&contract);
    json!({
        "contract": "manual_toolbox_gate_v1", "workflow_gate_contract": "tool_menu_interface_v1",
        "gate_decision_mode": gate_decision_mode,
        "semantic_route_classifier_active": false, "info_task_route_classifier_active": false, "workflow_route_classifier_active": false,
        "system_may_select_tools": false, "tool_recommendations_allowed": false,
        "gate_1_question_type": "multiple_choice", "gate_1_allowed_outputs": gate_1_allowed_outputs,
        "reason_code": reason_code,
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "llm_should_answer_directly": llm_should_answer_directly,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "selected_work_category": selected_work_category,
        "gate_1_submission_status": gate_1_submission_status,
        "gate_1_decision_source": gate_1_decision_source,
        "gate_submission": gate_submission.clone(),
        "gate_prompt": gate_prompt,
        "info_source": info_source,
        "selected_tool_family": selected_tool_family,
        "decision_authority_mode": decision_authority_mode,
        "gate_enforcement_mode": gate_enforcement_mode,
        "gate_is_advisory": gate_is_advisory,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "tool_menu_by_family": tool_menu_by_family,
        "manual_tool_selection": manual_tool_selection, "auto_decisions_disabled": auto_decisions_disabled,
        "manual_gate_mode": manual_gate_mode, "meta_control_message": meta_control,
        "status_check_message": status_check, "meta_diagnostic_request": meta_diagnostic_request,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit,
        "gates": {
            "gate_1": {
                "name": "work_category",
                "question": gate_prompt,
                "required": false,
                "selection_mode": "multiple_choice",
                "options": gate_1_options,
                "reason_code": reason_code,
                "submission_status": gate_1_submission_status,
                "decision_source": gate_1_decision_source,
                "gate_submission": gate_submission
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
                "wait_for_tools": needs_tool_access == Some(true),
                "skip_when_no_tools": needs_tool_access == Some(false),
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
                "options": gate_5_options
            },
            "gate_6": {
                "name": "final_output",
                "output_contract": gate_6_contract,
                "retry_limit": workflow_retry_limit
            }
        }
    })
}

fn workflow_library_prompt_context(message: &str, latent_tool_candidates: &[Value]) -> String {
    let _ = latent_tool_candidates;
    let _ = message;
    let contract = default_workflow_tool_menu_contract();
    contract
        .get("llm_gate_instruction")
        .and_then(Value::as_str)
        .map(|template| render_workflow_instruction_template(&contract, template))
        .unwrap_or_default()
}

fn workflow_tool_request_prompt_context(category_key: &str, category_label: &str) -> String {
    let contract = default_workflow_tool_menu_contract();
    let category_key = clean_text(category_key, 120);
    let category_label = clean_text(category_label, 120);
    let tools = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&category_key))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tools_json = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
    clean_text(
        &format!(
            "Private workflow gate submission only. Selected category: `{category_label}`. \
Pick one catalog tool from this JSON menu and provide its request payload. \
Reply exactly as: `Category: {category_label}. Tool family: {category_label}. Tool: <catalog tool key>. Request payload: <JSON>.` \
Do not narrate or say what you would choose. Visible chat is only Gate 6 final answer.\n\nTool menu JSON:\n{tools_json}"
        ),
        4_000,
    )
}

fn turn_workflow_requires_final_llm(
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> bool {
    let pending_confirmation_wait = response_tools.is_empty()
        && workflow_events.iter().any(|event| {
            matches!(
                event.get("kind").and_then(Value::as_str).unwrap_or(""),
                "manual_toolbox_pending_tool_request" | "pending_confirmation_required"
            )
        });
    if pending_confirmation_wait {
        return false;
    }
    if !response_tools.is_empty() || !workflow_events.is_empty() {
        return true;
    }
    let cleaned_draft = clean_text(draft_response, 4_000);
    if cleaned_draft.is_empty() {
        return true;
    }
    if response_is_exact_no_tool_gate_submission(&cleaned_draft) {
        return true;
    }
    if response_contains_no_tool_gate_token_fragment(&cleaned_draft) {
        return true;
    }
    if response_is_visible_workflow_gate_choice(&cleaned_draft) {
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
    let workflow_mode = clean_text(workflow_mode, 80);
    let cleaned_draft = clean_text(draft_response, 2_000);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    if workflow_mode == "direct_conversation_recovery"
        || workflow_mode == "direct_no_tool_exit"
        || workflow_mode == "direct_simple_conversation"
    {
        return vec![
            json!({
                "stage": "gate_1_work_category_menu",
                "status": "answered_no_tool_category",
                "selection_mode": "multiple_choice",
                "question": "What kind of work is this?"
            }),
            json!({
                "stage": "gate_6_llm_final_output",
                "required": requires_final_llm,
                "status": final_stage_status
            }),
        ];
    }
    if !requires_final_llm && response_tools.is_empty() && workflow_events.is_empty() {
        return vec![
            json!({
                "stage": "gate_1_work_category_menu",
                "status": "answered_no_tool_category",
                "selection_mode": "multiple_choice",
                "question": "What kind of work is this?"
            }),
            json!({
                "stage": "gate_6_llm_final_output",
                "status": "skipped_not_required",
                "source": "initial_llm_answer"
            }),
        ];
    }
    vec![
        json!({
            "stage": "gate_1_work_category_menu",
            "status": "presented"
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

fn turn_workflow_visibility(final_stage_status: &str) -> Value {
    let status = clean_text(final_stage_status, 80);
    let (ui_status, agent_process_status, debug_status) = match status.as_str() {
        "pending_final_llm" => (
            "Workflow at final synthesis; waiting for the LLM-authored answer.",
            "Gate 6 active: compose final answer from current context.",
            "gate_6_llm_final_output.pending_final_llm",
        ),
        "synthesized" => (
            "Workflow complete; final answer was authored by the LLM.",
            "Gate 6 complete: final answer submitted.",
            "gate_6_llm_final_output.synthesized",
        ),
        "skipped_not_required" | "skipped_test" | "no_post_synthesis_required" => (
            "Workflow complete; a no-tool work category was selected and direct LLM answer is ready.",
            "Gate 1 selected a no-tool category: respond directly without tool menus.",
            "gate_1_work_category_menu.no_tool_category",
        ),
        "skipped_missing_model" => (
            "Workflow paused; model provider is unavailable for final synthesis.",
            "Gate 6 blocked: model provider unavailable.",
            "gate_6_llm_final_output.skipped_missing_model",
        ),
        "withheld_non_llm_fallback_response" => (
            "Workflow withheld a non-LLM fallback; waiting for an LLM-authored answer.",
            "Gate 6 blocked: non-LLM fallback text cannot be sent as chat.",
            "gate_6_llm_final_output.withheld_non_llm_fallback_response",
        ),
        "synthesis_failed" | "invoke_failed" => (
            "Workflow final synthesis failed; no system fallback text will be injected.",
            "Gate 6 failed: retry needs an LLM-authored response.",
            "gate_6_llm_final_output.final_synthesis_failed",
        ),
        _ => (
            "Workflow state visible; waiting for the next LLM-controlled step.",
            "Follow the currently presented workflow gate.",
            "workflow.state_visible",
        ),
    };
    json!({
        "current_stage": "gate_6_llm_final_output",
        "current_stage_status": status,
        "ui_status": ui_status,
        "agent_process_status": agent_process_status,
        "debug_status": debug_status,
        "final_chat_authority": "llm_only",
        "system_injected_chat_text_allowed": false,
        "formats": {
            "ui": ui_status,
            "agent_process": agent_process_status,
            "debug": debug_status
        }
    })
}

fn turn_workflow_direct_response_path(workflow_mode: &str, workflow_events: &[Value]) -> &'static str {
    let mode = clean_text(workflow_mode, 80);
    if mode == "direct_conversation_recovery"
        || mode == "direct_no_tool_exit"
        || mode == "direct_simple_conversation"
    {
        return "gate_1_no_tool_category";
    }
    let has_pending = workflow_events.iter().any(|event| {
        matches!(
            event.get("kind").and_then(Value::as_str).unwrap_or(""),
            "manual_toolbox_pending_tool_request" | "pending_confirmation_required"
        )
    });
    if has_pending {
        return "gate_1_yes_pending_tool_confirmation";
    }
    let has_manual_toolbox_menu = workflow_events.iter().any(|event| {
        matches!(
            event.get("kind").and_then(Value::as_str).unwrap_or(""),
                "manual_toolbox_candidate_menu"
        )
    });
    if has_manual_toolbox_menu {
        return "gate_1_pending_llm_tool_choice";
    }
    "gate_1_unresolved"
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
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    let visibility = turn_workflow_visibility(final_stage_status);
    let direct_response_path = turn_workflow_direct_response_path(workflow_mode, workflow_events);
    json!({
        "contract": "agent_workflow_library_v1",
        "current_stage": visibility
            .get("current_stage")
            .cloned()
            .unwrap_or_else(|| json!("gate_6_llm_final_output")),
        "current_stage_status": visibility
            .get("current_stage_status")
            .cloned()
            .unwrap_or_else(|| json!(final_stage_status)),
        "ui_status": visibility
            .get("ui_status")
            .cloned()
            .unwrap_or_else(|| json!("Workflow state visible.")),
        "agent_process_status": visibility
            .get("agent_process_status")
            .cloned()
            .unwrap_or_else(|| json!("Follow the currently presented workflow gate.")),
        "debug_status": visibility
            .get("debug_status")
            .cloned()
            .unwrap_or_else(|| json!("workflow.state_visible")),
        "visibility": visibility,
        "workflow_gate": {
            "required": false,
            "status": "presented"
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
            "direct_response_path": direct_response_path
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
    let visibility = turn_workflow_visibility(status);
    workflow["current_stage"] = visibility
        .get("current_stage")
        .cloned()
        .unwrap_or_else(|| json!("gate_6_llm_final_output"));
    workflow["current_stage_status"] = visibility
        .get("current_stage_status")
        .cloned()
        .unwrap_or_else(|| json!(clean_text(status, 80)));
    workflow["ui_status"] = visibility
        .get("ui_status")
        .cloned()
        .unwrap_or_else(|| json!("Workflow state visible."));
    workflow["agent_process_status"] = visibility
        .get("agent_process_status")
        .cloned()
        .unwrap_or_else(|| json!("Follow the currently presented workflow gate."));
    workflow["debug_status"] = visibility
        .get("debug_status")
        .cloned()
        .unwrap_or_else(|| json!("workflow.state_visible"));
    workflow["visibility"] = visibility;
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response" || value == "gate_6_llm_final_output")
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
            "i'll use the web search",
            "i will use the web search",
            "please hold while i gather",
            "once i have that information",
            "let me start that process",
            "let's inspect",
            "need to perform a web search",
            "i need to perform a web search",
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

fn manual_toolbox_response_exposes_unresolved_tool_need(response: &str) -> bool {
    let lowered = clean_text(response, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "i don't have current web search results",
        "i do not have current web search results",
        "i don't have usable tool findings",
        "i do not have usable tool findings",
        "i'll need to perform a web search",
        "i will need to perform a web search",
        "web search didn't return",
        "web search did not return",
        "web search returned limited",
        "search returned limited",
        "tool returned no new results",
        "let me run that search",
        "if you'd like me to search",
        "if you would like me to search",
        "if you'd like me to fetch",
        "if you would like me to fetch",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn response_is_no_tool_category_gate_submission(response: &str) -> bool {
    workflow_category_token_matches(response, Some(false))
}

fn response_is_tool_bearing_category_gate_submission(response: &str) -> bool {
    workflow_category_token_matches(response, Some(true))
}

fn response_is_manual_toolbox_gate_choice(response: &str) -> bool {
    let lowered = clean_text(response, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let has_embedded_gate_payload = lowered.contains("category:")
        && (lowered.contains("tool family:")
            || lowered.contains("tool:")
            || lowered.contains("request payload:")
            || lowered.contains("payload:"));
    if has_embedded_gate_payload {
        return true;
    }
    let starts_with_category = lowered.starts_with("category:");
    starts_with_category
        && (lowered.contains("tool family")
            || lowered.contains("tool:")
            || lowered.contains("request payload")
            || lowered.contains("payload:"))
}

fn response_is_exact_no_tool_gate_submission(response: &str) -> bool {
    response_is_no_tool_category_gate_submission(response)
}

fn manual_toolbox_pending_request_from_response(response: &str, message: &str) -> Option<Value> {
    if !response_is_manual_toolbox_gate_choice(response) {
        return None;
    }
    let family = manual_toolbox_selection_field(response, "tool family:", &["tool:", "request payload:", "payload:"])
        .if_empty_then(|| manual_toolbox_selection_field(response, "category:", &["tool family:", "tool:", "request payload:", "payload:"]));
    let tool_label = manual_toolbox_selection_field(response, "tool:", &["request payload:", "payload:"]);
    let payload_text = manual_toolbox_selection_field(response, "request payload:", &[])
        .if_empty_then(|| manual_toolbox_selection_field(response, "payload:", &[]));
    if family.is_empty() || tool_label.is_empty() || payload_text.trim().is_empty() {
        return None;
    }
    let tool_name = canonical_manual_toolbox_tool_name(&family, &tool_label);
    if tool_name.is_empty() {
        return None;
    }
    let input = manual_toolbox_payload_json(&payload_text)?;
    if !input.is_object() {
        return None;
    }
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600)
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "manual_toolbox_gate",
        "tool_name": tool_name,
        "selected_tool_family": family,
        "selected_tool_label": tool_label,
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn manual_toolbox_selection_field(response: &str, label: &str, end_labels: &[&str]) -> String {
    let lowered = response.to_ascii_lowercase();
    let Some(start) = lowered.find(label) else {
        return String::new();
    };
    let value_start = start + label.len();
    let mut value_end = response.len();
    for end_label in end_labels {
        if let Some(end) = lowered[value_start..].find(end_label) {
            value_end = value_end.min(value_start + end);
        }
    }
    clean_text(response.get(value_start..value_end).unwrap_or("").trim_matches([' ', '.', '\n', '\r']), 2_000)
}

trait EmptyStringExt {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String {
        if self.trim().is_empty() { fallback() } else { self }
    }
}

fn manual_toolbox_payload_json(payload_text: &str) -> Option<Value> {
    let start = payload_text.find('{')?;
    let end = payload_text.rfind('}')?;
    if end < start {
        return None;
    }
    serde_json::from_str(payload_text.get(start..=end)?).ok()
}

fn canonical_manual_toolbox_tool_name(family: &str, tool_label: &str) -> String {
    workflow_tool_key_for_selection(&default_workflow_tool_menu_contract(), family, tool_label)
}

fn response_is_visible_workflow_gate_choice(response: &str) -> bool {
    let lowered = clean_text(response, 2_000).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return false;
    }
    let compact = trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    response_is_manual_toolbox_gate_choice(trimmed)
        || response_is_no_tool_category_gate_submission(trimmed)
        || response_is_tool_bearing_category_gate_submission(trimmed)
        || workflow_category_phrase_matches(&compact, None)
        || trimmed.starts_with("category:")
        || (trimmed.contains("category:")
            && (trimmed.contains("tool family:")
                || trimmed.contains("tool:")
                || trimmed.contains("request payload:")
                || trimmed.contains("payload:")))
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
    let cleaned = clean_chat_text(cleaned.trim(), 32_000);
    normalize_response_field_json_wrapper(&cleaned).unwrap_or(cleaned)
}

#[cfg(test)]
mod workflow_control_tests {
    use super::*;

    #[test]
    fn visible_workflow_gate_choice_uses_json_declared_tokens() {
        assert!(response_is_visible_workflow_gate_choice("Respond directly"));
        assert!(response_is_visible_workflow_gate_choice(
            "Respond directly. Category: Respond directly. Tool family: None. Request payload: {}"
        ));
        assert!(response_is_visible_workflow_gate_choice(
            "This kind of work is `Respond directly`."
        ));
        let web_category = workflow_gate_option_labels(
            &default_workflow_tool_menu_contract(),
            Some(true),
        )
        .into_iter()
        .find(|label| label == "Web research")
        .expect("web research option comes from workflow JSON");
        assert!(response_is_visible_workflow_gate_choice(&format!(
            "Category: {web_category}. Tool family: {web_category}. Tool: web_search. Request payload: {{\"source\":\"web\",\"query\":\"x\"}}."
        )));
        assert!(!response_is_visible_workflow_gate_choice("Need tools? Yes"));
        assert!(!response_is_visible_workflow_gate_choice(
            "I need tools to answer this well, but I have not run them yet."
        ));
    }

    #[test]
    fn workflow_prompt_contract_requires_private_exact_gate_submission() {
        let prompt =
            workflow_library_prompt_context("Use web search to compare agent frameworks.", &[]);
        assert!(prompt.contains("Private workflow gate submission only"));
        assert!(prompt.contains("Reply with one token only"));
        assert!(prompt.contains("open the private tool menu"));
        assert!(prompt.contains("Do not narrate"));
        assert!(!prompt.contains("present exactly one gate"));
        assert!(!prompt.contains("If Yes, continue"));
    }

    #[test]
    fn no_tool_gate_submission_is_exact_private_token() {
        assert!(response_is_exact_no_tool_gate_submission("Respond directly"));
        assert!(!response_is_exact_no_tool_gate_submission("No, I can answer directly."));
        assert!(!response_is_exact_no_tool_gate_submission(
            "No. I would use web search later."
        ));
    }

    #[test]
    fn embedded_no_tool_gate_token_fragment_requires_final_llm() {
        let draft = "I will answer now. 1 = Respond directly";
        assert!(response_contains_no_tool_gate_token_fragment(draft));
        assert!(turn_workflow_requires_final_llm(&[], &[], draft));
    }

    #[test]
    fn manual_toolbox_tool_names_come_from_workflow_json_catalog() {
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "web_search"),
            "web_search"
        );
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "Search web"),
            "web_search"
        );
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "I would choose web search"),
            ""
        );
    }

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
