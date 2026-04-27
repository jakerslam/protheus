#[cfg(test)]
mod receipt_validator_tests {
    use super::*;

    #[test]
    fn every_tool_attempt_gets_execution_receipt() {
        let mut broker = ToolBroker::default();
        let out = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-receipt".to_string(),
                    task_id: "task-receipt".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"receipt test"}),
                    lineage: vec![],
                    caller: BrokerCaller::Client,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"usable evidence"}]})),
            )
            .expect("execution");
        assert_eq!(
            out.execution_receipt.status,
            ToolExecutionReceiptStatus::Success
        );
        assert_eq!(out.execution_receipt.evidence_count, 1);
        assert!(out.execution_receipt.error_code.is_none());
        assert!(!out.execution_receipt.receipt_hash.is_empty());
        assert_eq!(
            broker.execution_receipts_for_trace("trace-receipt").len(),
            1
        );
    }

    #[test]
    fn unknown_tool_returns_tool_not_found_receipt() {
        let mut broker = ToolBroker::default();
        let envelope = broker.execute_and_envelope(
            ToolCallRequest {
                trace_id: "trace-missing".to_string(),
                task_id: "task-missing".to_string(),
                tool_name: "missing_tool".to_string(),
                args: json!({"query":"no-op must not pass"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"results":[{"summary":"should not run"}]})),
        );
        assert_eq!(
            envelope.execution_receipt.status,
            ToolExecutionReceiptStatus::Error
        );
        assert_eq!(
            envelope.execution_receipt.error_code.as_deref(),
            Some("tool_not_found")
        );
        assert_eq!(envelope.execution_receipt.evidence_count, 0);
        assert!(envelope.normalized_result.is_none());
    }

    #[test]
    fn anti_bot_payload_is_structured_error_before_synthesis() {
        let mut broker = ToolBroker::default();
        let out = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-captcha".to_string(),
                    task_id: "task-captcha".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"duckduckgo challenge"}),
                    lineage: vec![],
                    caller: BrokerCaller::Client,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| {
                    Ok(json!({
                        "results": [{
                            "title": "challenge",
                            "summary": "Please confirm this search was made by a human CAPTCHA"
                        }]
                    }))
                },
            )
            .expect("structured tool error is normalized");
        assert_eq!(out.normalized_result.status, NormalizedToolStatus::Error);
        assert!(out.raw_payload.is_null());
        assert_eq!(
            out.execution_receipt.status,
            ToolExecutionReceiptStatus::Error
        );
        assert_eq!(
            out.execution_receipt.error_code.as_deref(),
            Some("anti_bot_challenge")
        );
        assert_eq!(out.execution_receipt.evidence_count, 0);
    }

    #[test]
    fn substrate_report_exposes_backend_and_tool_counts() {
        let broker = ToolBroker::default();
        let report = broker.substrate_health_report();
        let total = report.available_tool_count
            + report.degraded_tool_count
            + report.blocked_tool_count
            + report.unavailable_tool_count;
        assert_eq!(total, broker.capability_catalog().len());
        assert!(report
            .backends
            .iter()
            .any(|row| row.backend == "workspace_fs"));
        assert!(!report.receipt_hash.is_empty());
    }
}
