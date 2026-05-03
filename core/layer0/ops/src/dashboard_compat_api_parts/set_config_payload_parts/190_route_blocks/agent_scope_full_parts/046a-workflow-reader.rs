#[derive(Clone, Debug)]
struct WorkflowDefinition {
    name: String,
    workflow_type: String,
    default_workflow: bool,
    description: String,
    stages: Vec<String>,
    final_response_policy: String,
    gate_contract: String,
    workflow_source_of_truth_contract: Value,
    tool_menu_interface_contract: Value,
    final_output_contract: Value,
    source_path: String,
}

static WORKFLOW_LIBRARY_REGISTRY: std::sync::OnceLock<Vec<WorkflowDefinition>> =
    std::sync::OnceLock::new();

fn workflow_definition_to_json(definition: &WorkflowDefinition) -> Value {
    json!({
        "name": definition.name,
        "workflow_type": definition.workflow_type,
        "default": definition.default_workflow,
        "description": definition.description,
        "stages": definition.stages,
        "final_response_policy": definition.final_response_policy,
        "gate_contract": definition.gate_contract,
        "workflow_source_of_truth_contract": definition.workflow_source_of_truth_contract,
        "tool_menu_interface_contract": definition.tool_menu_interface_contract,
        "final_output_contract": definition.final_output_contract,
        "source_path": definition.source_path
    })
}

fn workflow_contract_string_at(value: &Value, pointer: &str, max_len: usize) -> Option<String> {
    let row = clean_text(value.pointer(pointer).and_then(Value::as_str)?, max_len);
    (!row.is_empty()).then_some(row)
}

fn workflow_contract_array_at(value: &Value, pointer: &str) -> Option<Vec<Value>> {
    let rows = value.pointer(pointer).and_then(Value::as_array)?.clone();
    (!rows.is_empty()).then_some(rows)
}

fn workflow_contract_object_at(value: &Value, pointer: &str) -> Option<Value> {
    value
        .pointer(pointer)
        .filter(|row| row.is_object())
        .cloned()
}

fn workflow_contract_array_strings_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_tool_menu_contract_is_complete(contract: &Value) -> bool {
    if workflow_contract_string_at(contract, "/version", 80).is_none()
        || workflow_contract_string_at(contract, "/llm_gate_instruction", 1_400).is_none()
        || workflow_contract_string_at(contract, "/llm_tool_request_instruction", 4_000).is_none()
        || contract
            .get("system_injected_chat_text_allowed")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return false;
    }
    let gate_order = workflow_contract_array_strings_at(contract, "/gate_order");
    let allowed_shapes = workflow_contract_array_strings_at(contract, "/gate_shapes_allowed");
    let Some(gates) = contract.get("gates").and_then(Value::as_object) else {
        return false;
    };
    if gate_order.is_empty() || allowed_shapes.is_empty() || gates.is_empty() {
        return false;
    }
    let tool_request_fields =
        workflow_contract_array_strings_at(contract, "/tool_request_submission_contract/field_order");
    let tool_request_contract_complete = !tool_request_fields.is_empty()
        && tool_request_fields.iter().all(|field| {
            workflow_contract_string_at(
                contract,
                &format!("/tool_request_submission_contract/field_labels/{field}"),
                80,
            )
            .is_some()
        })
        && contract
            .pointer("/tool_request_submission_contract/chat_injection_allowed")
            .and_then(Value::as_bool)
            == Some(false)
        && contract
            .pointer("/tool_request_submission_contract/system_may_infer_missing_fields")
            .and_then(Value::as_bool)
            == Some(false);
    let declared_gates_are_valid = gate_order.iter().all(|gate_id| {
        gates
            .get(gate_id)
            .and_then(Value::as_object)
            .and_then(|gate| gate.get("input_kind").and_then(Value::as_str))
            .map(|input_kind| allowed_shapes.iter().any(|shape| shape == input_kind))
            .unwrap_or(false)
    });
    let declares_final_output = gates.values().any(|gate| {
        gate.get("final_output_contract")
            .filter(|contract| contract.is_object())
            .is_some()
    });
    tool_request_contract_complete && declared_gates_are_valid && declares_final_output
}

