use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedToolStatus {
    Ok,
    Error,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedToolMetrics {
    pub duration_ms: u64,
    pub output_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceVector {
    pub relevance: f64,
    pub reliability: f64,
    pub freshness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedToolResult {
    pub result_id: String,
    pub result_content_id: String,
    pub result_event_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub tool_name: String,
    pub status: NormalizedToolStatus,
    pub normalized_args: Value,
    pub dedupe_hash: String,
    pub lineage: Vec<String>,
    pub timestamp: u64,
    pub metrics: NormalizedToolMetrics,
    pub raw_ref: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceCard {
    pub evidence_id: String,
    pub evidence_content_id: String,
    pub evidence_event_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub derived_from_result_id: String,
    pub source_ref: String,
    pub source_location: String,
    pub excerpt: String,
    pub summary: String,
    pub confidence_vector: ConfidenceVector,
    pub dedupe_hash: String,
    pub lineage: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTaskStatus {
    Queued,
    Running,
    Completed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerBudgetUsed {
    pub tool_calls: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerOutput {
    pub task_id: String,
    pub status: WorkerTaskStatus,
    pub produced_evidence_ids: Vec<String>,
    pub open_questions: Vec<String>,
    pub recommended_next_actions: Vec<String>,
    pub blockers: Vec<String>,
    pub budget_used: WorkerBudgetUsed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Supported,
    Partial,
    Unsupported,
    Conflicting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claim {
    pub claim_id: String,
    pub claim_content_id: String,
    pub claim_event_id: String,
    pub text: String,
    pub evidence_ids: Vec<String>,
    pub status: ClaimStatus,
    pub confidence_vector: ConfidenceVector,
    pub conflict_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimBundle {
    pub claim_bundle_id: String,
    pub claim_bundle_content_id: String,
    pub claim_bundle_event_id: String,
    pub task_id: String,
    pub claims: Vec<Claim>,
    pub unresolved_questions: Vec<String>,
    pub conflicts: Vec<String>,
    pub coverage_score: f64,
}

pub const NORMALIZED_TOOL_RESULT_FIELDS: &[&str] = &[
    "result_id",
    "result_content_id",
    "result_event_id",
    "trace_id",
    "task_id",
    "tool_name",
    "status",
    "normalized_args",
    "dedupe_hash",
    "lineage",
    "timestamp",
    "metrics",
    "raw_ref",
    "errors",
];

pub const EVIDENCE_CARD_FIELDS: &[&str] = &[
    "evidence_id",
    "evidence_content_id",
    "evidence_event_id",
    "trace_id",
    "task_id",
    "derived_from_result_id",
    "source_ref",
    "source_location",
    "excerpt",
    "summary",
    "confidence_vector",
    "dedupe_hash",
    "lineage",
    "timestamp",
];

pub const WORKER_OUTPUT_FIELDS: &[&str] = &[
    "task_id",
    "status",
    "produced_evidence_ids",
    "open_questions",
    "recommended_next_actions",
    "blockers",
    "budget_used",
];

pub const CLAIM_FIELDS: &[&str] = &[
    "claim_id",
    "claim_content_id",
    "claim_event_id",
    "text",
    "evidence_ids",
    "status",
    "confidence_vector",
    "conflict_refs",
];

pub const CLAIM_BUNDLE_FIELDS: &[&str] = &[
    "claim_bundle_id",
    "claim_bundle_content_id",
    "claim_bundle_event_id",
    "task_id",
    "claims",
    "unresolved_questions",
    "conflicts",
    "coverage_score",
];

pub const TOOL_ATTEMPT_RECEIPT_FIELDS: &[&str] = &[
    "attempt_id",
    "attempt_sequence",
    "trace_id",
    "task_id",
    "caller",
    "tool_name",
    "status",
    "outcome",
    "reason_code",
    "reason",
    "latency_ms",
    "required_args",
    "backend",
    "discoverable",
    "timestamp",
];

pub const TOOL_CAPABILITY_PROBE_FIELDS: &[&str] = &[
    "tool_name",
    "caller",
    "available",
    "discoverable",
    "status",
    "reason_code",
    "reason",
    "required_args",
    "backend",
    "backend_class",
    "backend_status",
    "backend_reason_code",
    "backend_reason",
    "daemon_healthy",
    "ws_healthy",
    "auth_healthy",
    "resident_ipc_authoritative",
];

pub fn published_tool_alias_contract_v1() -> Vec<Value> {
    vec![
        json!({"requested_tool_name": "workspace_read", "canonical_tool_name": "file_read"}),
        json!({"requested_tool_name": "workspace_read_many", "canonical_tool_name": "file_read_many"}),
        json!({"requested_tool_name": "read_many_files", "canonical_tool_name": "file_read_many"}),
        json!({"requested_tool_name": "workspace_search", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "file_search", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "file_list", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "context_search", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "context_resolve", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "workspace_context", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "local_context", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "context_mentions", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "slash_command_route", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "provider_status", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "model_provider_status", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "platform_info", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "language_from_path", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "tab_filter", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "worktree_include", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "mcp_status", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "tool_route", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "route_tool_call", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "execute_tool", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "git_status", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "worktree_inspect", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "shell_exec", "canonical_tool_name": "terminal_exec"}),
        json!({"requested_tool_name": "mcp_list", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "mcp_servers", "canonical_tool_name": "workspace_analyze"}),
        json!({"requested_tool_name": "web_lookup", "canonical_tool_name": "web_search"}),
        json!({"requested_tool_name": "browse_web", "canonical_tool_name": "web_search"}),
    ]
}

pub fn published_schema_contract_v1() -> Value {
    json!({
        "version": "tooling_schema_v5",
        "normalized_tool_result": NORMALIZED_TOOL_RESULT_FIELDS,
        "tool_attempt_receipt": TOOL_ATTEMPT_RECEIPT_FIELDS,
        "tool_capability_probe": TOOL_CAPABILITY_PROBE_FIELDS,
        "tool_alias_contract": published_tool_alias_contract_v1(),
        "tool_cd_catalog": crate::tool_contracts::published_tool_cd_catalog_v1(),
        "evidence_card": EVIDENCE_CARD_FIELDS,
        "worker_output": WORKER_OUTPUT_FIELDS,
        "claim": CLAIM_FIELDS,
        "claim_bundle": CLAIM_BUNDLE_FIELDS
    })
}
