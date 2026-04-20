        let now = now_iso_runtime();
        let expired_at = "2000-01-01T00:00:00.000Z".to_string();
        let live_at = "2999-01-01T00:00:00.000Z".to_string();
        let _ = compute_save_active_sessions(&SaveActiveSessionsInput {
            file_path: Some(active_sessions_path.to_string_lossy().to_string()),
            store: Some(json!({
                "sessions":[
                    {"session_id":"exp","objective":"old","signature":"old sig","target":"directive","impact":"high","certainty":0.5,"expires_at": expired_at},
                    {"session_id":"live","objective":"new","signature":"new sig","target":"directive","impact":"high","certainty":0.6,"expires_at": live_at}
                ]
            })),
            now_iso: Some(now.clone()),
        });
        let sweep = compute_sweep_expired_sessions(&SweepExpiredSessionsInput {
            paths: Some(json!({
                "active_sessions_path": active_sessions_path.to_string_lossy().to_string(),
                "receipts_path": receipts_path.to_string_lossy().to_string(),
                "library_path": library_path.to_string_lossy().to_string(),
                "events_dir": temp_root.join("events").to_string_lossy().to_string()
            })),
            policy: Some(json!({"telemetry":{"emit_events":false},"library":{"max_entries":200}})),
            date_str: Some("2026-03-04".to_string()),
            now_iso: Some(now.clone()),
        });
        assert_eq!(sweep.expired_count, 1);
        assert_eq!(sweep.sessions.len(), 1);

        let _ = fs::write(
            temp_root.join("regime.json"),
            serde_json::to_string(&json!({
                "selected_regime":"constrained",
                "candidate_confidence":0.8,
                "context":{"trit":{"trit":-1}}
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("mirror.json"),
            serde_json::to_string(
                &json!({"pressure_score":0.7,"confidence":0.75,"reasons":["pressure","drift"]}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("drift_governor.json"),
            serde_json::to_string(&json!({"last_decision":{"trit_shadow":{"belief":{"trit":-1}}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("simulation").join("2026-03-04.json"),
            serde_json::to_string(&json!({"checks_effective":{"drift_rate":{"value":0.09},"yield_rate":{"value":0.4}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("red_team").join("latest.json"),
            serde_json::to_string(
                &json!({"summary":{"critical_fail_cases":2,"pass_cases":1,"fail_cases":3}}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );

        let signals = compute_load_impossibility_signals(&LoadImpossibilitySignalsInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "paths": {
                            "regime_latest_path":"regime.json",
                            "mirror_latest_path":"mirror.json",
                            "simulation_dir":"simulation",
                            "red_team_runs_dir":"red_team",
                            "drift_governor_path":"drift_governor.json"
                        }
                    }
                }
            })),
            date_str: Some("2026-03-04".to_string()),
            root: Some(temp_root.to_string_lossy().to_string()),
        });
        assert_eq!(
            value_path(Some(&signals.signals), &["trit", "value"])
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            -1
        );

        let trigger = compute_evaluate_impossibility_trigger(&EvaluateImpossibilityTriggerInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "enabled": true,
                        "min_impossibility_score": 0.58,
                        "min_signal_count": 2,
                        "thresholds": {"predicted_drift_warn":0.03,"predicted_yield_warn":0.68},
                        "weights": {
                            "trit_pain":0.2,
                            "mirror_pressure":0.2,
                            "predicted_drift":0.18,
                            "predicted_yield_gap":0.18,
                            "red_team_critical":0.14,
                            "regime_constrained":0.1
                        }
                    }
                }
            })),
            signals: Some(signals.signals.clone()),
            force: Some(false),
        });
        assert!(trigger.triggered);
        assert!(trigger.signal_count >= 2);

        let fp_policy = json!({
            "first_principles": {
                "enabled": true,
                "auto_extract_on_success": true,
                "max_strategy_bonus": 0.12,
                "allow_failure_cluster_extraction": true,
                "failure_cluster_min": 4
            },
            "library": {
                "min_similarity_for_reuse": 0.2,
                "token_weight": 0.6,
                "trit_weight": 0.3,
                "target_weight": 0.1
            }
        });
        let session = json!({
            "session_id":"sfp",
            "objective":"Reduce drift safely",
            "objective_id":"BL-263",
            "target":"directive",
            "certainty":0.8,
            "filter_stack":["drift_guard"],
            "signature":"drift guard stable",
            "signature_tokens":["drift","guard","stable"]
        });
        let first_principle = compute_extract_first_principle(&ExtractFirstPrincipleInput {
            policy: Some(fp_policy.clone()),
            session: Some(session.clone()),
            args: Some(json!({})),
            result: Some("success".to_string()),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(first_principle.principle.is_some());

        let failure_principle =
            compute_extract_failure_cluster_principle(&ExtractFailureClusterPrincipleInput {
                paths: Some(json!({"library_path": library_path.to_string_lossy().to_string()})),
                policy: Some(fp_policy),
                session: Some(session.clone()),
                now_iso: Some(now_iso_runtime()),
            });
        assert!(failure_principle.principle.is_some());

        let persisted = compute_persist_first_principle(&PersistFirstPrincipleInput {
            paths: Some(json!({
                "first_principles_latest_path": fp_latest_path.to_string_lossy().to_string(),
                "first_principles_history_path": fp_history_path.to_string_lossy().to_string(),
                "first_principles_lock_path": fp_lock_path.to_string_lossy().to_string()
            })),
            session: Some(session),
            principle: first_principle.principle.clone(),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(persisted.principle.is_object());
        assert!(fp_latest_path.exists());
    }

