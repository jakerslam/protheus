    #[test]
    fn dashboard_troubleshooting_report_message_dedupes_identical_outbox_request() {
        let root = tempfile::tempdir().expect("tempdir");
        let first = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-dedupe",
                "message_id":"msg-dedupe",
                "note":"dedupe check",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(first.ok);
        let second = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-dedupe",
                "message_id":"msg-dedupe-2",
                "note":"dedupe check",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(second.ok);
        let second_payload = second.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            second_payload
                .pointer("/outbox_item/deduped")
                .and_then(Value::as_bool),
            Some(true)
        );
        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_flush_reports_retry_timing() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-retry",
                "message_id":"msg-retry",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true}),
        );
        assert!(flush.ok);
        let payload = flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("failed_count").and_then(Value::as_i64),
            Some(1)
        );
        assert!(
            payload
                .get("next_retry_after_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
        assert!(
            payload
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_flush_auth_missing_sets_auth_retry_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-auth-retry",
                "message_id":"msg-auth-retry",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true}),
        );
        assert!(flush.ok);
        let flush_payload = flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            flush_payload.get("auth_blocked_count").and_then(Value::as_i64),
            Some(1)
        );
        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/items/0/retry_lane")
                .and_then(Value::as_str),
            Some("auth_required")
        );
        assert!(
            state_payload
                .pointer("/issue_outbox/items/0/retry_after_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 3600
        );
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/error_histogram/0/error_bucket")
                .and_then(Value::as_str),
            Some("auth_missing")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_preview_is_non_destructive() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-preview",
                "message_id":"msg-preview",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let before_state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        let before_payload = before_state.payload.unwrap_or_else(|| json!({}));
        let before_depth = before_payload
            .pointer("/issue_outbox/depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let preview = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.preview",
            &json!({"max_items": 5}),
        );
        assert!(preview.ok);
        let preview_payload = preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_flush_preview")
        );
        assert_eq!(
            preview_payload.get("dry_run").and_then(Value::as_bool),
            Some(true)
        );
        let after_state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        let after_payload = after_state.payload.unwrap_or_else(|| json!({}));
        let after_depth = after_payload
            .pointer("/issue_outbox/depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert_eq!(before_depth, after_depth);
        assert!(
            preview_payload
                .pointer("/error_histogram")
                .and_then(Value::as_array)
                .is_some()
        );
    }

