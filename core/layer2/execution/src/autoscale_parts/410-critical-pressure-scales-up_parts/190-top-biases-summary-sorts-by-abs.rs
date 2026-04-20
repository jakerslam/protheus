    #[test]
    fn top_biases_summary_sorts_by_abs_bias_then_total() {
        let out = compute_top_biases_summary(&TopBiasesSummaryInput {
            entries: vec![
                TopBiasSummaryEntryInput {
                    key: Some("a".to_string()),
                    bias: 2.0,
                    total: 10.0,
                    shipped: 3.0,
                    no_change: 4.0,
                    reverted: 3.0,
                },
                TopBiasSummaryEntryInput {
                    key: Some("b".to_string()),
                    bias: -5.0,
                    total: 2.0,
                    shipped: 1.0,
                    no_change: 1.0,
                    reverted: 0.0,
                },
            ],
            limit: 2,
        });
        assert_eq!(out.rows.len(), 2);
        assert_eq!(out.rows[0].key, "b");
    }

    #[test]
    fn autoscale_json_top_biases_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "top_biases_summary",
            "top_biases_summary_input": {
                "entries": [
                    {"key":"x","bias":3,"total":5,"shipped":2,"no_change":2,"reverted":1},
                    {"key":"y","bias":1,"total":8,"shipped":4,"no_change":3,"reverted":1}
                ],
                "limit": 1
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale top_biases_summary");
        assert!(out.contains("\"mode\":\"top_biases_summary\""));
    }

    #[test]
    fn criteria_pattern_penalty_accumulates_hits() {
        let out = compute_criteria_pattern_penalty(&CriteriaPatternPenaltyInput {
            keys: vec!["cap|metric".to_string()],
            patterns: vec![CriteriaPatternPenaltyPatternInput {
                key: "cap|metric".to_string(),
                failures: 4.0,
                passes: 0.0,
                last_failure_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
            }],
            fail_threshold: 2.0,
            penalty_per_hit: 3.0,
            max_penalty: 20.0,
            window_days: 365.0,
            now_ms: chrono::DateTime::parse_from_rfc3339("2026-03-04T06:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc)
                .timestamp_millis() as f64,
        });
        assert_eq!(out.penalty, 9.0);
        assert_eq!(out.hit_patterns.len(), 1);
    }

    #[test]
    fn autoscale_json_criteria_pattern_penalty_path_works() {
        let payload = serde_json::json!({
            "mode": "criteria_pattern_penalty",
            "criteria_pattern_penalty_input": {
                "keys": ["cap|metric"],
                "patterns": [{"key":"cap|metric","failures":4,"passes":1,"last_failure_ts":"2026-03-04T00:00:00.000Z"}],
                "fail_threshold": 2,
                "penalty_per_hit": 3,
                "max_penalty": 20,
                "window_days": 365,
                "now_ms": 1772600000000.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria_pattern_penalty");
        assert!(out.contains("\"mode\":\"criteria_pattern_penalty\""));
    }

    #[test]
    fn strategy_threshold_overrides_prefers_override_values() {
        let out = compute_strategy_threshold_overrides(&StrategyThresholdOverridesInput {
            min_signal_quality: Some(55.0),
            min_sensory_signal_score: Some(60.0),
            min_sensory_relevance_score: Some(62.0),
            min_directive_fit: Some(45.0),
            min_actionability_score: Some(50.0),
            min_eye_score_ema: Some(48.0),
            override_min_signal_quality: Some(70.0),
            override_min_sensory_signal_score: None,
            override_min_sensory_relevance_score: None,
            override_min_directive_fit: Some(52.0),
            override_min_actionability_score: None,
            override_min_eye_score_ema: None,
        });
        assert_eq!(out.min_signal_quality, 70.0);
        assert_eq!(out.min_directive_fit, 52.0);
        assert_eq!(out.min_sensory_signal_score, 60.0);
    }

    #[test]
    fn autoscale_json_strategy_threshold_overrides_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_threshold_overrides",
            "strategy_threshold_overrides_input": {
                "min_signal_quality": 55,
                "min_sensory_signal_score": 60,
                "min_sensory_relevance_score": 62,
                "min_directive_fit": 45,
                "min_actionability_score": 50,
                "min_eye_score_ema": 48,
                "override_min_signal_quality": 70
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_threshold_overrides");
        assert!(out.contains("\"mode\":\"strategy_threshold_overrides\""));
    }

    #[test]
    fn effective_allowed_risks_prefers_strategy_list() {
        let out = compute_effective_allowed_risks(&EffectiveAllowedRisksInput {
            default_risks: vec!["low".to_string(), "medium".to_string()],
            strategy_allowed_risks: vec!["high".to_string()],
        });
        assert_eq!(out.risks, vec!["high".to_string()]);
    }

    #[test]
    fn autoscale_json_effective_allowed_risks_path_works() {
        let payload = serde_json::json!({
            "mode": "effective_allowed_risks",
            "effective_allowed_risks_input": {
                "default_risks": ["low","medium"],
                "strategy_allowed_risks": ["high"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale effective_allowed_risks");
        assert!(out.contains("\"mode\":\"effective_allowed_risks\""));
    }

    #[test]
    fn directive_pulse_context_clamps_and_normalizes_fields() {
        let out = compute_directive_pulse_context(&DirectivePulseContextInput {
            enabled: true,
            available: true,
            objectives: vec![serde_json::json!({"id":"t1","tier":1})],
            error: Some("  ".to_string()),
            window_days: 99.0,
            urgency_hours: -1.0,
            no_progress_limit: 0.0,
            cooldown_hours: 300.0,
            tier_attempts_today: std::collections::BTreeMap::from([
                ("1".to_string(), 2.0),
                ("2".to_string(), -1.0),
            ]),
            attempts_today: 4.2,
            objective_stats: vec![DirectivePulseContextObjectiveStatInput {
                objective_id: Some(" obj_a ".to_string()),
                tier: Some(1.0),
                attempts: Some(3.0),
                shipped: Some(1.0),
                no_change: Some(1.0),
                reverted: Some(1.0),
                no_progress_streak: Some(2.0),
                last_attempt_ts: Some(" 2026-03-04T00:00:00.000Z ".to_string()),
                last_shipped_ts: Some("".to_string()),
            }],
        });
        assert_eq!(out.window_days, 60.0);
        assert_eq!(out.urgency_hours, 1.0);
        assert_eq!(out.no_progress_limit, 1.0);
        assert_eq!(out.cooldown_hours, 168.0);
        assert_eq!(out.attempts_today, 4.0);
        assert_eq!(
            out.tier_attempts_today.get("1").copied().unwrap_or(0.0),
            2.0
        );
        assert_eq!(
            out.tier_attempts_today.get("2").copied().unwrap_or(0.0),
            0.0
        );
        assert!(out.error.is_none());
        assert_eq!(out.objective_stats.len(), 1);
        assert_eq!(out.objective_stats[0].objective_id, "obj_a");
        assert_eq!(
            out.objective_stats[0].last_attempt_ts.as_deref(),
            Some("2026-03-04T00:00:00.000Z")
        );
        assert_eq!(out.objective_stats[0].last_shipped_ts, None);
    }

    #[test]
    fn autoscale_json_directive_pulse_context_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_context",
            "directive_pulse_context_input": {
                "enabled": true,
                "available": true,
                "objectives": [{"id":"t1","tier":1}],
                "window_days": 14,
                "urgency_hours": 24,
                "no_progress_limit": 3,
                "cooldown_hours": 6,
                "tier_attempts_today": {"1": 1},
                "attempts_today": 1,
                "objective_stats": [
                    {
                        "objective_id": "obj_a",
                        "tier": 1,
                        "attempts": 1,
                        "shipped": 1,
                        "no_change": 0,
                        "reverted": 0,
                        "no_progress_streak": 0
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_pulse_context");
        assert!(out.contains("\"mode\":\"directive_pulse_context\""));
    }

    #[test]
    fn directive_pulse_stats_aggregates_attempts_and_outcomes() {
        let out = compute_directive_pulse_stats(&DirectivePulseStatsInput {
            date_str: Some("2026-03-04".to_string()),
            window_days: Some(14.0),
            events: vec![
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-04".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("shipped".to_string()),
                    objective_id: Some("obj_a".to_string()),
                    tier: Some(1.0),
                    ts: Some("2026-03-04T01:00:00.000Z".to_string()),
                },
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-04".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("no_change".to_string()),
                    objective_id: Some("obj_a".to_string()),
                    tier: Some(1.0),
                    ts: Some("2026-03-04T02:00:00.000Z".to_string()),
                },
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-03".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("reverted".to_string()),
                    objective_id: Some("obj_b".to_string()),
                    tier: Some(2.0),
                    ts: Some("2026-03-03T03:00:00.000Z".to_string()),
                },
            ],
        });
        assert_eq!(out.attempts_today, 2.0);
        assert_eq!(
            out.tier_attempts_today.get("1").copied().unwrap_or(0.0),
            2.0
        );
        assert_eq!(out.objective_stats.len(), 2);
        let a = out
            .objective_stats
            .iter()
            .find(|row| row.objective_id == "obj_a")
            .expect("obj_a");
        assert_eq!(a.attempts, 2);
        assert_eq!(a.shipped, 1);
        assert_eq!(a.no_change, 1);
        assert_eq!(a.reverted, 0);
    }

    #[test]
    fn autoscale_json_directive_pulse_stats_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_stats",
            "directive_pulse_stats_input": {
                "date_str": "2026-03-04",
                "window_days": 14,
                "events": [
                    {
                        "day": "2026-03-04",
                        "event_type": "autonomy_run",
                        "result": "executed",
                        "outcome": "shipped",
                        "objective_id": "obj_a",
                        "tier": 1,
                        "ts": "2026-03-04T01:00:00.000Z"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_pulse_stats");
        assert!(out.contains("\"mode\":\"directive_pulse_stats\""));
    }

