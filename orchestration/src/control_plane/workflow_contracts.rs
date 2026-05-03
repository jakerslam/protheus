// Layer ownership: orchestration (non-canonical orchestration coordination only).
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const REQUIRED_TERMINAL_STATES: &[&str] =
    &["completed", "needs_input", "blocked", "failed", "aborted"];
pub const REQUIRED_TELEMETRY_STREAMS: &[&str] = &[
    "workflow_state",
    "agent_internal_notes",
    "tool_trace",
    "eval_trace",
    "final_answer",
];
pub const REQUIRED_TOOL_FAMILIES: &[&str] =
    &["workspace", "web", "memory", "agent", "shell", "browser"];
pub const WORKFLOW_CONTRACT_SCHEMA_VERSION: &str = "typed_execution_contract_v1";
pub const WORKFLOW_SOURCE_OF_TRUTH_SCHEMA_VERSION: &str = "workflow_source_of_truth_contract_v1";
pub const WORKFLOW_INTERACTION_SOURCE: &str = "json_workflow_spec";
pub const WORKFLOW_RUST_READER_ROLE: &str = "validate_execute_trace_only";
pub const REQUIRED_JSON_OWNS: &[&str] = &[
    "interaction_gates",
    "gate_options",
    "gate_transitions",
    "tool_family_menus",
    "tool_input_schemas",
    "confirmation_states",
    "loopbacks",
    "final_output_contract",
];
pub const REQUIRED_RUST_OWNS: &[&str] = &[
    "json_loading",
    "schema_validation",
    "state_transition_execution",
    "tool_execution_handoff",
    "receipt_binding",
    "trace_export",
    "kernel_policy_enforcement",
];

const TOOL_FAMILY_SCHEMAS: &[(&str, &str, &str)] = &[
    (
        "workspace",
        "workspace_tool_request_v1",
        "workspace_tool_observation_v1",
    ),
    ("web", "web_tool_request_v1", "web_tool_observation_v1"),
    (
        "memory",
        "memory_tool_request_v1",
        "memory_tool_observation_v1",
    ),
    (
        "agent",
        "agent_tool_request_v1",
        "agent_tool_observation_v1",
    ),
    (
        "shell",
        "shell_tool_request_v1",
        "shell_tool_observation_v1",
    ),
    (
        "browser",
        "browser_tool_request_v1",
        "browser_tool_observation_v1",
    ),
];

