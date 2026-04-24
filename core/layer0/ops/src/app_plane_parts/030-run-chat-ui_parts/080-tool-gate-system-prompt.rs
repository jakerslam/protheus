fn chat_ui_tool_gate_system_prompt(raw_input: &str) -> String {
    let gate = chat_ui_turn_tool_decision_tree(raw_input);
    let gate_prompt = clean(
        gate.get("gate_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Need tools? Yes/No"),
        120,
    );
    clean(
        &format!(
            "Workflow toolbox interface: use only one gate at a time and do not expose workflow telemetry as chat text. Gate 1 is multiple choice: `{gate_prompt}`. Valid choices are `No` and `Yes`. If No, answer directly and naturally. If Yes, continue to the next workflow gate; final visible chat text must be authored by you, not by the system.",
        ),
        700,
    )
}
