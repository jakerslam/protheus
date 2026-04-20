    #[test]
    fn autoscale_json_policy_hold_result_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_result",
            "policy_hold_result_input": {
                "result": "no_candidates_policy_daily_cap"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_result");
        assert!(out.contains("\"mode\":\"policy_hold_result\""));
    }

    #[test]
    fn policy_hold_run_event_classifies_expected_values() {
        let explicit = compute_policy_hold_run_event(&PolicyHoldRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            policy_hold: Some(true),
            result: Some("executed".to_string()),
        });
        assert!(explicit.is_policy_hold_run_event);

        let by_result = compute_policy_hold_run_event(&PolicyHoldRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            policy_hold: Some(false),
            result: Some("stop_init_gate_readiness".to_string()),
        });
        assert!(by_result.is_policy_hold_run_event);

        let non_hold = compute_policy_hold_run_event(&PolicyHoldRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            policy_hold: Some(false),
            result: Some("executed".to_string()),
        });
        assert!(!non_hold.is_policy_hold_run_event);
    }

    #[test]
    fn autoscale_json_policy_hold_run_event_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_run_event",
            "policy_hold_run_event_input": {
                "event_type": "autonomy_run",
                "policy_hold": false,
                "result": "stop_init_gate_readiness"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_run_event");
        assert!(out.contains("\"mode\":\"policy_hold_run_event\""));
    }

    #[test]
    fn dod_evidence_diff_computes_expected_deltas() {
        let out = compute_dod_evidence_diff(&DodEvidenceDiffInput {
            before_artifacts: Some(4.0),
            before_entries: Some(10.0),
            before_revenue_actions: Some(2.0),
            before_registry_total: Some(8.0),
            before_registry_active: Some(5.0),
            before_registry_candidate: Some(3.0),
            before_habit_runs: Some(12.0),
            before_habit_errors: Some(1.0),
            after_artifacts: Some(7.0),
            after_entries: Some(14.0),
            after_revenue_actions: Some(2.0),
            after_registry_total: Some(9.0),
            after_registry_active: Some(6.0),
            after_registry_candidate: Some(3.0),
            after_habit_runs: Some(15.0),
            after_habit_errors: Some(2.0),
        });
        assert_eq!(out.artifacts_delta, 3.0);
        assert_eq!(out.entries_delta, 4.0);
        assert_eq!(out.revenue_actions_delta, 0.0);
        assert_eq!(out.registry_total_delta, 1.0);
        assert_eq!(out.registry_active_delta, 1.0);
        assert_eq!(out.registry_candidate_delta, 0.0);
        assert_eq!(out.habit_runs_delta, 3.0);
        assert_eq!(out.habit_errors_delta, 1.0);
    }

    #[test]
    fn autoscale_json_dod_evidence_diff_path_works() {
        let payload = serde_json::json!({
            "mode": "dod_evidence_diff",
            "dod_evidence_diff_input": {
                "before_artifacts": 1,
                "before_entries": 2,
                "before_revenue_actions": 0,
                "before_registry_total": 3,
                "before_registry_active": 2,
                "before_registry_candidate": 1,
                "before_habit_runs": 5,
                "before_habit_errors": 1,
                "after_artifacts": 3,
                "after_entries": 3,
                "after_revenue_actions": 1,
                "after_registry_total": 4,
                "after_registry_active": 2,
                "after_registry_candidate": 2,
                "after_habit_runs": 9,
                "after_habit_errors": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale dod_evidence_diff");
        assert!(out.contains("\"mode\":\"dod_evidence_diff\""));
    }

    #[test]
    fn score_only_result_classifies_expected_values() {
        let score_only = compute_score_only_result(&ScoreOnlyResultInput {
            result: Some("score_only_preview".to_string()),
        });
        assert!(score_only.is_score_only);

        let non_score_only = compute_score_only_result(&ScoreOnlyResultInput {
            result: Some("executed".to_string()),
        });
        assert!(!non_score_only.is_score_only);
    }

    #[test]
    fn autoscale_json_score_only_result_path_works() {
        let payload = serde_json::json!({
            "mode": "score_only_result",
            "score_only_result_input": {
                "result": "score_only_evidence"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale score_only_result");
        assert!(out.contains("\"mode\":\"score_only_result\""));
    }

    #[test]
    fn score_only_failure_like_classifies_expected_values() {
        let structural = compute_score_only_failure_like(&ScoreOnlyFailureLikeInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_preview_structural_cooldown".to_string()),
            preview_verification_present: Some(false),
            preview_verification_passed: None,
            preview_verification_outcome: None,
        });
        assert!(structural.is_failure_like);

        let no_change = compute_score_only_failure_like(&ScoreOnlyFailureLikeInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("score_only_preview".to_string()),
            preview_verification_present: Some(true),
            preview_verification_passed: Some(true),
            preview_verification_outcome: Some("no_change".to_string()),
        });
        assert!(no_change.is_failure_like);

        let clean = compute_score_only_failure_like(&ScoreOnlyFailureLikeInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("score_only_preview".to_string()),
            preview_verification_present: Some(true),
            preview_verification_passed: Some(true),
            preview_verification_outcome: Some("shipped".to_string()),
        });
        assert!(!clean.is_failure_like);
    }

    #[test]
    fn autoscale_json_score_only_failure_like_path_works() {
        let payload = serde_json::json!({
            "mode": "score_only_failure_like",
            "score_only_failure_like_input": {
                "event_type": "autonomy_run",
                "result": "score_only_preview",
                "preview_verification_present": true,
                "preview_verification_passed": true,
                "preview_verification_outcome": "no_change"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale score_only_failure_like");
        assert!(out.contains("\"mode\":\"score_only_failure_like\""));
    }

    #[test]
    fn gate_exhausted_attempt_classifies_expected_values() {
        let exhausted = compute_gate_exhausted_attempt(&GateExhaustedAttemptInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_stale_signal".to_string()),
        });
        assert!(exhausted.is_gate_exhausted);

        let non_exhausted = compute_gate_exhausted_attempt(&GateExhaustedAttemptInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
        });
        assert!(!non_exhausted.is_gate_exhausted);
    }

    #[test]
    fn autoscale_json_gate_exhausted_attempt_path_works() {
        let payload = serde_json::json!({
            "mode": "gate_exhausted_attempt",
            "gate_exhausted_attempt_input": {
                "event_type": "autonomy_run",
                "result": "stop_repeat_gate_candidate_exhausted"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale gate_exhausted_attempt");
        assert!(out.contains("\"mode\":\"gate_exhausted_attempt\""));
    }

    #[test]
    fn consecutive_gate_exhausted_attempts_counts_tail_streak() {
        let out =
            compute_consecutive_gate_exhausted_attempts(&ConsecutiveGateExhaustedAttemptsInput {
                events: vec![
                    ConsecutiveGateExhaustedAttemptEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("executed".to_string()),
                    },
                    ConsecutiveGateExhaustedAttemptEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_stale_signal".to_string()),
                    },
                    ConsecutiveGateExhaustedAttemptEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                    },
                    ConsecutiveGateExhaustedAttemptEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("lock_busy".to_string()),
                    },
                ],
            });
        assert_eq!(out.count, 2);
    }

    #[test]
    fn autoscale_json_consecutive_gate_exhausted_attempts_path_works() {
        let payload = serde_json::json!({
            "mode": "consecutive_gate_exhausted_attempts",
            "consecutive_gate_exhausted_attempts_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "stop_repeat_gate_stale_signal"},
                    {"event_type": "autonomy_run", "result": "stop_repeat_gate_candidate_exhausted"}
                ]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale consecutive_gate_exhausted_attempts");
        assert!(out.contains("\"mode\":\"consecutive_gate_exhausted_attempts\""));
    }

    #[test]
    fn runs_since_reset_index_prefers_last_reset_marker() {
        let out = compute_runs_since_reset_index(&RunsSinceResetIndexInput {
            events: vec![
                RunsSinceResetEventInput {
                    event_type: Some("autonomy_run".to_string()),
                },
                RunsSinceResetEventInput {
                    event_type: Some("autonomy_reset".to_string()),
                },
                RunsSinceResetEventInput {
                    event_type: Some("autonomy_run".to_string()),
                },
                RunsSinceResetEventInput {
                    event_type: Some("autonomy_reset".to_string()),
                },
                RunsSinceResetEventInput {
                    event_type: Some("autonomy_run".to_string()),
                },
            ],
        });
        assert_eq!(out.start_index, 4);
    }

    #[test]
    fn autoscale_json_runs_since_reset_index_path_works() {
        let payload = serde_json::json!({
            "mode": "runs_since_reset_index",
            "runs_since_reset_index_input": {
                "events": [
                    {"event_type": "autonomy_run"},
                    {"event_type": "autonomy_reset"},
                    {"event_type": "autonomy_run"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale runs_since_reset_index");
        assert!(out.contains("\"mode\":\"runs_since_reset_index\""));
    }

    #[test]
    fn attempt_event_indices_filters_attempt_rows() {
        let out = compute_attempt_event_indices(&AttemptEventIndicesInput {
            events: vec![
                AttemptEventIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                },
                AttemptEventIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("lock_busy".to_string()),
                },
                AttemptEventIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                },
            ],
        });
        assert_eq!(out.indices, vec![0, 2]);
    }

    #[test]
    fn autoscale_json_attempt_event_indices_path_works() {
        let payload = serde_json::json!({
            "mode": "attempt_event_indices",
            "attempt_event_indices_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "executed"},
                    {"event_type": "autonomy_run", "result": "lock_busy"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale attempt_event_indices");
        assert!(out.contains("\"mode\":\"attempt_event_indices\""));
    }

