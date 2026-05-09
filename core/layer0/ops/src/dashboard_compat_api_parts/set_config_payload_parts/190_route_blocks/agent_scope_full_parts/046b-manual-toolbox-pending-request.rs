fn workflow_has_manual_toolbox_candidate_menu(workflow: &Value) -> bool {
    workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        == Some("first_gate_pending_llm_tool_choice")
        || workflow
            .get("system_events")
            .and_then(Value::as_array)
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("kind")
                        .or_else(|| event.get("name"))
                        .or_else(|| event.get("type"))
                        .and_then(Value::as_str)
                        == Some("manual_toolbox_candidate_menu")
                })
            })
            .unwrap_or(false)
}

fn record_manual_toolbox_pending_request(workflow: &mut Value, response_text: &str, message: &str) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    let pending_request = manual_toolbox_pending_request_from_response(response_text, message);
    let Some(pending_request) = pending_request else {
        return;
    };
    record_manual_toolbox_pending_request_value(workflow, pending_request);
}

fn record_manual_toolbox_pending_request_value(workflow: &mut Value, mut pending_request: Value) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    if let Some((tool_name, input)) = pending_request
        .get("tool_name")
        .and_then(Value::as_str)
        .zip(pending_request.get("input").cloned())
    {
        if let Ok(repaired_input) =
            crate::infring_tooling_core_v1_bridge::repair_and_validate_args(tool_name, &input)
        {
            pending_request["input"] = repaired_input;
        }
    }
    workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
    workflow["response"] = Value::String(String::new());
    workflow["visible_response_source"] = Value::String("json_private_tool_request".to_string());
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_pending_tool_confirmation".to_string());
    if let Some(events) = workflow
        .get_mut("system_events")
        .and_then(Value::as_array_mut)
    {
        events.push(turn_workflow_event(
            "manual_toolbox_pending_tool_request",
            pending_request,
        ));
    }
}

fn workflow_tool_family_prompt_context(
    previous_category_key: &str,
    previous_category_label: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let families = contract
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let family_menu_json = serde_json::to_string(&families).unwrap_or_else(|_| "[]".to_string());
    contract
        .get("llm_tool_family_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace(
                        "{previous_category_key}",
                        &clean_text(previous_category_key, 120),
                    )
                    .replace(
                        "{previous_category_label}",
                        &clean_text(previous_category_label, 120),
                    )
                    .replace("{tool_family_menu_json}", &family_menu_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_selection_prompt_context(family_key: &str, family_label: &str) -> String {
    let contract = default_workflow_tool_menu_contract();
    let family_key = clean_text(family_key, 120);
    let family_label = clean_text(family_label, 120);
    let tools = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tools_json = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
    let allowed_tool_keys_json = tools
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(workflow_option_key)
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>()
        })
        .map(|keys| serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "[]".to_string());
    contract
        .get("llm_tool_selection_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_family_label}", &family_label)
                    .replace("{selected_tool_keys_json}", &allowed_tool_keys_json)
                    .replace("{selected_tool_menu_json}", &tools_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_payload_prompt_context(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let family_key = clean_text(family_key, 120);
    let tool_key = clean_text(tool_key, 120);
    let tool_label = clean_text(tool_label, 120);
    let tool = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .unwrap_or_else(|| json!({}));
    let request_format_json =
        serde_json::to_string(tool.get("request_format").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    let request_example_json =
        serde_json::to_string(tool.get("request_example").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    contract
        .get("llm_tool_payload_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_tool_key}", &tool_key)
                    .replace("{selected_tool_label}", &tool_label)
                    .replace("{selected_tool_request_format_json}", &request_format_json)
                    .replace(
                        "{selected_tool_request_example_json}",
                        &request_example_json,
                    ),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn manual_toolbox_private_gate_max_attempts() -> u64 {
    let contract = default_workflow_tool_menu_contract();
    let base_gate_count = contract
        .get("gate_order")
        .and_then(Value::as_array)
        .and_then(|gates| {
            gates
                .iter()
                .position(|gate| gate.as_str() == Some("gate_4_request_payload_input"))
                .map(|idx| idx as u64 + 1)
        })
        .unwrap_or(4);
    let retry_limit = contract
        .get("private_gate_retry_limit")
        .or_else(|| contract.get("workflow_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(4);
    base_gate_count.saturating_add(retry_limit).clamp(4, 8)
}

fn workflow_private_gate_retry_prompt_context(
    current_gate_id: &str,
    message: &str,
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let fallback = "INTERNAL RETRY — output is never shown to the user. The previous response for `{current_gate_id}` was rejected with reason `{last_reject_reason}`. Previous excerpt: {last_invalid_excerpt}. If the excerpt is empty, treat it as an empty response. Re-read the current gate system instruction and output only the exact JSON artifact required by that gate. Do not answer the user directly, do not write prose, and do not include markdown.";
    let template = contract
        .get("private_gate_retry_instruction")
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    let excerpt = if last_invalid_excerpt.trim().is_empty() {
        "(empty response)"
    } else {
        last_invalid_excerpt
    };
    clean_text(
        &format!(
            "{}\n\nContext-only user message. Do not answer it directly. Use it only to produce the artifact required for the current workflow gate:\n{}",
            template
                .replace("{current_gate_id}", &clean_text(current_gate_id, 120))
                .replace(
                    "{last_reject_reason}",
                    &clean_text(last_reject_reason, 160)
                )
                .replace("{last_invalid_excerpt}", &clean_text(excerpt, 320)),
            message
        ),
        8_000,
    )
}

fn workflow_tool_family_selection_from_response(response: &str) -> Option<(String, String)> {
    let contract = default_workflow_tool_menu_contract();
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool_family")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "family"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "tool_family_key")
                })
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_family")
                })
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "category"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "gate"))
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let family_key = workflow_family_key_for_selection(&contract, &token);
    if family_key.is_empty() {
        return None;
    }
    contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .and_then(|families| {
            families.iter().find_map(|family| {
                (workflow_option_key(family) == family_key)
                    .then(|| (family_key.clone(), workflow_option_label(family)))
            })
        })
}

fn workflow_tool_selection_from_response(
    family_key: &str,
    response: &str,
) -> Option<(String, String)> {
    let contract = default_workflow_tool_menu_contract();
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "selected_tool"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "tool_key"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_key")
                })
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let tool_key = workflow_tool_key_for_selection(&contract, family_key, &token);
    if tool_key.is_empty() {
        return None;
    }
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                (workflow_option_key(tool) == tool_key)
                    .then(|| (tool_key.clone(), workflow_option_label(tool)))
            })
        })
}