#[derive(Debug, Clone, Deserialize)]
struct WorkflowSpec {
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    workflow_type: String,
    #[serde(default)]
    workflow_role: String,
    #[serde(default)]
    stages: Vec<String>,
    #[serde(default)]
    subtemplates: Vec<Value>,
    workflow_source_of_truth_contract: Option<WorkflowSourceOfTruthContract>,
    typed_execution_contract: Option<TypedExecutionContract>,
    interaction_gate_contract: Option<InteractionGateContract>,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowRegistry {
    #[serde(default)]
    schema_version: String,
    #[serde(default)]
    default_workflow_id: String,
    #[serde(default)]
    promotion_lifecycle: Vec<String>,
    #[serde(default)]
    workflows: Vec<WorkflowRegistryEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowRegistryEntry {
    pub workflow_id: String,
    pub tier: String,
    pub source_framework: String,
    pub source_path: String,
    pub runtime_selectable: bool,
    pub promotion_status: String,
    #[serde(default)]
    pub test_scenarios: Vec<String>,
    #[serde(default)]
    pub promotion_requirements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TypedExecutionContract {
    #[serde(default)]
    gate_kind: String,
    #[serde(default)]
    input_kind: String,
    #[serde(default)]
    allowed_transitions: Vec<String>,
    #[serde(default)]
    timeout_ms: u64,
    #[serde(default)]
    retry_policy: RetryPolicy,
    #[serde(default)]
    terminal_states: Vec<String>,
    #[serde(default)]
    telemetry_streams: Vec<String>,
    #[serde(default)]
    tool_family_contracts: Vec<String>,
    #[serde(default)]
    visible_chat_policy: String,
    #[serde(default)]
    run_budgets: RunBudgets,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkflowSourceOfTruthContract {
    #[serde(default)]
    pub interaction_source: String,
    #[serde(default)]
    pub rust_reader_role: String,
    #[serde(default)]
    pub hardcoded_interaction_behavior_allowed: bool,
    #[serde(default)]
    pub json_owns: Vec<String>,
    #[serde(default)]
    pub rust_owns: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RetryPolicy {
    #[serde(default)]
    pub max_retries: u64,
    #[serde(default)]
    pub on_failure: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RunBudgets {
    #[serde(default)]
    pub max_stages: u64,
    #[serde(default)]
    pub max_model_turns: u64,
    #[serde(default)]
    pub max_tool_calls: u64,
    #[serde(default)]
    pub token_budget: u64,
    #[serde(default)]
    pub loop_signature_detector: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Hash)]
pub struct InteractionGateContract {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub tool_family_menu_stage: String,
    #[serde(default)]
    pub tool_request_payload_stage: String,
    #[serde(default)]
    pub tool_observation_stage: String,
    #[serde(default)]
    pub final_answer_stage: String,
    #[serde(default)]
    pub recovery_stage: String,
    #[serde(default)]
    pub gates: Vec<InteractionGateDefinition>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Hash)]
pub struct InteractionGateDefinition {
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub parser_kind: String,
    #[serde(default)]
    pub true_values: Vec<String>,
    #[serde(default)]
    pub false_values: Vec<String>,
    #[serde(default)]
    pub finish_values: Vec<String>,
    #[serde(default)]
    pub another_tool_values: Vec<String>,
    #[serde(default)]
    pub choice_base: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedWorkflowGraph {
    pub workflow_id: String,
    pub source_json_path: String,
    pub contract_schema_version: String,
    pub source_of_truth_schema_version: String,
    pub interaction_source: String,
    pub rust_reader_role: String,
    pub hardcoded_interaction_behavior_allowed: bool,
    pub json_owns: Vec<String>,
    pub rust_owns: Vec<String>,
    pub workflow_tier: String,
    pub source_framework: String,
    pub runtime_selectable: bool,
    pub promotion_status: String,
    pub workflow_type: String,
    pub workflow_role: String,
    pub subtemplate_count: usize,
    pub stages: Vec<String>,
    pub transitions: Vec<String>,
    pub gate_contract: StructuredGateContract,
    pub interaction_gate_contract: InteractionGateContract,
    pub terminal_states: Vec<String>,
    pub telemetry_streams: Vec<String>,
    pub tool_families: Vec<String>,
    pub visible_chat_policy: String,
    pub run_budgets: RunBudgets,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuredGateContract {
    pub gate_kind: String,
    pub input_kind: String,
    pub allowed_input_shapes: Vec<String>,
    pub resume_token_required: bool,
    pub visibility_scope: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolFamilyContract {
    pub family: &'static str,
    pub request_schema: &'static str,
    pub observation_schema: &'static str,
    pub receipt_binding_required: bool,
    pub timeout_ms: u64,
    pub retry_semantics: &'static str,
    pub visible_chat_leakage_forbidden: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowValidation {
    pub path: String,
    pub ok: bool,
    pub workflow_id: String,
    pub errors: Vec<String>,
    pub graph: Option<NormalizedWorkflowGraph>,
}

fn workflow_directory_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("INFRING_ORCHESTRATION_WORKFLOW_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/control_plane/workflows"));
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("orchestration/src/control_plane/workflows"));
    }
    candidates
}

fn collect_workflow_json_paths(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            paths.extend(collect_workflow_json_paths(&path));
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".workflow.json"))
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths
}

fn workflow_source_path_for_disk_path(path: &Path) -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(manifest)
        .ok()
        .map(|rel| format!("orchestration/{}", rel.display()))
        .unwrap_or_else(|| path.display().to_string())
        .replace('\\', "/")
}

fn workflow_source_entries() -> Vec<(String, String)> {
    for dir in workflow_directory_candidates() {
        let paths = collect_workflow_json_paths(&dir);
        let entries = paths
            .into_iter()
            .filter_map(|path| {
                let raw = std::fs::read_to_string(&path).ok()?;
                Some((workflow_source_path_for_disk_path(&path), raw))
            })
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            return entries;
        }
    }
    Vec::new()
}

