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
        let payload = serde_json::json!({
            "mode": "qos_lane_weights",
            "qos_lane_weights_input": {
                "pressure": "warning",
                "critical_weight": 1.0,
                "standard_weight": 1.0,
                "explore_weight": 1.0,
                "quarantine_weight": 1.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale qos_lane_weights");
        assert!(out.contains("\"mode\":\"qos_lane_weights\""));
    }

    #[test]
    fn qos_lane_usage_counts_modes() {
        let out = compute_qos_lane_usage(&QosLaneUsageInput {
            events: vec![
                QosLaneUsageEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    selection_mode: Some("qos_critical_exploit".to_string()),
                },
                QosLaneUsageEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    selection_mode: Some("qos_explore_explore".to_string()),
                },
                QosLaneUsageEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_repeat_gate_no_progress".to_string()),
                    selection_mode: Some("qos_standard_exploit".to_string()),
                },
            ],
        });
        assert_eq!(out.critical, 1);
        assert_eq!(out.explore, 1);
        assert_eq!(out.standard, 0);
        assert_eq!(out.quarantine, 0);
    }

    #[test]
    fn autoscale_json_qos_lane_usage_path_works() {
        let payload = serde_json::json!({
            "mode": "qos_lane_usage",
            "qos_lane_usage_input": {
                "events": [
                    {"event_type": "autonomy_run", "result": "executed", "selection_mode": "qos_standard_exploit"},
                    {"event_type": "autonomy_run", "result": "executed", "selection_mode": "qos_quarantine_explore"}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale qos_lane_usage");
        assert!(out.contains("\"mode\":\"qos_lane_usage\""));
    }