fn workflow_selected_tool_request_format_keys(family_key: &str, tool_key: &str) -> Vec<String> {
    default_workflow_tool_menu_contract()
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .and_then(|tool| tool.get("request_format").cloned())
        .and_then(|format| format.as_object().cloned())
        .map(|format| {
            format
                .keys()
                .map(|key| normalized_workflow_token(key))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_payload_object_matches_selected_tool(
    value: &Value,
    family_key: &str,
    tool_key: &str,
) -> bool {
    let Some(payload) = value.as_object() else {
        return false;
    };
    let expected_keys = workflow_selected_tool_request_format_keys(family_key, tool_key);
    if expected_keys.is_empty() {
        return false;
    }
    let reserved_keys = [
        "gate",
        "tool",
        "tool_name",
        "selected_tool",
        "selected_tool_name",
        "selected_tool_key",
        "tool_family",
        "selected_tool_family",
        "category",
        "final_answer",
        "message",
        "response",
        "content",
        "visible_response",
    ]
    .into_iter()
    .map(normalized_workflow_token)
    .collect::<Vec<_>>();
    let payload_keys = payload
        .keys()
        .map(|key| normalized_workflow_token(key))
        .collect::<Vec<_>>();
    !payload_keys
        .iter()
        .any(|key| reserved_keys.iter().any(|reserved| reserved == key))
        && expected_keys
            .iter()
            .any(|expected| payload_keys.iter().any(|key| key == expected))
}

fn workflow_request_payload_from_json_response(
    request: &Value,
    family_key: &str,
    tool_key: &str,
) -> Option<Value> {
    workflow_tool_request_object_field(
        request,
        &default_workflow_tool_menu_contract(),
        "request_payload",
    )
    .and_then(|value| workflow_tool_request_payload_from_json_value(&value))
    .or_else(|| {
        workflow_payload_object_matches_selected_tool(request, family_key, tool_key)
            .then(|| request.clone())
    })
}

fn workflow_request_payload_from_response(
    family_key: &str,
    tool_key: &str,
    response: &str,
) -> Option<Value> {
    workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_request_payload_from_json_response(&request, family_key, tool_key)
        })
        .or_else(|| {
            manual_toolbox_payload_json(response).and_then(|request| {
                workflow_request_payload_from_json_response(&request, family_key, tool_key)
            })
        })
        .filter(Value::is_object)
}

fn manual_toolbox_tool_invocation_markup_repair_enabled() -> bool {
    default_workflow_tool_menu_contract()
        .pointer("/tool_invocation_markup_repair_contract/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn workflow_xmlish_tag_values(text: &str, tag: &str, max_values: usize) -> Vec<String> {
    let tag = tag
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>()
        .to_ascii_lowercase();
    if tag.is_empty() || max_values == 0 {
        return Vec::new();
    }
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let lowered = text.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut values = Vec::new();
    while cursor < lowered.len() && values.len() < max_values {
        let Some(start_offset) = lowered[cursor..].find(&open) else {
            break;
        };
        let value_start = cursor + start_offset + open.len();
        let Some(end_offset) = lowered[value_start..].find(&close) else {
            break;
        };
        let value_end = value_start + end_offset;
        let value = clean_text(text.get(value_start..value_end).unwrap_or(""), 1_000);
        if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
            values.push(value);
        }
        cursor = value_end + close.len();
    }
    values
}

fn workflow_declared_tool_matches_for_selection(
    contract: &Value,
    tool_token: &str,
) -> Vec<(String, String, String, Value)> {
    let Some(families) = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    for (family_key, tools) in families {
        let tool_key = workflow_tool_key_for_selection(contract, family_key, tool_token);
        if tool_key.is_empty() {
            continue;
        }
        let Some(tool) = tools.as_array().and_then(|rows| {
            rows.iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        }) else {
            continue;
        };
        let already_seen = matches
            .iter()
            .any(|(seen_family, seen_tool, _, _)| seen_family == family_key && seen_tool == &tool_key);
        if !already_seen {
            matches.push((
                family_key.clone(),
                tool_key.clone(),
                workflow_option_label(&tool),
                tool,
            ));
        }
    }
    matches
}

fn workflow_request_format_object(tool: &Value) -> Map<String, Value> {
    tool.get("request_format")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn workflow_literal_request_format_default(value: &Value) -> Option<Value> {
    match value {
        Value::String(raw) => {
            let value = clean_text(raw, 120);
            (!value.is_empty() && !value.contains('<') && !value.contains('|'))
                .then_some(Value::String(value))
        }
        Value::Bool(_) | Value::Number(_) => Some(value.clone()),
        _ => None,
    }
}

fn workflow_query_pack_tool_for_family(
    contract: &Value,
    family_key: &str,
) -> Option<(String, String, Value)> {
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                let format = workflow_request_format_object(tool);
                (format.contains_key("query") && format.contains_key("queries")).then(|| {
                    (
                        workflow_option_key(tool),
                        workflow_option_label(tool),
                        tool.clone(),
                    )
                })
            })
        })
}

