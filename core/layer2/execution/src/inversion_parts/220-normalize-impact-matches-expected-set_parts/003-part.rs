                    "allowed": true,
                    "input": {"objective":"x","target":"belief","impact":"high"}
                })),
                now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
            });
        assert!(receipt.rel_path.is_some());
        let receipt_again =
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
                    "allowed": true,
                    "input": {"objective":"x","target":"belief","impact":"high"}
                })),
                now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
            });
        assert_eq!(receipt.rel_path, receipt_again.rel_path);
        let receipts_raw = fs::read_to_string(&receipts_path).expect("read persona lens receipts");
        let rows = receipts_raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        assert!(rows.len() >= 2);
        assert_eq!(rows[rows.len() - 1], rows[rows.len() - 2]);
        let parsed: Value =
            serde_json::from_str(rows[rows.len() - 1]).expect("parse persona lens receipt");
        assert_eq!(parsed.get("target").and_then(Value::as_str), Some("belief"));
        assert_eq!(
            parsed.get("type").and_then(Value::as_str),
            Some("inversion_persona_lens_gate")
        );

        let conclave = compute_append_conclave_correspondence(&AppendConclaveCorrespondenceInput {
            correspondence_path: Some(correspondence_path.to_string_lossy().to_string()),
            row: Some(json!({
                "ts": "2026-03-04T12:06:00.000Z",
                "session_or_step": "step-1",
                "pass": true,
                "winner": "vikram",
                "arbitration_rule": "safety_first",
                "high_risk_flags": ["none"],
                "query": "q",
                "proposal_summary": "s",
                "receipt_path": "r",
                "review_payload": {"ok": true}
            })),
        });
        assert!(conclave.ok);
        assert!(correspondence_path.exists());

        let persisted = compute_persist_decision(&PersistDecisionInput {
            latest_path: Some(latest_path.to_string_lossy().to_string()),
            history_path: Some(history_path.to_string_lossy().to_string()),
            payload: Some(json!({"decision":"x"})),
        });
        assert!(persisted.ok);

        let persisted_env = compute_persist_interface_envelope(&PersistInterfaceEnvelopeInput {
            latest_path: Some(interfaces_latest_path.to_string_lossy().to_string()),
            history_path: Some(interfaces_history_path.to_string_lossy().to_string()),
            envelope: Some(json!({"envelope":"x"})),
        });
        assert!(persisted_env.ok);

        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"a","ts":"2026-03-04T00:00:00.000Z","objective":"one"})),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"b","ts":"2026-03-04T00:01:00.000Z","objective":"two"})),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({"id":"c","ts":"2026-03-04T00:02:00.000Z","objective":"three"})),
        });
        let trimmed = compute_trim_library(&TrimLibraryInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            max_entries: Some(json!(2)),
        });
        assert_eq!(trimmed.rows.len(), 3);
    }

    #[test]
    fn helper_primitives_batch16_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch16");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(temp_root.join("first_principles"));
        let tier_path = temp_root.join("tier_governance.json");
        let lock_path = temp_root.join("first_principles").join("lock_state.json");

        let base_state = json!({
            "schema_id": "inversion_tier_governance_state",
            "schema_version": "1.0",
            "active_policy_version": "1.7",
            "scopes": {
                "1.7": {
                    "live_apply_attempts": {"tactical": ["2026-03-04T00:00:00.000Z"]},
                    "live_apply_successes": {"tactical": []},
                    "live_apply_safe_aborts": {"tactical": []},
                    "shadow_passes": {"tactical": []},
                    "shadow_critical_failures": {"tactical": []}
                }
            }
        });
        let policy = json!({
            "version": "1.7",
            "tier_transition": {
                "window_days_by_target": {"tactical": 45, "directive": 90},
                "minimum_window_days_by_target": {"tactical": 30, "directive": 60}
            },
            "shadow_pass_gate": {
                "window_days_by_target": {"tactical": 60, "directive": 120}
            },
            "first_principles": {
                "anti_downgrade": {
                    "enabled": true,
                    "require_same_or_higher_maturity": true,
                    "prevent_lower_confidence_same_band": true,
                    "same_band_confidence_floor_ratio": 0.92
                }
            }
        });

        let saved = compute_save_tier_governance_state(&SaveTierGovernanceStateInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            state: Some(base_state),
            policy_version: Some("1.7".to_string()),
            retention_days: Some(3650),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            value_path(Some(&saved.state), &["active_policy_version"])
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "1.7"
        );
        let loaded = compute_load_tier_governance_state(&LoadTierGovernanceStateInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy_version: Some("1.7".to_string()),
            now_iso: Some("2026-03-04T12:01:00.000Z".to_string()),
        });
        assert!(value_path(Some(&loaded.state), &["active_scope"]).is_some());

        let pushed = compute_push_tier_event(&PushTierEventInput {
            scope_map: Some(json!({"tactical": []})),
            target: Some("directive".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            pushed
                .map
                .as_object()
                .and_then(|m| m.get("directive"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );

        let added = compute_add_tier_event(&AddTierEventInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            metric: Some("live_apply_attempts".to_string()),
            target: Some("belief".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&added.state),
            &["scopes", "1.7", "live_apply_attempts", "belief"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_attempt = compute_increment_live_apply_attempt(&IncrementLiveApplyAttemptInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_attempt.state),
            &["scopes", "1.7", "live_apply_attempts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_success = compute_increment_live_apply_success(&IncrementLiveApplySuccessInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:03:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_success.state),
            &["scopes", "1.7", "live_apply_successes", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_abort =
            compute_increment_live_apply_safe_abort(&IncrementLiveApplySafeAbortInput {
                file_path: Some(tier_path.to_string_lossy().to_string()),
                policy: Some(policy.clone()),
                target: Some("identity".to_string()),
                now_iso: Some("2026-03-04T12:04:00.000Z".to_string()),
            });
        assert!(value_path(
            Some(&inc_abort.state),
            &["scopes", "1.7", "live_apply_safe_aborts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let shadow = compute_update_shadow_trial_counters(&UpdateShadowTrialCountersInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            session: Some(json!({"mode":"test","apply_requested": false,"target":"directive"})),
            result: Some("success".to_string()),
            destructive: Some(false),
            now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
        });
        assert!(shadow.state.is_some());

        let upsert = compute_upsert_first_principle_lock(&UpsertFirstPrincipleLockInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"mature"
            })),
            principle: Some(json!({"id":"fp_guard","confidence":0.91})),
            now_iso: Some("2026-03-04T12:06:00.000Z".to_string()),
        });
        assert!(value_path(Some(&upsert.state), &["locks", upsert.key.as_str()]).is_some());

        let check = compute_check_first_principle_downgrade(&CheckFirstPrincipleDowngradeInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            policy: Some(policy),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"developing"
            })),
            confidence: Some(0.5),
            now_iso: Some("2026-03-04T12:07:00.000Z".to_string()),
        });
        assert!(!check.allowed);
        assert_eq!(
            check.reason.as_deref().unwrap_or(""),
            "first_principle_downgrade_blocked_lower_maturity"
        );
    }

    #[test]
    fn helper_primitives_batch17_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch17");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(temp_root.join("events"));
        let _ = fs::create_dir_all(temp_root.join("simulation"));
        let _ = fs::create_dir_all(temp_root.join("red_team"));

        let library_path = temp_root.join("library.jsonl");
        let receipts_path = temp_root.join("receipts.jsonl");
        let active_sessions_path = temp_root.join("active_sessions.json");
        let fp_latest_path = temp_root.join("first_principles_latest.json");
        let fp_history_path = temp_root.join("first_principles_history.jsonl");
        let fp_lock_path = temp_root.join("first_principles_lock.json");

        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a1",
                "ts":"2026-03-04T00:00:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.9,
                "filter_stack":["drift_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a2",
                "ts":"2026-03-04T00:10:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.88,
                "filter_stack":["drift_guard","identity_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a3",
                "ts":"2026-03-04T00:20:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.86,
                "filter_stack":["drift_guard","fallback_pathing"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a4",
                "ts":"2026-03-04T00:30:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.84,
                "filter_stack":["drift_guard","constraint_reframe"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"ok1",
                "ts":"2026-03-04T01:00:00.000Z",
                "objective":"Ship safely",
                "signature":"safe lane pass",
                "signature_tokens":["safe","lane","pass"],
                "target":"directive",
                "impact":"high",
                "certainty":0.92,
                "filter_stack":["safe_path"],
                "outcome_trit":1,
                "result":"success",
                "maturity_band":"mature"
            })),
        });

        let detect = compute_detect_immutable_axiom_violation(
            &DetectImmutableAxiomViolationInput {
                policy: Some(json!({
                    "immutable_axioms": {
                        "enabled": true,
                        "axioms": [{
                            "id":"safety_guard",
                            "patterns":["drift guard"],
                            "regex":["drift\\s+guard"],
                            "intent_tags":["safety"],
                            "signals":{"action_terms":["drift"],"subject_terms":["guard"],"object_terms":[]},
                            "min_signal_groups": 1
                        }]
                    }
                })),
                decision_input: Some(json!({
                    "objective":"Need drift guard policy",
                    "signature":"drift guard now",
                    "filters":["constraint_reframe"],
                    "intent_tags":["safety"]
                })),
            },
        );
        assert_eq!(detect.hits, vec!["safety_guard".to_string()]);

        let maturity = compute_maturity_score(&ComputeMaturityScoreInput {
            state: Some(json!({
                "stats": {
                    "total_tests": 20,
                    "passed_tests": 15,
                    "destructive_failures": 2
                }
            })),
            policy: Some(json!({
                "maturity": {
                    "target_test_count": 40,
                    "score_weights": {"pass_rate":0.5,"non_destructive_rate":0.3,"experience":0.2},
                    "bands": {"novice":0.25,"developing":0.45,"mature":0.65,"seasoned":0.82}
                }
            })),
        });
        assert_eq!(maturity.band, "seasoned".to_string());
        assert!((maturity.score - 0.745).abs() < 0.000001);

        let candidates = compute_select_library_candidates(&SelectLibraryCandidatesInput {
            policy: Some(json!({
                "library": {
                    "min_similarity_for_reuse": 0.2,
                    "token_weight": 0.6,
                    "trit_weight": 0.3,
                    "target_weight": 0.1
                }
            })),
            query: Some(json!({
                "signature_tokens":["drift","guard","stable"],
                "trit_vector":[-1],
                "target":"directive"
            })),
            file_path: Some(library_path.to_string_lossy().to_string()),
        });
        assert!(!candidates.candidates.is_empty());

        let lane = compute_parse_lane_decision(&ParseLaneDecisionInput {
            args: Some(json!({"brain_lane":"right"})),
        });
        assert_eq!(lane.selected_lane, "right".to_string());
        assert_eq!(lane.source, "arg".to_string());

        let now = now_iso_runtime();
        let expired_at = "2000-01-01T00:00:00.000Z".to_string();
