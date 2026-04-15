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

    #[test]
    fn autoscale_json_exec_window_match_path_works() {
        let payload = serde_json::json!({
            "mode": "exec_window_match",
            "exec_window_match_input": {
                "ts_ms": 1772581500000.0,
                "start_ms": 1772581200000.0,
                "end_ms": 1772582200000.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale exec_window_match");
        assert!(out.contains("\"mode\":\"exec_window_match\""));
    }

    #[test]
    fn start_of_next_utc_day_returns_next_day_iso() {
        let out = compute_start_of_next_utc_day(&StartOfNextUtcDayInput {
            date_str: Some("2026-03-03".to_string()),
        });
        assert_eq!(out.iso_ts, Some("2026-03-04T00:00:00.000Z".to_string()));
    }

    #[test]
    fn autoscale_json_start_of_next_utc_day_path_works() {
        let payload = serde_json::json!({
            "mode": "start_of_next_utc_day",
            "start_of_next_utc_day_input": {
                "date_str": "2026-03-03"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale start_of_next_utc_day");
        assert!(out.contains("\"mode\":\"start_of_next_utc_day\""));
    }

    #[test]
    fn iso_after_minutes_builds_iso_and_clamps_negative() {
        let out = compute_iso_after_minutes(&IsoAfterMinutesInput {
            minutes: Some(30.0),
            now_ms: Some(1_772_539_200_000.0),
        });
        assert_eq!(out.iso_ts, Some("2026-03-03T12:30:00.000Z".to_string()));

        let clamped = compute_iso_after_minutes(&IsoAfterMinutesInput {
            minutes: Some(-15.0),
            now_ms: Some(1_772_539_200_000.0),
        });
        assert_eq!(clamped.iso_ts, Some("2026-03-03T12:00:00.000Z".to_string()));
    }

    #[test]
    fn autoscale_json_iso_after_minutes_path_works() {
        let payload = serde_json::json!({
            "mode": "iso_after_minutes",
            "iso_after_minutes_input": {
                "minutes": 5,
                "now_ms": 1772539200000.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale iso_after_minutes");
        assert!(out.contains("\"mode\":\"iso_after_minutes\""));
    }

    #[test]
    fn execute_confidence_history_match_prefers_capability_then_type() {
        let cap_match =
            compute_execute_confidence_history_match(&ExecuteConfidenceHistoryMatchInput {
                event_type: Some("autonomy_run".to_string()),
                event_capability_key: Some("deploy".to_string()),
                event_proposal_type: Some("run".to_string()),
                proposal_type: Some("other".to_string()),
                capability_key: Some("deploy".to_string()),
            });
        assert!(cap_match.matched);

        let type_match =
            compute_execute_confidence_history_match(&ExecuteConfidenceHistoryMatchInput {
                event_type: Some("autonomy_run".to_string()),
                event_capability_key: Some(String::new()),
                event_proposal_type: Some("ops".to_string()),
                proposal_type: Some("ops".to_string()),
                capability_key: Some(String::new()),
            });
        assert!(type_match.matched);
    }

    #[test]
    fn autoscale_json_execute_confidence_history_match_path_works() {
        let payload = serde_json::json!({
            "mode": "execute_confidence_history_match",
            "execute_confidence_history_match_input": {
                "event_type": "autonomy_run",
                "event_capability_key": "deploy",
                "event_proposal_type": "ops",
                "proposal_type": "ops",
                "capability_key": "deploy"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execute_confidence_history_match");
        assert!(out.contains("\"mode\":\"execute_confidence_history_match\""));
    }

    #[test]
    fn execute_confidence_cooldown_key_prefers_objective_then_capability_then_type() {
        let objective =
            compute_execute_confidence_cooldown_key(&ExecuteConfidenceCooldownKeyInput {
                capability_key: Some("system_exec".to_string()),
                objective_id: Some("T1_Objective".to_string()),
                proposal_type: Some("ops_remediation".to_string()),
            });
        assert_eq!(
            objective.cooldown_key,
            "exec_confidence:objective:t1_objective"
        );

        let capability =
            compute_execute_confidence_cooldown_key(&ExecuteConfidenceCooldownKeyInput {
                capability_key: Some("System Exec".to_string()),
                objective_id: Some("T12_Objective".to_string()),
                proposal_type: Some("ops_remediation".to_string()),
            });
        assert_eq!(
            capability.cooldown_key,
            "exec_confidence:capability:system_exec"
        );

        let by_type = compute_execute_confidence_cooldown_key(&ExecuteConfidenceCooldownKeyInput {
            capability_key: Some(String::new()),
            objective_id: Some(String::new()),
            proposal_type: Some("Directive Decomposition".to_string()),
        });
        assert_eq!(
            by_type.cooldown_key,
            "exec_confidence:type:directive_decomposition"
        );
    }

    #[test]
    fn autoscale_json_execute_confidence_cooldown_key_path_works() {
        let payload = serde_json::json!({
            "mode": "execute_confidence_cooldown_key",
            "execute_confidence_cooldown_key_input": {
                "capability_key": "System Exec",
                "objective_id": "T2_objective",
                "proposal_type": "ops_remediation"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execute_confidence_cooldown_key");
        assert!(out.contains("\"mode\":\"execute_confidence_cooldown_key\""));
    }

    #[test]
    fn recent_proposal_key_counts_counts_recent_attempts() {
        let out = compute_recent_proposal_key_counts(&RecentProposalKeyCountsInput {
            cutoff_ms: Some(1000.0),
            events: vec![
                RecentProposalKeyCountEventInput {
                    proposal_key: Some("proposal:a".to_string()),
                    ts_ms: Some(1500.0),
                    result: Some("executed".to_string()),
                    is_attempt: false,
                },
                RecentProposalKeyCountEventInput {
                    proposal_key: Some("proposal:a".to_string()),
                    ts_ms: Some(1600.0),
                    result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                    is_attempt: true,
                },
                RecentProposalKeyCountEventInput {
                    proposal_key: Some("proposal:b".to_string()),
                    ts_ms: Some(900.0),
                    result: Some("executed".to_string()),
                    is_attempt: true,
                },
            ],
        });
        assert_eq!(out.counts.get("proposal:a").copied().unwrap_or(0.0), 2.0);
        assert_eq!(out.counts.get("proposal:b").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn autoscale_json_recent_proposal_key_counts_path_works() {
        let payload = serde_json::json!({
            "mode": "recent_proposal_key_counts",
            "recent_proposal_key_counts_input": {
                "cutoff_ms": 1000.0,
                "events": [
                    { "proposal_key": "proposal:a", "ts_ms": 1200.0, "result": "executed", "is_attempt": false }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale recent_proposal_key_counts");
        assert!(out.contains("\"mode\":\"recent_proposal_key_counts\""));
    }

    #[test]
    fn autoscale_json_capability_attempt_count_for_date_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_attempt_count_for_date",
            "capability_attempt_count_for_date_input": {
                "keys": ["proposal:deploy"],
                "events": [
                    { "event_type": "autonomy_run", "capability_key": "proposal:deploy", "is_attempt": true },
                    { "event_type": "autonomy_run", "capability_key": "proposal:deploy", "is_attempt": false }
                ]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale capability_attempt_count_for_date");
        assert!(out.contains("\"mode\":\"capability_attempt_count_for_date\""));
    }

    #[test]
    fn autoscale_json_capability_outcome_stats_in_window_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_outcome_stats_in_window",
            "capability_outcome_stats_in_window_input": {
                "keys": ["proposal:deploy"],
                "events": [
                    { "event_type": "autonomy_run", "result": "executed", "capability_key": "proposal:deploy", "outcome": "shipped" }
                ]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale capability_outcome_stats_in_window");
        assert!(out.contains("\"mode\":\"capability_outcome_stats_in_window\""));
    }

    #[test]
    fn autoscale_json_execute_confidence_history_path_works() {
        let payload = serde_json::json!({
            "mode": "execute_confidence_history",
            "execute_confidence_history_input": {
                "window_days": 7,
                "proposal_type": "deploy",
                "capability_key": "proposal:deploy",
                "events": [
                    { "matched": true, "result": "executed", "outcome": "no_change" }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execute_confidence_history");
        assert!(out.contains("\"mode\":\"execute_confidence_history\""));
    }

    #[test]
    fn autoscale_json_execute_confidence_policy_path_works() {
        let payload = serde_json::json!({
            "mode": "execute_confidence_policy",
            "execute_confidence_policy_input": {
                "proposal_type": "deploy",
                "capability_key": "proposal:deploy",
                "risk": "low",
                "execution_mode": "canary_execute",
                "adaptive_enabled": true,
                "base_composite_margin": 12,
                "base_value_margin": 8,
                "low_risk_relax_composite": 2,
                "low_risk_relax_value": 1,
                "fallback_relax_every": 2,
                "fallback_relax_step": 1,
                "fallback_relax_max": 3,
                "fallback_relax_min_executed": 2,
                "fallback_relax_min_shipped": 1,
                "fallback_relax_min_ship_rate": 0.5,
                "no_change_tighten_min_executed": 3,
                "no_change_tighten_threshold": 0.5,
                "no_change_tighten_step": 1,
                "history": {
                    "executed": 4,
                    "shipped": 3,
                    "reverted": 0,
                    "no_change_rate": 0.25,
                    "confidence_fallback": 2
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execute_confidence_policy");
        assert!(out.contains("\"mode\":\"execute_confidence_policy\""));
    }

    #[test]
    fn directive_fit_assessment_scores_alignment() {
        let out = compute_directive_fit_assessment(&DirectiveFitAssessmentInput {
            min_directive_fit: 45.0,
            profile_available: true,
            active_directive_ids: vec!["T1_growth".to_string()],
            positive_phrase_hits: vec!["raise revenue".to_string()],
            positive_token_hits: vec!["growth".to_string(), "sales".to_string()],
            strategy_hits: vec!["scale".to_string()],
            negative_phrase_hits: Vec::new(),
            negative_token_hits: Vec::new(),
            strategy_token_count: 3.0,
            impact: Some("high".to_string()),
        });
        assert!(out.pass);
        assert!(out.score >= 45.0);
        assert!(out.matched_positive.contains(&"growth".to_string()));
        assert!(out.reasons.iter().all(|r| r != "below_min_directive_fit"));
    }