fn workflow_first_url_in_text(text: &str) -> String {
    text.split_whitespace()
        .find_map(|token| {
            let trimmed = token.trim_matches(|ch: char| {
                ch == '"' || ch == '\'' || ch == '`' || ch == '<' || ch == '>' || ch == ')' || ch == '('
            });
            (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
                .then(|| clean_text(trimmed.trim_end_matches([',', '.', ';']), 1_000))
        })
        .unwrap_or_default()
}

fn workflow_markup_field_values(response: &str, key: &str) -> Vec<String> {
    let mut tags = vec![key.to_string()];
    match normalized_workflow_token(key).as_str() {
        "queries" => tags.push("query".to_string()),
        "cmd" => tags.push("command".to_string()),
        "objective" => tags.push("task".to_string()),
        "pattern" => tags.push("query".to_string()),
        _ => {}
    }
    let mut values = Vec::new();
    for tag in tags {
        for value in workflow_xmlish_tag_values(response, &tag, 12) {
            if !values.iter().any(|existing| existing == &value) {
                values.push(value);
            }
        }
    }
    values
}

fn workflow_markup_payload_from_declared_tool(
    response: &str,
    message: &str,
    tool: &Value,
    query_values: &[String],
) -> Option<Value> {
    let request_format = workflow_request_format_object(tool);
    if request_format.is_empty() {
        return None;
    }
    let is_query_pack = request_format.contains_key("queries");
    let mut input = Map::new();
    for (key, format_value) in request_format.iter() {
        if key == "queries" {
            let values = if query_values.len() > 1 {
                query_values.to_vec()
            } else {
                workflow_markup_field_values(response, key)
            };
            if !values.is_empty() {
                input.insert(
                    key.clone(),
                    Value::Array(values.into_iter().map(Value::String).collect()),
                );
            }
            continue;
        }
        if key == "query" {
            let query_pack_fallback = clean_text(message, 1_000);
            if is_query_pack && query_values.len() > 1 && !query_pack_fallback.is_empty() {
                input.insert(key.clone(), Value::String(query_pack_fallback));
            } else if let Some(value) = query_values.first() {
                input.insert(key.clone(), Value::String(clean_text(value, 1_000)));
            } else if !query_pack_fallback.is_empty() {
                input.insert(key.clone(), Value::String(query_pack_fallback));
            }
            continue;
        }
        let values = workflow_markup_field_values(response, key);
        if let Some(value) = values.first() {
            input.insert(key.clone(), Value::String(clean_text(value, 1_000)));
            continue;
        }
        if key == "url" {
            let url = workflow_first_url_in_text(response)
                .if_empty_then(|| workflow_first_url_in_text(message));
            if !url.is_empty() {
                input.insert(key.clone(), Value::String(url));
            }
            continue;
        }
        if let Some(default_value) = workflow_literal_request_format_default(format_value) {
            input.insert(key.clone(), default_value);
        }
    }
    if request_format.contains_key("queries") && !input.contains_key("queries") {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 1_000))
            .unwrap_or_default();
        if !query.is_empty() {
            input.insert("queries".to_string(), json!([query]));
        }
    }
    for (key, format_value) in request_format.iter() {
        if !input.contains_key(key) {
            if let Some(default_value) = workflow_literal_request_format_default(format_value) {
                input.insert(key.clone(), default_value);
            }
        }
    }
    request_format
        .keys()
        .all(|key| input.contains_key(key))
        .then_some(Value::Object(input))
}

