// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRuntimeEvent {
    pub seq: usize,
    pub stage: String,
    pub event_kind: String,
    pub stream: String,
    pub payload: Value,
    pub visible_chat_eligible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolRequestEnvelope {
    pub family: String,
    pub tool_name: String,
    pub request_payload: String,
    pub request_schema: String,
    pub receipt_binding_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolFamilyDiagnostic {
    pub family: String,
    pub status: String,
    pub reason: String,
    pub selected_by_llm: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowBudgetSnapshot {
    pub max_stages: u64,
    pub stages_seen: u64,
    pub max_model_turns: u64,
    pub model_turns_seen: u64,
    pub max_tool_calls: u64,
    pub tool_calls_seen: u64,
    pub token_budget: u64,
    pub estimated_tokens_seen: u64,
    pub loop_guard_active: bool,
    pub budget_exceeded: bool,
    pub loop_signature_repeated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowInspectorArtifact {
    pub workflow_id: String,
    pub graph_hash: String,
    pub source_json_path: String,
    pub contract_schema_version: String,
    pub selected_graph_source: String,
    pub stage_statuses: Vec<Value>,
    pub trace_streams: BTreeMap<String, Vec<WorkflowRuntimeEvent>>,
    pub tool_family_diagnostics: Vec<ToolFamilyDiagnostic>,
    pub visible_chat_source: String,
    pub system_chat_injection_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowReplayReport {
    pub fixture_id: String,
    pub ok: bool,
    pub terminal_state: String,
    pub workflow_id: String,
    pub graph_hash: String,
    pub source_json_path: String,
    pub contract_schema_version: String,
    pub events: Vec<WorkflowRuntimeEvent>,
    pub tool_requests: Vec<ToolRequestEnvelope>,
    pub budget: WorkflowBudgetSnapshot,
    pub inspector: WorkflowInspectorArtifact,
    pub failures: Vec<String>,
}
