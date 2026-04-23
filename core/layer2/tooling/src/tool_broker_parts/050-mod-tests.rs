
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_rejects_unauthorized_tool_request() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace".to_string(),
                task_id: "task".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"echo hi"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert!(matches!(out, Err(BrokerError::UnauthorizedToolRequest(_))));
    }

    #[test]
    fn broker_argument_validation_normalizes_and_dedupes() {
        let mut broker = ToolBroker::default();
        let first = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"q":"  latency benchmarks  "}),
                    lineage: vec!["worker-1".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let second = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-2".to_string(),
                    task_id: "task-2".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_eq!(
            first
                .normalized_result
                .normalized_args
                .get("query")
                .and_then(Value::as_str),
            Some("latency benchmarks")
        );
        assert_eq!(
            first.normalized_result.dedupe_hash,
            second.normalized_result.dedupe_hash
        );
        assert_eq!(
            first.normalized_result.result_id,
            second.normalized_result.result_id
        );
    }

    #[test]
    fn broker_canonicalizes_tool_route_aliases_to_workspace_analyze() {
        let mut broker = ToolBroker::default();
        let out = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-tool-alias".to_string(),
                    task_id: "task-tool-alias".to_string(),
                    tool_name: "tool_route".to_string(),
                    args: json!({"query":"inspect route"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("tool alias");
        assert_eq!(out.normalized_result.tool_name, "workspace_analyze");
    }

    #[test]
    fn broker_uses_freshness_window_from_request_args_when_not_explicit() {
        let mut broker = ToolBroker::default();
        let out = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-freshness".to_string(),
                    task_id: "task-freshness".to_string(),
                    tool_name: "workspace_analyze".to_string(),
                    args: json!({"query":"inspect", "freshness_window_ms": 42000}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("freshness");
        assert!(out
            .normalized_result
            .lineage
            .iter()
            .any(|row| row == "freshness_window_ms:42000"));
    }

    #[test]
    fn broker_dedupe_hash_changes_when_policy_revision_changes() {
        let mut broker = ToolBroker::default();
        let first = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let second = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.v2".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_ne!(
            first.normalized_result.dedupe_hash,
            second.normalized_result.dedupe_hash
        );
        assert_ne!(
            first.normalized_result.result_id,
            second.normalized_result.result_id
        );
    }

    #[test]
    fn direct_broker_bypass_is_impossible_for_all_callers() {
        let broker = ToolBroker::default();
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::Client),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::Worker),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::System),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
    }

    #[test]
    fn broker_writes_append_only_ledger_events() {
        let mut broker = ToolBroker::default();
        let execution = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-ledger".to_string(),
                    task_id: "task-ledger".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"ledger event"}),
                    lineage: vec!["test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.ledger.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("execute");
        assert!(!broker.ledger_events().is_empty());
        let last = broker.ledger_events().last().expect("last event");
        assert_eq!(last.trace_id, "trace-ledger");
        assert_eq!(last.task_id, "task-ledger");
        assert_eq!(last.result_id, execution.normalized_result.result_id);
        assert_eq!(
            last.result_content_id,
            execution.normalized_result.result_content_id
        );
        assert_eq!(last.event_id, execution.normalized_result.result_event_id);
        assert_eq!(
            last.attempt_id.as_deref(),
            Some(execution.attempt.attempt.attempt_id.as_str())
        );
        assert_eq!(
            last.attempt_sequence,
            execution.attempt.attempt.attempt_sequence
        );
        assert!(broker.ledger_path().exists());
    }

    #[test]
    fn broker_can_recover_dedupe_state_from_ledger() {
        let ledger_path =
            std::env::temp_dir().join(format!("infring_tool_broker_recover_{}.jsonl", now_ms()));
        let mut writer = ToolBroker::default();
        writer.ledger_path = ledger_path.clone();
        let first = writer
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-recover-1".to_string(),
                    task_id: "task-recover-1".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"recoverable dedupe"}),
                    lineage: vec!["recover-test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.recover.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let first_result_id = first.normalized_result.result_id;
        let mut recovered = ToolBroker::default();
        recovered.ledger_path = ledger_path.clone();
        let recovered_count = recovered.recover_from_ledger().expect("recover");
        assert!(recovered_count >= 1);
        let second = recovered
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-recover-2".to_string(),
                    task_id: "task-recover-2".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"recoverable dedupe"}),
                    lineage: vec!["recover-test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.recover.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_eq!(second.normalized_result.result_id, first_result_id);
        let _ = std::fs::remove_file(&ledger_path);
    }

    #[test]
    fn capability_catalog_and_probe_are_deterministic() {
        let broker = ToolBroker::default();
        let catalog = broker.capability_catalog();
        assert!(catalog.iter().any(|row| row.tool_name == "web_search"));
        assert!(catalog.iter().any(|row| row.tool_name == "terminal_exec"));
        assert!(broker
            .grouped_capability_catalog()
            .iter()
            .any(|group| { group.tools.iter().any(|row| row.tool_name == "web_search") }));
        let allowed = broker.capability_probe(BrokerCaller::Client, "web_search");
        assert!(allowed.available);
        assert!(matches!(
            allowed.reason_code,
            ToolReasonCode::Ok | ToolReasonCode::BackendDegraded
        ));
        assert_eq!(allowed.backend, "retrieval_plane");
        assert_eq!(allowed.required_args, vec!["query".to_string()]);
        let unknown = broker.capability_probe(BrokerCaller::Client, "tool_that_does_not_exist");
        assert!(!unknown.available);
        assert_eq!(unknown.reason, "unknown_tool");
        assert_eq!(unknown.reason_code, ToolReasonCode::UnknownTool);
    }

    #[test]
    fn unauthorized_attempts_are_receipted() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-attempt".to_string(),
                task_id: "task-attempt".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"ls"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert!(out.is_err());
        let attempt = broker.attempt_receipts().last().expect("attempt");
        assert_eq!(attempt.outcome, "blocked");
        assert_eq!(attempt.status, ToolAttemptStatus::Blocked);
        assert_eq!(attempt.reason, "caller_not_authorized");
        assert_eq!(attempt.reason_code, ToolReasonCode::CallerNotAuthorized);
        assert_eq!(attempt.backend, "governed_terminal");
        assert_eq!(attempt.required_args, vec!["command".to_string()]);
        assert_eq!(attempt.attempt_sequence, 1);
    }

    #[test]
    fn successful_attempts_are_receipted() {
        let mut broker = ToolBroker::default();
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-ok".to_string(),
                task_id: "task-ok".to_string(),
                tool_name: "web_search".to_string(),
                args: json!({"query":"latency"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"results":[{"summary":"ok"}]})),
        );
        assert!(out.is_ok());
        let attempt = broker.attempt_receipts().last().expect("attempt");
        assert_eq!(attempt.outcome, "ok");
        assert_eq!(attempt.status, ToolAttemptStatus::Ok);
        assert_eq!(attempt.reason_code, ToolReasonCode::Ok);
        assert_eq!(attempt.attempt_sequence, 1);
    }

    #[test]
    fn execute_and_envelope_returns_structured_failure_attempt() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
        let attempt = broker.execute_and_envelope(
            ToolCallRequest {
                trace_id: "trace-envelope".to_string(),
                task_id: "task-envelope".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"ls"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert_eq!(attempt.attempt.status, ToolAttemptStatus::Blocked);
        assert_eq!(
            attempt.attempt.reason_code,
            ToolReasonCode::CallerNotAuthorized
        );
        assert!(attempt.normalized_result.is_none());
        assert!(attempt.error.is_some());
    }

    #[test]
    fn attempt_receipts_for_trace_filters_and_preserves_sequence() {
        let mut broker = ToolBroker::default();
        let _ = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-a".to_string(),
                task_id: "task-a".to_string(),
                tool_name: "web_search".to_string(),
                args: json!({"query":"a"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"results":[{"summary":"ok"}]})),
        );
        let _ = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-b".to_string(),
                task_id: "task-b".to_string(),
                tool_name: "web_search".to_string(),
                args: json!({"query":"b"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"results":[{"summary":"ok"}]})),
        );
        let trace_a = broker.attempt_receipts_for_trace("trace-a");
        assert_eq!(trace_a.len(), 1);
        assert_eq!(trace_a[0].trace_id, "trace-a");
        assert_eq!(trace_a[0].attempt_sequence, 1);
    }
}
