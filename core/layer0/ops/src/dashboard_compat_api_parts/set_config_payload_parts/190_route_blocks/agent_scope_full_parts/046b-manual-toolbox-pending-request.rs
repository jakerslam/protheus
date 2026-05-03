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
    let pending_request = manual_toolbox_pending_request_from_response(response_text, message);
    let Some(pending_request) = pending_request else {
        return;
    };
    workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
    workflow["response"] = Value::String(String::new());
    workflow["visible_response_source"] = Value::String("private_workflow_gate_submission".to_string());
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_pending_tool_confirmation".to_string());
    if let Some(events) = workflow.get_mut("system_events").and_then(Value::as_array_mut) {
        events.push(turn_workflow_event(
            "manual_toolbox_pending_tool_request",
            pending_request,
        ));
    }
}