fn workflow_registry_raw() -> Option<String> {
    if let Ok(path) = std::env::var("INFRING_ORCHESTRATION_WORKFLOW_REGISTRY") {
        if let Ok(raw) = std::fs::read_to_string(path) {
            return Some(raw);
        }
    }
    workflow_directory_candidates()
        .into_iter()
        .map(|dir| dir.join("workflow_registry.json"))
        .find_map(|path| std::fs::read_to_string(path).ok())
}

pub fn registered_workflow_validations() -> Vec<WorkflowValidation> {
    let registry = workflow_registry_by_path();
    workflow_source_entries()
        .into_iter()
        .map(|(path, raw)| validate_workflow_source(&path, &raw, registry.get(&path)))
        .collect()
}

pub fn registered_workflow_registry() -> Vec<WorkflowRegistryEntry> {
    parse_workflow_registry()
        .map(|registry| registry.workflows)
        .unwrap_or_default()
}

pub fn registered_workflow_graphs() -> Vec<NormalizedWorkflowGraph> {
    registered_workflow_validations()
        .into_iter()
        .filter_map(|row| row.graph)
        .collect()
}

fn validate_workflow_source(
    path: &str,
    raw: &str,
    registry_entry: Option<&WorkflowRegistryEntry>,
) -> WorkflowValidation {
    let parsed = serde_json::from_str::<WorkflowSpec>(raw);
    let Ok(spec) = parsed else {
        return validation(
            path,
            false,
            "unknown",
            vec!["json_parse_failed".to_string()],
            None,
        );
    };
    let workflow_id = normalized_id(&spec);
    let stages = clean_list(spec.stages);
    let mut errors = Vec::new();
    let Some(registry_entry) = registry_entry else {
        errors.push("missing_workflow_registry_entry".to_string());
        return validation(path, false, &workflow_id, errors, None);
    };
    validate_registry_entry(path, &workflow_id, registry_entry, &mut errors);
    if workflow_id.is_empty() {
        errors.push("missing_workflow_id".to_string());
    }
    let workflow_type = spec.workflow_type.trim().to_string();
    let workflow_role = spec.workflow_role.trim().to_string();
    if workflow_type != "control_plane_orchestration_workflow" {
        errors.push("invalid_workflow_type".to_string());
    }
    if !matches!(
        workflow_role.as_str(),
        "assistant_response_workflow" | "assimilation_workflow_template"
    ) {
        errors.push("invalid_workflow_role".to_string());
    }
    if workflow_role == "assimilation_workflow_template" {
        validate_assimilation_subtemplates(&spec.subtemplates, &mut errors);
    } else if !spec.subtemplates.is_empty() {
        errors.push("assistant_workflow_must_not_declare_subtemplates".to_string());
    }
    if stages.is_empty() {
        errors.push("missing_stages".to_string());
    }
    let Some(source_contract) = spec.workflow_source_of_truth_contract else {
        errors.push("missing_workflow_source_of_truth_contract".to_string());
        return validation(path, false, &workflow_id, errors, None);
    };
    let Some(contract) = spec.typed_execution_contract else {
        errors.push("missing_typed_execution_contract".to_string());
        return validation(path, false, &workflow_id, errors, None);
    };
    let Some(interaction_gate_contract) = spec.interaction_gate_contract else {
        errors.push("missing_interaction_gate_contract".to_string());
        return validation(path, false, &workflow_id, errors, None);
    };
    let graph = compile_graph(WorkflowGraphCompileInput {
        source_path: path,
        workflow_id: &workflow_id,
        workflow_type: &workflow_type,
        workflow_role: &workflow_role,
        registry_entry,
        subtemplate_count: spec.subtemplates.len(),
        stages,
        source_contract,
        contract,
        interaction_gate_contract,
        errors: &mut errors,
    });
    validation(path, errors.is_empty(), &workflow_id, errors, graph)
}

struct WorkflowGraphCompileInput<'a> {
    source_path: &'a str,
    workflow_id: &'a str,
    workflow_type: &'a str,
    workflow_role: &'a str,
    registry_entry: &'a WorkflowRegistryEntry,
    subtemplate_count: usize,
    stages: Vec<String>,
    source_contract: WorkflowSourceOfTruthContract,
    contract: TypedExecutionContract,
    interaction_gate_contract: InteractionGateContract,
    errors: &'a mut Vec<String>,
}