fn workflow_source_of_truth_contract_is_complete(contract: &Value) -> bool {
    workflow_contract_string_at(contract, "/interaction_source", 120).is_some()
        && workflow_contract_string_at(contract, "/rust_reader_role", 240).is_some()
        && contract
            .get("hardcoded_interaction_behavior_allowed")
            .and_then(Value::as_bool)
            == Some(false)
        && workflow_contract_array_at(contract, "/json_owns").is_some()
        && workflow_contract_array_at(contract, "/rust_owns").is_some()
}

fn parse_workflow_definition(source_path: &str, raw_spec: &str) -> Option<WorkflowDefinition> {
    let parsed: Value = serde_json::from_str(raw_spec).ok()?;
    let name = clean_text(parsed.get("name").and_then(Value::as_str).unwrap_or(""), 80);
    if name.is_empty() {
        return None;
    }
    let workflow_type = clean_text(parsed.get("workflow_type").and_then(Value::as_str)?, 80);
    if workflow_type.is_empty() {
        return None;
    }
    let description = clean_text(
        parsed
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let final_response_policy = clean_text(
        parsed
            .get("final_response_policy")
            .and_then(Value::as_str)?,
        120,
    );
    if final_response_policy.is_empty() {
        return None;
    }
    let gate_contract = clean_text(parsed.get("gate_contract").and_then(Value::as_str)?, 80);
    if gate_contract.is_empty() {
        return None;
    }
    let stages = parsed
        .get("stages")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stages.is_empty() {
        return None;
    }
    let workflow_source_of_truth_contract = parsed
        .get("workflow_source_of_truth_contract")
        .filter(|value| value.is_object())
        .cloned()?;
    if !workflow_source_of_truth_contract_is_complete(&workflow_source_of_truth_contract) {
        return None;
    }
    let tool_menu_interface_contract = parsed
        .get("tool_menu_interface_contract")
        .filter(|value| value.is_object())
        .cloned()?;
    if !workflow_tool_menu_contract_is_complete(&tool_menu_interface_contract) {
        return None;
    }
    let final_output_contract = tool_menu_interface_contract
        .get("gates")
        .and_then(Value::as_object)?
        .values()
        .find_map(|gate| {
            gate.get("final_output_contract")
                .filter(|value| value.is_object())
                .cloned()
        })?;
    Some(WorkflowDefinition {
        name,
        workflow_type,
        default_workflow: parsed
            .get("default")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        description,
        stages,
        final_response_policy,
        gate_contract,
        workflow_source_of_truth_contract,
        tool_menu_interface_contract,
        final_output_contract,
        source_path: source_path.to_string(),
    })
}

fn workflow_library_has_exactly_one_json_default(workflows: &[WorkflowDefinition]) -> bool {
    workflows.iter().filter(|row| row.default_workflow).count() == 1
}

fn workflow_spec_directory_candidates() -> Vec<std::path::PathBuf> {
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
    candidates
}

fn workflow_spec_sources_from_disk() -> Vec<(String, String)> {
    for dir in workflow_spec_directory_candidates() {
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
        let sources = paths
            .into_iter()
            .filter_map(|path| {
                let raw = std::fs::read_to_string(&path).ok()?;
                Some((path.to_string_lossy().to_string(), raw))
            })
            .collect::<Vec<_>>();
        if !sources.is_empty() {
            return sources;
        }
    }
    Vec::new()
}

fn load_workflow_library() -> Vec<WorkflowDefinition> {
    let parsed = workflow_spec_sources_from_disk()
        .into_iter()
        .filter_map(|(source_path, raw_spec)| parse_workflow_definition(&source_path, &raw_spec))
        .collect::<Vec<_>>();
    if parsed.is_empty() || !workflow_library_has_exactly_one_json_default(&parsed) {
        return Vec::new();
    }
    parsed
}

fn workflow_library_registry() -> &'static [WorkflowDefinition] {
    WORKFLOW_LIBRARY_REGISTRY
        .get_or_init(load_workflow_library)
        .as_slice()
}