fn manual_toolbox_pending_request_from_tool_invocation_markup(
    response: &str,
    message: &str,
) -> Option<Value> {
    if !manual_toolbox_tool_invocation_markup_repair_enabled() {
        return None;
    }
    let contract = default_workflow_tool_menu_contract();
    let tool_tags = contract
        .pointer("/tool_invocation_markup_repair_contract/accepted_tool_name_tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|tag| clean_text(tag, 80))
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|tags| !tags.is_empty())
        .unwrap_or_else(|| vec!["tool".to_string()]);
    let mut declared_tools = Vec::<(String, String, String, Value)>::new();
    for tag in tool_tags {
        for tool_token in workflow_xmlish_tag_values(response, &tag, 8) {
            let matches = workflow_declared_tool_matches_for_selection(&contract, &tool_token);
            if matches.len() != 1 {
                return None;
            }
            let candidate = matches.into_iter().next().unwrap();
            if !declared_tools.iter().any(|(family, tool_key, _, _)| {
                family == &candidate.0 && tool_key == &candidate.1
            }) {
                declared_tools.push(candidate);
            }
        }
    }
    if declared_tools.len() != 1 {
        return None;
    }
    let (family_key, mut tool_key, mut tool_label, mut tool) = declared_tools.remove(0);
    let query_values = workflow_xmlish_tag_values(response, "query", 12);
    if query_values.len() > 1 {
        let current_format = workflow_request_format_object(&tool);
        if !current_format.contains_key("queries") {
            if let Some((pack_tool_key, pack_tool_label, pack_tool)) =
                workflow_query_pack_tool_for_family(&contract, &family_key)
            {
                tool_key = pack_tool_key;
                tool_label = pack_tool_label;
                tool = pack_tool;
            }
        }
    }
    let input = workflow_markup_payload_from_declared_tool(response, message, &tool, &query_values)?;
    let mut pending_request = manual_toolbox_pending_request_from_parts(
        &family_key,
        &tool_key,
        &tool_label,
        input,
        message,
    )?;
    pending_request["source"] = json!("tool_invocation_markup_repair");
    pending_request["selection_source"] = json!("tool_invocation_markup_repair");
    pending_request["markup_repair"] = json!({
        "type": "declared_tool_invocation_markup",
        "chat_injection_allowed": false
    });
    Some(pending_request)
}

fn response_has_declared_tool_invocation_markup(response: &str, message: &str) -> bool {
    manual_toolbox_pending_request_from_tool_invocation_markup(response, message).is_some()
}

