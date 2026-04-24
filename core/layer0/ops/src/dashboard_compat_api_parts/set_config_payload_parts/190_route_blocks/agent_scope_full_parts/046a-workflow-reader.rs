#[derive(Clone, Debug)]
struct WorkflowDefinition {
    name: String,
    workflow_type: String,
    default_workflow: bool,
    description: String,
    stages: Vec<String>,
    final_response_policy: String,
    gate_contract: String,
    source_path: String,
}

const WORKFLOW_SPEC_COMPLEX_PROMPT_CHAIN_V1: &str =
    include_str!("workflows/complex_prompt_chain_v1.workflow.json");
const WORKFLOW_SPEC_SIMPLE_CONVERSATION_V1: &str =
    include_str!("workflows/simple_conversation_v1.workflow.json");
const WORKFLOW_SPEC_FORGECODE_STRUCTURED_ASSIMILATION_V1: &str =
    include_str!("workflows/forgecode_structured_assimilation_v1.workflow.json");
const WORKFLOW_SPEC_FORGECODE_RAW_CAPABILITY_ASSIMILATION_V1: &str =
    include_str!("workflows/forgecode_raw_capability_assimilation_v1.workflow.json");

const WORKFLOW_SPEC_SOURCES: &[(&str, &str)] = &[
    (
        "workflows/complex_prompt_chain_v1.workflow.json",
        WORKFLOW_SPEC_COMPLEX_PROMPT_CHAIN_V1,
    ),
    (
        "workflows/simple_conversation_v1.workflow.json",
        WORKFLOW_SPEC_SIMPLE_CONVERSATION_V1,
    ),
    (
        "workflows/forgecode_structured_assimilation_v1.workflow.json",
        WORKFLOW_SPEC_FORGECODE_STRUCTURED_ASSIMILATION_V1,
    ),
    (
        "workflows/forgecode_raw_capability_assimilation_v1.workflow.json",
        WORKFLOW_SPEC_FORGECODE_RAW_CAPABILITY_ASSIMILATION_V1,
    ),
];

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
        "source_path": definition.source_path
    })
}

fn workflow_spec_error_definition() -> WorkflowDefinition {
    WorkflowDefinition {
        name: "workflow_spec_error_v1".to_string(),
        workflow_type: "hard_agent_workflow".to_string(),
        default_workflow: true,
        description:
            "Fail-closed workflow used only when no valid JSON workflow specs could be loaded."
                .to_string(),
        stages: vec![
            "gate_1_need_tool_access_menu".to_string(),
            "gate_6_llm_final_output".to_string(),
        ],
        final_response_policy: "fail_closed_runtime_guard".to_string(),
        gate_contract: "workflow_gate_error_v1".to_string(),
        source_path: "builtin:spec_error".to_string(),
    }
}

fn parse_workflow_definition(source_path: &str, raw_spec: &str) -> Option<WorkflowDefinition> {
    let parsed: Value = serde_json::from_str(raw_spec).ok()?;
    let name = clean_text(parsed.get("name").and_then(Value::as_str).unwrap_or(""), 80);
    if name.is_empty() {
        return None;
    }
    let workflow_type = clean_text(
        parsed
            .get("workflow_type")
            .and_then(Value::as_str)
            .unwrap_or("hard_agent_workflow"),
        80,
    );
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
            .and_then(Value::as_str)
            .unwrap_or("llm_authored_when_online"),
        120,
    );
    let gate_contract = clean_text(
        parsed
            .get("gate_contract")
            .and_then(Value::as_str)
            .unwrap_or("tool_menu_interface_v1"),
        80,
    );
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
    Some(WorkflowDefinition {
        name,
        workflow_type: if workflow_type.is_empty() {
            "hard_agent_workflow".to_string()
        } else {
            workflow_type
        },
        default_workflow: parsed
            .get("default")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        description,
        stages,
        final_response_policy,
        gate_contract,
        source_path: source_path.to_string(),
    })
}

fn normalize_workflow_defaults(workflows: &mut [WorkflowDefinition]) {
    if workflows.is_empty() {
        return;
    }
    let default_count = workflows.iter().filter(|row| row.default_workflow).count();
    if default_count == 0 {
        workflows[0].default_workflow = true;
        return;
    }
    if default_count > 1 {
        let mut seen_default = false;
        for row in workflows.iter_mut() {
            if row.default_workflow {
                if seen_default {
                    row.default_workflow = false;
                } else {
                    seen_default = true;
                }
            }
        }
    }
}

fn load_workflow_library() -> Vec<WorkflowDefinition> {
    let mut parsed = WORKFLOW_SPEC_SOURCES
        .iter()
        .filter_map(|(source_path, raw_spec)| parse_workflow_definition(source_path, raw_spec))
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        parsed.push(workflow_spec_error_definition());
    }
    normalize_workflow_defaults(&mut parsed);
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

fn default_workflow_definition() -> WorkflowDefinition {
    workflow_library_registry()
        .iter()
        .find(|row| row.default_workflow)
        .cloned()
        .or_else(|| workflow_library_registry().first().cloned())
        .unwrap_or_else(workflow_spec_error_definition)
}

fn turn_workflow_library_catalog() -> Vec<Value> {
    workflow_library_registry()
        .iter()
        .map(workflow_definition_to_json)
        .collect::<Vec<_>>()
}

fn default_turn_workflow_name() -> String {
    default_workflow_definition().name
}

fn workflow_name_hint_from_mode(workflow_mode: &str) -> String {
    let cleaned = clean_text(workflow_mode, 120);
    if cleaned.is_empty() {
        return String::new();
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered == "model_direct_answer" {
        return "simple_conversation_v1".to_string();
    }
    if lowered == "model_inline_tool_execution" {
        return "complex_prompt_chain_v1".to_string();
    }
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
    let selected = if hint.is_empty() {
        default_workflow_definition()
    } else {
        workflow_definition_by_name(&hint).unwrap_or_else(default_workflow_definition)
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
    }

    #[test]
    fn workflow_reader_sources_current_workflows_from_json_specs() {
        let catalog = turn_workflow_library_catalog();
        for workflow in [
            "complex_prompt_chain_v1",
            "simple_conversation_v1",
        ] {
            let source = catalog
                .iter()
                .find(|row| {
                    row.get("name")
                        .and_then(Value::as_str)
                        .map(|name| name == workflow)
                        .unwrap_or(false)
                })
                .and_then(|row| row.get("source_path"))
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(
                source.starts_with("workflows/"),
                "workflow `{workflow}` not sourced from JSON spec: {source}"
            );
        }
    }
}
