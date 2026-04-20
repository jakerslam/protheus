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