fn compile_graph(input: WorkflowGraphCompileInput<'_>) -> Option<NormalizedWorkflowGraph> {
    let WorkflowGraphCompileInput {
        source_path,
        workflow_id,
        workflow_type,
        workflow_role,
        registry_entry,
        subtemplate_count,
        stages,
        source_contract,
        contract,
        interaction_gate_contract,
        errors,
    } = input;
    let stage_set: HashSet<&str> = stages.iter().map(String::as_str).collect();
    validate_source_of_truth_contract(&source_contract, errors);
    validate_contract_basics(&contract, errors);
    validate_interaction_gate_contract(&interaction_gate_contract, errors);
    let terminal_states = clean_list(contract.terminal_states);
    let terminal_set: HashSet<&str> = terminal_states.iter().map(String::as_str).collect();
    let transitions = parse_transitions(
        &contract.allowed_transitions,
        &stage_set,
        &terminal_set,
        errors,
    );
    require_subset(
        "terminal_state",
        REQUIRED_TERMINAL_STATES,
        &terminal_set,
        errors,
    );
    let telemetry_streams = clean_list(contract.telemetry_streams);
    let telemetry_set: HashSet<&str> = telemetry_streams.iter().map(String::as_str).collect();
    require_subset(
        "telemetry_stream",
        REQUIRED_TELEMETRY_STREAMS,
        &telemetry_set,
        errors,
    );
    let tool_families = clean_list(contract.tool_family_contracts);
    let tool_set: HashSet<&str> = tool_families.iter().map(String::as_str).collect();
    require_subset("tool_family", REQUIRED_TOOL_FAMILIES, &tool_set, errors);
    if !errors.is_empty() {
        return None;
    }
    Some(NormalizedWorkflowGraph {
        workflow_id: workflow_id.to_string(),
        source_json_path: source_path.to_string(),
        contract_schema_version: WORKFLOW_CONTRACT_SCHEMA_VERSION.to_string(),
        source_of_truth_schema_version: WORKFLOW_SOURCE_OF_TRUTH_SCHEMA_VERSION.to_string(),
        interaction_source: source_contract.interaction_source,
        rust_reader_role: source_contract.rust_reader_role,
        hardcoded_interaction_behavior_allowed: source_contract
            .hardcoded_interaction_behavior_allowed,
        json_owns: clean_list(source_contract.json_owns),
        rust_owns: clean_list(source_contract.rust_owns),
        workflow_tier: registry_entry.tier.clone(),
        source_framework: registry_entry.source_framework.clone(),
        runtime_selectable: registry_entry.runtime_selectable,
        promotion_status: registry_entry.promotion_status.clone(),
        workflow_type: workflow_type.to_string(),
        workflow_role: workflow_role.to_string(),
        subtemplate_count,
        stages,
        transitions,
        gate_contract: StructuredGateContract {
            gate_kind: contract.gate_kind,
            input_kind: contract.input_kind,
            allowed_input_shapes: vec!["multiple_choice".to_string(), "text_input".to_string()],
            resume_token_required: true,
            visibility_scope: "telemetry_only_until_final_llm_output".to_string(),
        },
        interaction_gate_contract,
        terminal_states,
        telemetry_streams,
        tool_families,
        visible_chat_policy: contract.visible_chat_policy,
        run_budgets: contract.run_budgets,
    })
}

fn parse_workflow_registry() -> Option<WorkflowRegistry> {
    serde_json::from_str(&workflow_registry_raw()?).ok()
}

fn workflow_registry_by_path() -> HashMap<String, WorkflowRegistryEntry> {
    registered_workflow_registry()
        .into_iter()
        .map(|entry| (entry.source_path.clone(), entry))
        .collect()
}

