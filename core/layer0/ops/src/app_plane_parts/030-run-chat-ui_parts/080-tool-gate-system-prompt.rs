fn chat_ui_tool_gate_system_prompt(raw_input: &str) -> String {
    let gate = chat_ui_turn_tool_decision_tree(raw_input);
    let gate_prompt = clean(
        gate.get("gate_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Need tool access for this query? T/F"),
        120,
    );
    let tool_family_menu = clean(
        &gate
            .get("tool_family_menu")
            .cloned()
            .unwrap_or_else(|| json!([]))
            .to_string(),
        1_200,
    );
    let tool_menu_by_family = clean(
        &gate
            .get("tool_menu_by_family")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string(),
        2_200,
    );
    clean(
        &format!(
            "Workflow interface contract: present fields only; do not recommend, infer, classify, or explain. Gate 1 is multiple choice: `{gate_prompt}` options `F) no tools, answer directly` and `T) use a tool`. If F, emit <workflow_gate>{{\"need_tool_access\":false}}</workflow_gate> and answer normally. If T, present only this numbered family menu: {tool_family_menu}. After family selection, present only that family's numbered tool menu from: {tool_menu_by_family}. After tool selection, present only the selected tool's request_format as a data-entry field. After results, present only `1) finish` or `2) another tool`; if finish, synthesize the final answer yourself. Workflow telemetry is not chat text.",
        ),
        3_200,
    )
}
