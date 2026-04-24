pub(crate) fn chat_ui_turn_tool_decision_tree(raw_input: &str) -> Value {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    let meta_control_message = chat_ui_turn_is_meta_control_message(raw_input);
    let meta_diagnostic_request = chat_ui_is_meta_diagnostic_request(&lowered);
    let status_check_message = chat_ui_message_is_tooling_status_check(raw_input);
    let explicit_web_intent = false;
    let explicit_tool_operation_intent = false;
    let auto_decisions_disabled = true;
    let requires_file_mutation = false;
    let requires_live_web = false;
    let requires_local_lookup = false;
    let has_sufficient_information = false;
    let should_call_tools = false;
    let info_source = "menu_only";
    let reason_code = "manual_menu_presented";
    let decision_authority_mode = "llm_menu_only_v1";
    let gate_enforcement_mode = "disabled";
    let gate_is_advisory = false;
    let automatic_tool_calls_allowed = false;
    let tool_selection_authority = "llm_submitted_menu_or_text_input";
    let llm_should_answer_directly = false;
    let workflow_retry_limit = 1;
    let needs_tool_access = false;
    let gate_prompt = "Need tool access for this query? T/F";
    let selected_tool_family = "unselected";
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
        }
    ]);
    let file_tool_menu = json!([
            {
                "option": 1,
                "key": "parse_workspace",
                "label": "Parse workspace",
                "request_format": {"path":"<path>", "operation":"inspect|read|mutate"},
                "request_example": {"path":"core/layer0/ops/src", "operation":"inspect"}
            },
            {
                "option": 2,
                "key": "read_file",
                "label": "Read file",
                "request_format": {"path":"<path>"},
                "request_example": {"path":"core/layer0/ops/src/main.rs"}
            },
            {
                "option": 3,
                "key": "apply_patch",
                "label": "Apply patch",
                "request_format": {"path":"<path>", "patch":"<unified diff>"},
                "request_example": {"path":"core/layer0/ops/src/main.rs", "patch":"*** Begin Patch\n*** Update File: core/layer0/ops/src/main.rs\n@@\n- old\n+ new\n*** End Patch"}
            }
        ]);
    let web_tool_menu = json!([
            {
                "option": 1,
                "key": "batch_query",
                "label": "Web search",
                "request_format": {"source":"web", "query":"<search criteria>", "aperture":"medium"},
                "request_example": {"source":"web", "query":"latest rust 2026 release notes", "aperture":"medium"}
            },
            {
                "option": 2,
                "key": "web_fetch",
                "label": "Fetch URL",
                "request_format": {"url":"<https url>"},
                "request_example": {"url":"https://www.rust-lang.org/"}
            }
        ]);
    let memory_workspace_tool_menu = json!([
            {
                "option": 1,
                "key": "read_memory",
                "label": "Read memory",
                "request_format": {"scope":"session|workspace", "query":"<criteria>"},
                "request_example": {"scope":"workspace", "query":"recent orchestration commits"}
            },
            {
                "option": 2,
                "key": "workspace_search",
                "label": "Search workspace",
                "request_format": {"path":"<path>", "pattern":"<criteria>"},
                "request_example": {"path":"core/layer0/ops/src", "pattern":"need_tool_access"}
            }
        ]);
    let tool_menu_by_family = json!({
        "file_tools": file_tool_menu,
        "web_tools": web_tool_menu,
        "memory_or_workspace_tools": memory_workspace_tool_menu,
        "none": []
    });
    let tool_menu = json!([]);
    let manual_tool_selection = true;
    json!({
        "contract": "tool_decision_tree_v3",
        "auto_decisions_disabled": auto_decisions_disabled,
        "manual_gate_mode": "llm_only_multiple_choice_v1",
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_tool_operation_intent": explicit_tool_operation_intent,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "gate_prompt": gate_prompt,
        "gate_decision_mode": "manual_need_tool_access",
        "reason_code": reason_code,
        "meta_diagnostic_request": meta_diagnostic_request,
        "info_source": info_source,
        "selected_tool_family": selected_tool_family,
        "decision_authority_mode": decision_authority_mode,
        "gate_enforcement_mode": gate_enforcement_mode,
        "gate_is_advisory": gate_is_advisory,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "tool_menu_by_family": tool_menu_by_family,
        "tool_family_selection_required": true,
        "request_payload_entry_required": true,
        "manual_tool_selection": manual_tool_selection,
        "meta_control_message": meta_control_message,
        "status_check_message": status_check_message,
        "llm_should_answer_directly": llm_should_answer_directly,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit
    })
}