pub fn workflow_registry_contract_ok() -> bool {
    let Some(registry) = parse_workflow_registry() else {
        return false;
    };
    if registry.schema_version != "workflow_registry_v1"
        || registry.default_workflow_id.trim().is_empty()
        || !registry.promotion_lifecycle.iter().any(|row| row == "lab")
        || !registry
            .promotion_lifecycle
            .iter()
            .any(|row| row == "official")
    {
        return false;
    }
    let source_entries = workflow_source_entries();
    let known_paths: HashSet<&str> = source_entries.iter().map(|(path, _)| path.as_str()).collect();
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    let mut official_count = 0usize;
    let mut lab_count = 0usize;
    for entry in &registry.workflows {
        if entry.workflow_id.trim().is_empty()
            || entry.source_path.trim().is_empty()
            || entry.source_framework.trim().is_empty()
            || entry.promotion_status.trim().is_empty()
            || !known_paths.contains(entry.source_path.as_str())
            || !ids.insert(entry.workflow_id.as_str())
            || !paths.insert(entry.source_path.as_str())
            || entry.test_scenarios.is_empty()
            || entry.promotion_requirements.is_empty()
        {
            return false;
        }
        match entry.tier.as_str() {
            "official" => {
                official_count += 1;
                if !entry.runtime_selectable
                    || entry.promotion_status != "official"
                    || !entry
                        .source_path
                        .starts_with("orchestration/src/control_plane/workflows/official/")
                {
                    return false;
                }
            }
            "lab" => {
                lab_count += 1;
                if entry.runtime_selectable
                    || entry.promotion_status != "lab"
                    || !entry
                        .source_path
                        .starts_with("orchestration/src/control_plane/workflows/lab/frameworks/")
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    official_count > 0
        && lab_count > 0
        && registry.workflows.len() == source_entries.len()
        && ids.contains(registry.default_workflow_id.as_str())
}

fn validate_registry_entry(
    source_path: &str,
    workflow_id: &str,
    entry: &WorkflowRegistryEntry,
    errors: &mut Vec<String>,
) {
    if entry.workflow_id != workflow_id {
        errors.push("workflow_registry_id_mismatch".to_string());
    }
    if entry.source_path != source_path {
        errors.push("workflow_registry_source_path_mismatch".to_string());
    }
    match entry.tier.as_str() {
        "official" => {
            if !entry.runtime_selectable {
                errors.push("official_workflow_not_runtime_selectable".to_string());
            }
            if !source_path.starts_with("orchestration/src/control_plane/workflows/official/") {
                errors.push("official_workflow_outside_official_dir".to_string());
            }
        }
        "lab" => {
            if entry.runtime_selectable {
                errors.push("lab_workflow_runtime_selectable".to_string());
            }
            if !source_path.starts_with("orchestration/src/control_plane/workflows/lab/") {
                errors.push("lab_workflow_outside_lab_dir".to_string());
            }
        }
        _ => errors.push("invalid_workflow_registry_tier".to_string()),
    }
}

fn validate_source_of_truth_contract(
    contract: &WorkflowSourceOfTruthContract,
    errors: &mut Vec<String>,
) {
    if contract.interaction_source != WORKFLOW_INTERACTION_SOURCE {
        errors.push("workflow_interaction_source_not_json".to_string());
    }
    if contract.rust_reader_role != WORKFLOW_RUST_READER_ROLE {
        errors.push("workflow_rust_reader_role_not_cd_player".to_string());
    }
    if contract.hardcoded_interaction_behavior_allowed {
        errors.push("hardcoded_workflow_interaction_behavior_allowed".to_string());
    }
    let json_owns = clean_list(contract.json_owns.clone());
    let json_owns_set: HashSet<&str> = json_owns.iter().map(String::as_str).collect();
    require_subset("json_owns", REQUIRED_JSON_OWNS, &json_owns_set, errors);
    let rust_owns = clean_list(contract.rust_owns.clone());
    let rust_owns_set: HashSet<&str> = rust_owns.iter().map(String::as_str).collect();
    require_subset("rust_owns", REQUIRED_RUST_OWNS, &rust_owns_set, errors);
}

fn validate_contract_basics(contract: &TypedExecutionContract, errors: &mut Vec<String>) {
    if !matches!(
        contract.input_kind.as_str(),
        "multiple_choice" | "text_input" | "multiple_choice_or_text_input"
    ) {
        errors.push("invalid_input_kind".to_string());
    }
    if contract.gate_kind.trim().is_empty() {
        errors.push("missing_gate_kind".to_string());
    }
    if contract.timeout_ms == 0 || contract.retry_policy.on_failure.trim().is_empty() {
        errors.push("missing_timeout_or_retry_policy".to_string());
    }
    if contract.visible_chat_policy != "llm_final_only_no_system_injection" {
        errors.push("visible_chat_policy_allows_system_injection".to_string());
    }
    let budgets = &contract.run_budgets;
    if budgets.max_stages == 0
        || budgets.max_model_turns == 0
        || budgets.max_tool_calls == 0
        || budgets.token_budget == 0
        || budgets.loop_signature_detector.trim().is_empty()
    {
        errors.push("missing_run_budget_semantics".to_string());
    }
}

fn validate_interaction_gate_contract(
    contract: &InteractionGateContract,
    errors: &mut Vec<String>,
) {
    if contract.version.trim().is_empty() {
        errors.push("missing_interaction_gate_contract_version".to_string());
    }
    for field in [
        ("tool_family_menu_stage", &contract.tool_family_menu_stage),
        (
            "tool_request_payload_stage",
            &contract.tool_request_payload_stage,
        ),
        ("tool_observation_stage", &contract.tool_observation_stage),
        ("final_answer_stage", &contract.final_answer_stage),
        ("recovery_stage", &contract.recovery_stage),
    ] {
        if field.1.trim().is_empty() {
            errors.push(format!("missing_interaction_gate_{}", field.0));
        }
    }
    if contract.gates.is_empty() {
        errors.push("missing_interaction_gate_definitions".to_string());
        return;
    }
    let mut stages = HashSet::new();
    let mut parser_kinds = HashSet::new();
    for gate in &contract.gates {
        let stage = gate.stage.trim();
        let parser_kind = gate.parser_kind.trim();
        if stage.is_empty() {
            errors.push("missing_interaction_gate_stage".to_string());
        } else if !stages.insert(stage.to_string()) {
            errors.push(format!("duplicate_interaction_gate_stage:{stage}"));
        }
        if parser_kind.is_empty() {
            errors.push(format!("missing_interaction_gate_parser_kind:{stage}"));
        } else {
            parser_kinds.insert(parser_kind.to_string());
        }
        match parser_kind {
            "need_tools" => {
                if gate.true_values.is_empty() || gate.false_values.is_empty() {
                    errors.push(format!("need_tools_gate_missing_values:{stage}"));
                }
            }
            "tool_family" => {
                if gate.choice_base == 0 {
                    errors.push(format!("tool_family_gate_missing_choice_base:{stage}"));
                }
            }
            "tool_name" | "request_payload" => {}
            "post_tool" => {
                if gate.finish_values.is_empty() || gate.another_tool_values.is_empty() {
                    errors.push(format!("post_tool_gate_missing_values:{stage}"));
                }
            }
            _ => errors.push(format!("unknown_interaction_gate_parser_kind:{parser_kind}")),
        }
    }
    for required in [
        "need_tools",
        "tool_family",
        "tool_name",
        "request_payload",
        "post_tool",
    ] {
        if !parser_kinds.contains(required) {
            errors.push(format!("missing_interaction_gate_parser_kind:{required}"));
        }
    }
}

fn validate_assimilation_subtemplates(subtemplates: &[Value], errors: &mut Vec<String>) {
    if subtemplates.is_empty() {
        errors.push("missing_assimilation_subtemplates".to_string());
        return;
    }
    let mut subtemplate_ids = HashSet::new();
    for (idx, subtemplate) in subtemplates.iter().enumerate() {
        let subtemplate_id = json_str_field(subtemplate, "id");
        if subtemplate_id.is_empty() {
            errors.push(format!("missing_subtemplate_id:{idx}"));
        } else if !subtemplate_id_ok(subtemplate_id) {
            errors.push(format!("invalid_subtemplate_id:{idx}:{subtemplate_id}"));
        } else if !subtemplate_ids.insert(subtemplate_id.to_string()) {
            errors.push(format!("duplicate_subtemplate_id:{idx}:{subtemplate_id}"));
        }
        if json_str_field(subtemplate, "description").is_empty() {
            errors.push(format!("missing_subtemplate_description:{idx}"));
        }
        for field in ["required_signals", "required_gates", "source_refs"] {
            if !json_nonempty_string_array(subtemplate, field) {
                errors.push(format!("missing_subtemplate_{field}:{idx}"));
            }
            if json_string_array_has_duplicates(subtemplate, field) {
                errors.push(format!("duplicate_subtemplate_{field}:{idx}"));
            }
        }
        for source_ref in json_string_array_values(subtemplate, "source_refs") {
            if !assimilation_source_ref_ok(&source_ref) {
                errors.push(format!("invalid_subtemplate_source_ref:{idx}:{source_ref}"));
            }
        }
    }
}

fn json_str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
}

fn json_nonempty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            !items.is_empty()
                && items.iter().all(
                    |item| matches!(item.as_str().map(str::trim), Some(raw) if !raw.is_empty()),
                )
        })
        .unwrap_or(false)
}

