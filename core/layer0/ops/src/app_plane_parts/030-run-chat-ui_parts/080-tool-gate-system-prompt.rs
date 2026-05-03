fn chat_ui_workflow_gate_option_labels(contract: &Value, has_tools: Option<bool>) -> Vec<String> {
    let first_gate_id = chat_ui_first_workflow_gate_id(contract);
    chat_ui_workflow_gate_options(contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .filter_map(|option| {
            let label = chat_ui_workflow_option_label(&option);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .collect()
}

fn chat_ui_workflow_example_tool_key(contract: &Value) -> String {
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| {
            families
                .values()
                .filter_map(Value::as_array)
                .flat_map(|tools| tools.iter())
                .filter_map(|tool| tool.get("key").and_then(Value::as_str))
                .next()
        })
        .map(|key| clean(key, 80))
        .unwrap_or_default()
}

fn chat_ui_workflow_tool_submission_format(contract: &Value) -> String {
    let first_gate_id = chat_ui_first_workflow_gate_id(contract);
    chat_ui_workflow_gate(contract, &first_gate_id)
        .pointer("/submission_contract/accepted_outputs")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .find(|row| !clean(row, 240).is_empty())
        })
        .map(|row| clean(row, 240))
        .unwrap_or_default()
}

fn chat_ui_render_workflow_instruction_template(contract: &Value, template: &str) -> String {
    let first_gate_id = chat_ui_first_workflow_gate_id(contract);
    let gate_prompt = chat_ui_workflow_gate(contract, &first_gate_id)
        .get("question")
        .and_then(Value::as_str)
        .map(|row| clean(row, 120))
        .unwrap_or_default();
    clean(
        &template
            .replace("{gate_question}", &gate_prompt)
            .replace(
                "{category_options}",
                &format!(
                    "`{}`",
                    chat_ui_workflow_gate_option_labels(contract, None).join("`, `")
                ),
            )
            .replace(
                "{no_tool_categories}",
                &format!(
                    "`{}`",
                    chat_ui_workflow_gate_option_labels(contract, Some(false)).join("`, `")
                ),
            )
            .replace(
                "{tool_bearing_categories}",
                &format!(
                    "`{}`",
                    chat_ui_workflow_gate_option_labels(contract, Some(true)).join("`, `")
                ),
            )
            .replace(
                "{tool_submission_format}",
                &chat_ui_workflow_tool_submission_format(contract),
            )
            .replace(
                "{example_tool_key}",
                &chat_ui_workflow_example_tool_key(contract),
            ),
        900,
    )
}

fn chat_ui_tool_gate_system_prompt(_raw_input: &str) -> String {
    let contract = chat_ui_default_workflow_contract();
    contract
        .get("llm_gate_instruction")
        .and_then(Value::as_str)
        .map(|template| chat_ui_render_workflow_instruction_template(&contract, template))
        .unwrap_or_default()
}
