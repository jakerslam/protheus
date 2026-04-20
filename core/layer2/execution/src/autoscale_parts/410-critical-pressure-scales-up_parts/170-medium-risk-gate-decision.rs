    #[test]
    fn autoscale_json_medium_risk_gate_decision_path_works() {
        let payload = serde_json::json!({
            "mode": "medium_risk_gate_decision",
            "medium_risk_gate_decision_input": {
                "risk": "medium",
                "composite_score": 72,
                "directive_fit_score": 68,
                "actionability_score": 66,
                "composite_min": 70,
                "directive_fit_min": 60,
                "actionability_min": 62
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale medium_risk_gate_decision");
        assert!(out.contains("\"mode\":\"medium_risk_gate_decision\""));
    }

    #[test]
    fn route_block_prefilter_blocks_when_rate_exceeded() {
        let out = compute_route_block_prefilter(&RouteBlockPrefilterInput {
            enabled: true,
            capability_key: Some("deploy".to_string()),
            window_hours: 24.0,
            min_observations: 3.0,
            max_block_rate: 0.5,
            row_present: true,
            attempts: 10.0,
            route_blocked: 6.0,
            route_block_rate: 0.6,
        });
        assert!(out.applicable);
        assert!(!out.pass);
        assert_eq!(out.reason, "route_block_rate_exceeded");
    }

    #[test]
    fn autoscale_json_route_block_prefilter_path_works() {
        let payload = serde_json::json!({
            "mode": "route_block_prefilter",
            "route_block_prefilter_input": {
                "enabled": true,
                "capability_key": "deploy",
                "window_hours": 24,
                "min_observations": 3,
                "max_block_rate": 0.5,
                "row_present": true,
                "attempts": 4,
                "route_blocked": 1,
                "route_block_rate": 0.25
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_block_prefilter");
        assert!(out.contains("\"mode\":\"route_block_prefilter\""));
    }

    #[test]
    fn route_execution_sample_event_matches_route_logic() {
        let blocked = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("score_only_fallback_route_block".to_string()),
            execution_target: Some("cell".to_string()),
            route_summary_present: false,
        });
        assert!(blocked.is_sample_event);

        let route_exec = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            execution_target: Some("route".to_string()),
            route_summary_present: false,
        });
        assert!(route_exec.is_sample_event);

        let non_sample = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("no_change".to_string()),
            execution_target: Some("route".to_string()),
            route_summary_present: true,
        });
        assert!(!non_sample.is_sample_event);
    }

    #[test]
    fn autoscale_json_route_execution_sample_event_path_works() {
        let payload = serde_json::json!({
            "mode": "route_execution_sample_event",
            "route_execution_sample_event_input": {
                "event_type": "autonomy_run",
                "result": "executed",
                "execution_target": "route",
                "route_summary_present": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_execution_sample_event");
        assert!(out.contains("\"mode\":\"route_execution_sample_event\""));
        assert!(out.contains("\"is_sample_event\":true"));
    }

    #[test]
    fn route_block_telemetry_summary_aggregates_by_capability() {
        let out = compute_route_block_telemetry_summary(&RouteBlockTelemetrySummaryInput {
            events: vec![
                RouteBlockTelemetryEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    execution_target: Some("route".to_string()),
                    route_summary_present: false,
                    capability_key: Some("deploy".to_string()),
                },
                RouteBlockTelemetryEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("score_only_fallback_route_block".to_string()),
                    execution_target: Some("cell".to_string()),
                    route_summary_present: false,
                    capability_key: Some("deploy".to_string()),
                },
            ],
            window_hours: 12.0,
        });
        assert_eq!(out.sample_events, 2.0);
        assert_eq!(out.by_capability.len(), 1);
        assert_eq!(out.by_capability[0].key, "deploy");
        assert_eq!(out.by_capability[0].attempts, 2.0);
        assert_eq!(out.by_capability[0].route_blocked, 1.0);
        assert!((out.by_capability[0].route_block_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn autoscale_json_route_block_telemetry_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "route_block_telemetry_summary",
            "route_block_telemetry_summary_input": {
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "executed",
                        "execution_target": "route",
                        "route_summary_present": false,
                        "capability_key": "deploy"
                    }
                ],
                "window_hours": 6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_block_telemetry_summary");
        assert!(out.contains("\"mode\":\"route_block_telemetry_summary\""));
        assert!(out.contains("\"sample_events\":1"));
    }

    #[test]
    fn is_stub_proposal_matches_title_marker() {
        let yes = compute_is_stub_proposal(&IsStubProposalInput {
            title: Some("[STUB] backlog".to_string()),
        });
        assert!(yes.is_stub);
        let no = compute_is_stub_proposal(&IsStubProposalInput {
            title: Some("shippable task".to_string()),
        });
        assert!(!no.is_stub);
    }

    #[test]
    fn autoscale_json_is_stub_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "is_stub_proposal",
            "is_stub_proposal_input": {
                "title": "[STUB] investigate"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale is_stub_proposal");
        assert!(out.contains("\"mode\":\"is_stub_proposal\""));
        assert!(out.contains("\"is_stub\":true"));
    }

    #[test]
    fn recent_autonomy_run_events_filters_by_type_time_and_cap() {
        let now = Utc::now().timestamp_millis();
        let recent = chrono::DateTime::from_timestamp_millis(now - 30 * 60 * 1000)
            .expect("recent dt")
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let old = chrono::DateTime::from_timestamp_millis(now - 5 * 60 * 60 * 1000)
            .expect("old dt")
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let out = compute_recent_autonomy_run_events(&RecentAutonomyRunEventsInput {
            events: vec![
                serde_json::json!({"type":"autonomy_run","ts":recent}),
                serde_json::json!({"type":"heartbeat","ts":recent}),
                serde_json::json!({"type":"autonomy_run","ts":old}),
            ],
            cutoff_ms: (now - 2 * 60 * 60 * 1000) as f64,
            cap: 50.0,
        });
        assert_eq!(out.events.len(), 1);
        assert_eq!(
            out.events[0]
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "autonomy_run"
        );
    }

    #[test]
    fn autoscale_json_recent_autonomy_run_events_path_works() {
        let now = Utc::now().timestamp_millis();
        let recent = chrono::DateTime::from_timestamp_millis(now - 30 * 60 * 1000)
            .expect("recent dt")
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let payload = serde_json::json!({
            "mode": "recent_autonomy_run_events",
            "recent_autonomy_run_events_input": {
                "events": [
                    {"type":"autonomy_run","ts": recent},
                    {"type":"heartbeat","ts": recent}
                ],
                "cutoff_ms": now - 2 * 60 * 60 * 1000,
                "cap": 50
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale recent_autonomy_run_events");
        assert!(out.contains("\"mode\":\"recent_autonomy_run_events\""));
    }

    #[test]
    fn proposal_meta_index_dedupes_first_seen_rows() {
        let out = compute_proposal_meta_index(&ProposalMetaIndexInput {
            entries: vec![
                ProposalMetaIndexEntryInput {
                    proposal_id: Some("p1".to_string()),
                    eye_id: Some("eye_a".to_string()),
                    topics: vec!["A".to_string(), "b".to_string()],
                },
                ProposalMetaIndexEntryInput {
                    proposal_id: Some("p1".to_string()),
                    eye_id: Some("eye_b".to_string()),
                    topics: vec!["c".to_string()],
                },
                ProposalMetaIndexEntryInput {
                    proposal_id: Some("p2".to_string()),
                    eye_id: Some("eye_c".to_string()),
                    topics: vec!["X".to_string()],
                },
            ],
        });
        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0].proposal_id, "p1");
        assert_eq!(out.entries[0].eye_id, "eye_a");
        assert_eq!(
            out.entries[0].topics,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(out.entries[1].proposal_id, "p2");
    }

    #[test]
    fn autoscale_json_proposal_meta_index_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_meta_index",
            "proposal_meta_index_input": {
                "entries": [
                    { "proposal_id": "p1", "eye_id": "eye_a", "topics": ["One"] },
                    { "proposal_id": "p1", "eye_id": "eye_b", "topics": ["Two"] }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_meta_index");
        assert!(out.contains("\"mode\":\"proposal_meta_index\""));
        assert!(out.contains("\"proposal_id\":\"p1\""));
    }

    #[test]
    fn new_log_events_slices_runs_and_errors_from_before_lengths() {
        let out = compute_new_log_events(&NewLogEventsInput {
            before_run_len: Some(1.0),
            before_error_len: Some(2.0),
            after_runs: vec![
                serde_json::json!({"id":"r1"}),
                serde_json::json!({"id":"r2"}),
            ],
            after_errors: vec![
                serde_json::json!("e1"),
                serde_json::json!("e2"),
                serde_json::json!("e3"),
            ],
        });
        assert_eq!(out.runs.len(), 1);
        assert_eq!(
            out.runs[0].get("id").and_then(|v| v.as_str()).unwrap_or(""),
            "r2"
        );
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.errors[0].as_str().unwrap_or(""), "e3");
    }

    #[test]
    fn autoscale_json_new_log_events_path_works() {
        let payload = serde_json::json!({
            "mode": "new_log_events",
            "new_log_events_input": {
                "before_run_len": 1,
                "before_error_len": 0,
                "after_runs": [{"id":"r1"},{"id":"r2"}],
                "after_errors": ["e1"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale new_log_events");
        assert!(out.contains("\"mode\":\"new_log_events\""));
        assert!(out.contains("\"runs\":[{\"id\":\"r2\"}]"));
    }

    #[test]
    fn outcome_buckets_returns_zeroed_counts() {
        let out = compute_outcome_buckets(&OutcomeBucketsInput {});
        assert_eq!(out.shipped, 0.0);
        assert_eq!(out.no_change, 0.0);
        assert_eq!(out.reverted, 0.0);
    }

    #[test]
    fn autoscale_json_outcome_buckets_path_works() {
        let payload = serde_json::json!({
            "mode": "outcome_buckets",
            "outcome_buckets_input": {}
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale outcome_buckets");
        assert!(out.contains("\"mode\":\"outcome_buckets\""));
        assert!(out.contains("\"shipped\":0.0"));
    }

