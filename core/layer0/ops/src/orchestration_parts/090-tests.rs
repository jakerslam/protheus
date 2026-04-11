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
}