fn workflow_definition_by_name(name: &str) -> Option<WorkflowDefinition> {
    let cleaned = clean_text(name, 80);
    if cleaned.is_empty() {
        return None;
    }
    workflow_library_registry()
        .iter()
        .find(|row| row.name.eq_ignore_ascii_case(&cleaned))
        .cloned()
}

fn default_workflow_definition() -> Option<WorkflowDefinition> {
    workflow_library_registry()
        .iter()
        .find(|row| row.default_workflow)
        .cloned()
        .or_else(|| workflow_library_registry().first().cloned())
}

fn turn_workflow_library_catalog() -> Vec<Value> {
    workflow_library_registry()
        .iter()
        .map(workflow_definition_to_json)
        .collect::<Vec<_>>()
}

fn default_turn_workflow_name() -> String {
    default_workflow_definition()
        .map(|workflow| workflow.name)
        .unwrap_or_default()
}

fn workflow_name_hint_from_mode(workflow_mode: &str) -> String {
    let cleaned = clean_text(workflow_mode, 120);
    if cleaned.is_empty() {
        return String::new();
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["workflow=", "workflow:", "workflow/"] {
        if let Some(idx) = lowered.find(marker) {
            let start = idx + marker.len();
            if start >= cleaned.len() {
                continue;
            }
            let tail = clean_text(&cleaned[start..], 80);
            if tail.is_empty() {
                continue;
            }
            let token = tail
                .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '|')
                .next()
                .unwrap_or("")
                .to_string();
            if !token.is_empty() {
                return token;
            }
        }
    }
    String::new()
}

fn selected_turn_workflow(workflow_mode: &str) -> Value {
    let hint = workflow_name_hint_from_mode(workflow_mode);
    let Some(selected) = (if hint.is_empty() {
        default_workflow_definition()
    } else {
        workflow_definition_by_name(&hint).or_else(default_workflow_definition)
    }) else {
        return json!({
            "workflow_loaded": false,
            "mode": clean_text(workflow_mode, 80),
            "selection_reason": "no_json_workflow_loaded"
        });
    };
    let selection_reason = if hint.is_empty() {
        "default_library_workflow".to_string()
    } else if workflow_definition_by_name(&hint).is_some() {
        "mode_hint_workflow".to_string()
    } else {
        "mode_hint_unknown_fallback_default".to_string()
    };
    json!({
        "name": selected.name,
        "workflow_type": selected.workflow_type,
        "mode": clean_text(workflow_mode, 80),
        "selection_reason": selection_reason,
        "final_response_policy": selected.final_response_policy,
        "gate_contract": selected.gate_contract,
        "workflow_source_of_truth_contract": selected.workflow_source_of_truth_contract,
        "tool_menu_interface_contract": selected.tool_menu_interface_contract,
        "final_output_contract": selected.final_output_contract,
        "source_path": selected.source_path
    })
}

#[cfg(test)]
mod workflow_reader_tests {
    use super::*;

    #[test]
    fn workflow_reader_loads_external_specs() {
        let catalog = turn_workflow_library_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|name| name == "complex_prompt_chain_v1")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn workflow_reader_enforces_single_default() {
        let defaults = workflow_library_registry()
            .iter()
            .filter(|row| row.default_workflow)
            .count();
        assert_eq!(defaults, 1);
        assert!(workflow_library_has_exactly_one_json_default(
            workflow_library_registry()
        ));
    }

    #[test]
    fn workflow_reader_sources_current_workflows_from_json_specs() {
        let catalog = turn_workflow_library_catalog();
        assert!(!catalog.is_empty());
        for row in catalog {
            let source = row
                .get("source_path")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(
                source.ends_with(".workflow.json") && !source.starts_with("builtin:"),
                "workflow not sourced from disk JSON spec: {source}"
            );
        }
    }

