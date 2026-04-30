// SRS: V13-WORKFLOW-GATE-003

fn workflow_json_tool_menu_specs() -> Vec<(&'static str, serde_json::Value)> {
    let workflow_dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/",
        "agent_scope_full_parts/workflows"
    );
    [
        "complex_prompt_chain_v1.workflow.json",
        "simple_conversation_v1.workflow.json",
        "forgecode_structured_assimilation_v1.workflow.json",
        "forgecode_raw_capability_assimilation_v1.workflow.json",
    ]
    .into_iter()
    .map(|file_name| {
        let path = format!("{workflow_dir}/{file_name}");
        let raw = std::fs::read_to_string(&path).expect("workflow json readable");
        let parsed = serde_json::from_str(&raw).expect("workflow json parseable");
        (file_name, parsed)
    })
    .collect()
}

fn workflow_json_contract<'a>(
    file_name: &str,
    workflow: &'a serde_json::Value,
) -> &'a serde_json::Value {
    workflow
        .get("tool_menu_interface_contract")
        .unwrap_or_else(|| panic!("{file_name} missing tool_menu_interface_contract"))
}

fn workflow_json_gate<'a>(
    file_name: &str,
    contract: &'a serde_json::Value,
    gate_id: &str,
) -> &'a serde_json::Value {
    contract
        .pointer(&format!("/gates/{gate_id}"))
        .unwrap_or_else(|| panic!("{file_name} missing gate {gate_id}"))
}

fn workflow_json_option<'a>(
    file_name: &str,
    gate: &'a serde_json::Value,
    key: &str,
) -> &'a serde_json::Value {
    gate.get("options")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("{file_name} gate missing options"))
        .iter()
        .find(|row| row.get("key").and_then(serde_json::Value::as_str) == Some(key))
        .unwrap_or_else(|| panic!("{file_name} missing option {key}"))
}

#[test]
fn workflow_json_tool_menu_contract_declares_private_no_cancel_and_loopback() {
    for (file_name, workflow) in workflow_json_tool_menu_specs() {
        let contract = workflow_json_contract(file_name, &workflow);
        assert_eq!(
            contract
                .get("visible_chat_policy")
                .and_then(serde_json::Value::as_str),
            Some("llm_final_only_no_system_injection"),
            "{file_name}"
        );
        assert_eq!(
            contract
                .get("system_injected_chat_text_allowed")
                .and_then(serde_json::Value::as_bool),
            Some(false),
            "{file_name}"
        );

        let no_option = workflow_json_option(
            file_name,
            workflow_json_gate(file_name, contract, "gate_1_need_tool_access_menu"),
            "no",
        );
        assert_eq!(
            no_option
                .get("private_token")
                .and_then(serde_json::Value::as_bool),
            Some(true),
            "{file_name}"
        );
        assert_eq!(
            no_option
                .get("visible_chat")
                .and_then(serde_json::Value::as_bool),
            Some(false),
            "{file_name}"
        );
        assert_eq!(
            no_option
                .get("transition")
                .and_then(serde_json::Value::as_str),
            Some("gate_6_llm_final_output"),
            "{file_name}"
        );

        let cancel_option = workflow_json_option(
            file_name,
            workflow_json_gate(file_name, contract, "gate_4b_tool_confirmation_menu"),
            "cancel",
        );
        assert_eq!(
            cancel_option
                .get("terminal_state")
                .and_then(serde_json::Value::as_str),
            Some("cancelled"),
            "{file_name}"
        );

        let another_tool_option = workflow_json_option(
            file_name,
            workflow_json_gate(file_name, contract, "gate_5_post_tool_menu"),
            "another_tool",
        );
        assert_eq!(
            another_tool_option
                .get("transition")
                .and_then(serde_json::Value::as_str),
            Some("gate_2_tool_family_menu"),
            "{file_name}"
        );
        let loopback_declared = contract
            .get("declared_loopbacks")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{file_name} missing declared_loopbacks"))
            .iter()
            .any(|row| {
                row.get("from").and_then(serde_json::Value::as_str) == Some("gate_5_post_tool_menu")
                    && row.get("on").and_then(serde_json::Value::as_str) == Some("another_tool")
                    && row.get("to").and_then(serde_json::Value::as_str)
                        == Some("gate_2_tool_family_menu")
            });
        assert!(loopback_declared, "{file_name}");
    }
}

#[test]
fn workflow_json_tool_menu_contract_all_gates_are_menu_or_text_input() {
    for (file_name, workflow) in workflow_json_tool_menu_specs() {
        let gates = workflow_json_contract(file_name, &workflow)
            .get("gates")
            .and_then(serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{file_name} missing gates"));
        for (gate_id, gate) in gates {
            let input_kind = gate
                .get("input_kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            assert!(
                matches!(input_kind, "multiple_choice" | "text_input"),
                "{file_name} {gate_id} uses invalid input_kind {input_kind}"
            );
        }
    }
}
