        let out = compute_value_signal_score(&ValueSignalScoreInput {
            expected_value: 55.0,
            time_to_value: 50.0,
            actionability: 70.0,
            directive_fit: 60.0,
        });
        assert!((out.score - 57.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_value_signal_score_path_works() {
        let payload = serde_json::json!({
            "mode": "value_signal_score",
            "value_signal_score_input": {
                "expected_value": 55,
                "time_to_value": 50,
                "actionability": 70,
                "directive_fit": 60
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale value_signal_score");
        assert!(out.contains("\"mode\":\"value_signal_score\""));
        assert!(out.contains("\"score\":57.0"));
    }

    #[test]
    fn composite_eligibility_score_applies_weighted_formula() {
        let out = compute_composite_eligibility_score(&CompositeEligibilityScoreInput {
            quality_score: 80.0,
            directive_fit_score: 50.0,
            actionability_score: 90.0,
        });
        assert_eq!(out.score, 75);
    }

    #[test]
    fn autoscale_json_composite_eligibility_score_path_works() {
        let payload = serde_json::json!({
            "mode": "composite_eligibility_score",
            "composite_eligibility_score_input": {
                "quality_score": 80,
                "directive_fit_score": 50,
                "actionability_score": 90
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale composite_eligibility_score");
        assert!(out.contains("\"mode\":\"composite_eligibility_score\""));
    }

    #[test]
    fn time_to_value_score_prefers_hours_then_impact() {
        let with_hours = compute_time_to_value_score(&TimeToValueScoreInput {
            time_to_cash_hours: Some(84.0),
            expected_impact: Some("low".to_string()),
        });
        assert_eq!(with_hours.score, 50);

        let from_impact = compute_time_to_value_score(&TimeToValueScoreInput {
            time_to_cash_hours: None,
            expected_impact: Some("medium".to_string()),
        });
        assert_eq!(from_impact.score, 55);
    }

    #[test]
    fn autoscale_json_time_to_value_score_path_works() {
        let payload = serde_json::json!({
            "mode": "time_to_value_score",
            "time_to_value_score_input": {
                "time_to_cash_hours": 24,
                "expected_impact": "high"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale time_to_value_score");
        assert!(out.contains("\"mode\":\"time_to_value_score\""));
    }

    #[test]
    fn value_density_score_scales_value_by_token_cost() {
        let out = compute_value_density_score(&ValueDensityScoreInput {
            expected_value: 60.0,
            est_tokens: 500.0,
        });
        assert_eq!(out.score, 100);

        let zero = compute_value_density_score(&ValueDensityScoreInput {
            expected_value: 0.0,
            est_tokens: 500.0,
        });
        assert_eq!(zero.score, 0);
    }

    #[test]
    fn directive_tier_weight_matches_tier_policy() {
        let p1 = compute_directive_tier_weight(&DirectiveTierWeightInput {
            tier: Some(1.0),
            fallback: Some(3.0),
        });
        assert!((p1.weight - 1.3).abs() < 0.000001);

        let p2 = compute_directive_tier_weight(&DirectiveTierWeightInput {
            tier: Some(2.0),
            fallback: Some(3.0),
        });
        assert!((p2.weight - 1.0).abs() < 0.000001);

        let p3 = compute_directive_tier_weight(&DirectiveTierWeightInput {
            tier: Some(3.0),
            fallback: Some(3.0),
        });
        assert!((p3.weight - 0.82).abs() < 0.000001);

        let fallback = compute_directive_tier_weight(&DirectiveTierWeightInput {
            tier: None,
            fallback: Some(2.0),
        });
        assert!((fallback.weight - 1.0).abs() < 0.000001);
    }

    #[test]
    fn normalize_directive_tier_clamps_and_rounds() {
        let out = compute_normalize_directive_tier(&NormalizeDirectiveTierInput {
            raw_tier: Some(0.4),
            fallback: Some(3.0),
        });
        assert_eq!(out.tier, 1);

        let rounded = compute_normalize_directive_tier(&NormalizeDirectiveTierInput {
            raw_tier: Some(2.6),
            fallback: Some(3.0),
        });
        assert_eq!(rounded.tier, 3);
    }

    #[test]
    fn autoscale_json_normalize_directive_tier_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_directive_tier",
            "normalize_directive_tier_input": {
                "raw_tier": 2.4,
                "fallback": 3
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_directive_tier");
        assert!(out.contains("\"mode\":\"normalize_directive_tier\""));
        assert!(out.contains("\"tier\":2"));
    }

    #[test]
    fn autoscale_json_directive_tier_weight_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_tier_weight",
            "directive_tier_weight_input": {
                "tier": 4,
                "fallback": 3
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_tier_weight");
        assert!(out.contains("\"mode\":\"directive_tier_weight\""));
        assert!(out.contains("\"weight\":0.7"));
    }

    #[test]
    fn directive_tier_min_share_matches_tier_policy() {
        let t1 = compute_directive_tier_min_share(&DirectiveTierMinShareInput {
            tier: Some(1.0),
            fallback: Some(3.0),
            t1_min_share: 0.35,
            t2_min_share: 0.2,
        });
        assert!((t1.min_share - 0.35).abs() < 0.000001);

        let t2 = compute_directive_tier_min_share(&DirectiveTierMinShareInput {
            tier: Some(2.0),
            fallback: Some(3.0),
            t1_min_share: 0.35,
            t2_min_share: 0.2,
        });
        assert!((t2.min_share - 0.2).abs() < 0.000001);

        let t3 = compute_directive_tier_min_share(&DirectiveTierMinShareInput {
            tier: Some(3.0),
            fallback: Some(3.0),
            t1_min_share: 0.35,
            t2_min_share: 0.2,
        });
        assert!((t3.min_share - 0.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_directive_tier_min_share_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_tier_min_share",
            "directive_tier_min_share_input": {
                "tier": 2,
                "fallback": 3,
                "t1_min_share": 0.35,
                "t2_min_share": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_tier_min_share");
        assert!(out.contains("\"mode\":\"directive_tier_min_share\""));
        assert!(out.contains("\"min_share\":0.2"));
    }

    #[test]
    fn directive_tier_coverage_bonus_matches_expected_formula() {
        let no_attempts = compute_directive_tier_coverage_bonus(&DirectiveTierCoverageBonusInput {
            tier: Some(1.0),
            fallback: Some(3.0),
            attempts_today: 0.0,
            current_for_tier: 0.0,
            t1_min_share: 0.35,
            t2_min_share: 0.2,
        });
        assert!((no_attempts.bonus - 8.0).abs() < 0.000001);

        let deficit = compute_directive_tier_coverage_bonus(&DirectiveTierCoverageBonusInput {
            tier: Some(2.0),
            fallback: Some(3.0),
            attempts_today: 10.0,
            current_for_tier: 0.0,
            t1_min_share: 0.35,
            t2_min_share: 0.2,
        });
        assert!((deficit.bonus - 12.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_directive_tier_coverage_bonus_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_tier_coverage_bonus",
            "directive_tier_coverage_bonus_input": {
                "tier": 2,
                "fallback": 3,
                "attempts_today": 8,
                "current_for_tier": 0,
                "t1_min_share": 0.35,
                "t2_min_share": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_tier_coverage_bonus");
        assert!(out.contains("\"mode\":\"directive_tier_coverage_bonus\""));
        assert!(out.contains("\"bonus\":12.0"));
    }

    #[test]
    fn directive_tier_reservation_need_reports_undercoverage() {
        let out = compute_directive_tier_reservation_need(&DirectiveTierReservationNeedInput {
            enabled: true,
            available: true,
            attempts_today: 10.0,
            tier1_attempts: 2.0,
            tier2_attempts: 3.0,
            tier1_min_share: 0.35,
            tier2_min_share: 0.2,
            candidate_tiers: vec![1.0, 1.0, 2.0, 3.0],
        });
        assert!(out.reserve);
        assert_eq!(out.tier, Some(1));
        assert_eq!(out.required_after_next, Some(4.0));
        assert_eq!(out.candidate_count, Some(2));
    }

    #[test]
    fn autoscale_json_directive_tier_reservation_need_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_tier_reservation_need",
            "directive_tier_reservation_need_input": {
                "enabled": true,
                "available": true,
                "attempts_today": 8,
                "tier1_attempts": 4,
                "tier2_attempts": 0,
                "tier1_min_share": 0.35,
                "tier2_min_share": 0.2,
                "candidate_tiers": [2, 2, 3]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_tier_reservation_need");
        assert!(out.contains("\"mode\":\"directive_tier_reservation_need\""));
        assert!(out.contains("\"tier\":2"));
        assert!(out.contains("\"reserve\":true"));
    }

    #[test]
    fn pulse_objective_cooldown_active_matches_threshold_and_age() {
        let now_ms = 1_700_000_000_000.0;
        let ts = DateTime::<Utc>::from_timestamp_millis((now_ms as i64) - (2 * 60 * 60 * 1000))
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let active = compute_pulse_objective_cooldown_active(&PulseObjectiveCooldownActiveInput {
            no_progress_streak: 4.0,
            no_progress_limit: 3.0,
            last_attempt_ts: Some(ts),
            cooldown_hours: 6.0,
            now_ms: Some(now_ms),
        });
        assert!(active.active);

        let inactive =
            compute_pulse_objective_cooldown_active(&PulseObjectiveCooldownActiveInput {
                no_progress_streak: 1.0,
                no_progress_limit: 3.0,
                last_attempt_ts: Some("2026-03-01T00:00:00.000Z".to_string()),
                cooldown_hours: 6.0,
                now_ms: Some(now_ms),
            });
        assert!(!inactive.active);
    }

    #[test]
    fn autoscale_json_pulse_objective_cooldown_active_path_works() {
        let now_ms = 1_700_000_000_000.0;
        let ts = DateTime::<Utc>::from_timestamp_millis((now_ms as i64) - (60 * 60 * 1000))
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let payload = serde_json::json!({
            "mode": "pulse_objective_cooldown_active",
            "pulse_objective_cooldown_active_input": {
                "no_progress_streak": 4,
                "no_progress_limit": 3,
                "last_attempt_ts": ts,
                "cooldown_hours": 6,
                "now_ms": now_ms
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale pulse_objective_cooldown_active");
        assert!(out.contains("\"mode\":\"pulse_objective_cooldown_active\""));
        assert!(out.contains("\"active\":true"));
    }

    #[test]
    fn directive_token_hits_matches_token_and_stem_logic() {
        let out = compute_directive_token_hits(&DirectiveTokenHitsInput {
            text_tokens: vec!["memory".to_string(), "drift".to_string()],
            text_stems: vec!["memor".to_string(), "drift".to_string()],
            directive_tokens: vec![
                "memory".to_string(),
                "memorize".to_string(),
                "security".to_string(),
            ],
        });
        assert_eq!(out.hits, vec!["memory".to_string(), "memorize".to_string()]);
    }

    #[test]
    fn autoscale_json_directive_token_hits_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_token_hits",
            "directive_token_hits_input": {
                "text_tokens": ["memory", "drift"],
                "text_stems": ["memor", "drift"],
                "directive_tokens": ["memory", "memorize", "security"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_token_hits");
        assert!(out.contains("\"mode\":\"directive_token_hits\""));
        assert!(out.contains("\"hits\":[\"memory\",\"memorize\"]"));
    }

    #[test]
    fn to_stem_matches_ts_semantics() {
        let short = compute_to_stem(&ToStemInput {
            token: Some("abc".to_string()),
        });
        assert_eq!(short.stem, "abc");

        let long = compute_to_stem(&ToStemInput {
            token: Some("memory".to_string()),
        });
        assert_eq!(long.stem, "memor");
    }

    #[test]
    fn autoscale_json_to_stem_path_works() {
        let payload = serde_json::json!({
            "mode": "to_stem",
            "to_stem_input": {
                "token": "security"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale to_stem");
        assert!(out.contains("\"mode\":\"to_stem\""));
        assert!(out.contains("\"stem\":\"secur\""));
    }

    #[test]
    fn normalize_directive_text_matches_ts_semantics() {
        let out = compute_normalize_directive_text(&NormalizeDirectiveTextInput {
            text: Some(" Memory++ Drift\nPlan! ".to_string()),
        });
        assert_eq!(out.normalized, "memory drift plan");
    }

    #[test]
    fn autoscale_json_normalize_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_directive_text",
            "normalize_directive_text_input": {
                "text": " Safety-first, always. "
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_directive_text");
        assert!(out.contains("\"mode\":\"normalize_directive_text\""));
        assert!(out.contains("\"normalized\":\"safety first always\""));
    }

    #[test]
    fn tokenize_directive_text_matches_ts_filters() {
        let out = compute_tokenize_directive_text(&TokenizeDirectiveTextInput {
            text: Some("The memory plan 123 avoids drift".to_string()),
            stopwords: vec!["the".to_string(), "plan".to_string()],
        });
        assert_eq!(
            out.tokens,
            vec![
                "memory".to_string(),
                "avoids".to_string(),
                "drift".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_tokenize_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "tokenize_directive_text",
            "tokenize_directive_text_input": {
                "text": "The memory plan 123 avoids drift",
                "stopwords": ["the", "plan"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale tokenize_directive_text");
        assert!(out.contains("\"mode\":\"tokenize_directive_text\""));
        assert!(out.contains("\"tokens\":[\"memory\",\"avoids\",\"drift\"]"));
    }