fn json_string_array_values(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|raw| !raw.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_array_has_duplicates(value: &Value, key: &str) -> bool {
    let mut seen = HashSet::new();
    for item in json_string_array_values(value, key) {
        if !seen.insert(item) {
            return true;
        }
    }
    false
}

fn assimilation_source_ref_ok(source_ref: &str) -> bool {
    let source_ref = source_ref.trim();
    if source_ref.is_empty()
        || source_ref.starts_with('/')
        || source_ref.starts_with("http://")
        || source_ref.starts_with("https://")
        || source_ref.contains("..")
    {
        return false;
    }
    [
        "local/workspace/assimilations/",
        "local/workspace/vendor/",
        "orchestration/",
        "docs/workspace/",
        "tests/tooling/",
        "validation/",
        "core/",
        "adapters/",
    ]
    .iter()
    .any(|prefix| source_ref.starts_with(prefix))
}

fn subtemplate_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 120
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn parse_transitions(
    raw: &[String],
    stage_set: &HashSet<&str>,
    terminal_set: &HashSet<&str>,
    errors: &mut Vec<String>,
) -> Vec<String> {
    let mut rows = Vec::new();
    for item in raw {
        let Some((from, to)) = item.split_once("->") else {
            errors.push(format!("invalid_transition:{item}"));
            continue;
        };
        let from = from.trim();
        let to = to.trim();
        if !stage_set.contains(from) {
            errors.push(format!("unknown_transition_from:{from}"));
        }
        if !stage_set.contains(to) && !terminal_set.contains(to) {
            errors.push(format!("unknown_transition_to:{to}"));
        }
        rows.push(format!("{from}->{to}"));
    }
    if rows.is_empty() {
        errors.push("missing_allowed_transitions".to_string());
    }
    rows
}

pub fn tool_family_contracts() -> Vec<ToolFamilyContract> {
    TOOL_FAMILY_SCHEMAS
        .iter()
        .map(
            |(family, request_schema, observation_schema)| ToolFamilyContract {
                family,
                request_schema,
                observation_schema,
                receipt_binding_required: true,
                timeout_ms: 120_000,
                retry_semantics: "bounded_single_retry_then_recover_or_escalate",
                visible_chat_leakage_forbidden: true,
            },
        )
        .collect()
}

pub fn tool_contracts_cover_required(tool_contracts: &[ToolFamilyContract]) -> bool {
    let families: BTreeSet<&str> = tool_contracts.iter().map(|row| row.family).collect();
    REQUIRED_TOOL_FAMILIES
        .iter()
        .all(|family| families.contains(*family))
        && tool_contracts.iter().all(|row| {
            row.receipt_binding_required
                && row.visible_chat_leakage_forbidden
                && row.timeout_ms > 0
                && !row.request_schema.is_empty()
                && !row.observation_schema.is_empty()
        })
}

fn require_subset(
    label: &str,
    required: &[&str],
    actual: &HashSet<&str>,
    errors: &mut Vec<String>,
) {
    for item in required {
        if !actual.contains(item) {
            errors.push(format!("missing_{label}:{item}"));
        }
    }
}

fn validation(
    path: &str,
    ok: bool,
    workflow_id: &str,
    errors: Vec<String>,
    graph: Option<NormalizedWorkflowGraph>,
) -> WorkflowValidation {
    WorkflowValidation {
        path: path.to_string(),
        ok,
        workflow_id: workflow_id.to_string(),
        errors,
        graph,
    }
}

fn normalized_id(spec: &WorkflowSpec) -> String {
    let source = if spec.name.trim().is_empty() {
        spec.id.trim()
    } else {
        spec.name.trim()
    };
    source.to_ascii_lowercase()
}

fn clean_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .collect()
}