fn manual_toolbox_pending_request_from_parts(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
    input: Value,
    message: &str,
) -> Option<Value> {
    let tool_name = canonical_manual_toolbox_tool_name(family_key, tool_key);
    if tool_name.is_empty() || !input.is_object() {
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
        "source": "split_manual_toolbox_gates",
        "tool_name": tool_name,
        "selected_tool_family": clean_text(family_key, 120),
        "selected_tool_label": clean_text(tool_label, 120),
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn manual_toolbox_latent_candidate_recovery_enabled() -> bool {
    default_workflow_tool_menu_contract()
        .pointer("/latent_candidate_recovery_contract/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn manual_toolbox_pending_request_from_latent_candidate(
    candidate: &Value,
    message: &str,
) -> Option<Value> {
    if !manual_toolbox_latent_candidate_recovery_enabled()
        || candidate.get("workflow_only").and_then(Value::as_bool) != Some(true)
    {
        return None;
    }
    let contract = default_workflow_tool_menu_contract();
    let family_token = [
        "selected_tool_family",
        "tool_family",
        "family",
        "selected_family",
    ]
    .into_iter()
    .find_map(|key| candidate.get(key).and_then(Value::as_str))
    .unwrap_or("");
    let family_key = workflow_family_key_for_selection(&contract, family_token);
    if family_key.is_empty() {
        return None;
    }
    let tool_token = [
        "selected_tool_key",
        "tool_key",
        "tool",
        "tool_name",
        "selected_tool",
    ]
    .into_iter()
    .find_map(|key| candidate.get(key).and_then(Value::as_str))
    .unwrap_or("");
    let tool_key = workflow_tool_key_for_selection(&contract, &family_key, tool_token);
    if tool_key.is_empty() {
        return None;
    }
    let tool_label = [
        "selected_tool_label",
        "tool_label",
        "label",
        "selected_label",
    ]
    .into_iter()
    .find_map(|key| candidate.get(key).and_then(Value::as_str))
    .map(|label| clean_text(label, 120))
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| tool_key.clone());
    let input = candidate
        .get("input")
        .or_else(|| candidate.get("request_payload"))
        .cloned()
        .filter(Value::is_object)?;
    let mut pending_request = manual_toolbox_pending_request_from_parts(
        &family_key,
        &tool_key,
        &tool_label,
        input,
        message,
    )?;
    pending_request["source"] = json!("latent_candidate_recovery");
    if let Some(selection_source) = candidate.get("selection_source").and_then(Value::as_str) {
        pending_request["selection_source"] = json!(clean_text(selection_source, 120));
    }
    if let Some(discovery_receipt) = candidate.get("discovery_receipt").and_then(Value::as_str) {
        pending_request["latent_candidate_receipt"] = json!(clean_text(discovery_receipt, 240));
    }
    Some(pending_request)
}

fn manual_toolbox_pending_request_from_latent_candidates(
    candidates: &Value,
    message: &str,
) -> Option<Value> {
    let mut valid_candidates = candidates
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|candidate| {
                    manual_toolbox_pending_request_from_latent_candidate(candidate, message)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (valid_candidates.len() == 1).then(|| valid_candidates.remove(0))
}

fn manual_toolbox_active_gate_id(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "gate_1_work_category_menu"
    } else if family_key.is_empty() {
        "gate_2_tool_family_menu"
    } else if tool_key.is_empty() {
        "gate_3_tool_menu"
    } else {
        "gate_4_request_payload_input"
    }
}

fn manual_toolbox_pending_direct_response_path(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_llm_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_llm_tool_family"
    } else if tool_key.is_empty() {
        "gate_3_pending_llm_tool_choice"
    } else {
        "gate_4_pending_llm_tool_request"
    }
}

fn manual_toolbox_pending_stage_status(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_tool_family_selection"
    } else if tool_key.is_empty() {
        "gate_3_pending_tool_selection"
    } else {
        "gate_4_pending_request_payload"
    }
}

fn manual_toolbox_invalid_reject_reason(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "tool_category_without_selection_diagnostic_only"
    } else if family_key.is_empty() {
        "tool_family_without_selection_diagnostic_only"
    } else if tool_key.is_empty() {
        "tool_without_selection_diagnostic_only"
    } else {
        "tool_request_without_payload_submission_diagnostic_only"
    }
}
