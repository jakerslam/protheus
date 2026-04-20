        let saved_lock =
            compute_save_first_principle_lock_state(&SaveFirstPrincipleLockStateInput {
                file_path: Some(first_principles_path.to_string_lossy().to_string()),
                state: Some(json!({"locks":{"k":{"confidence":0.9}}})),
                now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
            });
        assert!(saved_lock
            .state
            .as_object()
            .and_then(|m| m.get("locks"))
            .and_then(|v| v.as_object())
            .is_some());
        let loaded_lock =
            compute_load_first_principle_lock_state(&LoadFirstPrincipleLockStateInput {
                file_path: Some(first_principles_path.to_string_lossy().to_string()),
                now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
            });
        assert!(loaded_lock
            .state
            .as_object()
            .and_then(|m| m.get("locks"))
            .and_then(|v| v.as_object())
            .is_some());

        let _ = compute_append_observer_approval(&AppendObserverApprovalInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            observer_id: Some("observer_a".to_string()),
            note: Some("first".to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        let _ = compute_append_observer_approval(&AppendObserverApprovalInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            observer_id: Some("observer_a".to_string()),
            note: Some("duplicate".to_string()),
            now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
        });
        let loaded_observers = compute_load_observer_approvals(&LoadObserverApprovalsInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
        });
        assert_eq!(loaded_observers.rows.len(), 2);
        let observer_count = compute_count_observer_approvals(&CountObserverApprovalsInput {
            file_path: Some(approvals_path.to_string_lossy().to_string()),
            target: Some("belief".to_string()),
            window_days: Some(json!(365)),
        });
        assert_eq!(observer_count.count, 1);

        let ensured = compute_ensure_correspondence_file(&EnsureCorrespondenceFileInput {
            file_path: Some(correspondence_path.to_string_lossy().to_string()),
            header: Some("# Shadow Conclave Correspondence\n\n".to_string()),
        });
        assert!(ensured.ok);
        assert!(correspondence_path.exists());
    }

    #[test]
    fn helper_primitives_batch15_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch15");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let maturity_path = temp_root.join("maturity.json");
        let sessions_path = temp_root.join("active_sessions.json");
        let events_dir = temp_root.join("events");
        let receipts_path = temp_root.join("lens_gate_receipts.jsonl");
        let correspondence_path = temp_root.join("correspondence.md");
        let latest_path = temp_root.join("latest.json");
        let history_path = temp_root.join("history.jsonl");
        let interfaces_latest_path = temp_root.join("interfaces_latest.json");
        let interfaces_history_path = temp_root.join("interfaces_history.jsonl");
        let library_path = temp_root.join("library.jsonl");

        let policy = json!({
            "maturity": {
                "target_test_count": 40,
                "score_weights": {
                    "pass_rate": 0.5,
                    "non_destructive_rate": 0.3,
                    "experience": 0.2
                },
                "bands": {
                    "novice": 0.25,
                    "developing": 0.45,
                    "mature": 0.65,
                    "seasoned": 0.82
                }
            }
        });

        let saved_maturity = compute_save_maturity_state(&SaveMaturityStateInput {
            file_path: Some(maturity_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            state: Some(json!({
                "stats": {
                    "total_tests": 20,
                    "passed_tests": 15,
                    "destructive_failures": 1
                }
            })),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert!(saved_maturity.computed.get("score").is_some());
        let loaded_maturity = compute_load_maturity_state(&LoadMaturityStateInput {
            file_path: Some(maturity_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
        });
        assert!(loaded_maturity.state.get("band").is_some());

        let saved_sessions = compute_save_active_sessions(&SaveActiveSessionsInput {
            file_path: Some(sessions_path.to_string_lossy().to_string()),
            store: Some(json!({"sessions":[{"session_id":"s1"},{"session_id":"s2"}]})),
            now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
        });
        assert_eq!(
            saved_sessions
                .store
                .as_object()
                .and_then(|m| m.get("sessions"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            2
        );
        let loaded_sessions = compute_load_active_sessions(&LoadActiveSessionsInput {
            file_path: Some(sessions_path.to_string_lossy().to_string()),
            now_iso: Some("2026-03-04T12:03:00.000Z".to_string()),
        });
        assert_eq!(
            loaded_sessions
                .store
                .as_object()
                .and_then(|m| m.get("sessions"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            2
        );

        let emitted = compute_emit_event(&EmitEventInput {
            events_dir: Some(events_dir.to_string_lossy().to_string()),
            date_str: Some("2026-03-04".to_string()),
            event_type: Some("lane_selection".to_string()),
            payload: Some(json!({"ok": true})),
            emit_events: Some(true),
            now_iso: Some("2026-03-04T12:04:00.000Z".to_string()),
        });
        assert!(emitted.emitted);

