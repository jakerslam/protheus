fn chat_ui_turn_tool_decision_tree(raw_input: &str) -> Value {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    let explicit_web_intent = chat_ui_has_explicit_web_intent(&lowered);
    let meta_control_message = chat_ui_turn_is_meta_control_message(raw_input);
    let status_check_message = if meta_control_message {
        false
    } else {
        chat_ui_message_is_tooling_status_check(raw_input)
    };
    let requires_file_mutation = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_file_mutation(raw_input)
    };
    let requires_live_web = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_requests_live_web(raw_input) && explicit_web_intent
    };
    let requires_local_lookup = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_local_lookup(raw_input)
    };
    let has_sufficient_information =
        meta_control_message
            || status_check_message
            || (!requires_file_mutation && !requires_live_web && !requires_local_lookup);
    let should_call_tools =
        !has_sufficient_information && (requires_file_mutation || requires_live_web || requires_local_lookup);
    let info_source = if requires_live_web {
        "web"
    } else if requires_local_lookup || requires_file_mutation {
        "local"
    } else {
        "none"
    };
    let recommended_tool_family = if requires_file_mutation {
        "file_tools"
    } else if requires_live_web {
        "web_tools"
    } else if requires_local_lookup {
        "memory_or_workspace_tools"
    } else {
        "none"
    };
    json!({
        "contract": "tool_decision_tree_v2",
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "should_call_tools": should_call_tools,
        "info_source": info_source,
        "recommended_tool_family": recommended_tool_family,
        "meta_control_message": meta_control_message,
        "status_check_message": status_check_message
    })
}
