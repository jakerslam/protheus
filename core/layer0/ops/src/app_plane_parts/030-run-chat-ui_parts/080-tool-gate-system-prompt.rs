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
    let should_call_tools = gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_tool_family = clean(
        gate.get("recommended_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    clean(
        &format!(
            "Deterministic tool gate for this turn: requires_file_mutation={requires_file_mutation}, has_sufficient_information={has_sufficient_information}, status_check_message={status_check_message}, explicit_web_intent={explicit_web_intent}, info_source={info_source}, should_call_tools={should_call_tools}, recommended_tool_family={recommended_tool_family}. Decision tree: (1) If file mutation is required, use file tools. (2) If enough information is already available, answer directly with no tool calls. (3) If information is missing, use local memory/workspace tools for local facts and web tools only for online/current facts. (4) Web tools are never default; call them only when explicit web intent is present. Meta/control or tooling status-check turns are direct-answer turns and should not trigger web tools.",
        ),
        4_000,
    )
}
