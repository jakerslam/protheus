fn workflow_has_manual_toolbox_candidate_menu(workflow: &Value) -> bool {
    workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        == Some("gate_1_pending_llm_tool_choice")
        || workflow
            .get("system_events")
            .and_then(Value::as_array)
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("name")
                        .or_else(|| event.get("type"))
                        .and_then(Value::as_str)
                        == Some("manual_toolbox_candidate_menu")
                })
            })
            .unwrap_or(false)
}

fn manual_toolbox_natural_pending_request(
    response_text: &str,
    message: &str,
) -> Option<Value> {
    let response = clean_text(response_text, 1_200);
    let lowered = response.to_ascii_lowercase();
    if lowered.is_empty()
        || !(lowered.contains("choose")
            || lowered.contains("select")
            || lowered.contains("use web search")
            || lowered.contains("use the web search"))
    {
        return None;
    }
    let (tool_name, family, label, input) = if lowered.contains("web search")
        || lowered.contains("search")
    {
        (
            "batch_query",
            "web_search",
            "web search",
            json!({
                "source": "web",
                "query": clean_text(message, 600),
                "aperture": "medium"
            }),
        )
    } else {
        return None;
    };
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "source": "manual_toolbox_natural_language_choice",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600),
        "choice_text": response
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "manual_toolbox_natural_language_choice",
        "tool_name": tool_name,
        "selected_tool_family": family,
        "selected_tool_label": label,
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn record_manual_toolbox_pending_request(
    workflow: &mut Value,
    response_text: &str,
    message: &str,
) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    let pending_request = manual_toolbox_pending_request_from_response(response_text, message)
        .or_else(|| {
            workflow_has_manual_toolbox_candidate_menu(workflow)
                .then(|| manual_toolbox_natural_pending_request(response_text, message))
                .flatten()
        });
    let Some(pending_request) = pending_request else {
        return;
    };
    workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("gate_1_yes_pending_tool_confirmation".to_string());
    if let Some(events) = workflow.get_mut("system_events").and_then(Value::as_array_mut) {
        events.push(turn_workflow_event(
            "manual_toolbox_pending_tool_request",
            pending_request,
        ));
    }
}
