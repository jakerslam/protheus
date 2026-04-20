    #[test]
    fn criteria_pattern_keys_normalizes_and_sorts_unique() {
        let out = compute_criteria_pattern_keys(&CriteriaPatternKeysInput {
            capability_key_hint: Some("".to_string()),
            capability_descriptor_key: Some("actuation:run".to_string()),
            rows: vec![
                CriteriaPatternKeysRowInput {
                    metric: Some("latency_ms".to_string()),
                },
                CriteriaPatternKeysRowInput {
                    metric: Some("Latency Ms".to_string()),
                },
                CriteriaPatternKeysRowInput { metric: None },
            ],
        });
        assert_eq!(out.keys, vec!["actuation:run|latency_ms".to_string()]);
    }

    #[test]
    fn autoscale_json_criteria_pattern_keys_path_works() {
        let payload = serde_json::json!({
            "mode": "criteria_pattern_keys",
            "criteria_pattern_keys_input": {
                "capability_key_hint": "actuation:run",
                "rows": [{"metric":"latency_ms"}]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria_pattern_keys");
        assert!(out.contains("\"mode\":\"criteria_pattern_keys\""));
    }

    #[test]
    fn success_criteria_requirement_merges_exempt_types() {
        let out = compute_success_criteria_requirement(&SuccessCriteriaRequirementInput {
            require_success_criteria: Some(true),
            min_success_criteria_count: Some(2.0),
            policy_exempt_types: vec!["directive_clarification".to_string()],
            env_exempt_types: vec![
                "directive_clarification".to_string(),
                "remediation".to_string(),
            ],
        });
        assert!(out.required);
        assert_eq!(out.min_count, 2.0);
        assert_eq!(
            out.exempt_types,
            vec![
                "directive_clarification".to_string(),
                "remediation".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_success_criteria_requirement_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_requirement",
            "success_criteria_requirement_input": {
                "require_success_criteria": true,
                "min_success_criteria_count": 1,
                "policy_exempt_types": ["directive_clarification"],
                "env_exempt_types": ["remediation"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale success_criteria_requirement");
        assert!(out.contains("\"mode\":\"success_criteria_requirement\""));
    }

    #[test]
    fn success_criteria_policy_for_proposal_applies_exemptions() {
        let out =
            compute_success_criteria_policy_for_proposal(&SuccessCriteriaPolicyForProposalInput {
                base_required: true,
                base_min_count: 1.0,
                base_exempt_types: vec!["directive_clarification".to_string()],
                proposal_type: Some("directive_clarification".to_string()),
            });
        assert!(!out.required);
        assert!(out.exempt);
        assert_eq!(out.min_count, 1.0);
    }

    #[test]
    fn autoscale_json_success_criteria_policy_for_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_policy_for_proposal",
            "success_criteria_policy_for_proposal_input": {
                "base_required": true,
                "base_min_count": 1,
                "base_exempt_types": ["directive_clarification"],
                "proposal_type": "directive_clarification"
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale success_criteria_policy_for_proposal");
        assert!(out.contains("\"mode\":\"success_criteria_policy_for_proposal\""));
    }

    #[test]
    fn capability_descriptor_prefers_actuation_kind() {
        let out = compute_capability_descriptor(&CapabilityDescriptorInput {
            actuation_kind: Some("route_execute".to_string()),
            proposal_type: Some("optimization".to_string()),
        });
        assert_eq!(out.key, "actuation:route_execute");
        assert_eq!(out.aliases, vec!["actuation".to_string()]);
    }

    #[test]
    fn autoscale_json_capability_descriptor_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_descriptor",
            "capability_descriptor_input": {
                "actuation_kind": null,
                "proposal_type": "optimization"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_descriptor");
        assert!(out.contains("\"mode\":\"capability_descriptor\""));
    }

    #[test]
    fn normalize_token_usage_shape_uses_fallback_fields() {
        let out = compute_normalize_token_usage_shape(&NormalizeTokenUsageShapeInput {
            prompt_tokens: None,
            input_tokens: Some(11.0),
            completion_tokens: None,
            output_tokens: Some(5.0),
            total_tokens: None,
            tokens_used: None,
            source: Some("route_execute_metrics".to_string()),
        });
        assert!(out.has_value);
        let usage = out.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, Some(11.0));
        assert_eq!(usage.completion_tokens, Some(5.0));
        assert_eq!(usage.total_tokens, Some(16.0));
    }

    #[test]
    fn autoscale_json_normalize_token_usage_shape_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_token_usage_shape",
            "normalize_token_usage_shape_input": {
                "prompt_tokens": 10,
                "completion_tokens": 4,
                "source": "route_execute_metrics"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_token_usage_shape");
        assert!(out.contains("\"mode\":\"normalize_token_usage_shape\""));
    }

    #[test]
    fn default_backlog_autoscale_state_uses_input_module() {
        let out = compute_default_backlog_autoscale_state(&DefaultBacklogAutoscaleStateInput {
            module: "autonomy_spawn".to_string(),
        });
        assert_eq!(out.schema_id, "autonomy_backlog_autoscale");
        assert_eq!(out.schema_version, "1.0.0");
        assert_eq!(out.module, "autonomy_spawn");
        assert_eq!(out.current_cells, 0.0);
        assert_eq!(out.target_cells, 0.0);
        assert_eq!(out.last_run_ts, None);
    }

    #[test]
    fn autoscale_json_default_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "default_backlog_autoscale_state",
            "default_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale default_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"default_backlog_autoscale_state\""));
    }

    #[test]
    fn normalize_backlog_autoscale_state_normalizes_cells_and_strings() {
        let out = compute_normalize_backlog_autoscale_state(&NormalizeBacklogAutoscaleStateInput {
            module: "autonomy_backlog_autoscale".to_string(),
            src: Some(serde_json::json!({
                "module": " autonomy_backlog_autoscale ",
                "current_cells": 2.8,
                "target_cells": "5",
                "last_run_ts": " 2026-03-04T00:00:00.000Z ",
                "last_high_pressure_ts": "",
                "last_action": " scale_up ",
                "updated_at": null
            })),
        });
        assert_eq!(out.module, "autonomy_backlog_autoscale");
        assert_eq!(out.current_cells, 2.8);
        assert_eq!(out.target_cells, 5.0);
        assert_eq!(
            out.last_run_ts,
            Some("2026-03-04T00:00:00.000Z".to_string())
        );
        assert_eq!(out.last_high_pressure_ts, None);
        assert_eq!(out.last_action, Some("scale_up".to_string()));
        assert_eq!(out.updated_at, None);
    }

    #[test]
    fn autoscale_json_normalize_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_backlog_autoscale_state",
            "normalize_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale",
                "src": {
                    "current_cells": 1,
                    "target_cells": 3
                }
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale normalize_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"normalize_backlog_autoscale_state\""));
    }

    #[test]
    fn spawn_allocated_cells_prefers_active_then_current_then_allocated() {
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: Some(4.2),
            current_cells: Some(7.0),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(4));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: Some(7.8),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(7));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: None,
            allocated_cells: Some(2.0),
        });
        assert_eq!(out.active_cells, Some(2));
    }

    #[test]
    fn autoscale_json_spawn_allocated_cells_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_allocated_cells",
            "spawn_allocated_cells_input": {
                "active_cells": null,
                "current_cells": 3.4,
                "allocated_cells": 8
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_allocated_cells");
        assert!(out.contains("\"mode\":\"spawn_allocated_cells\""));
    }

    #[test]
    fn spawn_capacity_boost_snapshot_counts_recent_spawn_grants() {
        let out = compute_spawn_capacity_boost_snapshot(&SpawnCapacityBoostSnapshotInput {
            enabled: true,
            lookback_minutes: 30.0,
            min_granted_cells: 1.0,
            now_ms: 1_700_000_000_000.0,
            rows: vec![
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:13:20.000Z".to_string()),
                    granted_cells: Some(2.0),
                },
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:12:20.000Z".to_string()),
                    granted_cells: Some(1.0),
                },
            ],
        });
        assert!(out.active);
        assert_eq!(out.grant_count, 2);
        assert_eq!(out.granted_cells, 3.0);
        assert_eq!(out.latest_ts, Some("2023-11-14T22:12:20.000Z".to_string()));
    }

    #[test]
    fn autoscale_json_spawn_capacity_boost_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_capacity_boost_snapshot",
            "spawn_capacity_boost_snapshot_input": {
                "enabled": true,
                "lookback_minutes": 30,
                "min_granted_cells": 1,
                "now_ms": 1700000000000i64,
                "rows": [
                    {
                        "type": "spawn_request",
                        "ts": "2023-11-14T22:13:20.000Z",
                        "granted_cells": 1
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_capacity_boost_snapshot");
        assert!(out.contains("\"mode\":\"spawn_capacity_boost_snapshot\""));
    }

    #[test]
    fn inversion_maturity_score_matches_expected_banding() {
        let out = compute_inversion_maturity_score(&InversionMaturityScoreInput {
            total_tests: 40.0,
            passed_tests: 32.0,
            destructive_failures: 1.0,
            target_test_count: 40.0,
            weight_pass_rate: 0.5,
            weight_non_destructive_rate: 0.3,
            weight_experience: 0.2,
            band_novice: 0.25,
            band_developing: 0.45,
            band_mature: 0.65,
            band_seasoned: 0.82,
        });
        assert_eq!(out.band, "legendary");
        assert!(out.score >= 0.82);
        assert!(out.pass_rate >= 0.8 && out.pass_rate <= 0.81);
        assert!(out.non_destructive_rate >= 0.97 && out.non_destructive_rate <= 0.98);
    }

