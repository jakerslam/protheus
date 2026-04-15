
    #[test]
    fn helper_primitives_batch10_match_contract() {
        let out = compute_build_output_interfaces(&BuildOutputInterfacesInput {
            outputs: Some(json!({
                "default_channel": "strategy_hint",
                "belief_update": { "enabled": true, "test_enabled": true, "live_enabled": false, "require_sandbox_verification": false, "require_explicit_emit": false },
                "strategy_hint": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "workflow_hint": { "enabled": false, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "code_change_proposal": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": true, "require_explicit_emit": true }
            })),
            mode: Some("test".to_string()),
            sandbox_verified: Some(json!(false)),
            explicit_code_proposal_emit: Some(json!(false)),
            channel_payloads: Some(json!({
                "strategy_hint": { "hint": "x" }
            })),
            base_payload: Some(json!({ "base": true })),
        });

        assert_eq!(out.default_channel, "strategy_hint".to_string());
        assert_eq!(out.active_channel, Some("strategy_hint".to_string()));
        let channels = out.channels.as_object().expect("channels object");
        assert_eq!(channels.len(), 4);
        let proposal = channels
            .get("code_change_proposal")
            .and_then(|v| v.as_object())
            .expect("proposal object");
        assert!(!proposal
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true));
        assert!(proposal
            .get("gated_reasons")
            .and_then(|v| v.as_array())
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("sandbox_verification_required")))
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch11_match_contract() {
        let out = compute_build_code_change_proposal_draft(&BuildCodeChangeProposalDraftInput {
            base: Some(json!({
                "objective": "Harden inversion lane",
                "objective_id": "BL-214",
                "ts": "2026-03-04T00:00:00.000Z",
                "mode": "test",
                "impact": "high",
                "target": "directive",
                "certainty": 0.7333333,
                "maturity_band": "developing",
                "reasons": ["one", "two"],
                "shadow_mode": true
            })),
            args: Some(json!({
                "code_change_title": "Migrate proposal draft builder",
                "code_change_summary": "Rust-first proposal generation with parity fallback.",
                "code_change_files": ["client/runtime/systems/autonomy/inversion_controller.ts"],
                "code_change_tests": ["tests/client-memory-tools/inversion_helper_batch11_rust_parity.test.ts"],
                "code_change_risk": "low"
            })),
            opts: Some(json!({
                "session_id": "ivs_123",
                "sandbox_verified": true
            })),
        });
        let proposal = out.proposal.as_object().expect("proposal object");
        assert_eq!(
            proposal.get("type").and_then(|v| v.as_str()).unwrap_or(""),
            "code_change_proposal"
        );
        assert!(proposal
            .get("proposal_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("icp_"));
        assert!(proposal
            .get("sandbox_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch12_match_contract() {
        let out = compute_normalize_library_row(&NormalizeLibraryRowInput {
            row: Some(json!({
                "id": " abc ",
                "ts": "2026-03-04T00:00:00.000Z",
                "objective": " Ship lane ",
                "objective_id": " BL-215 ",
                "signature": "  reduce drift safely ",
                "target": "directive",
                "impact": "high",
                "certainty": 1.2,
                "filter_stack": ["risk_guard", "  "],
                "outcome_trit": 2,
                "result": "OK",
                "maturity_band": "Developing",
                "principle_id": " p1 ",
                "session_id": " s1 "
            })),
        });
        let row = out.row.as_object().expect("row object");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "abc");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );
        assert_eq!(
            row.get("impact").and_then(|v| v.as_str()).unwrap_or(""),
            "high"
        );
        assert_eq!(
            row.get("outcome_trit")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn helper_primitives_batch13_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch13");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let state_dir = temp_root.join("state");
        let _ = compute_ensure_dir(&EnsureDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(state_dir.exists());

        let json_path = state_dir.join("x.json");
        let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            value: Some(json!({"a": 1})),
        });
        let read_json = compute_read_json(&ReadJsonInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some(json!({})),
        });
        assert_eq!(read_json.value, json!({"a": 1}));

        let jsonl_path = state_dir.join("x.jsonl");
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
            row: Some(json!({"k": "v"})),
        });
        let read_jsonl = compute_read_jsonl(&ReadJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
        });
        assert_eq!(read_jsonl.rows.len(), 1);

        let read_text = compute_read_text(&ReadTextInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some("fallback".to_string()),
        });
        assert!(read_text.text.contains("\"a\""));

        let latest = compute_latest_json_file_in_dir(&LatestJsonFileInDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(latest.file_path.is_some());

        let out_channel = compute_normalize_output_channel(&NormalizeOutputChannelInput {
            base_out: Some(json!({"enabled": false, "test_enabled": true})),
            src_out: Some(json!({"enabled": true})),
        });
        assert!(out_channel.enabled);
        assert!(out_channel.test_enabled);

        let normalized_repo = compute_normalize_repo_path(&NormalizeRepoPathInput {
            value: Some("client/runtime/config/x.json".to_string()),
            fallback: Some("/tmp/fallback.json".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert!(normalized_repo.path.contains("/tmp/root"));

        let paths = compute_runtime_paths(&RuntimePathsInput {
            policy_path: Some("/tmp/policy.json".to_string()),
            inversion_state_dir_env: Some("/tmp/state-root".to_string()),
            dual_brain_policy_path_env: Some("/tmp/dual.json".to_string()),
            default_state_dir: Some("/tmp/default-state".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert_eq!(
            paths
                .paths
                .as_object()
                .and_then(|m| m.get("state_dir"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "/tmp/state-root"
        );
    }

    #[test]
    fn helper_primitives_batch14_match_contract() {
        let out_axioms = compute_normalize_axiom_list(&NormalizeAxiomListInput {
            raw_axioms: Some(json!([
                {
                    "id": " A1 ",
                    "patterns": [" Do no harm ", ""],
                    "regex": ["^never\\s+harm"],
                    "intent_tags": [" safety ", "guard"],
                    "signals": {
                        "action_terms": ["harm"],
                        "subject_terms": ["operator"],
                        "object_terms": ["user"]
                    },
                    "min_signal_groups": 2,
                    "semantic_requirements": {
                        "actions": ["protect"],
                        "subjects": ["human"],
                        "objects": ["safety"]
                    }
                }
            ])),
            base_axioms: Some(json!([])),
        });
        assert_eq!(out_axioms.axioms.len(), 1);
        let axiom = out_axioms.axioms[0].as_object().expect("axiom object");
        assert_eq!(axiom.get("id").and_then(|v| v.as_str()).unwrap_or(""), "a1");

        let out_suite = compute_normalize_harness_suite(&NormalizeHarnessSuiteInput {
            raw_suite: Some(json!([
                {
                    "id": " HX-1 ",
                    "objective": " validate lane ",
                    "impact": "critical",
                    "target": "directive",
                    "difficulty": "hard"
                }
            ])),
            base_suite: Some(json!([])),
        });
        assert_eq!(out_suite.suite.len(), 1);
        let row = out_suite.suite[0].as_object().expect("suite row");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "hx-1");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );

        let temp_root = std::env::temp_dir().join("inv_batch14");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let harness_path = temp_root.join("harness.json");
        let first_principles_path = temp_root.join("lock_state.json");
        let approvals_path = temp_root.join("observer_approvals.jsonl");
        let correspondence_path = temp_root.join("correspondence.md");

        let saved_harness = compute_save_harness_state(&SaveHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            state: Some(json!({"last_run_ts":"2026-03-04T00:00:00.000Z","cursor":7})),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            saved_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );
        let loaded_harness = compute_load_harness_state(&LoadHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            loaded_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );

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

        let receipt =
            compute_append_persona_lens_gate_receipt(&AppendPersonaLensGateReceiptInput {
                state_dir: Some(temp_root.to_string_lossy().to_string()),
                root: Some(temp_root.to_string_lossy().to_string()),
                cfg_receipts_path: Some(receipts_path.to_string_lossy().to_string()),
                payload: Some(json!({
                    "enabled": true,
                    "persona_id": "vikram",
                    "mode": "auto",
                    "effective_mode": "enforce",
                    "status": "enforced",
                    "fail_closed": false,
                    "drift_rate": 0.01,
                    "drift_threshold": 0.02,
                    "parity_confidence": 0.9,
                    "parity_confident": true,
                    "reasons": ["ok"]
                })),
                decision: Some(json!({
