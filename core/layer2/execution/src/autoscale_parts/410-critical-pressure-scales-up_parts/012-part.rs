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

    #[test]
    fn recent_run_events_flattens_day_buckets_in_order() {
        let out = compute_recent_run_events(&RecentRunEventsInput {
            day_events: vec![
                vec![serde_json::json!({"id":"a"}), serde_json::json!({"id":"b"})],
                vec![serde_json::json!({"id":"c"})],
            ],
        });
        assert_eq!(out.events.len(), 3);
        assert_eq!(
            out.events[0]
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "a"
        );
        assert_eq!(
            out.events[2]
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "c"
        );
    }

    #[test]
    fn autoscale_json_recent_run_events_path_works() {
        let payload = serde_json::json!({
            "mode": "recent_run_events",
            "recent_run_events_input": {
                "day_events": [
                    [{"id":"a"}],
                    [{"id":"b"}]
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale recent_run_events");
        assert!(out.contains("\"mode\":\"recent_run_events\""));
        assert!(out.contains("\"id\":\"a\""));
        assert!(out.contains("\"id\":\"b\""));
    }

    #[test]
    fn all_decision_events_flattens_day_buckets_in_order() {
        let out = compute_all_decision_events(&AllDecisionEventsInput {
            day_events: vec![
                vec![serde_json::json!({"proposal_id":"p1"})],
                vec![serde_json::json!({"proposal_id":"p2"})],
            ],
        });
        assert_eq!(out.events.len(), 2);
        assert_eq!(
            out.events[0]
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "p1"
        );
        assert_eq!(
            out.events[1]
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "p2"
        );
    }

    #[test]
    fn autoscale_json_all_decision_events_path_works() {
        let payload = serde_json::json!({
            "mode": "all_decision_events",
            "all_decision_events_input": {
                "day_events": [
                    [{"proposal_id":"p1"}],
                    [{"proposal_id":"p2"}]
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale all_decision_events");
        assert!(out.contains("\"mode\":\"all_decision_events\""));
        assert!(out.contains("\"proposal_id\":\"p1\""));
        assert!(out.contains("\"proposal_id\":\"p2\""));
    }

    #[test]
    fn cooldown_active_state_matches_threshold_behavior() {
        let active = compute_cooldown_active_state(&CooldownActiveStateInput {
            until_ms: Some(1100.0),
            now_ms: Some(1000.0),
        });
        assert!(active.active);
        assert!(!active.expired);

        let boundary = compute_cooldown_active_state(&CooldownActiveStateInput {
            until_ms: Some(1000.0),
            now_ms: Some(1000.0),
        });
        assert!(boundary.active);
        assert!(!boundary.expired);

        let expired = compute_cooldown_active_state(&CooldownActiveStateInput {
            until_ms: Some(999.0),
            now_ms: Some(1000.0),
        });
        assert!(!expired.active);
        assert!(expired.expired);
    }

    #[test]
    fn autoscale_json_cooldown_active_state_path_works() {
        let payload = serde_json::json!({
            "mode": "cooldown_active_state",
            "cooldown_active_state_input": {
                "until_ms": 1200,
                "now_ms": 1000
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale cooldown_active_state");
        assert!(out.contains("\"mode\":\"cooldown_active_state\""));
        assert!(out.contains("\"active\":true"));
    }

    #[test]
    fn bump_count_increments_from_current() {
        let out = compute_bump_count(&BumpCountInput {
            current_count: Some(3.0),
        });
        assert_eq!(out.count, 4.0);
    }

    #[test]
    fn autoscale_json_bump_count_path_works() {
        let payload = serde_json::json!({
            "mode": "bump_count",
            "bump_count_input": {
                "current_count": 7
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale bump_count");
        assert!(out.contains("\"mode\":\"bump_count\""));
        assert!(out.contains("\"count\":8.0"));
    }

    #[test]
    fn lock_age_minutes_returns_none_for_invalid_and_minutes_for_valid_ts() {
        let invalid = compute_lock_age_minutes(&LockAgeMinutesInput {
            lock_ts: Some("bad-ts".to_string()),
            now_ms: Some(1_000_000.0),
        });
        assert!(invalid.age_minutes.is_none());

        let valid = compute_lock_age_minutes(&LockAgeMinutesInput {
            lock_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
            now_ms: Some(
                chrono::DateTime::parse_from_rfc3339("2026-03-04T01:00:00.000Z")
                    .unwrap()
                    .timestamp_millis() as f64,
            ),
        });
        assert!(valid.age_minutes.is_some());
        assert!((valid.age_minutes.unwrap_or(0.0) - 60.0).abs() < 1e-6);
    }

    #[test]
    fn autoscale_json_lock_age_minutes_path_works() {
        let payload = serde_json::json!({
            "mode": "lock_age_minutes",
            "lock_age_minutes_input": {
                "lock_ts": "2026-03-04T00:00:00.000Z",
                "now_ms": chrono::DateTime::parse_from_rfc3339("2026-03-04T00:30:00.000Z").unwrap().timestamp_millis()
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale lock_age_minutes");
        assert!(out.contains("\"mode\":\"lock_age_minutes\""));
        assert!(out.contains("\"age_minutes\":30.0"));
    }

    #[test]
    fn hash_obj_hashes_json_payload_and_returns_none_when_missing() {
        let missing = compute_hash_obj(&HashObjInput { json: None });
        assert!(missing.hash.is_none());

        let out = compute_hash_obj(&HashObjInput {
            json: Some("{\"a\":1}".to_string()),
        });
        assert!(out.hash.is_some());
        assert_eq!(
            out.hash.unwrap_or_default(),
            "015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
        );
    }

    #[test]
    fn autoscale_json_hash_obj_path_works() {
        let payload = serde_json::json!({
            "mode": "hash_obj",
            "hash_obj_input": {
                "json": "{\"x\":2}"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale hash_obj");
        assert!(out.contains("\"mode\":\"hash_obj\""));
        assert!(out.contains("\"hash\":\""));
    }

    #[test]
    fn assess_success_criteria_quality_flags_unknown_and_unsupported() {
        let out = compute_assess_success_criteria_quality(&AssessSuccessCriteriaQualityInput {
            checks: vec![
                AssessSuccessCriteriaQualityCheckInput {
                    evaluated: false,
                    reason: Some("unsupported_metric".to_string()),
                },
                AssessSuccessCriteriaQualityCheckInput {
                    evaluated: false,
                    reason: Some("artifact_delta_unavailable".to_string()),
                },
                AssessSuccessCriteriaQualityCheckInput {
                    evaluated: true,
                    reason: Some("ok".to_string()),
                },
            ],
            total_count: 3.0,
            unknown_count: 2.0,
            synthesized: true,
        });
        assert!(out.insufficient);
        assert!(out.reasons.contains(&"synthesized_criteria".to_string()));
        assert_eq!(out.unknown_exempt_count, 1.0);
        assert_eq!(out.unknown_count, 1.0);
        assert_eq!(out.unsupported_count, 1.0);
    }

    #[test]
    fn autoscale_json_assess_success_criteria_quality_path_works() {
        let payload = serde_json::json!({
            "mode": "assess_success_criteria_quality",
            "assess_success_criteria_quality_input": {
                "checks": [
                    {"evaluated": false, "reason": "unsupported_metric"},
                    {"evaluated": true, "reason": "ok"}
                ],
                "total_count": 2,
                "unknown_count": 1,
                "synthesized": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale assess_success_criteria_quality");
        assert!(out.contains("\"mode\":\"assess_success_criteria_quality\""));
        assert!(out.contains("\"insufficient\":false") || out.contains("\"insufficient\":true"));
    }
