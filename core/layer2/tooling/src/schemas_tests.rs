use crate::capability::ToolCapabilityProbe;
use crate::schemas::{
    published_schema_contract_v1, Claim, ClaimStatus, ConfidenceVector, EvidenceCard, CLAIM_FIELDS,
    EVIDENCE_CARD_FIELDS, NORMALIZED_TOOL_RESULT_FIELDS, TOOL_ATTEMPT_RECEIPT_FIELDS,
    TOOL_CAPABILITY_PROBE_FIELDS,
};
use crate::tool_broker::ToolAttemptReceipt;
use serde_json::{json, Value};

#[test]
fn schema_contract_publishes_frozen_field_sets() {
    let contract = published_schema_contract_v1();
    assert_eq!(
        contract.get("version").and_then(Value::as_str),
        Some("tooling_schema_v11")
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
        source_scope: "example.com".to_string(),
        source_location: "payload".to_string(),
        excerpt: "x".to_string(),
        summary: "y".to_string(),
        artifact_refs: vec![],
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
    assert!(keys.contains(&"artifact_refs".to_string()));
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
        contract_surface: Some(
            crate::capability_contract_surface::ToolCapabilityContractSurface {
                operations: vec!["search".to_string()],
                optional_args: vec![
                    "aperture".to_string(),
                    "timeout_ms".to_string(),
                    "source_scope".to_string(),
                ],
                supports_bulk: false,
                max_bulk_items: 1,
                cost_tier: "low".to_string(),
                requires_network: true,
                request_fingerprint_dedupe: true,
                per_domain_concurrency_default: 0,
                request_delay_ms_default: 0,
                blocked_response_retry_allowed: false,
                max_blocked_retries_default: 0,
                default_extraction_type: "markdown".to_string(),
                allowed_extraction_types: vec!["markdown".to_string()],
                selector_hint_allowed: true,
                selector_hint_fallback_mode: "whole_page".to_string(),
                main_content_only_default: true,
                max_chars: 12_000,
                include_artifact_refs: false,
                allowed_artifact_kinds: vec![],
                readiness_supported_fields: vec!["timeout_ms".to_string()],
                default_timeout_ms: 30_000,
                dynamic_page_allowed: false,
                disable_resources_allowed: false,
                block_ads_allowed: false,
                blocked_domains_allowed: false,
                blocked_domains_source: "none".to_string(),
                ad_block_profile_default: None,
                session_state_scope: "stateless".to_string(),
                session_reuse_allowed: false,
                session_pooling_mode: "none".to_string(),
                session_max_pages_default: 1,
                session_max_parallel_items_default: 1,
                session_request_overrides_allowed: false,
                session_close_on_complete_default: true,
                implicit_session_on_invoke: false,
                explicit_session_close_supported: false,
                explicit_session_list_supported: false,
                session_handle_arg: None,
            },
        ),
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
    assert!(keys.contains(&"contract_surface".to_string()));
    assert!(keys.contains(&"backend_class".to_string()));
    assert_eq!(keys.len(), TOOL_CAPABILITY_PROBE_FIELDS.len());
}

#[test]
fn schema_contract_publishes_tool_alias_contract_rows() {
    let contract = published_schema_contract_v1();
    let rows = contract
        .get("tool_alias_contract")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (requested, canonical) in [
        ("workspace_read", "file_read"),
        ("tool_route", "workspace_analyze"),
        ("workspace_read_many", "file_read_many"),
        ("route_tool_call", "workspace_analyze"),
        ("local_context", "workspace_analyze"),
        ("shell_exec", "terminal_exec"),
        ("mcp_status", "workspace_analyze"),
    ] {
        assert!(
            rows.iter().any(|row| {
                row.get("requested_tool_name").and_then(Value::as_str) == Some(requested)
                    && row.get("canonical_tool_name").and_then(Value::as_str) == Some(canonical)
            }),
            "schema contract must publish {requested} alias mapping"
        );
    }
}

#[test]
fn schema_contract_publishes_tool_cd_catalog() {
    let contract = published_schema_contract_v1();
    let tool_cd_catalog = contract
        .get("tool_cd_catalog")
        .and_then(Value::as_object)
        .expect("tool_cd_catalog");
    assert_eq!(
        tool_cd_catalog.get("version").and_then(Value::as_str),
        Some("tool_cd_web_retrieval_v0")
    );
    let contracts = tool_cd_catalog
        .get("contracts")
        .and_then(Value::as_array)
        .expect("contracts");
    assert!(contracts.iter().any(|row| {
        row.get("tool_id").and_then(Value::as_str) == Some("web_search")
            && row
                .get("quality_lanes")
                .and_then(Value::as_array)
                .map(|lanes| lanes.iter().any(|lane| lane.as_str() == Some("low_signal")))
                .unwrap_or(false)
    }));
}

#[test]
fn capability_schema_serializes_tool_cd_metadata_without_changing_probe_contract() {
    let capability = crate::capability::ToolCapability {
        tool_name: "web_search".to_string(),
        domain: crate::capability::ToolCapabilityDomain::Web,
        capability_family: Some("web_retrieval".to_string()),
        retrieval_mode: Some("search".to_string()),
        quality_lanes: vec!["usable".to_string(), "low_signal".to_string()],
        contract_surface: Some(
            crate::capability_contract_surface::ToolCapabilityContractSurface {
                operations: vec!["search".to_string()],
                optional_args: vec![
                    "aperture".to_string(),
                    "timeout_ms".to_string(),
                    "source_scope".to_string(),
                ],
                supports_bulk: false,
                max_bulk_items: 1,
                cost_tier: "low".to_string(),
                requires_network: true,
                request_fingerprint_dedupe: true,
                per_domain_concurrency_default: 0,
                request_delay_ms_default: 0,
                blocked_response_retry_allowed: false,
                max_blocked_retries_default: 0,
                default_extraction_type: "markdown".to_string(),
                allowed_extraction_types: vec!["markdown".to_string(), "html".to_string()],
                selector_hint_allowed: true,
                selector_hint_fallback_mode: "whole_page".to_string(),
                main_content_only_default: true,
                max_chars: 12_000,
                include_artifact_refs: false,
                allowed_artifact_kinds: vec![],
                readiness_supported_fields: vec!["timeout_ms".to_string()],
                default_timeout_ms: 30_000,
                dynamic_page_allowed: false,
                disable_resources_allowed: false,
                block_ads_allowed: false,
                blocked_domains_allowed: false,
                blocked_domains_source: "none".to_string(),
                ad_block_profile_default: None,
                session_state_scope: "stateless".to_string(),
                session_reuse_allowed: false,
                session_pooling_mode: "none".to_string(),
                session_max_pages_default: 1,
                session_max_parallel_items_default: 1,
                session_request_overrides_allowed: false,
                session_close_on_complete_default: true,
                implicit_session_on_invoke: false,
                explicit_session_close_supported: false,
                explicit_session_list_supported: false,
                session_handle_arg: None,
            },
        ),
        required_args: vec!["query".to_string()],
        allowed_callers: vec![crate::tool_broker::BrokerCaller::Client],
        backend: "retrieval_plane".to_string(),
        backend_class: crate::backend_registry::ToolBackendClass::RetrievalPlane,
        read_only: true,
        discoverable: true,
        status: crate::capability::ToolCapabilityStatus::Available,
    };
    let value = serde_json::to_value(capability).expect("serialize");
    assert_eq!(
        value.get("capability_family").and_then(Value::as_str),
        Some("web_retrieval")
    );
    assert_eq!(
        value.get("retrieval_mode").and_then(Value::as_str),
        Some("search")
    );
    assert_eq!(
        value
            .get("quality_lanes")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(2)
    );
    assert_eq!(
        value
            .get("contract_surface")
            .and_then(Value::as_object)
            .and_then(|row| row.get("default_timeout_ms"))
            .and_then(Value::as_u64),
        Some(30_000)
    );
}

#[test]
fn normalized_tool_result_schema_serializes_quality_reasons() {
    let value = serde_json::to_value(crate::schemas::NormalizedToolResult {
        result_id: "r1".to_string(),
        result_content_id: "rc1".to_string(),
        result_event_id: "re1".to_string(),
        trace_id: "trace-1".to_string(),
        task_id: "task-1".to_string(),
        tool_name: "web_fetch".to_string(),
        status: crate::schemas::NormalizedToolStatus::Ok,
        normalized_args: json!({"url":"https://example.com"}),
        dedupe_hash: "d1".to_string(),
        lineage: vec!["l1".to_string()],
        timestamp: 1,
        metrics: crate::schemas::NormalizedToolMetrics {
            duration_ms: 12,
            output_bytes: 34,
        },
        raw_ref: "raw://r1".to_string(),
        errors: vec![],
        quality_lanes: vec!["blocked".to_string()],
        quality_reasons: vec!["blocked_status".to_string(), "retryable_status".to_string()],
        safety_flags: vec!["sanitizer_applied".to_string()],
    })
    .expect("serialize");
    assert_eq!(
        value
            .get("quality_reasons")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(2)
    );
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
    assert_eq!(CLAIM_FIELDS.len(), 8);
}
