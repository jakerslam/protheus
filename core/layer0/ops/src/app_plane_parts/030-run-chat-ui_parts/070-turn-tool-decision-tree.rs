pub(crate) fn chat_ui_turn_tool_decision_tree(raw_input: &str) -> Value {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    let meta_control_message = chat_ui_turn_is_meta_control_message(raw_input);
    let meta_diagnostic_request = chat_ui_is_meta_diagnostic_request(&lowered);
    let explicit_web_intent = chat_ui_has_explicit_web_intent(&lowered) && !meta_diagnostic_request;
    let status_check_message = chat_ui_message_is_tooling_status_check(raw_input);
    let requires_file_mutation = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_file_mutation(raw_input)
    };
    let requires_live_web = if meta_control_message || status_check_message || meta_diagnostic_request {
        false
    } else {
        chat_ui_requests_live_web(raw_input)
    };
    let requires_local_lookup = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_local_lookup(raw_input)
    };
    let has_sufficient_information =
        meta_control_message
            || status_check_message
            || meta_diagnostic_request
            || (!requires_file_mutation && !requires_live_web && !requires_local_lookup);
    let should_call_tools =
        !has_sufficient_information && (requires_file_mutation || requires_live_web || requires_local_lookup);
    let workflow_route = if should_call_tools { "task" } else { "info" };
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
    let reason_code = if requires_file_mutation {
        "file_mutation_required"
    } else if requires_live_web {
        "explicit_live_web_required"
    } else if requires_local_lookup {
        "local_lookup_required"
    } else if meta_diagnostic_request {
        "meta_diagnostic_direct_answer"
    } else if meta_control_message {
        "meta_control_direct_answer"
    } else if status_check_message {
        "tool_status_check_direct_answer"
    } else if has_sufficient_information {
        "sufficient_info_direct_answer"
    } else {
        "insufficient_signal_default_direct_answer"
    };
    let automatic_tool_calls_allowed = requires_live_web && explicit_web_intent;
    let tool_selection_authority = "llm_selected";
    let llm_should_answer_directly = !should_call_tools;
    let workflow_retry_limit = 1;
    let needs_tool_access = should_call_tools;
    let gate_prompt = "Need tool access for this query?";
    let selected_tool_family = if needs_tool_access {
        recommended_tool_family
    } else {
        "none"
    };
    let tool_family_menu = json!([
        {
            "option": 1,
            "key": "file_tools",
            "label": "File / Workspace",
            "selected": selected_tool_family == "file_tools"
        },
        {
            "option": 2,
            "key": "web_tools",
            "label": "Web Search / Fetch",
            "selected": selected_tool_family == "web_tools"
        },
        {
            "option": 3,
            "key": "memory_or_workspace_tools",
            "label": "Memory / Workspace Read",
            "selected": selected_tool_family == "memory_or_workspace_tools"
        },
        {
            "option": 4,
            "key": "none",
            "label": "Direct answer (no tools)",
            "selected": selected_tool_family == "none"
        }
    ]);
    let tool_menu = match selected_tool_family {
        "file_tools" => json!([
            {
                "option": 1,
                "key": "parse_workspace",
                "label": "Parse workspace",
                "request_format": {"path":"<path>", "operation":"inspect|read|mutate"}
            },
            {
                "option": 2,
                "key": "read_file",
                "label": "Read file",
                "request_format": {"path":"<path>"}
            },
            {
                "option": 3,
                "key": "apply_patch",
                "label": "Apply patch",
                "request_format": {"path":"<path>", "patch":"<unified diff>"}
            }
        ]),
        "web_tools" => json!([
            {
                "option": 1,
                "key": "batch_query",
                "label": "Web search",
                "request_format": {"source":"web", "query":"<search criteria>", "aperture":"medium"}
            },
            {
                "option": 2,
                "key": "web_fetch",
                "label": "Fetch URL",
                "request_format": {"url":"<https url>"}
            }
        ]),
        "memory_or_workspace_tools" => json!([
            {
                "option": 1,
                "key": "read_memory",
                "label": "Read memory",
                "request_format": {"scope":"session|workspace", "query":"<criteria>"}
            },
            {
                "option": 2,
                "key": "workspace_search",
                "label": "Search workspace",
                "request_format": {"path":"<path>", "pattern":"<criteria>"}
            }
        ]),
        _ => json!([]),
    };
    let manual_tool_selection = needs_tool_access;
    json!({
        "contract": "tool_decision_tree_v3",
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "gate_prompt": gate_prompt,
        "workflow_route": workflow_route,
        "reason_code": reason_code,
        "meta_diagnostic_request": meta_diagnostic_request,
        "info_source": info_source,
        "recommended_tool_family": recommended_tool_family,
        "selected_tool_family": selected_tool_family,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "manual_tool_selection": manual_tool_selection,
        "meta_control_message": meta_control_message,
        "status_check_message": status_check_message,
        "llm_should_answer_directly": llm_should_answer_directly,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit
    })
}
