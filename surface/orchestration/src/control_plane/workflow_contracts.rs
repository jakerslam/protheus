// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

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

const WORKFLOW_SOURCES: &[(&str, &str)] = &[
    (
        "surface/orchestration/src/control_plane/workflows/clarify_then_coordinate.workflow.json",
        include_str!("workflows/clarify_then_coordinate.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/research_synthesize_verify.workflow.json",
        include_str!("workflows/research_synthesize_verify.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/plan_execute_review.workflow.json",
        include_str!("workflows/plan_execute_review.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/diagnose_retry_escalate.workflow.json",
        include_str!("workflows/diagnose_retry_escalate.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/codex_tooling_synthesis.workflow.json",
        include_str!("workflows/codex_tooling_synthesis.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/forgecode_agent_composition.workflow.json",
        include_str!("workflows/forgecode_agent_composition.workflow.json"),
    ),
    (
        "surface/orchestration/src/control_plane/workflows/forgecode_raw_capability_assimilation.workflow.json",
        include_str!("workflows/forgecode_raw_capability_assimilation.workflow.json"),
    ),
];

#[derive(Debug, Clone, Deserialize)]
struct WorkflowSpec {
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    stages: Vec<String>,
    typed_execution_contract: Option<TypedExecutionContract>,
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

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedWorkflowGraph {
    pub workflow_id: String,
    pub stages: Vec<String>,
    pub transitions: Vec<String>,
    pub gate_contract: StructuredGateContract,
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

pub fn registered_workflow_validations() -> Vec<WorkflowValidation> {
    WORKFLOW_SOURCES
        .iter()
        .map(|(path, raw)| validate_workflow_source(path, raw))
        .collect()
}

pub fn registered_workflow_graphs() -> Vec<NormalizedWorkflowGraph> {
    registered_workflow_validations()
        .into_iter()
        .filter_map(|row| row.graph)
        .collect()
}

fn validate_workflow_source(path: &str, raw: &str) -> WorkflowValidation {
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
    if workflow_id.is_empty() {
        errors.push("missing_workflow_id".to_string());
    }
    if stages.is_empty() {
        errors.push("missing_stages".to_string());
    }
    let Some(contract) = spec.typed_execution_contract else {
        errors.push("missing_typed_execution_contract".to_string());
        return validation(path, false, &workflow_id, errors, None);
    };
    let graph = compile_graph(&workflow_id, stages, contract, &mut errors);
    validation(path, errors.is_empty(), &workflow_id, errors, graph)
}

fn compile_graph(
    workflow_id: &str,
    stages: Vec<String>,
    contract: TypedExecutionContract,
    errors: &mut Vec<String>,
) -> Option<NormalizedWorkflowGraph> {
    let stage_set: HashSet<&str> = stages.iter().map(String::as_str).collect();
    validate_contract_basics(&contract, errors);
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
        stages,
        transitions,
        gate_contract: StructuredGateContract {
            gate_kind: contract.gate_kind,
            input_kind: contract.input_kind,
            allowed_input_shapes: vec!["multiple_choice".to_string(), "text_input".to_string()],
            resume_token_required: true,
            visibility_scope: "telemetry_only_until_final_llm_output".to_string(),
        },
        terminal_states,
        telemetry_streams,
        tool_families,
        visible_chat_policy: contract.visible_chat_policy,
        run_budgets: contract.run_budgets,
    })
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
