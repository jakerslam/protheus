    #[test]
    fn autoscale_json_directive_fit_assessment_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_fit_assessment",
            "directive_fit_assessment_input": {
                "min_directive_fit": 50,
                "profile_available": true,
                "active_directive_ids": ["T1_growth"],
                "positive_phrase_hits": ["raise revenue"],
                "positive_token_hits": ["growth"],
                "strategy_hits": ["scale"],
                "negative_phrase_hits": [],
                "negative_token_hits": [],
                "strategy_token_count": 2,
                "impact": "high"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_fit_assessment");
        assert!(out.contains("\"mode\":\"directive_fit_assessment\""));
    }

    #[test]
    fn signal_quality_assessment_scores_expected_fields() {
        let out = compute_signal_quality_assessment(&SignalQualityAssessmentInput {
            min_signal_quality: 45.0,
            min_sensory_signal: 40.0,
            min_sensory_relevance: 42.0,
            min_eye_score_ema: 45.0,
            eye_id: Some("eye_revenue".to_string()),
            score_source: Some("sensory_relevance_score".to_string()),
            impact: Some("high".to_string()),
            risk: Some("low".to_string()),
            domain: Some("example.com".to_string()),
            url_scheme: Some("https".to_string()),
            title_has_stub: false,
            combined_item_score: Some(70.0),
            sensory_relevance_score: Some(72.0),
            sensory_relevance_tier: Some("high".to_string()),
            sensory_quality_score: Some(68.0),
            sensory_quality_tier: Some("high".to_string()),
            eye_known: true,
            eye_status: Some("active".to_string()),
            eye_score_ema: Some(64.0),
            parser_type: Some("rss".to_string()),
            parser_disallowed: false,
            domain_allowlist_enforced: true,
            domain_allowed: true,
            eye_proposed_total: Some(8.0),
            eye_yield_rate: Some(0.35),
            calibration_eye_bias: 1.5,
            calibration_topic_bias: 0.5,
        });
        assert!(out.pass);
        assert!(out.score >= 45.0);
        assert_eq!(out.eye_id, "eye_revenue");
        assert_eq!(out.score_source, "sensory_relevance_score");
    }

    #[test]
    fn autoscale_json_signal_quality_assessment_path_works() {
        let payload = serde_json::json!({
            "mode": "signal_quality_assessment",
            "signal_quality_assessment_input": {
                "min_signal_quality": 45,
                "min_sensory_signal": 40,
                "min_sensory_relevance": 42,
                "min_eye_score_ema": 45,
                "eye_id": "eye_revenue",
                "score_source": "sensory_relevance_score",
                "impact": "high",
                "risk": "low",
                "domain": "example.com",
                "url_scheme": "https",
                "title_has_stub": false,
                "combined_item_score": 70,
                "sensory_relevance_score": 72,
                "sensory_relevance_tier": "high",
                "sensory_quality_score": 68,
                "sensory_quality_tier": "high",
                "eye_known": true,
                "eye_status": "active",
                "eye_score_ema": 64,
                "parser_type": "rss",
                "parser_disallowed": false,
                "domain_allowlist_enforced": true,
                "domain_allowed": true,
                "eye_proposed_total": 8,
                "eye_yield_rate": 0.35,
                "calibration_eye_bias": 1.5,
                "calibration_topic_bias": 0.5
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale signal_quality_assessment");
        assert!(out.contains("\"mode\":\"signal_quality_assessment\""));
    }

    #[test]
    fn actionability_assessment_scores_actionable_candidate() {
        let out = compute_actionability_assessment(&ActionabilityAssessmentInput {
            min_actionability: 45.0,
            risk: Some("low".to_string()),
            impact: Some("high".to_string()),
            validation_count: 2.0,
            specific_validation_count: 2.0,
            has_next_cmd: true,
            generic_route_task: false,
            next_cmd_has_dry_run: false,
            looks_like_discovery_cmd: false,
            has_action_verb: true,
            has_opportunity: true,
            has_concrete_target: true,
            is_meta_coordination: false,
            is_explainer: false,
            mentions_proposal: false,
            relevance_score: Some(70.0),
            directive_fit_score: Some(72.0),
            criteria_requirement_applied: true,
            criteria_exempt_type: false,
            criteria_min_count: 1.0,
            measurable_criteria_count: 2.0,
            criteria_total_count: 2.0,
            criteria_pattern_penalty: 0.0,
            criteria_pattern_hits: Some(serde_json::json!([])),
            is_executable_proposal: true,
            has_rollback_signal: true,
            subdirective_required: false,
            subdirective_has_concrete_target: true,
            subdirective_has_expected_delta: true,
            subdirective_has_verification_step: true,
            subdirective_target_count: 1.0,
            subdirective_verify_count: 1.0,
            subdirective_success_criteria_count: 2.0,
        });
        assert!(out.pass);
        assert!(out.score >= 45.0);
    }

    #[test]
    fn autoscale_json_actionability_assessment_path_works() {
        let payload = serde_json::json!({
            "mode": "actionability_assessment",
            "actionability_assessment_input": {
                "min_actionability": 45,
                "risk": "low",
                "impact": "high",
                "validation_count": 2,
                "specific_validation_count": 2,
                "has_next_cmd": true,
                "generic_route_task": false,
                "next_cmd_has_dry_run": false,
                "looks_like_discovery_cmd": false,
                "has_action_verb": true,
                "has_opportunity": true,
                "has_concrete_target": true,
                "is_meta_coordination": false,
                "is_explainer": false,
                "mentions_proposal": false,
                "relevance_score": 70,
                "directive_fit_score": 72,
                "criteria_requirement_applied": true,
                "criteria_exempt_type": false,
                "criteria_min_count": 1,
                "measurable_criteria_count": 2,
                "criteria_total_count": 2,
                "criteria_pattern_penalty": 0,
                "criteria_pattern_hits": [],
                "is_executable_proposal": true,
                "has_rollback_signal": true,
                "subdirective_required": false,
                "subdirective_has_concrete_target": true,
                "subdirective_has_expected_delta": true,
                "subdirective_has_verification_step": true,
                "subdirective_target_count": 1,
                "subdirective_verify_count": 1,
                "subdirective_success_criteria_count": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale actionability_assessment");
        assert!(out.contains("\"mode\":\"actionability_assessment\""));
    }

    #[test]
    fn proposal_status_for_queue_pressure_prefers_overlay_then_explicit_status() {
        let out =
            compute_proposal_status_for_queue_pressure(&ProposalStatusForQueuePressureInput {
                overlay_decision: Some("accept".to_string()),
                proposal_status: Some("rejected".to_string()),
            });
        assert_eq!(out.status, "accepted");

        let out2 =
            compute_proposal_status_for_queue_pressure(&ProposalStatusForQueuePressureInput {
                overlay_decision: None,
                proposal_status: Some("closed_won".to_string()),
            });
        assert_eq!(out2.status, "closed");

        let out3 =
            compute_proposal_status_for_queue_pressure(&ProposalStatusForQueuePressureInput {
                overlay_decision: None,
                proposal_status: Some("pending".to_string()),
            });
        assert_eq!(out3.status, "pending");
    }

    #[test]
    fn autoscale_json_proposal_status_for_queue_pressure_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_status_for_queue_pressure",
            "proposal_status_for_queue_pressure_input": {
                "overlay_decision": "accept",
                "proposal_status": "queued"
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale proposal_status_for_queue_pressure");
        assert!(out.contains("\"mode\":\"proposal_status_for_queue_pressure\""));
    }

    #[test]
    fn no_progress_result_classifies_core_cases() {
        let executed_no_change = compute_no_progress_result(&NoProgressResultInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            outcome: Some("no_change".to_string()),
        });
        assert!(executed_no_change.is_no_progress);

        let executed_shipped = compute_no_progress_result(&NoProgressResultInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            outcome: Some("shipped".to_string()),
        });
        assert!(!executed_shipped.is_no_progress);

        let blocked = compute_no_progress_result(&NoProgressResultInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_init_gate_quality_exhausted".to_string()),
            outcome: None,
        });
        assert!(blocked.is_no_progress);
    }

    #[test]
    fn autoscale_json_no_progress_result_path_works() {
        let payload = serde_json::json!({
            "mode": "no_progress_result",
            "no_progress_result_input": {
                "event_type": "autonomy_run",
                "result": "stop_repeat_gate_no_progress",
                "outcome": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale no_progress_result");
        assert!(out.contains("\"mode\":\"no_progress_result\""));
    }

    #[test]
    fn attempt_run_event_classifies_core_cases() {
        let executed = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
        });
        assert!(executed.is_attempt);

        let blocked = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
        });
        assert!(blocked.is_attempt);

        let non_attempt = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_no_progress".to_string()),
        });
        assert!(!non_attempt.is_attempt);
    }

    #[test]
    fn autoscale_json_attempt_run_event_path_works() {
        let payload = serde_json::json!({
            "mode": "attempt_run_event",
            "attempt_run_event_input": {
                "event_type": "autonomy_run",
                "result": "stop_init_gate_quality_exhausted"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale attempt_run_event");
        assert!(out.contains("\"mode\":\"attempt_run_event\""));
    }

    #[test]
    fn safety_stop_run_event_classifies_core_cases() {
        let escalation = compute_safety_stop_run_event(&SafetyStopRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_human_escalation_pending".to_string()),
        });
        assert!(escalation.is_safety_stop);

        let capability = compute_safety_stop_run_event(&SafetyStopRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_capability_cooldown".to_string()),
        });
        assert!(capability.is_safety_stop);

        let non_safety = compute_safety_stop_run_event(&SafetyStopRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_no_progress".to_string()),
        });
        assert!(!non_safety.is_safety_stop);
    }

    #[test]
    fn autoscale_json_safety_stop_run_event_path_works() {
        let payload = serde_json::json!({
            "mode": "safety_stop_run_event",
            "safety_stop_run_event_input": {
                "event_type": "autonomy_run",
                "result": "stop_init_gate_tier1_governance"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale safety_stop_run_event");
        assert!(out.contains("\"mode\":\"safety_stop_run_event\""));
    }

    #[test]
    fn non_yield_category_classifies_policy_and_safety_and_progress() {
        let budget_hold = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("no_candidates_policy_daily_cap".to_string()),
            outcome: None,
            policy_hold: Some(true),
            hold_reason: Some("budget guard blocked".to_string()),
            route_block_reason: None,
        });
        assert_eq!(budget_hold.category, Some("budget_hold".to_string()));

        let safety = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("stop_repeat_gate_human_escalation_pending".to_string()),
            outcome: None,
            policy_hold: Some(false),
            hold_reason: None,
            route_block_reason: None,
        });
        assert_eq!(safety.category, Some("safety_stop".to_string()));

        let no_progress = compute_non_yield_category(&NonYieldCategoryInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            outcome: Some("no_change".to_string()),
            policy_hold: Some(false),
            hold_reason: None,
            route_block_reason: None,
        });
        assert_eq!(no_progress.category, Some("no_progress".to_string()));
    }

    #[test]
    fn autoscale_json_non_yield_category_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_category",
            "non_yield_category_input": {
                "event_type": "autonomy_run",
                "result": "executed",
                "outcome": "no_change",
                "policy_hold": false,
                "hold_reason": "",
                "route_block_reason": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_category");
        assert!(out.contains("\"mode\":\"non_yield_category\""));
    }

    #[test]
    fn non_yield_reason_prefers_explicit_then_falls_back() {
        let explicit = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("policy_hold".to_string()),
            hold_reason: Some("Gate Manual".to_string()),
            route_block_reason: None,
            reason: None,
            result: Some("stop_init_gate_readiness".to_string()),
            outcome: None,
        });
        assert_eq!(explicit.reason, "gate manual");

        let no_progress_executed = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("no_progress".to_string()),
            hold_reason: None,
            route_block_reason: None,
            reason: None,
            result: Some("executed".to_string()),
            outcome: Some("no_change".to_string()),
        });
        assert_eq!(no_progress_executed.reason, "executed_no_change");

        let fallback = compute_non_yield_reason(&NonYieldReasonInput {
            category: Some("safety_stop".to_string()),
            hold_reason: None,
            route_block_reason: None,
            reason: None,
            result: None,
            outcome: None,
        });
        assert_eq!(fallback.reason, "safety_stop_unknown");
    }

    #[test]
    fn autoscale_json_non_yield_reason_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_reason",
            "non_yield_reason_input": {
                "category": "no_progress",
                "result": "executed",
                "outcome": "no_change"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_reason");
        assert!(out.contains("\"mode\":\"non_yield_reason\""));
    }

    #[test]
    fn proposal_type_from_run_event_prefers_direct_then_capability_key() {
        let direct = compute_proposal_type_from_run_event(&ProposalTypeFromRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            proposal_type: Some("Unknown".to_string()),
            capability_key: Some("proposal:directive".to_string()),
        });
        assert_eq!(direct.proposal_type, "unknown".to_string());

        let derived = compute_proposal_type_from_run_event(&ProposalTypeFromRunEventInput {
            event_type: Some("autonomy_run".to_string()),
            proposal_type: Some(String::new()),
            capability_key: Some("proposal:directive".to_string()),
        });
        assert_eq!(derived.proposal_type, "directive".to_string());
    }

    #[test]
    fn autoscale_json_proposal_type_from_run_event_path_works() {
