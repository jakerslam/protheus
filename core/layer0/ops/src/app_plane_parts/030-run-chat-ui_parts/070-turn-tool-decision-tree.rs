fn chat_ui_default_workflow_contract() -> Value {
    chat_ui_workflow_specs_from_disk()
        .into_iter()
        .filter_map(|raw| serde_json::from_str::<Value>(&raw).ok())
        .find(|spec| spec.get("default").and_then(Value::as_bool).unwrap_or(false))
        .or_else(|| {
            chat_ui_workflow_specs_from_disk()
                .into_iter()
                .filter_map(|raw| serde_json::from_str::<Value>(&raw).ok())
                .next()
        })
        .and_then(|spec| spec.get("tool_menu_interface_contract").cloned())
        .unwrap_or_else(|| json!({}))
}

fn chat_ui_workflow_specs_from_disk() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("INFRING_WORKFLOW_DIR") {
        candidates.push(std::path::PathBuf::from(dir));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows"));
    }
    candidates.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows",
    ));
    for dir in candidates {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.ends_with(".workflow.json"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        paths.sort();
        let specs = paths
            .into_iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect::<Vec<_>>();
        if !specs.is_empty() {
            return specs;
        }
    }
    Vec::new()
}

fn chat_ui_workflow_gate(contract: &Value, gate_id: &str) -> Value {
    contract
        .pointer(&format!("/gates/{gate_id}"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn chat_ui_workflow_option_key(option: &Value) -> String {
    clean(option.get("key").and_then(Value::as_str).unwrap_or(""), 120)
}

fn chat_ui_workflow_option_label(option: &Value) -> String {
    clean(option.get("label").and_then(Value::as_str).unwrap_or(""), 120)
}

fn chat_ui_workflow_gate_options(contract: &Value, gate_id: &str) -> Vec<Value> {
    chat_ui_workflow_gate(contract, gate_id)
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn chat_ui_workflow_gate_order(contract: &Value) -> Vec<String> {
    contract
        .get("gate_order")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn chat_ui_first_workflow_gate_id(contract: &Value) -> String {
    chat_ui_workflow_gate_order(contract)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn chat_ui_workflow_gate_option_keys(contract: &Value, has_tools: bool) -> Value {
    let first_gate_id = chat_ui_first_workflow_gate_id(contract);
    json!(
        chat_ui_workflow_gate_options(contract, &first_gate_id)
            .into_iter()
            .filter(|option| {
                option
                    .get("has_tools")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    == has_tools
            })
            .filter_map(|option| {
                let key = chat_ui_workflow_option_key(&option);
                if key.is_empty() {
                    None
                } else {
                    Some(key)
                }
            })
            .collect::<Vec<_>>()
    )
}

fn chat_ui_workflow_allowed_outputs(contract: &Value) -> Value {
    let first_gate_id = chat_ui_first_workflow_gate_id(contract);
    chat_ui_workflow_gate(contract, &first_gate_id)
        .pointer("/submission_contract/accepted_outputs")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn chat_ui_workflow_family_menu(contract: &Value, selected_tool_family: &str) -> Value {
    let selected = clean(selected_tool_family, 120);
    let rows = contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Value::Array(
        rows.into_iter()
            .map(|mut row| {
                let key = clean(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
                row["selected"] = Value::Bool(!selected.is_empty() && selected == key);
                row
            })
            .collect(),
    )
}

pub(crate) fn chat_ui_turn_tool_decision_tree(raw_input: &str) -> Value {
    let _ = raw_input;
    let contract = chat_ui_default_workflow_contract();
    let first_gate_id = chat_ui_first_workflow_gate_id(&contract);
    let gate_1 = chat_ui_workflow_gate(&contract, &first_gate_id);
    let meta_control_message = false;
    let meta_diagnostic_request = false;
    let status_check_message = false;
    let selected_tool_family = "unselected";
    let selected_work_category = Value::Null;
    let work_category_outputs = chat_ui_workflow_allowed_outputs(&contract);
    let gate_prompt = clean(gate_1.get("question").and_then(Value::as_str).unwrap_or(""), 120);
    let gate_1_resume_token = if first_gate_id.is_empty() {
        String::new()
    } else {
        format!("{first_gate_id}.awaiting_llm_submission")
    };
    let gate_1_submission = json!({
        "gate_id": first_gate_id,
        "input_shape": {
            "type": gate_1.get("input_kind").and_then(Value::as_str).unwrap_or("multiple_choice"),
            "allowed_outputs": work_category_outputs.clone()
        },
        "llm_submission": Value::Null,
        "accepted": false,
        "resume_token": gate_1_resume_token
    });
    json!({
        "contract": "manual_toolbox_gate_v1",
        "workflow_gate_contract": contract.get("version").cloned().unwrap_or_else(|| json!("tool_menu_interface_v1")),
        "auto_decisions_disabled": true,
        "system_may_select_tools": false,
        "tool_recommendations_allowed": false,
        "gate_1_question_type": "multiple_choice",
        "gate_1_allowed_outputs": work_category_outputs,
        "first_gate_id": gate_1_submission.get("gate_id").cloned().unwrap_or(Value::Null),
        "current_gate_id": gate_1_submission.get("gate_id").cloned().unwrap_or(Value::Null),
        "manual_gate_mode": "llm_only_multiple_choice_v1",
        "requires_file_mutation": false,
        "requires_local_lookup": false,
        "requires_live_web": false,
        "explicit_tool_operation_intent": false,
        "explicit_web_intent": false,
        "has_sufficient_information": false,
        "should_call_tools": false,
        "needs_tool_access": Value::Null,
        "selected_work_category": selected_work_category,
        "no_tool_categories": chat_ui_workflow_gate_option_keys(&contract, false),
        "tool_bearing_categories": chat_ui_workflow_gate_option_keys(&contract, true),
        "gate_1_submission_status": "awaiting_llm_submission",
        "gate_1_decision_source": "pending_llm_submission",
        "gate_submission": gate_1_submission,
        "gate_prompt": gate_prompt,
        "gate_decision_mode": "manual_work_category_v1",
        "reason_code": "manual_menu_presented",
        "meta_diagnostic_request": meta_diagnostic_request,
        "info_source": "menu_only",
        "selected_tool_family": selected_tool_family,
        "decision_authority_mode": "llm_menu_only_v1",
        "gate_enforcement_mode": "disabled",
        "gate_is_advisory": false,
        "tool_family_menu": chat_ui_workflow_family_menu(&contract, selected_tool_family),
        "tool_menu": json!([]),
        "tool_menu_by_family": contract.get("tool_menu_by_family").cloned().unwrap_or_else(|| json!({})),
        "tool_family_selection_required": true,
        "request_payload_entry_required": true,
        "manual_tool_selection": true,
        "semantic_message_detectors_active": false,
        "meta_control_message": meta_control_message,
        "status_check_message": status_check_message,
        "llm_should_answer_directly": false,
        "automatic_tool_calls_allowed": false,
        "tool_selection_authority": "llm_submitted_menu_or_text_input",
        "workflow_retry_limit": 1
    })
}
