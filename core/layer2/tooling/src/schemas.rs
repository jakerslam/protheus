use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub text: String,
    pub evidence_ids: Vec<String>,
    pub status: ClaimStatus,
    pub confidence_vector: ConfidenceVector,
    pub conflict_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimBundle {
    pub claim_bundle_id: String,
    pub task_id: String,
    pub claims: Vec<Claim>,
    pub unresolved_questions: Vec<String>,
    pub conflicts: Vec<String>,
    pub coverage_score: f64,
}
