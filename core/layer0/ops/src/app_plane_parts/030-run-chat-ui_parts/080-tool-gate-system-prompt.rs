fn chat_ui_tool_gate_system_prompt(raw_input: &str) -> String {
    let gate = chat_ui_turn_tool_decision_tree(raw_input);
    let requires_file_mutation = gate
        .get("requires_file_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_sufficient_information = gate
        .get("has_sufficient_information")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let status_check_message = gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let explicit_web_intent = gate
        .get("explicit_web_intent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let info_source = clean(
        gate.get("info_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        40,
    );
    let should_call_tools_hint = gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let needs_tool_access = gate
        .get("needs_tool_access")
        .and_then(Value::as_bool)
        .unwrap_or(should_call_tools_hint);
    let recommended_tool_family = clean(
        gate.get("recommended_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let workflow_route_hint = clean(
        gate.get("workflow_route_hint")
            .or_else(|| gate.get("workflow_route"))
            .and_then(Value::as_str)
            .unwrap_or("none"),
        40,
    );
    let reason_code = clean(
        gate.get("reason_code")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        120,
    );
    let tool_selection_authority = clean(
        gate.get("tool_selection_authority")
            .and_then(Value::as_str)
            .unwrap_or("llm_selected"),
        80,
    );
    let decision_authority_mode = clean(
        gate.get("decision_authority_mode")
            .and_then(Value::as_str)
            .unwrap_or("llm_controlled_advisory_v1"),
        80,
    );
    let gate_enforcement_mode = clean(
        gate.get("gate_enforcement_mode")
            .and_then(Value::as_str)
            .unwrap_or("advisory_only"),
        80,
    );
    let gate_is_advisory = gate
        .get("gate_is_advisory")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let automatic_tool_calls_allowed = gate
        .get("automatic_tool_calls_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let llm_should_answer_directly = gate
        .get("llm_should_answer_directly")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let workflow_retry_limit = gate
        .get("workflow_retry_limit")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let selected_tool_family_hint = clean(
        gate.get("selected_tool_family_hint")
            .or_else(|| gate.get("selected_tool_family"))
            .and_then(Value::as_str)
            .unwrap_or(recommended_tool_family.as_str()),
        80,
    );
    clean(
        &format!(
            "Advisory workflow hints for this turn (non-authoritative): reason_code={reason_code}, requires_file_mutation_hint={requires_file_mutation}, has_sufficient_information_hint={has_sufficient_information}, status_check_message={status_check_message}, explicit_web_intent_hint={explicit_web_intent}, info_source_hint={info_source}, should_call_tools_hint={should_call_tools_hint}, needs_tool_access_hint={needs_tool_access}, workflow_route_hint={workflow_route_hint}, recommended_tool_family_hint={recommended_tool_family}, selected_tool_family_hint={selected_tool_family_hint}, tool_selection_authority={tool_selection_authority}, decision_authority_mode={decision_authority_mode}, gate_enforcement_mode={gate_enforcement_mode}, gate_is_advisory={gate_is_advisory}, automatic_tool_calls_allowed={automatic_tool_calls_allowed}, llm_should_answer_directly_hint={llm_should_answer_directly}, retry_limit={workflow_retry_limit}. Gate 1 authority contract: the LLM decides `need_tool_access` explicitly per turn; hints must never be treated as automatic classification. If tools are needed, emit one workflow decision envelope before any tool call: <workflow_gate>{{\"need_tool_access\":true,\"tool_family\":\"<family>\",\"tool_name\":\"<tool>\"}}</workflow_gate>. If tools are not needed, emit: <workflow_gate>{{\"need_tool_access\":false}}</workflow_gate> and answer directly without function calls. Canonical gate names are `need_tool_access`, `tool_family_selection`, and `post_tool_decision`. Never emit or reference deprecated gate names such as `task_or_info_route`. Web tools are never default; call them only when explicitly selected. Automatic tool triggers are prohibited; tool calls are intentional LLM selections. Meta/control or tooling status-check turns are direct-answer turns and should not trigger web tools.",
        ),
        4_000,
    )
}
