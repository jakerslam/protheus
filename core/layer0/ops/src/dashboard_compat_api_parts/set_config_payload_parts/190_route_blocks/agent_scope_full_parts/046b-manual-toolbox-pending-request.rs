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
        let query = manual_toolbox_natural_web_query(&response)?;
        (
            "batch_query",
            "web_search",
            "web search",
            json!({
                "source": "web",
                "query": query,
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

fn manual_toolbox_natural_web_query(response_text: &str) -> Option<String> {
    let cleaned = clean_text(response_text, 600);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let marker_start = lowered
        .find("web search")
        .or_else(|| lowered.find("search"))?;
    let after_marker = cleaned
        .get(marker_start..)
        .unwrap_or("")
        .split_once(" for ")
        .map(|(_, tail)| tail)
        .or_else(|| {
            cleaned
                .get(marker_start..)
                .unwrap_or("")
                .split_once(" about ")
                .map(|(_, tail)| tail)
        })
        .or_else(|| {
            cleaned
                .get(marker_start..)
                .unwrap_or("")
                .split_once(" on ")
                .map(|(_, tail)| tail)
        })?;
    let query_candidate = after_marker
        .split("Tool candidates:")
        .next()
        .unwrap_or(after_marker)
        .split("Latest user request:")
        .next()
        .unwrap_or(after_marker)
        .split("Write only")
        .next()
        .unwrap_or(after_marker)
        .lines()
        .next()
        .unwrap_or(after_marker);
    let query = clean_text(
        query_candidate
            .trim()
            .trim_start_matches(':')
            .trim()
            .trim_matches(['.', ',', ';', ':', '"', '\'', '`']),
        600,
    );
    let lowered_query = query.to_ascii_lowercase();
    if query.split_whitespace().count() < 2
        || lowered_query == "that"
        || lowered_query == "this"
        || lowered_query == "the request"
        || lowered_query.contains("tool candidates")
        || lowered_query.contains("latest user request")
        || lowered_query.contains("write only")
    {
        None
    } else {
        Some(query)
    }
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
