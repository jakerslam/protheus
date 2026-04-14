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

pub fn published_schema_contract_v1() -> Value {
    json!({
        "version": "tooling_schema_v4",
        "normalized_tool_result": NORMALIZED_TOOL_RESULT_FIELDS,
        "tool_attempt_receipt": TOOL_ATTEMPT_RECEIPT_FIELDS,
        "tool_capability_probe": TOOL_CAPABILITY_PROBE_FIELDS,
        "evidence_card": EVIDENCE_CARD_FIELDS,
        "worker_output": WORKER_OUTPUT_FIELDS,
        "claim": CLAIM_FIELDS,
        "claim_bundle": CLAIM_BUNDLE_FIELDS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ToolCapabilityProbe;
    use crate::tool_broker::ToolAttemptReceipt;
    use serde_json::json;

    #[test]
    fn schema_contract_publishes_frozen_field_sets() {
        let contract = published_schema_contract_v1();
        assert_eq!(
            contract.get("version").and_then(Value::as_str),
            Some("tooling_schema_v4")
        );
        assert_eq!(
            contract
                .get("normalized_tool_result")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(NORMALIZED_TOOL_RESULT_FIELDS.len())
        );
        assert_eq!(
            contract
                .get("tool_attempt_receipt")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(TOOL_ATTEMPT_RECEIPT_FIELDS.len())
        );
        assert_eq!(
            contract
                .get("evidence_card")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(EVIDENCE_CARD_FIELDS.len())
        );
    }

    #[test]
    fn evidence_card_schema_includes_trace_and_task_ids() {
        let card = EvidenceCard {
            evidence_id: "e1".to_string(),
            evidence_content_id: "e1".to_string(),
            evidence_event_id: "ev1".to_string(),
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            derived_from_result_id: "r1".to_string(),
            source_ref: "https://example.com".to_string(),
            source_location: "payload".to_string(),
            excerpt: "x".to_string(),
            summary: "y".to_string(),
            confidence_vector: ConfidenceVector {
                relevance: 0.5,
                reliability: 0.6,
                freshness: 0.7,
            },
            dedupe_hash: "d".to_string(),
            lineage: vec!["l1".to_string()],
            timestamp: 1,
        };
        let value = serde_json::to_value(card).expect("serialize");
        let keys = value
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(keys.contains(&"trace_id".to_string()));
        assert!(keys.contains(&"task_id".to_string()));
        assert_eq!(keys.len(), EVIDENCE_CARD_FIELDS.len());
    }

    #[test]
    fn tool_attempt_receipt_schema_includes_reason_and_backend() {
        let attempt = ToolAttemptReceipt {
            attempt_id: "attempt-1".to_string(),
            attempt_sequence: 1,
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            caller: crate::tool_broker::BrokerCaller::Client,
            tool_name: "web_search".to_string(),
            status: crate::tool_broker::ToolAttemptStatus::Ok,
            outcome: "ok".to_string(),
            reason_code: crate::capability::ToolReasonCode::Ok,
            reason: "ok".to_string(),
            latency_ms: 1,
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            discoverable: true,
            timestamp: 1,
        };
        let value = serde_json::to_value(attempt).expect("serialize");
        let keys = value
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(keys.contains(&"reason_code".to_string()));
        assert!(keys.contains(&"backend".to_string()));
        assert_eq!(keys.len(), TOOL_ATTEMPT_RECEIPT_FIELDS.len());
    }

    #[test]
    fn tool_capability_probe_schema_includes_status_and_required_args() {
        let probe = ToolCapabilityProbe {
            tool_name: "web_search".to_string(),
            caller: crate::tool_broker::BrokerCaller::Client,
            available: true,
            discoverable: true,
            status: crate::capability::ToolCapabilityStatus::Available,
            reason_code: crate::capability::ToolReasonCode::Ok,
            reason: "ok".to_string(),
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            backend_class: crate::backend_registry::ToolBackendClass::RetrievalPlane,
            backend_status: crate::capability::ToolCapabilityStatus::Available,
            backend_reason_code: crate::capability::ToolReasonCode::Ok,
            backend_reason: "provider_reachable".to_string(),
            daemon_healthy: Some(true),
            ws_healthy: None,
            auth_healthy: Some(true),
            resident_ipc_authoritative: true,
        };
        let value = serde_json::to_value(probe).expect("serialize");
        let keys = value
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(keys.contains(&"status".to_string()));
        assert!(keys.contains(&"required_args".to_string()));
        assert!(keys.contains(&"backend_class".to_string()));
        assert_eq!(keys.len(), TOOL_CAPABILITY_PROBE_FIELDS.len());
    }

    #[test]
    fn claim_schema_requires_evidence_refs() {
        let claim = Claim {
            claim_id: "c1".to_string(),
            claim_content_id: "c1".to_string(),
            claim_event_id: "ce1".to_string(),
            text: "Claim".to_string(),
            evidence_ids: vec!["e1".to_string()],
            status: ClaimStatus::Supported,
            confidence_vector: ConfidenceVector {
                relevance: 0.9,
                reliability: 0.9,
                freshness: 0.9,
            },
            conflict_refs: Vec::new(),
        };
        let value = serde_json::to_value(claim).expect("serialize");
        assert_eq!(
            value
                .get("evidence_ids")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(value.get("text"), Some(&json!("Claim")));
    }
}
