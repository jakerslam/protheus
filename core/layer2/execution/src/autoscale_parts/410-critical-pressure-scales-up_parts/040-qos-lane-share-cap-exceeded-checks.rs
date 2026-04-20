    #[test]
    fn qos_lane_share_cap_exceeded_checks_explore_and_quarantine() {
        let explore = compute_qos_lane_share_cap_exceeded(&QosLaneShareCapExceededInput {
            lane: Some("explore".to_string()),
            explore_usage: 4.0,
            quarantine_usage: 1.0,
            executed_count: 10.0,
            explore_max_share: 0.35,
            quarantine_max_share: 0.2,
        });
        assert!(explore.exceeded);

        let quarantine = compute_qos_lane_share_cap_exceeded(&QosLaneShareCapExceededInput {
            lane: Some("quarantine".to_string()),
            explore_usage: 1.0,
            quarantine_usage: 1.0,
            executed_count: 10.0,
            explore_max_share: 0.35,
            quarantine_max_share: 0.2,
        });
        assert!(!quarantine.exceeded);
    }

    #[test]
    fn autoscale_json_qos_lane_share_cap_exceeded_path_works() {
        let payload = serde_json::json!({
            "mode": "qos_lane_share_cap_exceeded",
            "qos_lane_share_cap_exceeded_input": {
                "lane": "explore",
                "explore_usage": 4,
                "quarantine_usage": 1,
                "executed_count": 10,
                "explore_max_share": 0.35,
                "quarantine_max_share": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale qos_lane_share_cap_exceeded");
        assert!(out.contains("\"mode\":\"qos_lane_share_cap_exceeded\""));
    }

    #[test]
    fn qos_lane_from_candidate_routes_expected_lane() {
        let quarantine = compute_qos_lane_from_candidate(&QosLaneFromCandidateInput {
            queue_underflow_backfill: true,
            pulse_tier: 2,
            proposal_type: Some("directive_clarification".to_string()),
            deprioritized_source: false,
            risk: Some("medium".to_string()),
        });
        assert_eq!(quarantine.lane, "quarantine");

        let explore = compute_qos_lane_from_candidate(&QosLaneFromCandidateInput {
            queue_underflow_backfill: false,
            pulse_tier: 5,
            proposal_type: Some("other".to_string()),
            deprioritized_source: false,
            risk: Some("medium".to_string()),
        });
        assert_eq!(explore.lane, "explore");
    }

    #[test]
    fn autoscale_json_qos_lane_from_candidate_path_works() {
        let payload = serde_json::json!({
            "mode": "qos_lane_from_candidate",
            "qos_lane_from_candidate_input": {
                "queue_underflow_backfill": false,
                "pulse_tier": 1,
                "proposal_type": "directive_decomposition",
                "deprioritized_source": false,
                "risk": "low"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale qos_lane_from_candidate");
        assert!(out.contains("\"mode\":\"qos_lane_from_candidate\""));
    }

    #[test]
    fn eye_outcome_count_window_counts_matching_rows() {
        let out = compute_eye_outcome_count_window(&EyeOutcomeWindowCountInput {
            events: vec![
                EyeOutcomeEventInput {
                    event_type: Some("outcome".to_string()),
                    outcome: Some("success".to_string()),
                    evidence_ref: Some("eye:foo".to_string()),
                    ts: Some("2026-03-03T10:00:00.000Z".to_string()),
                },
                EyeOutcomeEventInput {
                    event_type: Some("outcome".to_string()),
                    outcome: Some("success".to_string()),
                    evidence_ref: Some("eye:bar".to_string()),
                    ts: Some("2026-03-03T10:00:00.000Z".to_string()),
                },
                EyeOutcomeEventInput {
                    event_type: Some("outcome".to_string()),
                    outcome: Some("success".to_string()),
                    evidence_ref: Some("eye:foo".to_string()),
                    ts: Some("2026-02-20T10:00:00.000Z".to_string()),
                },
            ],
            eye_ref: Some("eye:foo".to_string()),
            outcome: Some("success".to_string()),
            end_date_str: Some("2026-03-03".to_string()),
            days: Some(7),
        });
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_eye_outcome_count_window_path_works() {
        let payload = serde_json::json!({
            "mode": "eye_outcome_count_window",
            "eye_outcome_count_window_input": {
                "eye_ref": "eye:foo",
                "outcome": "success",
                "end_date_str": "2026-03-03",
                "days": 3,
                "events": [
                    {
                        "event_type": "outcome",
                        "outcome": "success",
                        "evidence_ref": "eye:foo",
                        "ts": "2026-03-03T10:00:00.000Z"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale eye_outcome_count_window");
        assert!(out.contains("\"mode\":\"eye_outcome_count_window\""));
    }

    #[test]
    fn eye_outcome_count_last_hours_counts_matching_rows() {
        let out = compute_eye_outcome_count_last_hours(&EyeOutcomeLastHoursCountInput {
            events: vec![
                EyeOutcomeEventInput {
                    event_type: Some("outcome".to_string()),
                    outcome: Some("success".to_string()),
                    evidence_ref: Some("eye:foo".to_string()),
                    ts: Some("2026-03-03T11:00:00.000Z".to_string()),
                },
                EyeOutcomeEventInput {
                    event_type: Some("outcome".to_string()),
                    outcome: Some("success".to_string()),
                    evidence_ref: Some("eye:foo".to_string()),
                    ts: Some("2026-03-02T10:00:00.000Z".to_string()),
                },
            ],
            eye_ref: Some("eye:foo".to_string()),
            outcome: Some("success".to_string()),
            hours: Some(3.0),
            now_ms: Some(1_772_503_200_000.0),
        });
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_eye_outcome_count_last_hours_path_works() {
        let payload = serde_json::json!({
            "mode": "eye_outcome_count_last_hours",
            "eye_outcome_count_last_hours_input": {
                "eye_ref": "eye:foo",
                "outcome": "success",
                "hours": 6,
                "now_ms": 1772503200000.0,
                "events": [
                    {
                        "event_type": "outcome",
                        "outcome": "success",
                        "evidence_ref": "eye:foo",
                        "ts": "2026-03-03T11:00:00.000Z"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale eye_outcome_count_last_hours");
        assert!(out.contains("\"mode\":\"eye_outcome_count_last_hours\""));
    }

    #[test]
    fn sorted_counts_orders_by_count_then_result() {
        let out = compute_sorted_counts(&SortedCountsInput {
            counts: std::collections::BTreeMap::from([
                ("b".to_string(), 2.0),
                ("a".to_string(), 2.0),
                ("c".to_string(), 1.0),
            ]),
        });
        assert_eq!(
            out.items,
            vec![
                SortedCountItem {
                    result: "a".to_string(),
                    count: 2
                },
                SortedCountItem {
                    result: "b".to_string(),
                    count: 2
                },
                SortedCountItem {
                    result: "c".to_string(),
                    count: 1
                }
            ]
        );
    }

    #[test]
    fn autoscale_json_sorted_counts_path_works() {
        let payload = serde_json::json!({
            "mode": "sorted_counts",
            "sorted_counts_input": {
                "counts": {
                    "executed": 2,
                    "stop_repeat_gate_no_progress": 1
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale sorted_counts");
        assert!(out.contains("\"mode\":\"sorted_counts\""));
    }

    #[test]
    fn normalize_proposal_status_maps_expected_values() {
        let out = compute_normalize_proposal_status(&NormalizeProposalStatusInput {
            raw_status: Some("closed_won".to_string()),
            fallback: Some("pending".to_string()),
        });
        assert_eq!(out.normalized_status, "closed");

        let out2 = compute_normalize_proposal_status(&NormalizeProposalStatusInput {
            raw_status: Some("queued".to_string()),
            fallback: Some("pending".to_string()),
        });
        assert_eq!(out2.normalized_status, "pending");
    }

    #[test]
    fn autoscale_json_normalize_proposal_status_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_proposal_status",
            "normalize_proposal_status_input": {
                "raw_status": "closed_won",
                "fallback": "pending"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_proposal_status");
        assert!(out.contains("\"mode\":\"normalize_proposal_status\""));
    }

    #[test]
    fn proposal_status_maps_overlay_decision_values() {
        let accepted = compute_proposal_status(&ProposalStatusInput {
            overlay_decision: Some("accept".to_string()),
        });
        assert_eq!(accepted.status, "accepted");

        let rejected = compute_proposal_status(&ProposalStatusInput {
            overlay_decision: Some("reject".to_string()),
        });
        assert_eq!(rejected.status, "rejected");

        let parked = compute_proposal_status(&ProposalStatusInput {
            overlay_decision: Some("park".to_string()),
        });
        assert_eq!(parked.status, "parked");

        let pending = compute_proposal_status(&ProposalStatusInput {
            overlay_decision: Some("other".to_string()),
        });
        assert_eq!(pending.status, "pending");
    }

    #[test]
    fn autoscale_json_proposal_status_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_status",
            "proposal_status_input": {
                "overlay_decision": "accept"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_status");
        assert!(out.contains("\"mode\":\"proposal_status\""));
    }

    #[test]
    fn proposal_outcome_status_normalizes_or_none() {
        let out = compute_proposal_outcome_status(&ProposalOutcomeStatusInput {
            overlay_outcome: Some(" SHIPPED ".to_string()),
        });
        assert_eq!(out.outcome, Some("shipped".to_string()));

        let out2 = compute_proposal_outcome_status(&ProposalOutcomeStatusInput {
            overlay_outcome: Some("   ".to_string()),
        });
        assert_eq!(out2.outcome, None);
    }

    #[test]
    fn autoscale_json_proposal_outcome_status_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_outcome_status",
            "proposal_outcome_status_input": {
                "overlay_outcome": "shipped"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_outcome_status");
        assert!(out.contains("\"mode\":\"proposal_outcome_status\""));
    }

    #[test]
    fn queue_underflow_backfill_allows_only_accepted_without_outcome() {
        let allow = compute_queue_underflow_backfill(&QueueUnderflowBackfillInput {
            underflow_backfill_max: 2.0,
            status: Some("accepted".to_string()),
            overlay_outcome: Some(String::new()),
        });
        assert!(allow.allow);

        let deny = compute_queue_underflow_backfill(&QueueUnderflowBackfillInput {
            underflow_backfill_max: 2.0,
            status: Some("accepted".to_string()),
            overlay_outcome: Some("shipped".to_string()),
        });
        assert!(!deny.allow);
    }