    #[test]
    fn workflow_reader_projects_final_output_contract_from_json_spec() {
        let selected = selected_turn_workflow("");
        assert_eq!(
            selected
                .pointer("/final_output_contract/visible_chat_source")
                .and_then(Value::as_str),
            Some("llm_final_answer_only")
        );
        let chat_excludes = selected
            .pointer("/final_output_contract/chat_excludes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            chat_excludes
                .iter()
                .any(|value| value.as_str() == Some("agent_internal_notes")),
            "{}",
            selected
        );
        assert!(
            chat_excludes
                .iter()
                .any(|value| value.as_str() == Some("prompt_analysis")),
            "{}",
            selected
        );
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_json_authority_contract() {
        let raw_spec = json!({
            "name": "missing_authority_contract_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: Rust must not invent the workflow contract.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "gates": {
                    "gate_6_llm_final_output": {
                        "final_output_contract": {
                            "visible_chat_source": "llm_final_answer_only"
                        }
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_when_json_permits_hardcoded_interaction_behavior() {
        let raw_spec = json!({
            "name": "bad_authority_escape_hatch_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: JSON may not authorize Rust-authored interaction gates.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": true,
                "json_owns": ["interaction_gates"],
                "rust_owns": ["json_loading"]
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "system_injected_chat_text_allowed": false,
                "llm_gate_instruction": "Template.",
                "gate_order": ["gate_1_work_category_menu"],
                "gate_shapes_allowed": ["multiple_choice"],
                "terminal_states": ["completed"],
                "declared_loopbacks": [{"from": "gate_5_post_tool_menu", "on": "another_tool", "to": "gate_2_tool_family_menu"}],
                "tool_family_menu": [{"key": "respond_directly"}],
                "tool_menu_by_family": {"none": []},
                "gates": {
                    "gate_1_work_category_menu": {
                        "input_kind": "multiple_choice",
                        "question": "Question",
                        "submission_contract": {"accepted_outputs": ["Respond directly"]},
                        "options": [{"key": "respond_directly", "label": "Respond directly"}]
                    },
                    "gate_2_tool_family_menu": {"input_kind": "multiple_choice"},
                    "gate_3_tool_menu": {"input_kind": "multiple_choice"},
                    "gate_4_request_payload_input": {"input_kind": "text_input"},
                    "gate_4b_tool_confirmation_menu": {"input_kind": "multiple_choice"},
                    "gate_5_post_tool_menu": {"input_kind": "multiple_choice"},
                    "gate_6_llm_final_output": {
                        "input_kind": "text_input",
                        "final_output_contract": {"visible_chat_source": "llm_final_answer_only"}
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_final_output_contract() {
        let raw_spec = json!({
            "name": "missing_final_contract_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: final-output separation must come from JSON.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": false
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "gates": {}
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_tool_menu_contract_details() {
        let raw_spec = json!({
            "name": "missing_tool_menu_details_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: Rust must not synthesize missing menus.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": false,
                "json_owns": ["interaction_gates"],
                "rust_owns": ["json_loading"]
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "system_injected_chat_text_allowed": false,
                "llm_gate_instruction": "Template.",
                "gates": {
                    "gate_6_llm_final_output": {
                        "input_kind": "text_input",
                        "final_output_contract": {"visible_chat_source": "llm_final_answer_only"}
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_does_not_invent_default_when_json_omits_it() {
        let mut workflows = workflow_library_registry().to_vec();
        for workflow in workflows.iter_mut() {
            workflow.default_workflow = false;
        }

        assert!(!workflow_library_has_exactly_one_json_default(&workflows));
    }

    #[test]
    fn workflow_reader_does_not_map_runtime_modes_to_hidden_workflows() {
        assert_eq!(workflow_name_hint_from_mode("model_direct_answer"), "");
        assert_eq!(workflow_name_hint_from_mode("model_inline_tool_execution"), "");
        assert_eq!(
            workflow_name_hint_from_mode("workflow=complex_prompt_chain_v1"),
            "complex_prompt_chain_v1"
        );
    }
}
