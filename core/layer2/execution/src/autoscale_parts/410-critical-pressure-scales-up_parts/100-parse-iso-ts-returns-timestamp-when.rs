    #[test]
    fn parse_iso_ts_returns_timestamp_when_valid() {
        let valid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("2026-03-01T00:00:00.000Z".to_string()),
        });
        assert!(valid.timestamp_ms.is_some());

        let invalid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("not-a-date".to_string()),
        });
        assert!(invalid.timestamp_ms.is_none());
    }

    #[test]
    fn autoscale_json_parse_iso_ts_path_works() {
        let payload = serde_json::json!({
            "mode": "parse_iso_ts",
            "parse_iso_ts_input": {
                "ts": "2026-03-01T00:00:00.000Z"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale parse_iso_ts");
        assert!(out.contains("\"mode\":\"parse_iso_ts\""));
        assert!(out.contains("\"timestamp_ms\":"));
    }

    #[test]
    fn extract_objective_id_token_matches_expected_patterns() {
        let direct = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("T12_build_router".to_string()),
        });
        assert_eq!(direct.objective_id.as_deref(), Some("T12_build_router"));

        let embedded = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("objective: T8_fix_drift soon".to_string()),
        });
        assert_eq!(embedded.objective_id.as_deref(), Some("T8_fix_drift"));

        let none = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("no token".to_string()),
        });
        assert!(none.objective_id.is_none());
    }

    #[test]
    fn autoscale_json_extract_objective_id_token_path_works() {
        let payload = serde_json::json!({
            "mode": "extract_objective_id_token",
            "extract_objective_id_token_input": {
                "value": "objective: T8_fix_drift soon"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale extract_objective_id_token");
        assert!(out.contains("\"mode\":\"extract_objective_id_token\""));
        assert!(out.contains("\"objective_id\":\"T8_fix_drift\""));
    }

    #[test]
    fn autoscale_json_value_density_score_path_works() {
        let payload = serde_json::json!({
            "mode": "value_density_score",
            "value_density_score_input": {
                "expected_value": 40,
                "est_tokens": 1000
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale value_density_score");
        assert!(out.contains("\"mode\":\"value_density_score\""));
    }

    #[test]
    fn execution_reserve_snapshot_applies_reserve_math() {
        let out = compute_execution_reserve_snapshot(&ExecutionReserveSnapshotInput {
            cap: 1000.0,
            used: 950.0,
            reserve_enabled: true,
            reserve_ratio: 0.12,
            reserve_min_tokens: 600.0,
        });
        assert!(out.enabled);
        assert_eq!(out.reserve_tokens, 600.0);
        assert_eq!(out.reserve_remaining, 50.0);
    }

    #[test]
    fn autoscale_json_execution_reserve_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "execution_reserve_snapshot",
            "execution_reserve_snapshot_input": {
                "cap": 1000,
                "used": 950,
                "reserve_enabled": true,
                "reserve_ratio": 0.12,
                "reserve_min_tokens": 600
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execution_reserve_snapshot");
        assert!(out.contains("\"mode\":\"execution_reserve_snapshot\""));
    }

    #[test]
    fn budget_pacing_gate_blocks_high_token_low_value_when_tight() {
        let out = compute_budget_pacing_gate(&BudgetPacingGateInput {
            est_tokens: 1800.0,
            value_signal_score: 45.0,
            risk: Some("medium".to_string()),
            snapshot_tight: true,
            snapshot_autopause_active: false,
            snapshot_remaining_ratio: 0.18,
            snapshot_pressure: Some("hard".to_string()),
            execution_floor_deficit: false,
            execution_reserve_enabled: true,
            execution_reserve_remaining: 200.0,
            execution_reserve_min_value_signal: 70.0,
            budget_pacing_enabled: true,
            min_remaining_ratio: 0.2,
            high_token_threshold: 1200.0,
            min_value_signal_score: 60.0,
        });
        assert!(!out.pass);
        assert_eq!(
            out.reason.as_deref(),
            Some("budget_pacing_high_token_low_value")
        );
    }

    #[test]
    fn autoscale_json_budget_pacing_gate_path_works() {
        let payload = serde_json::json!({
            "mode": "budget_pacing_gate",
            "budget_pacing_gate_input": {
                "est_tokens": 300,
                "value_signal_score": 80,
                "risk": "low",
                "snapshot_tight": true,
                "snapshot_autopause_active": false,
                "snapshot_remaining_ratio": 0.12,
                "snapshot_pressure": "warn",
                "execution_floor_deficit": true,
                "execution_reserve_enabled": true,
                "execution_reserve_remaining": 500,
                "execution_reserve_min_value_signal": 70,
                "budget_pacing_enabled": true,
                "min_remaining_ratio": 0.2,
                "high_token_threshold": 1200,
                "min_value_signal_score": 60
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale budget_pacing_gate");
        assert!(out.contains("\"mode\":\"budget_pacing_gate\""));
        assert!(out.contains("\"pass\":true"));
    }

    #[test]
    fn capability_cap_prefers_primary_then_aliases() {
        let out = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([
                ("proposal:ops_remediation".to_string(), 4.2),
                ("proposal:feature".to_string(), 2.0),
            ]),
            primary_key: Some("proposal:ops_remediation".to_string()),
            aliases: vec!["proposal:feature".to_string()],
        });
        assert_eq!(out.cap, Some(4));

        let alias = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([("alias:key".to_string(), 3.0)]),
            primary_key: Some("missing:key".to_string()),
            aliases: vec!["alias:key".to_string()],
        });
        assert_eq!(alias.cap, Some(3));
    }

    #[test]
    fn autoscale_json_capability_cap_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_cap",
            "capability_cap_input": {
                "caps": {
                    "proposal:ops_remediation": 5
                },
                "primary_key": "proposal:ops_remediation",
                "aliases": ["proposal:feature"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_cap");
        assert!(out.contains("\"mode\":\"capability_cap\""));
        assert!(out.contains("\"cap\":5"));
    }

    #[test]
    fn estimate_tokens_for_candidate_prefers_direct_then_route_then_fallback() {
        let direct = compute_estimate_tokens_for_candidate(&EstimateTokensForCandidateInput {
            direct_est_tokens: 700.0,
            route_tokens_est: 300.0,
            fallback_estimate: 200.0,
        });
        assert_eq!(direct.est_tokens, 700);

        let route = compute_estimate_tokens_for_candidate(&EstimateTokensForCandidateInput {
            direct_est_tokens: 0.0,
            route_tokens_est: 320.0,
            fallback_estimate: 200.0,
        });
        assert_eq!(route.est_tokens, 320);
    }

    #[test]
    fn autoscale_json_estimate_tokens_for_candidate_path_works() {
        let payload = serde_json::json!({
            "mode": "estimate_tokens_for_candidate",
            "estimate_tokens_for_candidate_input": {
                "direct_est_tokens": 0,
                "route_tokens_est": 340,
                "fallback_estimate": 200
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale estimate_tokens_for_candidate");
        assert!(out.contains("\"mode\":\"estimate_tokens_for_candidate\""));
    }

    #[test]
    fn minutes_since_ts_uses_now_and_preserves_sign() {
        let out = compute_minutes_since_ts(&MinutesSinceTsInput {
            ts: Some("2026-03-03T11:00:00.000Z".to_string()),
            now_ms: Some(1_772_539_200_000.0),
        });
        let minutes = out.minutes_since.expect("minutes_since");
        assert!((minutes - 60.0).abs() < 0.000001);

        let future = compute_minutes_since_ts(&MinutesSinceTsInput {
            ts: Some("2026-03-03T13:00:00.000Z".to_string()),
            now_ms: Some(1_772_539_200_000.0),
        });
        let future_minutes = future.minutes_since.expect("future minutes");
        assert!((future_minutes + 60.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_minutes_since_ts_path_works() {
        let payload = serde_json::json!({
            "mode": "minutes_since_ts",
            "minutes_since_ts_input": {
                "ts": "2026-03-03T11:00:00.000Z",
                "now_ms": 1772539200000.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale minutes_since_ts");
        assert!(out.contains("\"mode\":\"minutes_since_ts\""));
    }

    #[test]
    fn date_window_builds_descending_iso_dates() {
        let out = compute_date_window(&DateWindowInput {
            end_date_str: Some("2026-03-03".to_string()),
            days: Some(3.0),
        });
        assert_eq!(
            out.dates,
            vec![
                "2026-03-03".to_string(),
                "2026-03-02".to_string(),
                "2026-03-01".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_date_window_path_works() {
        let payload = serde_json::json!({
            "mode": "date_window",
            "date_window_input": {
                "end_date_str": "2026-03-03",
                "days": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale date_window");
        assert!(out.contains("\"mode\":\"date_window\""));
    }

    #[test]
    fn in_window_checks_bounds_against_end_date() {
        let inside = compute_in_window(&InWindowInput {
            ts: Some("2026-03-03T12:00:00.000Z".to_string()),
            end_date_str: Some("2026-03-03".to_string()),
            days: Some(1.0),
        });
        assert!(inside.in_window);

        let outside = compute_in_window(&InWindowInput {
            ts: Some("2026-03-02T23:59:59.000Z".to_string()),
            end_date_str: Some("2026-03-03".to_string()),
            days: Some(1.0),
        });
        assert!(!outside.in_window);
    }

    #[test]
    fn autoscale_json_in_window_path_works() {
        let payload = serde_json::json!({
            "mode": "in_window",
            "in_window_input": {
                "ts": "2026-03-03T12:00:00.000Z",
                "end_date_str": "2026-03-03",
                "days": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale in_window");
        assert!(out.contains("\"mode\":\"in_window\""));
    }

    #[test]
    fn exec_window_match_checks_numeric_boundaries() {
        let inside = compute_exec_window_match(&ExecWindowMatchInput {
            ts_ms: Some(1_772_581_500_000.0),
            start_ms: Some(1_772_581_200_000.0),
            end_ms: Some(1_772_582_200_000.0),
        });
        assert!(inside.in_window);

        let outside = compute_exec_window_match(&ExecWindowMatchInput {
            ts_ms: Some(1_772_580_000_000.0),
            start_ms: Some(1_772_581_200_000.0),
            end_ms: Some(1_772_582_200_000.0),
        });
        assert!(!outside.in_window);
    }

