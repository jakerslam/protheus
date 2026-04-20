    #[test]
    fn dashboard_troubleshooting_deadletter_state_and_requeue_flow() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter",
                "message_id":"msg-deadletter",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);

        let first_flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        assert!(first_flush.ok);
        let first_payload = first_flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            first_payload.get("failed_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            first_payload.get("quarantined_count").and_then(Value::as_i64),
            Some(0)
        );

        let second_flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        assert!(second_flush.ok);
        let second_payload = second_flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            second_payload.get("quarantined_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            second_payload.get("deadletter_depth").and_then(Value::as_i64),
            Some(1)
        );

        let deadletter_state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        assert!(deadletter_state.ok);
        let deadletter_payload = deadletter_state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            deadletter_payload.get("depth").and_then(Value::as_i64),
            Some(1)
        );

        let requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"max_items": 1}),
        );
        assert!(requeue.ok);
        let requeue_payload = requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            requeue_payload.get("requeued_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            requeue_payload
                .get("deadletter_depth_after")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            requeue_payload.get("outbox_depth_after").and_then(Value::as_i64),
            Some(1)
        );

        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_deadletter/depth")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_requeue_supports_item_filter_and_purge() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-filter",
                "message_id":"msg-deadletter-filter",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let deadletter_state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        assert!(deadletter_state.ok);
        let deadletter_payload = deadletter_state.payload.unwrap_or_else(|| json!({}));
        let item_id = deadletter_payload
            .pointer("/items/0/row/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!item_id.is_empty());

        let no_match_requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"item_ids": ["does-not-exist"], "max_items": 5}),
        );
        assert!(no_match_requeue.ok);
        let no_match_payload = no_match_requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            no_match_payload.get("requeued_count").and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            no_match_payload
                .get("selected_filter_applied")
                .and_then(Value::as_bool),
            Some(true)
        );

        let matched_requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"item_ids": [item_id], "max_items": 5}),
        );
        assert!(matched_requeue.ok);
        let matched_payload = matched_requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            matched_payload.get("requeued_count").and_then(Value::as_i64),
            Some(1)
        );

        let purge = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge",
            &json!({"all": true}),
        );
        assert!(purge.ok);
        let purge_payload = purge.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            purge_payload.get("remaining_depth").and_then(Value::as_i64),
            Some(0)
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_purge_requires_selector() {
        let root = tempfile::tempdir().expect("tempdir");
        let purge = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge",
            &json!({}),
        );
        assert!(purge.ok);
        let payload = purge.payload.unwrap_or_else(|| json!({}));
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("deadletter_purge_selector_required")
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_preview_lanes_are_non_destructive() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-preview",
                "message_id":"msg-deadletter-preview",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let state_before = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let state_before_payload = state_before.payload.unwrap_or_else(|| json!({}));
        let before_depth = state_before_payload
            .get("depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let requeue_preview = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue.preview",
            &json!({"max_items": 5}),
        );
        assert!(requeue_preview.ok);
        let requeue_preview_payload = requeue_preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            requeue_preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_deadletter_requeue_preview")
        );
        let purge_preview = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge.preview",
            &json!({"all": true}),
        );
        assert!(purge_preview.ok);
        let purge_preview_payload = purge_preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            purge_preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_deadletter_purge_preview")
        );
        let state_after = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let state_after_payload = state_after.payload.unwrap_or_else(|| json!({}));
        let after_depth = state_after_payload
            .get("depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert_eq!(before_depth, after_depth);
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_inspect_alias_matches_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-alias",
                "message_id":"msg-deadletter-alias",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let inspect = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.inspect",
            &json!({"limit": 10}),
        );
        assert!(state.ok);
        assert!(inspect.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        let inspect_payload = inspect.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload.get("depth").and_then(Value::as_i64),
            inspect_payload.get("depth").and_then(Value::as_i64)
        );
    }

    #[test]
    fn request_query_param_extracts_since_hash() {
        let path = "/api/dashboard/snapshot?since=abc123&x=1";
        assert_eq!(request_path_only(path), "/api/dashboard/snapshot");
        assert_eq!(
            request_query_param(path, "since").as_deref(),
            Some("abc123")
        );
    }

