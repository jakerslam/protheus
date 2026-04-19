#[cfg(test)]
mod orchestration_regression_tests {
    use super::*;

    #[test]
    fn normalize_decision_defaults_to_retry_without_partials() {
        assert_eq!(normalize_decision("", false), "retry");
        assert_eq!(normalize_decision("continue", false), "continue");
    }

    #[test]
    fn finding_validation_rejects_invalid_severity() {
        let finding = json!({
            "audit_id": "a",
            "item_id": "b",
            "severity": "fatal",
            "status": "open",
            "location": "x:1",
            "evidence": [{ "type": "receipt", "value": "r" }],
            "timestamp": now_iso()
        });
        let normalized = normalize_finding(&finding);
        let (ok, reason) = validate_finding(&normalized);
        assert!(!ok);
        assert_eq!(reason, "finding_invalid_severity");
    }

    #[test]
    fn partial_retrieval_uses_task_group_partial_results_when_available() {
        let root = tempfile::tempdir().expect("tempdir");
        let root_dir = root.path().join("taskgroup-store");
        let root_dir_string = root_dir.display().to_string();
        let ensured = ensure_task_group(
            root.path(),
            &json!({
                "task_group_id": "audit-20260411000000-abc123",
                "task_type": "audit"
            }),
            Some(root_dir_string.as_str()),
        );
        assert_eq!(ensured.get("ok").and_then(Value::as_bool), Some(true));

        let tracked = track_batch_completion(
            root.path(),
            "audit-20260411000000-abc123",
            &[json!({
                "agent_id": "agent-1",
                "status": "timeout",
                "details": {
                    "processed_count": 1,
                    "partial_results_count": 1,
                    "partial_results": [{ "item_id": "REQ-1", "severity": "high" }]
                }
            })],
            Some(root_dir_string.as_str()),
        );
        assert_eq!(tracked.get("ok").and_then(Value::as_bool), Some(true));

        let out = retrieve_partial_results(
            root.path(),
            &json!({
                "task_id": "audit-task-1",
                "task_group_id": "audit-20260411000000-abc123",
                "root_dir": root_dir_string
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("source").and_then(Value::as_str), Some("task_group"));
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("continue"));
        assert_eq!(
            out.get("findings_sofar")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn latest_checkpoint_uses_most_recent_nonempty_partial_checkpoint() {
        let root = tempfile::tempdir().expect("tempdir");
        let root_dir = root.path().join("scratchpad-store");
        let root_dir_string = root_dir.display().to_string();

        let first = append_checkpoint(
            root.path(),
            "task-1",
            &json!({
                "processed_count": 1,
                "partial_results": [{ "item_id": "REQ-2", "severity": "medium" }]
            }),
            Some(root_dir_string.as_str()),
        );
        assert_eq!(first.get("ok").and_then(Value::as_bool), Some(true));

        let second = append_checkpoint(
            root.path(),
            "task-1",
            &json!({
                "processed_count": 2,
                "partial_results": []
            }),
            Some(root_dir_string.as_str()),
        );
        assert_eq!(second.get("ok").and_then(Value::as_bool), Some(true));

        let out = latest_checkpoint_from_scratchpad(
            root.path(),
            "task-1",
            Some(root_dir_string.as_str()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("source").and_then(Value::as_str), Some("checkpoint"));
        assert_eq!(out.get("items_completed").and_then(Value::as_i64), Some(1));
        assert_eq!(
            out.get("findings_sofar")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn load_scratchpad_rejects_corrupt_existing_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let root_dir = root.path().join("scratchpad-store");
        std::fs::create_dir_all(&root_dir).expect("create dir");
        std::fs::write(root_dir.join("task-1.json"), "{not-json").expect("write corrupt");

        let err = load_scratchpad(root.path(), "task-1", Some(root_dir.to_str().expect("utf8")))
            .expect_err("corrupt scratchpad should fail closed");
        assert!(err.contains("scratchpad_parse_failed"));
    }

    #[test]
    fn invoke_exposes_thin_client_helper_ops() {
        let root = tempfile::tempdir().expect("tempdir");

        let generated = invoke(
            root.path(),
            "taskgroup.generate_id",
            &json!({
                "task_type": "audit",
                "now_ms": 1711929600000i64,
                "nonce": "abc123"
            }),
        );
        assert_eq!(generated.get("ok").and_then(Value::as_bool), Some(true));
        let task_group_id = generated
            .get("task_group_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(task_group_id.starts_with("audit-"));
        assert!(task_group_id.ends_with("-abc123"));

        let counts = invoke(
            root.path(),
            "taskgroup.status_counts",
            &json!({
                "task_group": {
                    "agents": [
                        { "status": "done" },
                        { "status": "timeout" },
                        { "status": "pending" }
                    ]
                }
            }),
        );
        assert_eq!(counts.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            counts
                .pointer("/counts/done")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            1
        );
        assert_eq!(
            counts
                .pointer("/counts/timeout")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            1
        );
        assert_eq!(
            counts
                .pointer("/counts/pending")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            1
        );

        let derived = invoke(
            root.path(),
            "taskgroup.derive_status",
            &json!({
                "task_group": {
                    "agents": [
                        { "status": "done" },
                        { "status": "timeout" },
                        { "status": "pending" }
                    ]
                }
            }),
        );
        assert_eq!(derived.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(derived.get("status").and_then(Value::as_str), Some("running"));

        let summarized = invoke(
            root.path(),
            "completion.summarize",
            &json!({
                "task_group": {
                    "task_group_id": task_group_id,
                    "status": "done",
                    "coordinator_session": "session-1",
                    "agents": [
                        { "status": "done", "details": { "partial_results_count": 1 } },
                        { "status": "failed", "details": {} }
                    ]
                },
                "include_notification": true
            }),
        );
        assert_eq!(summarized.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            summarized
                .pointer("/summary/complete")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            summarized
                .pointer("/summary/partial_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_ne!(summarized.get("notification"), Some(&Value::Null));

        let normalized_scope = invoke(
            root.path(),
            "scope.normalize",
            &json!({
                "scope": {
                    "scope_id": "scope-sec",
                    "series": ["v6-sec"],
                    "paths": ["core/layer0/ops/*"]
                }
            }),
        );
        assert_eq!(normalized_scope.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            normalized_scope
                .pointer("/scope/series/0")
                .and_then(Value::as_str),
            Some("V6-SEC")
        );

        let in_scope = invoke(
            root.path(),
            "scope.finding_in_scope",
            &json!({
                "finding": {
                    "item_id": "V6-SEC-001",
                    "location": "core/layer0/ops/src/orchestration_parts/080-invoke.rs:1"
                },
                "scope": {
                    "scope_id": "scope-sec",
                    "series": ["V6-SEC"],
                    "paths": ["core/layer0/ops/*"]
                }
            }),
        );
        assert_eq!(in_scope.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(in_scope.get("in_scope").and_then(Value::as_bool), Some(true));

        let from_history = invoke(
            root.path(),
            "partial.from_session_history",
            &json!({
                "session_history": [{
                    "session_id": "session-1",
                    "items_completed": 2,
                    "partial_results": [{ "item_id": "REQ-2", "severity": "medium" }]
                }]
            }),
        );
        assert_eq!(from_history.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            from_history.get("source").and_then(Value::as_str),
            Some("session_history")
        );
        assert_eq!(
            from_history
                .get("findings_sofar")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );

        let scratchpad_root = root.path().join("scratchpad-store");
        let scratchpad_root_string = scratchpad_root.display().to_string();
        let appended = append_checkpoint(
            root.path(),
            "task-1",
            &json!({
                "processed_count": 3,
                "partial_results": [{ "item_id": "REQ-3", "severity": "high" }]
            }),
            Some(scratchpad_root_string.as_str()),
        );
        assert_eq!(appended.get("ok").and_then(Value::as_bool), Some(true));

        let from_checkpoint = invoke(
            root.path(),
            "partial.latest_checkpoint",
            &json!({
                "task_id": "task-1",
                "root_dir": scratchpad_root_string
            }),
        );
        assert_eq!(from_checkpoint.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            from_checkpoint.get("source").and_then(Value::as_str),
            Some("checkpoint")
        );
        assert_eq!(
            from_checkpoint
                .get("findings_sofar")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }
}
