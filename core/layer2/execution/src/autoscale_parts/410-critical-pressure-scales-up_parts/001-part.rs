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

    #[test]
    fn capacity_counted_attempt_indices_filters_expected_rows() {
        let out = compute_capacity_counted_attempt_indices(&CapacityCountedAttemptIndicesInput {
            events: vec![
                CapacityCountedAttemptIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    policy_hold: Some(false),
                    proposal_id: Some("p1".to_string()),
                },
                CapacityCountedAttemptIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("lock_busy".to_string()),
                    policy_hold: Some(false),
                    proposal_id: Some("p2".to_string()),
                },
                CapacityCountedAttemptIndexEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                    policy_hold: Some(false),
                    proposal_id: Some("p3".to_string()),
                },
            ],
        });
        assert_eq!(out.indices, vec![0, 2]);
    }

    #[test]
    fn autoscale_json_capacity_counted_attempt_indices_path_works() {
        let payload = serde_json::json!({
            "mode": "capacity_counted_attempt_indices",
            "capacity_counted_attempt_indices_input": {
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "executed",
                        "policy_hold": false,
                        "proposal_id": "p1"
                    },
                    {
                        "event_type": "autonomy_run",
                        "result": "lock_busy",
                        "policy_hold": false,
                        "proposal_id": "p2"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capacity_counted_attempt_indices");
        assert!(out.contains("\"mode\":\"capacity_counted_attempt_indices\""));
    }

    #[test]
    fn consecutive_no_progress_runs_counts_tail_until_break() {
        let out = compute_consecutive_no_progress_runs(&ConsecutiveNoProgressRunsInput {
            events: vec![
                ConsecutiveNoProgressEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("no_change".to_string()),
                },
                ConsecutiveNoProgressEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_no_progress".to_string()),
                    outcome: None,
                },
                ConsecutiveNoProgressEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("shipped".to_string()),
                },
            ],
        });
        assert_eq!(out.count, 0);

        let out2 = compute_consecutive_no_progress_runs(&ConsecutiveNoProgressRunsInput {
            events: vec![
                ConsecutiveNoProgressEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("reverted".to_string()),
                },
                ConsecutiveNoProgressEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_no_progress".to_string()),
                    outcome: None,
                },
            ],
        });
        assert_eq!(out2.count, 2);
    }

    #[test]
    fn autoscale_json_consecutive_no_progress_runs_path_works() {
        let payload = serde_json::json!({
            "mode": "consecutive_no_progress_runs",
            "consecutive_no_progress_runs_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "executed", "outcome": "no_change"},
                    {"event_type": "autonomy_run", "result": "stop_repeat_gate_no_progress"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale consecutive_no_progress_runs");
        assert!(out.contains("\"mode\":\"consecutive_no_progress_runs\""));
    }

    #[test]
    fn shipped_count_counts_executed_shipped_rows() {
        let out = compute_shipped_count(&ShippedCountInput {
            events: vec![
                ShippedCountEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("shipped".to_string()),
                },
                ShippedCountEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("reverted".to_string()),
                },
                ShippedCountEventInput {
                    event_type: Some("outcome".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("shipped".to_string()),
                },
            ],
        });
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_shipped_count_path_works() {
        let payload = serde_json::json!({
            "mode": "shipped_count",
            "shipped_count_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "executed", "outcome": "shipped"},
                    {"event_type": "autonomy_run", "result": "executed", "outcome": "reverted"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale shipped_count");
        assert!(out.contains("\"mode\":\"shipped_count\""));
    }

    #[test]
    fn executed_count_by_risk_counts_expected_rows() {
        let out = compute_executed_count_by_risk(&ExecutedCountByRiskInput {
            events: vec![
                ExecutedCountByRiskEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    risk: Some("medium".to_string()),
                    proposal_risk: None,
                },
                ExecutedCountByRiskEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    risk: None,
                    proposal_risk: Some("high".to_string()),
                },
                ExecutedCountByRiskEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_no_progress".to_string()),
                    risk: Some("medium".to_string()),
                    proposal_risk: None,
                },
            ],
            risk: Some("medium".to_string()),
        });
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_executed_count_by_risk_path_works() {
        let payload = serde_json::json!({
            "mode": "executed_count_by_risk",
            "executed_count_by_risk_input": {
                "risk": "high",
                "events": [
                    {"event_type": "autonomy_run", "result": "executed", "risk": "high"},
                    {"event_type": "autonomy_run", "result": "executed", "risk": "medium"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale executed_count_by_risk");
        assert!(out.contains("\"mode\":\"executed_count_by_risk\""));
    }

    #[test]
    fn run_result_tally_counts_autonomy_run_results() {
        let out = compute_run_result_tally(&RunResultTallyInput {
            events: vec![
                RunResultTallyEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                },
                RunResultTallyEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_no_progress".to_string()),
                },
                RunResultTallyEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                },
                RunResultTallyEventInput {
                    event_type: Some("outcome".to_string()),
                    result: Some("executed".to_string()),
                },
            ],
        });
        assert_eq!(out.counts.get("executed").copied().unwrap_or(0), 2);
        assert_eq!(
            out.counts
                .get("stop_repeat_gate_no_progress")
                .copied()
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn autoscale_json_run_result_tally_path_works() {
        let payload = serde_json::json!({
            "mode": "run_result_tally",
            "run_result_tally_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "executed"},
                    {"event_type": "autonomy_run", "result": "executed"},
                    {"event_type": "autonomy_run", "result": "stop_repeat_gate_no_progress"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale run_result_tally");
        assert!(out.contains("\"mode\":\"run_result_tally\""));
    }

    #[test]
    fn qos_lane_weights_adjust_by_pressure() {
        let warning = compute_qos_lane_weights(&QosLaneWeightsInput {
            pressure: Some("warning".to_string()),
            critical_weight: 1.0,
            standard_weight: 1.0,
            explore_weight: 1.0,
            quarantine_weight: 1.0,
        });
        assert!((warning.explore - 0.75).abs() < 0.000001);
        assert!((warning.quarantine - 0.35).abs() < 0.000001);

        let critical = compute_qos_lane_weights(&QosLaneWeightsInput {
            pressure: Some("critical".to_string()),
            critical_weight: 1.0,
            standard_weight: 1.0,
            explore_weight: 1.0,
            quarantine_weight: 1.0,
        });
        assert!((critical.critical - 1.2).abs() < 0.000001);
        assert!((critical.standard - 1.1).abs() < 0.000001);
        assert!((critical.explore - 0.3).abs() < 0.000001);
        assert!((critical.quarantine - 0.1).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_qos_lane_weights_path_works() {
