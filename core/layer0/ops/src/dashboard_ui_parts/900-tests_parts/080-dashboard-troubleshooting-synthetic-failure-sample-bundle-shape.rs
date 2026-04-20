    #[test]
    fn dashboard_troubleshooting_synthetic_failure_sample_bundle_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "I could not complete the requested web retrieval due to tool surface unavailability.",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "web_invariant": {
                    "classification": "tool_not_invoked"
                },
                "hard_guard": {
                    "applied": true
                }
            },
            "error": "web_tool_not_invoked"
        });

        let capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "try searching for the top agentic frameworks",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(
            capture.get("failure_detected").and_then(Value::as_bool),
            Some(true)
        );

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"sess-synth-1",
                "message_id":"msg-synth-1",
                "note":"synthetic failure for troubleshooting harness verification",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let lane_payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            lane_payload.get("queued").and_then(Value::as_bool),
            Some(true)
        );

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let latest_snapshot =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let latest_eval =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let outbox = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL))
            .unwrap_or_else(|| json!({}));

        assert_eq!(
            recent.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_recent_workflows")
        );
        assert_eq!(
            latest_snapshot.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_snapshot")
        );
        assert_eq!(
            latest_eval.get("type").and_then(Value::as_str),
            Some("dashboard_workflow_eval_report")
        );
        assert_eq!(
            latest_eval.pointer("/eval/model").and_then(Value::as_str),
            Some("gpt-5.4")
        );
        assert_eq!(
            outbox.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_issue_outbox")
        );
        assert_eq!(
            outbox.pointer("/items/0/issue_request/source").and_then(Value::as_str),
            Some("dashboard_report_popup")
        );

        let sample_bundle = json!({
            "sample_kind": "synthetic_troubleshooting_failure_bundle",
            "files": {
                "recent_workflows": recent,
                "latest_snapshot": latest_snapshot,
                "latest_eval_report": latest_eval,
                "issue_outbox": outbox
            }
        });
        println!("=== SYNTHETIC_TROUBLESHOOTING_SAMPLE_BEGIN ===");
        println!(
            "{}",
            serde_json::to_string_pretty(&sample_bundle).expect("sample json")
        );
        println!("=== SYNTHETIC_TROUBLESHOOTING_SAMPLE_END ===");
    }

    #[test]
    fn dashboard_troubleshooting_synthetic_hallucination_bundle_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "Given a tree, you are supposed to list all the leaves in the order of top down and left to right.",
            "tools": [
                {
                    "name": "web_search",
                    "status": "error",
                    "error": "query_result_mismatch",
                    "query": "top agentic frameworks"
                }
            ],
            "response_finalization": {
                "outcome": "classification_guard_low_signal_fail_closed",
                "tool_transaction": {
                    "classification": "low_signal",
                    "status": "degraded"
                },
                "web_invariant": {
                    "classification": "low_signal"
                },
                "hard_guard": {
                    "applied": true
                }
            },
            "error": "query_result_mismatch"
        });

        let capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "find top agentic frameworks and summarize",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(
            capture.get("failure_detected").and_then(Value::as_bool),
            Some(true)
        );

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"sess-synth-2",
                "message_id":"msg-synth-2",
                "note":"synthetic hallucination style dump for troubleshooting harness verification",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(payload.get("queued").and_then(Value::as_bool), Some(true));

        let latest_eval =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        assert_eq!(
            latest_eval.pointer("/eval/model").and_then(Value::as_str),
            Some("gpt-5.4")
        );
        let recommendations = latest_eval
            .get("recommendations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let recommendation_text = recommendations
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(recommendation_text.contains("mismatched-to-intent"));
        assert!(recommendation_text.contains("alignment scoring"));

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let snapshot =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let outbox = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL))
            .unwrap_or_else(|| json!({}));
        let sample_bundle = json!({
            "sample_kind": "synthetic_troubleshooting_hallucination_bundle",
            "files": {
                "recent_workflows": recent,
                "latest_snapshot": snapshot,
                "latest_eval_report": latest_eval,
                "issue_outbox": outbox
            }
        });
        println!("=== SYNTHETIC_HALLUCINATION_SAMPLE_BEGIN ===");
        println!(
            "{}",
            serde_json::to_string_pretty(&sample_bundle).expect("hallucination sample json")
        );
        println!("=== SYNTHETIC_HALLUCINATION_SAMPLE_END ===");
    }

    #[test]
    fn dashboard_troubleshooting_capture_dedupes_repeated_exchange_signature() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "tool path failed",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "hard_guard": {"applied": true}
            },
            "error": "web_tool_not_invoked"
        });
        let first = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
        let second = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(first.get("deduped").and_then(Value::as_bool), Some(false));
        assert_eq!(second.get("deduped").and_then(Value::as_bool), Some(true));

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let entries = recent
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].get("repeat_count").and_then(Value::as_i64),
            Some(2)
        );
        assert_eq!(
            entries[0].get("workflow_signal").and_then(Value::as_str),
            Some("error")
        );
    }

    #[test]
    fn dashboard_troubleshooting_state_exposes_recent_and_outbox_window_contracts() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "tool path failed",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "hard_guard": {"applied": true}
            },
            "error": "web_tool_not_invoked"
        });
        for idx in 0..3 {
            let _ = dashboard_troubleshooting_capture_chat_exchange(
                root.path(),
                "probe-spark8",
                &format!("repeat this failure {idx}"),
                &lane_payload,
                false,
                true,
            );
        }
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.state",
            &json!({"limit": 1}),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload
                .pointer("/recent/window/total_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            payload
                .pointer("/recent/window/visible_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/window/show_top_indicator")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/issue_outbox/window/total_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1,
            true
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_supports_classification_filters() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_degraded"
                        }
                    },
                    {
                        "stale": true,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "classification_filter": ["tool_not_found"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/filters/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/total_entry_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            payload
                .pointer("/recent/classification_histogram/0/classification")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
    }

