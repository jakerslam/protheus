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

    #[test]
    fn manual_gate_prefilter_blocks_when_rate_exceeded() {
        let out = compute_manual_gate_prefilter(&ManualGatePrefilterInput {
            enabled: true,
            capability_key: Some("deploy".to_string()),
            window_hours: 24.0,
            min_observations: 3.0,
            max_manual_block_rate: 0.4,
            row_present: true,
            attempts: 10.0,
            manual_blocked: 5.0,
            manual_block_rate: 0.5,
        });
        assert!(out.applicable);
        assert!(!out.pass);
        assert_eq!(out.reason, "manual_gate_rate_exceeded");
    }

    #[test]
    fn autoscale_json_manual_gate_prefilter_path_works() {
        let payload = serde_json::json!({
            "mode": "manual_gate_prefilter",
            "manual_gate_prefilter_input": {
                "enabled": true,
                "capability_key": "deploy",
                "window_hours": 24,
                "min_observations": 3,
                "max_manual_block_rate": 0.4,
                "row_present": true,
                "attempts": 4,
                "manual_blocked": 1,
                "manual_block_rate": 0.25
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale manual_gate_prefilter");
        assert!(out.contains("\"mode\":\"manual_gate_prefilter\""));
    }

    #[test]
    fn execute_confidence_cooldown_active_requires_key_and_active_state() {
        let out =
            compute_execute_confidence_cooldown_active(&ExecuteConfidenceCooldownActiveInput {
                cooldown_key: Some("exec:cooldown:key".to_string()),
                cooldown_active: true,
            });
        assert!(out.active);
        let out =
            compute_execute_confidence_cooldown_active(&ExecuteConfidenceCooldownActiveInput {
                cooldown_key: Some("".to_string()),
                cooldown_active: true,
            });
        assert!(!out.active);
    }

    #[test]
    fn autoscale_json_execute_confidence_cooldown_active_path_works() {
        let payload = serde_json::json!({
            "mode": "execute_confidence_cooldown_active",
            "execute_confidence_cooldown_active_input": {
                "cooldown_key": "exec:cooldown:key",
                "cooldown_active": true
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale execute_confidence_cooldown_active");
        assert!(out.contains("\"mode\":\"execute_confidence_cooldown_active\""));
    }

