            composite_weight: 0.35,
            actionability_weight: 0.2,
            directive_fit_weight: 0.15,
            signal_quality_weight: 0.15,
            expected_value_weight: 0.1,
            value_density_weight: 0.08,
            risk_penalty_weight: 0.05,
            time_to_value_weight: 0.0,
            composite: 80.0,
            actionability: 70.0,
            directive_fit: 60.0,
            signal_quality: 75.0,
            expected_value: 55.0,
            value_density: 50.0,
            risk_penalty: 50.0,
            time_to_value: 40.0,
            non_yield_penalty: 1.5,
            collective_shadow_penalty: 0.5,
            collective_shadow_bonus: 0.2,
        });
        assert!((out.score - 67.45).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_score",
            "strategy_rank_score_input": {
                "composite_weight": 0.35,
                "actionability_weight": 0.2,
                "directive_fit_weight": 0.15,
                "signal_quality_weight": 0.15,
                "expected_value_weight": 0.1,
                "value_density_weight": 0.08,
                "risk_penalty_weight": 0.05,
                "time_to_value_weight": 0.0,
                "composite": 80,
                "actionability": 70,
                "directive_fit": 60,
                "signal_quality": 75,
                "expected_value": 55,
                "value_density": 50,
                "risk_penalty": 50,
                "time_to_value": 40,
                "non_yield_penalty": 1.5,
                "collective_shadow_penalty": 0.5,
                "collective_shadow_bonus": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_score");
        assert!(out.contains("\"mode\":\"strategy_rank_score\""));
        assert!(out.contains("\"score\":67.45"));
    }

    #[test]
    fn strategy_rank_adjusted_matches_pulse_and_objective_bonus_formula() {
        let out = compute_strategy_rank_adjusted(&StrategyRankAdjustedInput {
            base: 65.4,
            pulse_score: 82.0,
            pulse_weight: 0.25,
            objective_allocation_score: 70.0,
            base_objective_weight: 0.3,
            canary_mode: false,
        });
        assert!((out.adjusted - 93.25).abs() < 0.000001);
        assert!((out.bonus.total - 27.85).abs() < 0.000001);
        assert!((out.bonus.objective_weight - 0.105).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_adjusted_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_adjusted",
            "strategy_rank_adjusted_input": {
                "base": 65.4,
                "pulse_score": 82,
                "pulse_weight": 0.25,
                "objective_allocation_score": 70,
                "base_objective_weight": 0.3,
                "canary_mode": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_adjusted");
        assert!(out.contains("\"mode\":\"strategy_rank_adjusted\""));
        assert!(out.contains("\"adjusted\":93.25"));
    }

    #[test]
    fn trit_shadow_rank_score_normalizes_belief_with_confidence_bonus() {
        let out = compute_trit_shadow_rank_score(&TritShadowRankScoreInput {
            score: 0.35,
            confidence: 0.6,
        });
        assert!((out.score - 73.5).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_trit_shadow_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "trit_shadow_rank_score",
            "trit_shadow_rank_score_input": {
                "score": 0.35,
                "confidence": 0.6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale trit_shadow_rank_score");
        assert!(out.contains("\"mode\":\"trit_shadow_rank_score\""));
        assert!(out.contains("\"score\":73.5"));
    }

    #[test]
    fn strategy_circuit_cooldown_matches_error_classification() {
        let out = compute_strategy_circuit_cooldown(&StrategyCircuitCooldownInput {
            last_error_code: Some("HTTP 503".to_string()),
            last_error: None,
            http_429_cooldown_hours: 1.0,
            http_5xx_cooldown_hours: 6.0,
            dns_error_cooldown_hours: 3.0,
        });
        assert!((out.cooldown_hours - 6.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_circuit_cooldown_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_circuit_cooldown",
            "strategy_circuit_cooldown_input": {
                "last_error_code": "rate_limit_hit",
                "http_429_cooldown_hours": 2,
                "http_5xx_cooldown_hours": 8,
                "dns_error_cooldown_hours": 4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_circuit_cooldown");
        assert!(out.contains("\"mode\":\"strategy_circuit_cooldown\""));
        assert!(out.contains("\"cooldown_hours\":2.0"));
    }

    #[test]
    fn strategy_trit_shadow_adjusted_applies_bonus_blend() {
        let out = compute_strategy_trit_shadow_adjusted(&StrategyTritShadowAdjustedInput {
            base_score: 68.75,
            bonus_raw: 12.345,
            bonus_blend: 0.4,
        });
        assert!((out.bonus_applied - 4.938).abs() < 0.000001);
        assert!((out.adjusted_score - 73.688).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_trit_shadow_adjusted_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_trit_shadow_adjusted",
            "strategy_trit_shadow_adjusted_input": {
                "base_score": 68.75,
                "bonus_raw": 12.345,
                "bonus_blend": 0.4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_trit_shadow_adjusted");
        assert!(out.contains("\"mode\":\"strategy_trit_shadow_adjusted\""));
        assert!(out.contains("\"bonus_applied\":4.938"));
        assert!(out.contains("\"adjusted_score\":73.688"));
    }

    #[test]
    fn non_yield_penalty_score_applies_weighted_formula_and_clamp() {
        let out = compute_non_yield_penalty_score(&NonYieldPenaltyScoreInput {
            policy_hold_rate: 0.25,
            no_progress_rate: 0.5,
            stop_rate: 0.125,
            shipped_rate: 0.2,
            policy_hold_weight: 8.0,
            no_progress_weight: 6.0,
            stop_weight: 4.0,
            shipped_relief_weight: 3.0,
            max_penalty: 12.0,
        });
        assert!((out.penalty - 4.9).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_non_yield_penalty_score_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_penalty_score",
            "non_yield_penalty_score_input": {
                "policy_hold_rate": 0.25,
                "no_progress_rate": 0.5,
                "stop_rate": 0.125,
                "shipped_rate": 0.2,
                "policy_hold_weight": 8,
                "no_progress_weight": 6,
                "stop_weight": 4,
                "shipped_relief_weight": 3,
                "max_penalty": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_penalty_score");
        assert!(out.contains("\"mode\":\"non_yield_penalty_score\""));
        assert!(out.contains("\"penalty\":4.9"));
    }

    #[test]
    fn collective_shadow_adjustments_clamps_penalty_and_bonus() {
        let out = compute_collective_shadow_adjustments(&CollectiveShadowAdjustmentsInput {
            penalty_raw: 18.4,
            bonus_raw: std::f64::consts::E,
            max_penalty: 12.0,
            max_bonus: 6.0,
        });
        assert!((out.penalty - 12.0).abs() < 0.000001);
        let expected_bonus = (std::f64::consts::E * 1000.0).round() / 1000.0;
        assert!((out.bonus - expected_bonus).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_collective_shadow_adjustments_path_works() {
        let payload = serde_json::json!({
                "mode": "collective_shadow_adjustments",
                "collective_shadow_adjustments_input": {
                    "penalty_raw": 18.4,
                    "bonus_raw": std::f64::consts::E,
                    "max_penalty": 12,
                    "max_bonus": 6
                }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale collective_shadow_adjustments");
        assert!(out.contains("\"mode\":\"collective_shadow_adjustments\""));
        assert!(out.contains("\"penalty\":12.0"));
        assert!(out.contains("\"bonus\":2.718"));
    }

    #[test]
    fn strategy_trit_shadow_ranking_summary_orders_and_flags_divergence() {
        let out =
            compute_strategy_trit_shadow_ranking_summary(&StrategyTritShadowRankingSummaryInput {
                rows: vec![
                    StrategyTritShadowRankRowInput {
                        index: 0,
                        proposal_id: "a".to_string(),
                        legacy_rank: 92.0,
                        trit_rank: 71.0,
                        trit_label: "neutral".to_string(),
                        trit_confidence: 0.4,
                        trit_top_sources: vec!["x".to_string()],
                    },
                    StrategyTritShadowRankRowInput {
                        index: 1,
                        proposal_id: "b".to_string(),
                        legacy_rank: 80.0,
                        trit_rank: 95.0,
                        trit_label: "positive".to_string(),
                        trit_confidence: 0.8,
                        trit_top_sources: vec!["y".to_string()],
                    },
                ],
                selected_proposal_id: Some("a".to_string()),
                selection_mode: Some("qos_standard_legacy".to_string()),
                top_k: 3,
            });
        assert_eq!(out.legacy_top_proposal_id.as_deref(), Some("a"));
        assert_eq!(out.trit_top_proposal_id.as_deref(), Some("b"));
        assert!(out.diverged_from_legacy_top);
        assert!(out.diverged_from_selected);
        assert_eq!(
            out.top.first().map(|row| row.proposal_id.as_str()),
            Some("b")
        );
    }

    #[test]
    fn autoscale_json_strategy_trit_shadow_ranking_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_trit_shadow_ranking_summary",
            "strategy_trit_shadow_ranking_summary_input": {
                "rows": [
                    {
                        "index": 0,
                        "proposal_id": "a",
                        "legacy_rank": 92,
                        "trit_rank": 71,
                        "trit_label": "neutral",
                        "trit_confidence": 0.4,
                        "trit_top_sources": ["x"]
                    },
                    {
                        "index": 1,
                        "proposal_id": "b",
                        "legacy_rank": 80,
                        "trit_rank": 95,
                        "trit_label": "positive",
                        "trit_confidence": 0.8,
                        "trit_top_sources": ["y"]
                    }
                ],
                "selected_proposal_id": "a",
                "selection_mode": "qos_standard_legacy",
                "top_k": 3
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale strategy_trit_shadow_ranking_summary");
        assert!(out.contains("\"mode\":\"strategy_trit_shadow_ranking_summary\""));
        assert!(out.contains("\"trit_top_proposal_id\":\"b\""));
    }

    #[test]
    fn shadow_scope_matches_evaluates_scope_types() {
        let proposal_scope = compute_shadow_scope_matches(&ShadowScopeMatchesInput {
            scope_type: Some("proposal_type".to_string()),
            scope_value: Some("ops_remediation".to_string()),
            risk_levels: vec![],
            risk: Some("low".to_string()),
            proposal_type: Some("ops_remediation".to_string()),
            capability_key: Some("system_exec".to_string()),
            objective_id: Some("obj-1".to_string()),
        });
        assert!(proposal_scope.matched);

        let global_scope = compute_shadow_scope_matches(&ShadowScopeMatchesInput {
            scope_type: Some("global".to_string()),
            scope_value: None,
            risk_levels: vec!["high".to_string()],
            risk: Some("low".to_string()),
            proposal_type: None,
            capability_key: None,
            objective_id: None,
        });
        assert!(!global_scope.matched);
    }

    #[test]
    fn autoscale_json_shadow_scope_matches_path_works() {
        let payload = serde_json::json!({
            "mode": "shadow_scope_matches",
            "shadow_scope_matches_input": {
                "scope_type": "capability_key",
                "scope_value": "system_exec",
                "risk_levels": [],
                "risk": "medium",
                "proposal_type": "ops_remediation",
                "capability_key": "system_exec",
                "objective_id": "obj-1"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale shadow_scope_matches");
        assert!(out.contains("\"mode\":\"shadow_scope_matches\""));
        assert!(out.contains("\"matched\":true"));
    }

    #[test]
    fn collective_shadow_aggregate_computes_confidence_and_weighted_totals() {
        let out = compute_collective_shadow_aggregate(&CollectiveShadowAggregateInput {
            entries: vec![
                CollectiveShadowAggregateEntryInput {
                    kind: Some("avoid".to_string()),
                    confidence: 0.8,
                    score_impact: 10.0,
                },
                CollectiveShadowAggregateEntryInput {
                    kind: Some("reinforce".to_string()),
                    confidence: 0.5,
                    score_impact: 6.0,
                },
            ],
        });
        assert_eq!(out.matches, 2);
        assert!((out.confidence_avg - 0.65).abs() < 0.000001);
        assert!((out.penalty_raw - 8.0).abs() < 0.000001);
        assert!((out.bonus_raw - 3.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_collective_shadow_aggregate_path_works() {
        let payload = serde_json::json!({
            "mode": "collective_shadow_aggregate",
            "collective_shadow_aggregate_input": {
                "entries": [
                    { "kind": "avoid", "confidence": 0.8, "score_impact": 10 },
                    { "kind": "reinforce", "confidence": 0.5, "score_impact": 6 }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale collective_shadow_aggregate");
        assert!(out.contains("\"mode\":\"collective_shadow_aggregate\""));
        assert!(out.contains("\"penalty_raw\":8.0"));
        assert!(out.contains("\"bonus_raw\":3.0"));
    }

    #[test]
    fn expected_value_signal_applies_currency_rank_blending() {
        let out = compute_expected_value_signal(&ExpectedValueSignalInput {
            explicit_score: None,
            expected_value_usd: None,
            oracle_priority_score: Some(80.0),
            impact_weight: 2.0,
            selected_currency: Some("revenue".to_string()),
            currency_multiplier: 1.25,
            matched_first_sentence_contains_selected: true,
            currency_ranking_enabled: true,
            oracle_applies: true,
            oracle_pass: true,
            rank_blend: 0.35,
            bonus_cap: 12.0,
        });
        assert_eq!(out.source, "value_oracle_priority_score");
        assert_eq!(out.base_score, 80.0);
        assert_eq!(out.currency_adjusted_score, Some(100.0));
        assert_eq!(out.score, 89.0);
        assert_eq!(out.currency_delta, 9.0);
    }

    #[test]
    fn autoscale_json_expected_value_signal_path_works() {
        let payload = serde_json::json!({
            "mode": "expected_value_signal",
            "expected_value_signal_input": {
                "explicit_score": 42,
                "expected_value_usd": null,
                "oracle_priority_score": null,
                "impact_weight": 2.0,
                "selected_currency": "revenue",
                "currency_multiplier": 1.25,
                "matched_first_sentence_contains_selected": false,
                "currency_ranking_enabled": true,
                "oracle_applies": true,
                "oracle_pass": true,
                "rank_blend": 0.35,
                "bonus_cap": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale expected_value_signal");
        assert!(out.contains("\"mode\":\"expected_value_signal\""));
        assert!(out.contains("\"source\":\"expected_value_score\""));
        assert!(out.contains("\"score\":42.0"));
    }

    #[test]
    fn value_signal_score_matches_weighted_formula() {
