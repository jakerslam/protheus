// FILE_SIZE_EXCEPTION: reason=Atomic test-module block generated during safe decomposition; owner=jay; expires=2026-04-12
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_pressure_scales_up() {
        let out = compute_plan(&PlanInput {
            queue_pressure: QueuePressure {
                pressure: "critical".to_string(),
                pending: 40.0,
                pending_ratio: 0.7,
            },
            min_cells: 1,
            max_cells: 5,
            current_cells: 2,
            run_interval_minutes: 15.0,
            idle_release_minutes: 120.0,
            autopause_active: false,
            last_run_minutes_ago: Some(30.0),
            last_high_pressure_minutes_ago: Some(5.0),
            trit_shadow_blocked: false,
        });
        assert_eq!(out.action, "scale_up");
        assert_eq!(out.target_cells, 5);
    }

    #[test]
    fn budget_blocked_batch_caps_to_one() {
        let out = compute_batch_max(&BatchMaxInput {
            enabled: true,
            max_batch: 6,
            daily_remaining: Some(4),
            pressure: "critical".to_string(),
            current_cells: 4,
            budget_blocked: true,
            trit_shadow_blocked: false,
        });
        assert_eq!(out.max, 1);
        assert_eq!(out.reason, "budget_blocked");
    }

    #[test]
    fn autoscale_json_path_works() {
        let payload = serde_json::json!({
            "mode": "batch_max",
            "batch_input": {
                "enabled": true,
                "max_batch": 6,
                "daily_remaining": 2,
                "pressure": "warning",
                "current_cells": 3,
                "budget_blocked": false,
                "trit_shadow_blocked": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale");
        assert!(out.contains("\"mode\":\"batch_max\""));
    }

    #[test]
    fn autoscale_json_alias_mode_routes_to_criteria_gate() {
        let payload = serde_json::json!({
            "mode": "criteria-check",
            "criteria_gate_input": {
                "min_count": 1,
                "total_count": 1,
                "contract_not_allowed_count": 0,
                "unsupported_count": 0,
                "structurally_supported_count": 1,
                "contract_violation_count": 0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria alias");
        assert!(out.contains("\"mode\":\"criteria_gate\""));
        assert!(out.contains("\"pass\":true"));
    }

    #[test]
    fn autoscale_json_unsupported_mode_reports_raw_and_normalized() {
        let payload = serde_json::json!({
            "mode": "Unknown Mode"
        })
        .to_string();
        let err = run_autoscale_json(&payload).expect_err("unsupported mode");
        assert!(err.contains("autoscale_mode_unsupported"));
        assert!(err.contains("raw=unknown mode"));
        assert!(err.contains("normalized=unknown_mode"));
    }

    #[test]
    fn autoscale_json_blank_mode_fails_closed() {
        let payload = serde_json::json!({
            "mode": "   "
        })
        .to_string();
        let err = run_autoscale_json(&payload).expect_err("blank mode");
        assert!(err.contains("autoscale_mode_missing"));
    }

    #[test]
    fn dynamic_caps_warn_downshift_works() {
        let out = compute_dynamic_caps(&DynamicCapsInput {
            enabled: true,
            base_daily_cap: 6,
            base_canary_cap: None,
            candidate_pool_size: 24,
            queue_pressure: "warning".to_string(),
            policy_hold_level: "normal".to_string(),
            policy_hold_applicable: false,
            spawn_boost_enabled: false,
            spawn_boost_active: false,
            shipped_today: 0.0,
            no_progress_streak: 0.0,
            gate_exhaustion_streak: 0.0,
            warn_factor: 0.75,
            critical_factor: 0.5,
            min_input_pool: 8,
        });
        assert!(out.low_yield);
        assert!(out.daily_runs_cap < 6);
        assert!(out.input_candidates_cap.is_some());
        assert!(out
            .reasons
            .iter()
            .any(|r| r == "downshift_queue_backlog_warning"));
    }

    #[test]
    fn token_usage_prefers_actual_metrics() {
        let out = compute_token_usage(&TokenUsageInput {
            selected_model_tokens_est: Some(180.0),
            route_budget_request_tokens_est: None,
            route_tokens_est: Some(90.0),
            fallback_est_tokens: Some(60.0),
            metrics_prompt_tokens: Some(25.0),
            metrics_input_tokens: None,
            metrics_completion_tokens: Some(15.0),
            metrics_output_tokens: None,
            metrics_total_tokens: None,
            metrics_tokens_used: None,
            metrics_source: Some("route_execute_metrics".to_string()),
        });
        assert!(out.available);
        assert_eq!(out.actual_total_tokens, Some(40.0));
        assert_eq!(out.effective_tokens, 40.0);
        assert_eq!(out.source, "route_execute_metrics");
    }

    #[test]
    fn token_usage_falls_back_to_estimate() {
        let out = compute_token_usage(&TokenUsageInput {
            selected_model_tokens_est: Some(220.0),
            route_budget_request_tokens_est: Some(210.0),
            route_tokens_est: Some(200.0),
            fallback_est_tokens: Some(180.0),
            metrics_prompt_tokens: None,
            metrics_input_tokens: None,
            metrics_completion_tokens: None,
            metrics_output_tokens: None,
            metrics_total_tokens: None,
            metrics_tokens_used: None,
            metrics_source: None,
        });
        assert!(!out.available);
        assert_eq!(out.actual_total_tokens, None);
        assert_eq!(out.effective_tokens, 220.0);
        assert_eq!(out.source, "estimated_fallback");
    }

    #[test]
    fn normalize_queue_classifies_by_thresholds() {
        let out = compute_normalize_queue(&NormalizeQueueInput {
            pressure: Some("".to_string()),
            pending: Some(46.0),
            total: Some(120.0),
            pending_ratio: None,
            warn_pending_count: 45.0,
            critical_pending_count: 80.0,
            warn_pending_ratio: 0.30,
            critical_pending_ratio: 0.45,
        });
        assert_eq!(out.pressure, "warning");
        assert_eq!(out.pending, 46.0);
        assert_eq!(out.total, 120.0);
        assert!(out.pending_ratio > 0.38 && out.pending_ratio < 0.39);
    }

    #[test]
    fn normalize_queue_respects_explicit_pressure() {
        let out = compute_normalize_queue(&NormalizeQueueInput {
            pressure: Some("critical".to_string()),
            pending: Some(1.0),
            total: Some(100.0),
            pending_ratio: Some(0.01),
            warn_pending_count: 45.0,
            critical_pending_count: 80.0,
            warn_pending_ratio: 0.30,
            critical_pending_ratio: 0.45,
        });
        assert_eq!(out.pressure, "critical");
        assert_eq!(out.pending_ratio, 0.01);
    }

    #[test]
    fn criteria_gate_fails_on_contract_or_support_gaps() {
        let out = compute_criteria_gate(&CriteriaGateInput {
            min_count: Some(2.0),
            total_count: Some(2.0),
            contract_not_allowed_count: Some(1.0),
            unsupported_count: Some(0.0),
            structurally_supported_count: None,
            contract_violation_count: Some(1.0),
        });
        assert!(!out.pass);
        assert!(out
            .reasons
            .iter()
            .any(|r| r == "criteria_contract_violation"));
        assert!(out
            .reasons
            .iter()
            .any(|r| r == "criteria_supported_count_below_min"));
    }

    #[test]
    fn criteria_gate_passes_when_counts_are_satisfied() {
        let out = compute_criteria_gate(&CriteriaGateInput {
            min_count: Some(2.0),
            total_count: Some(3.0),
            contract_not_allowed_count: Some(0.0),
            unsupported_count: Some(0.0),
            structurally_supported_count: Some(3.0),
            contract_violation_count: Some(0.0),
        });
        assert!(out.pass);
        assert!(out.reasons.is_empty());
    }

    #[test]
    fn structural_preview_criteria_failure_detects_blocking_patterns() {
        let primary =
            compute_structural_preview_criteria_failure(&StructuralPreviewCriteriaFailureInput {
                primary_failure: Some("metric_not_allowed_for_capability".to_string()),
                contract_not_allowed_count: Some(0.0),
                unsupported_count: Some(0.0),
                total_count: Some(0.0),
            });
        assert!(primary.has_failure);

        let unsupported =
            compute_structural_preview_criteria_failure(&StructuralPreviewCriteriaFailureInput {
                primary_failure: Some(String::new()),
                contract_not_allowed_count: Some(0.0),
                unsupported_count: Some(2.0),
                total_count: Some(3.0),
            });
        assert!(unsupported.has_failure);

        let pass =
            compute_structural_preview_criteria_failure(&StructuralPreviewCriteriaFailureInput {
                primary_failure: Some(String::new()),
                contract_not_allowed_count: Some(0.0),
                unsupported_count: Some(1.0),
                total_count: Some(4.0),
            });
        assert!(!pass.has_failure);
    }

    #[test]
    fn autoscale_json_structural_preview_criteria_failure_path_works() {
        let payload = serde_json::json!({
            "mode": "structural_preview_criteria_failure",
            "structural_preview_criteria_failure_input": {
                "primary_failure": "",
                "contract_not_allowed_count": 0,
                "unsupported_count": 2,
                "total_count": 3
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale structural_preview_criteria_failure");
        assert!(out.contains("\"mode\":\"structural_preview_criteria_failure\""));
    }

    #[test]
    fn policy_hold_blocks_budget_pressure() {
        let out = compute_policy_hold(&PolicyHoldInput {
            target: "route".to_string(),
            gate_decision: "ALLOW".to_string(),
            route_decision: "ALLOW".to_string(),
            needs_manual_review: false,
            executable: true,
            budget_reason: "budget guard blocked".to_string(),
            route_reason: "".to_string(),
            budget_blocked_flag: false,
            budget_global_blocked: false,
            budget_enforcement_blocked: false,
        });
        assert!(out.hold);
        assert_eq!(out.hold_scope, Some("budget".to_string()));
    }

    #[test]
    fn policy_hold_blocks_manual_non_executable_routes() {
        let out = compute_policy_hold(&PolicyHoldInput {
            target: "route".to_string(),
            gate_decision: "MANUAL".to_string(),
            route_decision: "ALLOW".to_string(),
            needs_manual_review: false,
            executable: false,
            budget_reason: "".to_string(),
            route_reason: "".to_string(),
            budget_blocked_flag: false,
            budget_global_blocked: false,
            budget_enforcement_blocked: false,
        });
        assert!(out.hold);
        assert_eq!(out.hold_scope, Some("proposal".to_string()));
        assert_eq!(out.hold_reason, Some("gate_manual".to_string()));
    }

    #[test]
    fn policy_hold_result_detects_known_policy_hold_codes() {
        let out = compute_policy_hold_result(&PolicyHoldResultInput {
            result: Some("stop_init_gate_readiness".to_string()),
        });
        assert!(out.is_policy_hold);

        let non_hold = compute_policy_hold_result(&PolicyHoldResultInput {
            result: Some("stop_init_gate_quality_exhausted".to_string()),
        });
        assert!(!non_hold.is_policy_hold);
    }

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

    #[test]
    fn autoscale_json_queue_underflow_backfill_path_works() {
        let payload = serde_json::json!({
            "mode": "queue_underflow_backfill",
            "queue_underflow_backfill_input": {
                "underflow_backfill_max": 2,
                "status": "accepted",
                "overlay_outcome": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale queue_underflow_backfill");
        assert!(out.contains("\"mode\":\"queue_underflow_backfill\""));
    }

    #[test]
    fn proposal_risk_score_prefers_explicit_then_maps_risk() {
        let explicit = compute_proposal_risk_score(&ProposalRiskScoreInput {
            explicit_risk_score: Some(61.8),
            risk: Some("low".to_string()),
        });
        assert_eq!(explicit.risk_score, 62);

        let high = compute_proposal_risk_score(&ProposalRiskScoreInput {
            explicit_risk_score: None,
            risk: Some("high".to_string()),
        });
        assert_eq!(high.risk_score, 90);
    }

    #[test]
    fn autoscale_json_proposal_risk_score_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_risk_score",
            "proposal_risk_score_input": {
                "explicit_risk_score": null,
                "risk": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_risk_score");
        assert!(out.contains("\"mode\":\"proposal_risk_score\""));
    }

    #[test]
    fn proposal_score_applies_weighted_penalties() {
        let out = compute_proposal_score(&ProposalScoreInput {
            impact_weight: 3.0,
            risk_penalty: 2.0,
            age_hours: 24.0,
            is_stub: false,
            no_change_count: 1.0,
            reverted_count: 0.0,
        });
        assert!((out.score - 1.9).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_proposal_score_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_score",
            "proposal_score_input": {
                "impact_weight": 3,
                "risk_penalty": 2,
                "age_hours": 24,
                "is_stub": false,
                "no_change_count": 1,
                "reverted_count": 0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_score");
        assert!(out.contains("\"mode\":\"proposal_score\""));
    }

    #[test]
    fn proposal_admission_preview_returns_object_only() {
        let object_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!({"allow": true, "reason": "ok"})),
        });
        assert!(object_preview.preview.is_some());

        let array_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!(["ok"])),
        });
        assert!(array_preview.preview.is_some());

        let scalar_preview = compute_proposal_admission_preview(&ProposalAdmissionPreviewInput {
            admission_preview: Some(serde_json::json!("not-an-object")),
        });
        assert!(scalar_preview.preview.is_none());
    }

    #[test]
    fn autoscale_json_proposal_admission_preview_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_admission_preview",
            "proposal_admission_preview_input": {
                "admission_preview": {
                    "allow": true,
                    "reason": "ok"
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_admission_preview");
        assert!(out.contains("\"mode\":\"proposal_admission_preview\""));
    }

    #[test]
    fn impact_weight_maps_expected_impact() {
        let high = compute_impact_weight(&ImpactWeightInput {
            expected_impact: Some("high".to_string()),
        });
        assert_eq!(high.weight, 3);
        let low = compute_impact_weight(&ImpactWeightInput {
            expected_impact: Some("low".to_string()),
        });
        assert_eq!(low.weight, 1);
    }

    #[test]
    fn autoscale_json_impact_weight_path_works() {
        let payload = serde_json::json!({
            "mode": "impact_weight",
            "impact_weight_input": {
                "expected_impact": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale impact_weight");
        assert!(out.contains("\"mode\":\"impact_weight\""));
    }

    #[test]
    fn list_proposal_files_filters_and_sorts() {
        let out = compute_list_proposal_files(&ListProposalFilesInput {
            entries: vec![
                "README.md".to_string(),
                "2026-03-02.json".to_string(),
                "2026-03-01.json".to_string(),
                "2026-03-01.jsonl".to_string(),
            ],
        });
        assert_eq!(
            out.files,
            vec!["2026-03-01.json".to_string(), "2026-03-02.json".to_string()]
        );
    }

    #[test]
    fn autoscale_json_list_proposal_files_path_works() {
        let payload = serde_json::json!({
            "mode": "list_proposal_files",
            "list_proposal_files_input": {
                "entries": ["2026-03-02.json", "bad.txt", "2026-03-01.json"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale list_proposal_files");
        assert!(out.contains("\"mode\":\"list_proposal_files\""));
        assert!(out.contains("\"files\":[\"2026-03-01.json\",\"2026-03-02.json\"]"));
    }

    #[test]
    fn risk_penalty_maps_risk_levels() {
        let high = compute_risk_penalty(&RiskPenaltyInput {
            risk: Some("high".to_string()),
        });
        assert_eq!(high.penalty, 2);
        let low = compute_risk_penalty(&RiskPenaltyInput {
            risk: Some("low".to_string()),
        });
        assert_eq!(low.penalty, 0);
    }

    #[test]
    fn autoscale_json_risk_penalty_path_works() {
        let payload = serde_json::json!({
            "mode": "risk_penalty",
            "risk_penalty_input": {
                "risk": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale risk_penalty");
        assert!(out.contains("\"mode\":\"risk_penalty\""));
    }

    #[test]
    fn estimate_tokens_maps_expected_impact() {
        let high = compute_estimate_tokens(&EstimateTokensInput {
            expected_impact: Some("high".to_string()),
        });
        assert_eq!(high.est_tokens, 1400);
        let low = compute_estimate_tokens(&EstimateTokensInput {
            expected_impact: Some("low".to_string()),
        });
        assert_eq!(low.est_tokens, 300);
    }

    #[test]
    fn autoscale_json_estimate_tokens_path_works() {
        let payload = serde_json::json!({
            "mode": "estimate_tokens",
            "estimate_tokens_input": {
                "expected_impact": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale estimate_tokens");
        assert!(out.contains("\"mode\":\"estimate_tokens\""));
    }

    #[test]
    fn proposal_remediation_depth_prefers_explicit_then_trigger() {
        let explicit = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: Some(2.4),
            trigger: Some("consecutive_failures".to_string()),
        });
        assert_eq!(explicit.depth, 2);

        let trigger = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: None,
            trigger: Some("multi_eye_transport_failure".to_string()),
        });
        assert_eq!(trigger.depth, 1);

        let none = compute_proposal_remediation_depth(&ProposalRemediationDepthInput {
            remediation_depth: None,
            trigger: Some("".to_string()),
        });
        assert_eq!(none.depth, 0);
    }

    #[test]
    fn autoscale_json_proposal_remediation_depth_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_remediation_depth",
            "proposal_remediation_depth_input": {
                "remediation_depth": null,
                "trigger": "consecutive_failures"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_remediation_depth");
        assert!(out.contains("\"mode\":\"proposal_remediation_depth\""));
        assert!(out.contains("\"depth\":1"));
    }

    #[test]
    fn proposal_dedup_key_uses_remediation_and_id_paths() {
        let remediation = compute_proposal_dedup_key(&ProposalDedupKeyInput {
            proposal_type: Some("ops_remediation".to_string()),
            source_eye_id: Some("github_release".to_string()),
            remediation_kind: Some("transport".to_string()),
            proposal_id: Some("abc-1".to_string()),
        });
        assert_eq!(
            remediation.dedup_key,
            "ops_remediation|github_release|transport"
        );

        let generic = compute_proposal_dedup_key(&ProposalDedupKeyInput {
            proposal_type: Some("feature".to_string()),
            source_eye_id: Some("unknown_eye".to_string()),
            remediation_kind: None,
            proposal_id: Some("abc-1".to_string()),
        });
        assert_eq!(generic.dedup_key, "feature|unknown_eye|abc-1");
    }

    #[test]
    fn autoscale_json_proposal_dedup_key_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_dedup_key",
            "proposal_dedup_key_input": {
                "proposal_type": "ops_remediation",
                "source_eye_id": "github_release",
                "remediation_kind": "transport",
                "proposal_id": "abc-1"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_dedup_key");
        assert!(out.contains("\"mode\":\"proposal_dedup_key\""));
        assert!(out.contains("\"dedup_key\":\"ops_remediation|github_release|transport\""));
    }

    #[test]
    fn proposal_semantic_fingerprint_builds_unique_sorted_stems() {
        let out = compute_proposal_semantic_fingerprint(&ProposalSemanticFingerprintInput {
            proposal_id: Some("p-1".to_string()),
            proposal_type: Some("ops_remediation".to_string()),
            source_eye: Some("GitHub_Release".to_string()),
            objective_id: Some("T1_Objective".to_string()),
            text_blob: Some("Rust bridge parity tests for transport fixes".to_string()),
            stopwords: vec!["for".to_string()],
            min_tokens: Some(3.0),
        });
        assert_eq!(out.proposal_id, Some("p-1".to_string()));
        assert_eq!(out.proposal_type, "ops_remediation".to_string());
        assert_eq!(out.source_eye, Some("github_release".to_string()));
        assert_eq!(out.objective_id, Some("T1_Objective".to_string()));
        assert!(out.token_stems.windows(2).all(|w| w[0] <= w[1]));
        assert!(out.token_count >= 3);
        assert!(out.eligible);
    }

    #[test]
    fn autoscale_json_proposal_semantic_fingerprint_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_semantic_fingerprint",
            "proposal_semantic_fingerprint_input": {
                "proposal_id": "p-1",
                "proposal_type": "ops_remediation",
                "source_eye": "github_release",
                "objective_id": "T1_Objective",
                "text_blob": "Rust bridge parity tests",
                "min_tokens": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_semantic_fingerprint");
        assert!(out.contains("\"mode\":\"proposal_semantic_fingerprint\""));
    }

    #[test]
    fn semantic_token_similarity_uses_jaccard_overlap() {
        let out = compute_semantic_token_similarity(&SemanticTokenSimilarityInput {
            left_tokens: vec![
                "bridge".to_string(),
                "rust".to_string(),
                "parity".to_string(),
                "rust".to_string(),
            ],
            right_tokens: vec![
                "rust".to_string(),
                "parity".to_string(),
                "tests".to_string(),
            ],
        });
        assert!(
            (out.similarity - 0.5).abs() < 1e-6,
            "similarity={}",
            out.similarity
        );

        let empty = compute_semantic_token_similarity(&SemanticTokenSimilarityInput {
            left_tokens: vec![],
            right_tokens: vec!["anything".to_string()],
        });
        assert_eq!(empty.similarity, 0.0);
    }

    #[test]
    fn autoscale_json_semantic_token_similarity_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_token_similarity",
            "semantic_token_similarity_input": {
                "left_tokens": ["rust", "bridge", "parity"],
                "right_tokens": ["parity", "tests"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_token_similarity");
        assert!(out.contains("\"mode\":\"semantic_token_similarity\""));
        assert!(out.contains("\"similarity\":0.25"));
    }

    #[test]
    fn semantic_context_comparable_requires_type_and_shared_context() {
        let pass = compute_semantic_context_comparable(&SemanticContextComparableInput {
            left_proposal_type: Some("ops_remediation".to_string()),
            right_proposal_type: Some("ops_remediation".to_string()),
            left_source_eye: Some("github_release".to_string()),
            right_source_eye: Some("github_release".to_string()),
            left_objective_id: None,
            right_objective_id: None,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(pass.comparable);

        let blocked = compute_semantic_context_comparable(&SemanticContextComparableInput {
            left_proposal_type: Some("ops_remediation".to_string()),
            right_proposal_type: Some("feature".to_string()),
            left_source_eye: Some("github_release".to_string()),
            right_source_eye: Some("github_release".to_string()),
            left_objective_id: None,
            right_objective_id: None,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(!blocked.comparable);
    }

    #[test]
    fn autoscale_json_semantic_context_comparable_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_context_comparable",
            "semantic_context_comparable_input": {
                "left_proposal_type": "ops_remediation",
                "right_proposal_type": "ops_remediation",
                "left_source_eye": "github_release",
                "right_source_eye": "github_release",
                "left_objective_id": "",
                "right_objective_id": "",
                "require_same_type": true,
                "require_shared_context": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_context_comparable");
        assert!(out.contains("\"mode\":\"semantic_context_comparable\""));
        assert!(out.contains("\"comparable\":true"));
    }

    #[test]
    fn semantic_near_duplicate_match_selects_best_eligible_candidate() {
        let out = compute_semantic_near_duplicate_match(&SemanticNearDuplicateMatchInput {
            fingerprint: SemanticNearDuplicateFingerprintInput {
                proposal_id: Some("new-1".to_string()),
                proposal_type: Some("ops_remediation".to_string()),
                source_eye: Some("github_release".to_string()),
                objective_id: Some("obj-a".to_string()),
                token_stems: vec![
                    "rust".to_string(),
                    "bridge".to_string(),
                    "parity".to_string(),
                ],
                eligible: true,
            },
            seen_fingerprints: vec![
                SemanticNearDuplicateFingerprintInput {
                    proposal_id: Some("old-1".to_string()),
                    proposal_type: Some("ops_remediation".to_string()),
                    source_eye: Some("github_release".to_string()),
                    objective_id: Some("obj-a".to_string()),
                    token_stems: vec![
                        "rust".to_string(),
                        "bridge".to_string(),
                        "tests".to_string(),
                    ],
                    eligible: true,
                },
                SemanticNearDuplicateFingerprintInput {
                    proposal_id: Some("old-2".to_string()),
                    proposal_type: Some("ops_remediation".to_string()),
                    source_eye: Some("github_release".to_string()),
                    objective_id: Some("obj-a".to_string()),
                    token_stems: vec![
                        "rust".to_string(),
                        "bridge".to_string(),
                        "parity".to_string(),
                    ],
                    eligible: true,
                },
            ],
            min_similarity: 0.5,
            require_same_type: true,
            require_shared_context: true,
        });
        assert!(out.matched);
        assert_eq!(out.proposal_id.as_deref(), Some("old-2"));
        assert!(
            (out.similarity - 1.0).abs() < 1e-6,
            "similarity={}",
            out.similarity
        );
    }

    #[test]
    fn autoscale_json_semantic_near_duplicate_match_path_works() {
        let payload = serde_json::json!({
            "mode": "semantic_near_duplicate_match",
            "semantic_near_duplicate_match_input": {
                "fingerprint": {
                    "proposal_id": "new-1",
                    "proposal_type": "ops_remediation",
                    "source_eye": "github_release",
                    "objective_id": "obj-a",
                    "token_stems": ["rust", "bridge", "parity"],
                    "eligible": true
                },
                "seen_fingerprints": [{
                    "proposal_id": "old-1",
                    "proposal_type": "ops_remediation",
                    "source_eye": "github_release",
                    "objective_id": "obj-a",
                    "token_stems": ["rust", "bridge", "tests"],
                    "eligible": true
                }],
                "min_similarity": 0.4,
                "require_same_type": true,
                "require_shared_context": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale semantic_near_duplicate_match");
        assert!(out.contains("\"mode\":\"semantic_near_duplicate_match\""));
        assert!(out.contains("\"matched\":true"));
    }

    #[test]
    fn strategy_rank_score_matches_weighted_formula() {
        let out = compute_strategy_rank_score(&StrategyRankScoreInput {
            composite_weight: 0.35,
            actionability_weight: 0.2,
            directive_fit_weight: 0.15,
            signal_quality_weight: 0.15,
            expected_value_weight: 0.1,
            value_density_weight: 0.08,
            risk_penalty_weight: 0.05,
            time_to_value_weight: 0.0,
            composite: 80.0,
            actionability: 70.0,
            directive_fit: 60.0,
            signal_quality: 75.0,
            expected_value: 55.0,
            value_density: 50.0,
            risk_penalty: 50.0,
            time_to_value: 40.0,
            non_yield_penalty: 1.5,
            collective_shadow_penalty: 0.5,
            collective_shadow_bonus: 0.2,
        });
        assert!((out.score - 67.45).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_score",
            "strategy_rank_score_input": {
                "composite_weight": 0.35,
                "actionability_weight": 0.2,
                "directive_fit_weight": 0.15,
                "signal_quality_weight": 0.15,
                "expected_value_weight": 0.1,
                "value_density_weight": 0.08,
                "risk_penalty_weight": 0.05,
                "time_to_value_weight": 0.0,
                "composite": 80,
                "actionability": 70,
                "directive_fit": 60,
                "signal_quality": 75,
                "expected_value": 55,
                "value_density": 50,
                "risk_penalty": 50,
                "time_to_value": 40,
                "non_yield_penalty": 1.5,
                "collective_shadow_penalty": 0.5,
                "collective_shadow_bonus": 0.2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_score");
        assert!(out.contains("\"mode\":\"strategy_rank_score\""));
        assert!(out.contains("\"score\":67.45"));
    }

    #[test]
    fn strategy_rank_adjusted_matches_pulse_and_objective_bonus_formula() {
        let out = compute_strategy_rank_adjusted(&StrategyRankAdjustedInput {
            base: 65.4,
            pulse_score: 82.0,
            pulse_weight: 0.25,
            objective_allocation_score: 70.0,
            base_objective_weight: 0.3,
            canary_mode: false,
        });
        assert!((out.adjusted - 93.25).abs() < 0.000001);
        assert!((out.bonus.total - 27.85).abs() < 0.000001);
        assert!((out.bonus.objective_weight - 0.105).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_rank_adjusted_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_rank_adjusted",
            "strategy_rank_adjusted_input": {
                "base": 65.4,
                "pulse_score": 82,
                "pulse_weight": 0.25,
                "objective_allocation_score": 70,
                "base_objective_weight": 0.3,
                "canary_mode": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_rank_adjusted");
        assert!(out.contains("\"mode\":\"strategy_rank_adjusted\""));
        assert!(out.contains("\"adjusted\":93.25"));
    }

    #[test]
    fn trit_shadow_rank_score_normalizes_belief_with_confidence_bonus() {
        let out = compute_trit_shadow_rank_score(&TritShadowRankScoreInput {
            score: 0.35,
            confidence: 0.6,
        });
        assert!((out.score - 73.5).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_trit_shadow_rank_score_path_works() {
        let payload = serde_json::json!({
            "mode": "trit_shadow_rank_score",
            "trit_shadow_rank_score_input": {
                "score": 0.35,
                "confidence": 0.6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale trit_shadow_rank_score");
        assert!(out.contains("\"mode\":\"trit_shadow_rank_score\""));
        assert!(out.contains("\"score\":73.5"));
    }

    #[test]
    fn strategy_circuit_cooldown_matches_error_classification() {
        let out = compute_strategy_circuit_cooldown(&StrategyCircuitCooldownInput {
            last_error_code: Some("HTTP 503".to_string()),
            last_error: None,
            http_429_cooldown_hours: 1.0,
            http_5xx_cooldown_hours: 6.0,
            dns_error_cooldown_hours: 3.0,
        });
        assert!((out.cooldown_hours - 6.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_circuit_cooldown_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_circuit_cooldown",
            "strategy_circuit_cooldown_input": {
                "last_error_code": "rate_limit_hit",
                "http_429_cooldown_hours": 2,
                "http_5xx_cooldown_hours": 8,
                "dns_error_cooldown_hours": 4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_circuit_cooldown");
        assert!(out.contains("\"mode\":\"strategy_circuit_cooldown\""));
        assert!(out.contains("\"cooldown_hours\":2.0"));
    }

    #[test]
    fn strategy_trit_shadow_adjusted_applies_bonus_blend() {
        let out = compute_strategy_trit_shadow_adjusted(&StrategyTritShadowAdjustedInput {
            base_score: 68.75,
            bonus_raw: 12.345,
            bonus_blend: 0.4,
        });
        assert!((out.bonus_applied - 4.938).abs() < 0.000001);
        assert!((out.adjusted_score - 73.688).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_strategy_trit_shadow_adjusted_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_trit_shadow_adjusted",
            "strategy_trit_shadow_adjusted_input": {
                "base_score": 68.75,
                "bonus_raw": 12.345,
                "bonus_blend": 0.4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_trit_shadow_adjusted");
        assert!(out.contains("\"mode\":\"strategy_trit_shadow_adjusted\""));
        assert!(out.contains("\"bonus_applied\":4.938"));
        assert!(out.contains("\"adjusted_score\":73.688"));
    }

    #[test]
    fn non_yield_penalty_score_applies_weighted_formula_and_clamp() {
        let out = compute_non_yield_penalty_score(&NonYieldPenaltyScoreInput {
            policy_hold_rate: 0.25,
            no_progress_rate: 0.5,
            stop_rate: 0.125,
            shipped_rate: 0.2,
            policy_hold_weight: 8.0,
            no_progress_weight: 6.0,
            stop_weight: 4.0,
            shipped_relief_weight: 3.0,
            max_penalty: 12.0,
        });
        assert!((out.penalty - 4.9).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_non_yield_penalty_score_path_works() {
        let payload = serde_json::json!({
            "mode": "non_yield_penalty_score",
            "non_yield_penalty_score_input": {
                "policy_hold_rate": 0.25,
                "no_progress_rate": 0.5,
                "stop_rate": 0.125,
                "shipped_rate": 0.2,
                "policy_hold_weight": 8,
                "no_progress_weight": 6,
                "stop_weight": 4,
                "shipped_relief_weight": 3,
                "max_penalty": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale non_yield_penalty_score");
        assert!(out.contains("\"mode\":\"non_yield_penalty_score\""));
        assert!(out.contains("\"penalty\":4.9"));
    }

    #[test]
    fn collective_shadow_adjustments_clamps_penalty_and_bonus() {
        let out = compute_collective_shadow_adjustments(&CollectiveShadowAdjustmentsInput {
            penalty_raw: 18.4,
            bonus_raw: std::f64::consts::E,
            max_penalty: 12.0,
            max_bonus: 6.0,
        });
        assert!((out.penalty - 12.0).abs() < 0.000001);
        let expected_bonus = (std::f64::consts::E * 1000.0).round() / 1000.0;
        assert!((out.bonus - expected_bonus).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_collective_shadow_adjustments_path_works() {
        let payload = serde_json::json!({
                "mode": "collective_shadow_adjustments",
                "collective_shadow_adjustments_input": {
                    "penalty_raw": 18.4,
                    "bonus_raw": std::f64::consts::E,
                    "max_penalty": 12,
                    "max_bonus": 6
                }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale collective_shadow_adjustments");
        assert!(out.contains("\"mode\":\"collective_shadow_adjustments\""));
        assert!(out.contains("\"penalty\":12.0"));
        assert!(out.contains("\"bonus\":2.718"));
    }

    #[test]
    fn strategy_trit_shadow_ranking_summary_orders_and_flags_divergence() {
        let out =
            compute_strategy_trit_shadow_ranking_summary(&StrategyTritShadowRankingSummaryInput {
                rows: vec![
                    StrategyTritShadowRankRowInput {
                        index: 0,
                        proposal_id: "a".to_string(),
                        legacy_rank: 92.0,
                        trit_rank: 71.0,
                        trit_label: "neutral".to_string(),
                        trit_confidence: 0.4,
                        trit_top_sources: vec!["x".to_string()],
                    },
                    StrategyTritShadowRankRowInput {
                        index: 1,
                        proposal_id: "b".to_string(),
                        legacy_rank: 80.0,
                        trit_rank: 95.0,
                        trit_label: "positive".to_string(),
                        trit_confidence: 0.8,
                        trit_top_sources: vec!["y".to_string()],
                    },
                ],
                selected_proposal_id: Some("a".to_string()),
                selection_mode: Some("qos_standard_legacy".to_string()),
                top_k: 3,
            });
        assert_eq!(out.legacy_top_proposal_id.as_deref(), Some("a"));
        assert_eq!(out.trit_top_proposal_id.as_deref(), Some("b"));
        assert!(out.diverged_from_legacy_top);
        assert!(out.diverged_from_selected);
        assert_eq!(
            out.top.first().map(|row| row.proposal_id.as_str()),
            Some("b")
        );
    }

    #[test]
    fn autoscale_json_strategy_trit_shadow_ranking_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_trit_shadow_ranking_summary",
            "strategy_trit_shadow_ranking_summary_input": {
                "rows": [
                    {
                        "index": 0,
                        "proposal_id": "a",
                        "legacy_rank": 92,
                        "trit_rank": 71,
                        "trit_label": "neutral",
                        "trit_confidence": 0.4,
                        "trit_top_sources": ["x"]
                    },
                    {
                        "index": 1,
                        "proposal_id": "b",
                        "legacy_rank": 80,
                        "trit_rank": 95,
                        "trit_label": "positive",
                        "trit_confidence": 0.8,
                        "trit_top_sources": ["y"]
                    }
                ],
                "selected_proposal_id": "a",
                "selection_mode": "qos_standard_legacy",
                "top_k": 3
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale strategy_trit_shadow_ranking_summary");
        assert!(out.contains("\"mode\":\"strategy_trit_shadow_ranking_summary\""));
        assert!(out.contains("\"trit_top_proposal_id\":\"b\""));
    }

    #[test]
    fn shadow_scope_matches_evaluates_scope_types() {
        let proposal_scope = compute_shadow_scope_matches(&ShadowScopeMatchesInput {
            scope_type: Some("proposal_type".to_string()),
            scope_value: Some("ops_remediation".to_string()),
            risk_levels: vec![],
            risk: Some("low".to_string()),
            proposal_type: Some("ops_remediation".to_string()),
            capability_key: Some("system_exec".to_string()),
            objective_id: Some("obj-1".to_string()),
        });
        assert!(proposal_scope.matched);

        let global_scope = compute_shadow_scope_matches(&ShadowScopeMatchesInput {
            scope_type: Some("global".to_string()),
            scope_value: None,
            risk_levels: vec!["high".to_string()],
            risk: Some("low".to_string()),
            proposal_type: None,
            capability_key: None,
            objective_id: None,
        });
        assert!(!global_scope.matched);
    }

    #[test]
    fn autoscale_json_shadow_scope_matches_path_works() {
        let payload = serde_json::json!({
            "mode": "shadow_scope_matches",
            "shadow_scope_matches_input": {
                "scope_type": "capability_key",
                "scope_value": "system_exec",
                "risk_levels": [],
                "risk": "medium",
                "proposal_type": "ops_remediation",
                "capability_key": "system_exec",
                "objective_id": "obj-1"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale shadow_scope_matches");
        assert!(out.contains("\"mode\":\"shadow_scope_matches\""));
        assert!(out.contains("\"matched\":true"));
    }

    #[test]
    fn collective_shadow_aggregate_computes_confidence_and_weighted_totals() {
        let out = compute_collective_shadow_aggregate(&CollectiveShadowAggregateInput {
            entries: vec![
                CollectiveShadowAggregateEntryInput {
                    kind: Some("avoid".to_string()),
                    confidence: 0.8,
                    score_impact: 10.0,
                },
                CollectiveShadowAggregateEntryInput {
                    kind: Some("reinforce".to_string()),
                    confidence: 0.5,
                    score_impact: 6.0,
                },
            ],
        });
        assert_eq!(out.matches, 2);
        assert!((out.confidence_avg - 0.65).abs() < 0.000001);
        assert!((out.penalty_raw - 8.0).abs() < 0.000001);
        assert!((out.bonus_raw - 3.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_collective_shadow_aggregate_path_works() {
        let payload = serde_json::json!({
            "mode": "collective_shadow_aggregate",
            "collective_shadow_aggregate_input": {
                "entries": [
                    { "kind": "avoid", "confidence": 0.8, "score_impact": 10 },
                    { "kind": "reinforce", "confidence": 0.5, "score_impact": 6 }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale collective_shadow_aggregate");
        assert!(out.contains("\"mode\":\"collective_shadow_aggregate\""));
        assert!(out.contains("\"penalty_raw\":8.0"));
        assert!(out.contains("\"bonus_raw\":3.0"));
    }

    #[test]
    fn expected_value_signal_applies_currency_rank_blending() {
        let out = compute_expected_value_signal(&ExpectedValueSignalInput {
            explicit_score: None,
            expected_value_usd: None,
            oracle_priority_score: Some(80.0),
            impact_weight: 2.0,
            selected_currency: Some("revenue".to_string()),
            currency_multiplier: 1.25,
            matched_first_sentence_contains_selected: true,
            currency_ranking_enabled: true,
            oracle_applies: true,
            oracle_pass: true,
            rank_blend: 0.35,
            bonus_cap: 12.0,
        });
        assert_eq!(out.source, "value_oracle_priority_score");
        assert_eq!(out.base_score, 80.0);
        assert_eq!(out.currency_adjusted_score, Some(100.0));
        assert_eq!(out.score, 89.0);
        assert_eq!(out.currency_delta, 9.0);
    }

    #[test]
    fn autoscale_json_expected_value_signal_path_works() {
        let payload = serde_json::json!({
            "mode": "expected_value_signal",
            "expected_value_signal_input": {
                "explicit_score": 42,
                "expected_value_usd": null,
                "oracle_priority_score": null,
                "impact_weight": 2.0,
                "selected_currency": "revenue",
                "currency_multiplier": 1.25,
                "matched_first_sentence_contains_selected": false,
                "currency_ranking_enabled": true,
                "oracle_applies": true,
                "oracle_pass": true,
                "rank_blend": 0.35,
                "bonus_cap": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale expected_value_signal");
        assert!(out.contains("\"mode\":\"expected_value_signal\""));
        assert!(out.contains("\"source\":\"expected_value_score\""));
        assert!(out.contains("\"score\":42.0"));
    }

    #[test]
    fn value_signal_score_matches_weighted_formula() {
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

    #[test]
    fn normalize_spaces_matches_ts_semantics() {
        let out = compute_normalize_spaces(&NormalizeSpacesInput {
            text: Some("  one\t two\nthree   ".to_string()),
        });
        assert_eq!(out.normalized, "one two three");
    }

    #[test]
    fn autoscale_json_normalize_spaces_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_spaces",
            "normalize_spaces_input": {
                "text": "  one\t two\nthree   "
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_spaces");
        assert!(out.contains("\"mode\":\"normalize_spaces\""));
        assert!(out.contains("\"normalized\":\"one two three\""));
    }

    #[test]
    fn parse_lower_list_matches_ts_semantics() {
        let from_list = compute_parse_lower_list(&ParseLowerListInput {
            list: vec![" A ".to_string(), "b".to_string(), "".to_string()],
            csv: Some("x,y".to_string()),
        });
        assert_eq!(from_list.items, vec!["a".to_string(), "b".to_string()]);

        let from_csv = compute_parse_lower_list(&ParseLowerListInput {
            list: vec![],
            csv: Some(" A, B ,,C ".to_string()),
        });
        assert_eq!(
            from_csv.items,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn autoscale_json_parse_lower_list_path_works() {
        let payload = serde_json::json!({
            "mode": "parse_lower_list",
            "parse_lower_list_input": {
                "list": [],
                "csv": "A, B ,, C"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale parse_lower_list");
        assert!(out.contains("\"mode\":\"parse_lower_list\""));
        assert!(out.contains("\"items\":[\"a\",\"b\",\"c\"]"));
    }

    #[test]
    fn canary_failed_checks_allowed_matches_subset_rules() {
        let allowed = compute_canary_failed_checks_allowed(&CanaryFailedChecksAllowedInput {
            failed_checks: vec!["lint".to_string(), "format".to_string()],
            allowed_checks: vec![
                "lint".to_string(),
                "format".to_string(),
                "typecheck".to_string(),
            ],
        });
        assert!(allowed.allowed);

        let blocked = compute_canary_failed_checks_allowed(&CanaryFailedChecksAllowedInput {
            failed_checks: vec!["lint".to_string(), "security".to_string()],
            allowed_checks: vec!["lint".to_string(), "format".to_string()],
        });
        assert!(!blocked.allowed);
    }

    #[test]
    fn autoscale_json_canary_failed_checks_allowed_path_works() {
        let payload = serde_json::json!({
            "mode": "canary_failed_checks_allowed",
            "canary_failed_checks_allowed_input": {
                "failed_checks": ["lint", "format"],
                "allowed_checks": ["lint", "format", "typecheck"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale canary_failed_checks_allowed");
        assert!(out.contains("\"mode\":\"canary_failed_checks_allowed\""));
        assert!(out.contains("\"allowed\":true"));
    }

    #[test]
    fn proposal_text_blob_matches_join_and_normalization() {
        let out = compute_proposal_text_blob(&ProposalTextBlobInput {
            title: Some("Fix Drift".to_string()),
            summary: Some("Improve safety".to_string()),
            suggested_next_command: Some("run checks".to_string()),
            suggested_command: None,
            notes: Some(" urgent ".to_string()),
            evidence: vec![ProposalTextBlobEvidenceEntryInput {
                evidence_ref: Some("ref://a".to_string()),
                path: Some("docs/client/a.md".to_string()),
                title: Some("Doc A".to_string()),
            }],
        });
        assert_eq!(
            out.blob,
            "fix drift | improve safety | run checks | urgent | ref://a | docs/client/a.md | doc a"
        );
    }

    #[test]
    fn autoscale_json_proposal_text_blob_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_text_blob",
            "proposal_text_blob_input": {
                "title": "Fix Drift",
                "summary": "Improve safety",
                "suggested_next_command": "run checks",
                "notes": "urgent",
                "evidence": [
                    {
                        "evidence_ref": "ref://a",
                        "path": "docs/client/a.md",
                        "title": "Doc A"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_text_blob");
        assert!(out.contains("\"mode\":\"proposal_text_blob\""));
        assert!(out.contains("\"blob\":\"fix drift | improve safety | run checks | urgent | ref://a | docs/client/a.md | doc a\""));
    }

    #[test]
    fn percent_mentions_from_text_matches_extraction_rules() {
        let out = compute_percent_mentions_from_text(&PercentMentionsFromTextInput {
            text: Some("improve by 12.5% then -2% then 140%".to_string()),
        });
        assert_eq!(out.values, vec![12.5, 100.0]);
    }

    #[test]
    fn autoscale_json_percent_mentions_from_text_path_works() {
        let payload = serde_json::json!({
            "mode": "percent_mentions_from_text",
            "percent_mentions_from_text_input": {
                "text": "gain 10% and 25%"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale percent_mentions_from_text");
        assert!(out.contains("\"mode\":\"percent_mentions_from_text\""));
        assert!(out.contains("\"values\":[10.0,25.0]"));
    }

    #[test]
    fn optimization_min_delta_percent_respects_mode() {
        let high = compute_optimization_min_delta_percent(&OptimizationMinDeltaPercentInput {
            high_accuracy_mode: true,
            high_accuracy_value: 3.5,
            base_value: 8.0,
        });
        assert!((high.min_delta_percent - 3.5).abs() < 0.000001);

        let normal = compute_optimization_min_delta_percent(&OptimizationMinDeltaPercentInput {
            high_accuracy_mode: false,
            high_accuracy_value: 3.5,
            base_value: 8.0,
        });
        assert!((normal.min_delta_percent - 8.0).abs() < 0.000001);
    }

    #[test]
    fn autoscale_json_optimization_min_delta_percent_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_min_delta_percent",
            "optimization_min_delta_percent_input": {
                "high_accuracy_mode": true,
                "high_accuracy_value": 3.5,
                "base_value": 8.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_min_delta_percent");
        assert!(out.contains("\"mode\":\"optimization_min_delta_percent\""));
        assert!(out.contains("\"min_delta_percent\":3.5"));
    }

    #[test]
    fn source_eye_ref_prefers_meta_then_evidence_then_unknown() {
        let meta = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: Some("primary".to_string()),
            first_evidence_ref: Some("eye:secondary".to_string()),
        });
        assert_eq!(meta.eye_ref, "eye:primary");

        let evidence = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: None,
            first_evidence_ref: Some("eye:secondary".to_string()),
        });
        assert_eq!(evidence.eye_ref, "eye:secondary");

        let unknown = compute_source_eye_ref(&SourceEyeRefInput {
            meta_source_eye: None,
            first_evidence_ref: Some("ref://other".to_string()),
        });
        assert_eq!(unknown.eye_ref, "eye:unknown_eye");
    }

    #[test]
    fn autoscale_json_source_eye_ref_path_works() {
        let payload = serde_json::json!({
            "mode": "source_eye_ref",
            "source_eye_ref_input": {
                "meta_source_eye": "market",
                "first_evidence_ref": "eye:other"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale source_eye_ref");
        assert!(out.contains("\"mode\":\"source_eye_ref\""));
        assert!(out.contains("\"eye_ref\":\"eye:market\""));
    }

    #[test]
    fn normalized_risk_only_allows_expected_levels() {
        let high = compute_normalized_risk(&NormalizedRiskInput {
            risk: Some("HIGH".to_string()),
        });
        assert_eq!(high.risk, "high");

        let fallback = compute_normalized_risk(&NormalizedRiskInput {
            risk: Some("critical".to_string()),
        });
        assert_eq!(fallback.risk, "low");
    }

    #[test]
    fn autoscale_json_normalized_risk_path_works() {
        let payload = serde_json::json!({
            "mode": "normalized_risk",
            "normalized_risk_input": {
                "risk": "medium"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalized_risk");
        assert!(out.contains("\"mode\":\"normalized_risk\""));
        assert!(out.contains("\"risk\":\"medium\""));
    }

    #[test]
    fn parse_iso_ts_returns_timestamp_when_valid() {
        let valid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("2026-03-01T00:00:00.000Z".to_string()),
        });
        assert!(valid.timestamp_ms.is_some());

        let invalid = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: Some("not-a-date".to_string()),
        });
        assert!(invalid.timestamp_ms.is_none());
    }

    #[test]
    fn autoscale_json_parse_iso_ts_path_works() {
        let payload = serde_json::json!({
            "mode": "parse_iso_ts",
            "parse_iso_ts_input": {
                "ts": "2026-03-01T00:00:00.000Z"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale parse_iso_ts");
        assert!(out.contains("\"mode\":\"parse_iso_ts\""));
        assert!(out.contains("\"timestamp_ms\":"));
    }

    #[test]
    fn extract_objective_id_token_matches_expected_patterns() {
        let direct = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("T12_build_router".to_string()),
        });
        assert_eq!(direct.objective_id.as_deref(), Some("T12_build_router"));

        let embedded = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("objective: T8_fix_drift soon".to_string()),
        });
        assert_eq!(embedded.objective_id.as_deref(), Some("T8_fix_drift"));

        let none = compute_extract_objective_id_token(&ExtractObjectiveIdTokenInput {
            value: Some("no token".to_string()),
        });
        assert!(none.objective_id.is_none());
    }

    #[test]
    fn autoscale_json_extract_objective_id_token_path_works() {
        let payload = serde_json::json!({
            "mode": "extract_objective_id_token",
            "extract_objective_id_token_input": {
                "value": "objective: T8_fix_drift soon"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale extract_objective_id_token");
        assert!(out.contains("\"mode\":\"extract_objective_id_token\""));
        assert!(out.contains("\"objective_id\":\"T8_fix_drift\""));
    }

    #[test]
    fn autoscale_json_value_density_score_path_works() {
        let payload = serde_json::json!({
            "mode": "value_density_score",
            "value_density_score_input": {
                "expected_value": 40,
                "est_tokens": 1000
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale value_density_score");
        assert!(out.contains("\"mode\":\"value_density_score\""));
    }

    #[test]
    fn execution_reserve_snapshot_applies_reserve_math() {
        let out = compute_execution_reserve_snapshot(&ExecutionReserveSnapshotInput {
            cap: 1000.0,
            used: 950.0,
            reserve_enabled: true,
            reserve_ratio: 0.12,
            reserve_min_tokens: 600.0,
        });
        assert!(out.enabled);
        assert_eq!(out.reserve_tokens, 600.0);
        assert_eq!(out.reserve_remaining, 50.0);
    }

    #[test]
    fn autoscale_json_execution_reserve_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "execution_reserve_snapshot",
            "execution_reserve_snapshot_input": {
                "cap": 1000,
                "used": 950,
                "reserve_enabled": true,
                "reserve_ratio": 0.12,
                "reserve_min_tokens": 600
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale execution_reserve_snapshot");
        assert!(out.contains("\"mode\":\"execution_reserve_snapshot\""));
    }

    #[test]
    fn budget_pacing_gate_blocks_high_token_low_value_when_tight() {
        let out = compute_budget_pacing_gate(&BudgetPacingGateInput {
            est_tokens: 1800.0,
            value_signal_score: 45.0,
            risk: Some("medium".to_string()),
            snapshot_tight: true,
            snapshot_autopause_active: false,
            snapshot_remaining_ratio: 0.18,
            snapshot_pressure: Some("hard".to_string()),
            execution_floor_deficit: false,
            execution_reserve_enabled: true,
            execution_reserve_remaining: 200.0,
            execution_reserve_min_value_signal: 70.0,
            budget_pacing_enabled: true,
            min_remaining_ratio: 0.2,
            high_token_threshold: 1200.0,
            min_value_signal_score: 60.0,
        });
        assert!(!out.pass);
        assert_eq!(
            out.reason.as_deref(),
            Some("budget_pacing_high_token_low_value")
        );
    }

    #[test]
    fn autoscale_json_budget_pacing_gate_path_works() {
        let payload = serde_json::json!({
            "mode": "budget_pacing_gate",
            "budget_pacing_gate_input": {
                "est_tokens": 300,
                "value_signal_score": 80,
                "risk": "low",
                "snapshot_tight": true,
                "snapshot_autopause_active": false,
                "snapshot_remaining_ratio": 0.12,
                "snapshot_pressure": "warn",
                "execution_floor_deficit": true,
                "execution_reserve_enabled": true,
                "execution_reserve_remaining": 500,
                "execution_reserve_min_value_signal": 70,
                "budget_pacing_enabled": true,
                "min_remaining_ratio": 0.2,
                "high_token_threshold": 1200,
                "min_value_signal_score": 60
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale budget_pacing_gate");
        assert!(out.contains("\"mode\":\"budget_pacing_gate\""));
        assert!(out.contains("\"pass\":true"));
    }

    #[test]
    fn capability_cap_prefers_primary_then_aliases() {
        let out = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([
                ("proposal:ops_remediation".to_string(), 4.2),
                ("proposal:feature".to_string(), 2.0),
            ]),
            primary_key: Some("proposal:ops_remediation".to_string()),
            aliases: vec!["proposal:feature".to_string()],
        });
        assert_eq!(out.cap, Some(4));

        let alias = compute_capability_cap(&CapabilityCapInput {
            caps: std::collections::BTreeMap::from([("alias:key".to_string(), 3.0)]),
            primary_key: Some("missing:key".to_string()),
            aliases: vec!["alias:key".to_string()],
        });
        assert_eq!(alias.cap, Some(3));
    }

    #[test]
    fn autoscale_json_capability_cap_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_cap",
            "capability_cap_input": {
                "caps": {
                    "proposal:ops_remediation": 5
                },
                "primary_key": "proposal:ops_remediation",
                "aliases": ["proposal:feature"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_cap");
        assert!(out.contains("\"mode\":\"capability_cap\""));
        assert!(out.contains("\"cap\":5"));
    }

    #[test]
    fn estimate_tokens_for_candidate_prefers_direct_then_route_then_fallback() {
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
        assert_eq!(explicit.reason, "gate_manual");

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
        let payload = serde_json::json!({
            "mode": "proposal_type_from_run_event",
            "proposal_type_from_run_event_input": {
                "event_type": "autonomy_run",
                "proposal_type": "",
                "capability_key": "proposal:unknown"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_type_from_run_event");
        assert!(out.contains("\"mode\":\"proposal_type_from_run_event\""));
    }

    #[test]
    fn run_event_objective_id_uses_truthy_priority_then_sanitizes() {
        let from_objective = compute_run_event_objective_id(&RunEventObjectiveIdInput {
            directive_pulse_present: Some(false),
            directive_pulse_objective_id: Some(String::new()),
            objective_id_present: Some(true),
            objective_id: Some("T1_alpha".to_string()),
            objective_binding_present: Some(true),
            objective_binding_objective_id: Some("T1_beta".to_string()),
            top_escalation_present: Some(true),
            top_escalation_objective_id: Some("T1_gamma".to_string()),
        });
        assert_eq!(from_objective.objective_id, "T1_alpha".to_string());

        let blocked_by_truthy_invalid = compute_run_event_objective_id(&RunEventObjectiveIdInput {
            directive_pulse_present: Some(true),
            directive_pulse_objective_id: Some("   ".to_string()),
            objective_id_present: Some(true),
            objective_id: Some("T1_valid".to_string()),
            objective_binding_present: Some(false),
            objective_binding_objective_id: Some(String::new()),
            top_escalation_present: Some(false),
            top_escalation_objective_id: Some(String::new()),
        });
        assert_eq!(blocked_by_truthy_invalid.objective_id, String::new());
    }

    #[test]
    fn autoscale_json_run_event_objective_id_path_works() {
        let payload = serde_json::json!({
            "mode": "run_event_objective_id",
            "run_event_objective_id_input": {
                "directive_pulse_present": false,
                "directive_pulse_objective_id": "",
                "objective_id_present": true,
                "objective_id": "T1_alpha",
                "objective_binding_present": false,
                "objective_binding_objective_id": "",
                "top_escalation_present": false,
                "top_escalation_objective_id": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale run_event_objective_id");
        assert!(out.contains("\"mode\":\"run_event_objective_id\""));
    }

    #[test]
    fn run_event_proposal_id_uses_truthy_priority_then_normalizes_spaces() {
        let from_direct = compute_run_event_proposal_id(&RunEventProposalIdInput {
            proposal_id_present: Some(true),
            proposal_id: Some("  p-001  ".to_string()),
            selected_proposal_id_present: Some(true),
            selected_proposal_id: Some("p-002".to_string()),
            top_escalation_present: Some(true),
            top_escalation_proposal_id: Some("p-003".to_string()),
        });
        assert_eq!(from_direct.proposal_id, "p-001".to_string());

        let from_selected = compute_run_event_proposal_id(&RunEventProposalIdInput {
            proposal_id_present: Some(false),
            proposal_id: Some(String::new()),
            selected_proposal_id_present: Some(true),
            selected_proposal_id: Some(" selected   proposal ".to_string()),
            top_escalation_present: Some(true),
            top_escalation_proposal_id: Some("p-003".to_string()),
        });
        assert_eq!(from_selected.proposal_id, "selected proposal".to_string());
    }

    #[test]
    fn autoscale_json_run_event_proposal_id_path_works() {
        let payload = serde_json::json!({
            "mode": "run_event_proposal_id",
            "run_event_proposal_id_input": {
                "proposal_id_present": false,
                "proposal_id": "",
                "selected_proposal_id_present": true,
                "selected_proposal_id": "p-009",
                "top_escalation_present": false,
                "top_escalation_proposal_id": ""
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale run_event_proposal_id");
        assert!(out.contains("\"mode\":\"run_event_proposal_id\""));
    }

    #[test]
    fn capacity_counted_attempt_event_classifies_expected_cases() {
        let executed = compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some("executed".to_string()),
            policy_hold: Some(false),
            proposal_id: Some(String::new()),
        });
        assert!(executed.capacity_counted);

        let policy_hold =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_init_gate_readiness".to_string()),
                policy_hold: Some(false),
                proposal_id: Some("p-001".to_string()),
            });
        assert!(!policy_hold.capacity_counted);

        let attempt_with_proposal =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                policy_hold: Some(false),
                proposal_id: Some("p-001".to_string()),
            });
        assert!(attempt_with_proposal.capacity_counted);

        let attempt_without_proposal =
            compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
                event_type: Some("autonomy_run".to_string()),
                result: Some("stop_repeat_gate_candidate_exhausted".to_string()),
                policy_hold: Some(false),
                proposal_id: Some(String::new()),
            });
        assert!(!attempt_without_proposal.capacity_counted);
    }

    #[test]
    fn autoscale_json_capacity_counted_attempt_event_path_works() {
        let payload = serde_json::json!({
            "mode": "capacity_counted_attempt_event",
            "capacity_counted_attempt_event_input": {
                "event_type": "autonomy_run",
                "result": "stop_repeat_gate_candidate_exhausted",
                "policy_hold": false,
                "proposal_id": "p-001"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capacity_counted_attempt_event");
        assert!(out.contains("\"mode\":\"capacity_counted_attempt_event\""));
    }

    #[test]
    fn repeat_gate_anchor_builds_binding_only_with_objective_id() {
        let out = compute_repeat_gate_anchor(&RepeatGateAnchorInput {
            proposal_id: Some(" p-001 ".to_string()),
            objective_id: Some("T1_alpha".to_string()),
            objective_binding_present: Some(true),
            objective_binding_pass: Some(false),
            objective_binding_required: Some(true),
            objective_binding_source: Some("".to_string()),
            objective_binding_valid: Some(false),
        });
        assert_eq!(out.proposal_id, Some("p-001".to_string()));
        assert_eq!(out.objective_id, Some("T1_alpha".to_string()));
        assert!(out.objective_binding.is_some());
        let binding = out.objective_binding.expect("binding");
        assert!(!binding.pass);
        assert!(binding.required);
        assert_eq!(binding.source, "repeat_gate_anchor".to_string());
        assert!(!binding.valid);
    }

    #[test]
    fn autoscale_json_repeat_gate_anchor_path_works() {
        let payload = serde_json::json!({
            "mode": "repeat_gate_anchor",
            "repeat_gate_anchor_input": {
                "proposal_id": "p-002",
                "objective_id": "T1_alpha",
                "objective_binding_present": true,
                "objective_binding_pass": true,
                "objective_binding_required": false,
                "objective_binding_source": "repeat_gate_anchor",
                "objective_binding_valid": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale repeat_gate_anchor");
        assert!(out.contains("\"mode\":\"repeat_gate_anchor\""));
    }

    #[test]
    fn route_execution_policy_hold_maps_summary_fields() {
        let out = compute_route_execution_policy_hold(&RouteExecutionPolicyHoldInput {
            target: Some("route".to_string()),
            gate_decision: Some("allow".to_string()),
            route_decision_raw: None,
            decision: Some("manual".to_string()),
            needs_manual_review: Some(false),
            executable: Some(false),
            budget_block_reason: None,
            budget_enforcement_reason: None,
            budget_global_reason: None,
            summary_reason: Some("manual route".to_string()),
            route_reason: None,
            budget_blocked: Some(false),
            budget_global_blocked: Some(false),
            budget_enforcement_blocked: Some(false),
        });
        assert!(out.hold);
        assert_eq!(out.hold_scope, Some("proposal".to_string()));
        assert_eq!(out.hold_reason, Some("gate_manual".to_string()));
    }

    #[test]
    fn autoscale_json_route_execution_policy_hold_path_works() {
        let payload = serde_json::json!({
            "mode": "route_execution_policy_hold",
            "route_execution_policy_hold_input": {
                "target": "route",
                "gate_decision": "ALLOW",
                "decision": "ALLOW",
                "needs_manual_review": false,
                "executable": true,
                "budget_block_reason": "budget guard blocked",
                "budget_blocked": false,
                "budget_global_blocked": false,
                "budget_enforcement_blocked": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale route_execution_policy_hold");
        assert!(out.contains("\"mode\":\"route_execution_policy_hold\""));
    }

    #[test]
    fn policy_hold_pressure_classifies_hard_when_rate_crosses_threshold() {
        let out = compute_policy_hold_pressure(&PolicyHoldPressureInput {
            events: vec![
                PolicyHoldPressureEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("no_candidates_policy_daily_cap".to_string()),
                    policy_hold: Some(true),
                    ts_ms: Some(1_000_000.0),
                },
                PolicyHoldPressureEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_init_gate_budget_autopause".to_string()),
                    policy_hold: Some(true),
                    ts_ms: Some(1_100_000.0),
                },
                PolicyHoldPressureEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    policy_hold: Some(false),
                    ts_ms: Some(1_200_000.0),
                },
            ],
            window_hours: Some(24.0),
            min_samples: Some(2.0),
            now_ms: Some(1_500_000.0),
            warn_rate: Some(0.25),
            hard_rate: Some(0.4),
        });
        assert!(out.applicable);
        assert_eq!(out.samples, 3);
        assert_eq!(out.policy_holds, 2);
        assert_eq!(out.level, "hard");
        assert!(out.rate >= 0.66 && out.rate <= 0.667);
    }

    #[test]
    fn autoscale_json_policy_hold_pressure_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_pressure",
            "policy_hold_pressure_input": {
                "events": [
                    { "event_type": "autonomy_run", "result": "no_candidates_policy_daily_cap", "policy_hold": true, "ts_ms": 1100000.0 },
                    { "event_type": "autonomy_run", "result": "executed", "policy_hold": false, "ts_ms": 1200000.0 }
                ],
                "window_hours": 24,
                "min_samples": 1,
                "now_ms": 1500000,
                "warn_rate": 0.25,
                "hard_rate": 0.4
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_pressure");
        assert!(out.contains("\"mode\":\"policy_hold_pressure\""));
    }

    #[test]
    fn policy_hold_pattern_detects_repeat_reason() {
        let out = compute_policy_hold_pattern(&PolicyHoldPatternInput {
            events: vec![
                PolicyHoldPatternEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_init_gate_readiness".to_string()),
                    objective_id: Some("T1_alpha".to_string()),
                    hold_reason: Some("gate_manual".to_string()),
                    route_block_reason: None,
                    policy_hold: Some(true),
                    ts_ms: Some(1_100_000.0),
                },
                PolicyHoldPatternEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_init_gate_readiness".to_string()),
                    objective_id: Some("T1_alpha".to_string()),
                    hold_reason: Some("gate_manual".to_string()),
                    route_block_reason: None,
                    policy_hold: Some(true),
                    ts_ms: Some(1_200_000.0),
                },
                PolicyHoldPatternEventInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    objective_id: Some("T1_alpha".to_string()),
                    hold_reason: None,
                    route_block_reason: None,
                    policy_hold: Some(false),
                    ts_ms: Some(1_300_000.0),
                },
            ],
            objective_id: Some("T1_alpha".to_string()),
            window_hours: Some(24.0),
            repeat_threshold: Some(2.0),
            now_ms: Some(1_500_000.0),
        });
        assert_eq!(out.total_holds, 2);
        assert_eq!(out.top_reason, Some("gate_manual".to_string()));
        assert_eq!(out.top_count, 2);
        assert!(out.should_dampen);
    }

    #[test]
    fn autoscale_json_policy_hold_pattern_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_pattern",
            "policy_hold_pattern_input": {
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "stop_init_gate_readiness",
                        "objective_id": "T1_alpha",
                        "hold_reason": "gate_manual",
                        "policy_hold": true,
                        "ts_ms": 1200000.0
                    }
                ],
                "objective_id": "T1_alpha",
                "window_hours": 24,
                "repeat_threshold": 2,
                "now_ms": 1500000
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_pattern");
        assert!(out.contains("\"mode\":\"policy_hold_pattern\""));
    }

    #[test]
    fn policy_hold_latest_event_prefers_last_policy_hold_run() {
        let out = compute_policy_hold_latest_event(&PolicyHoldLatestEventInput {
            events: vec![
                PolicyHoldLatestEventEntryInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    policy_hold: Some(false),
                    ts_ms: Some(1_000_000.0),
                    ts: Some("2026-03-01T00:00:00.000Z".to_string()),
                    hold_reason: None,
                    route_block_reason: None,
                },
                PolicyHoldLatestEventEntryInput {
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("stop_init_gate_readiness".to_string()),
                    policy_hold: Some(false),
                    ts_ms: Some(1_100_000.0),
                    ts: Some("2026-03-01T00:01:00.000Z".to_string()),
                    hold_reason: Some("gate_manual".to_string()),
                    route_block_reason: None,
                },
            ],
        });
        assert!(out.found);
        assert_eq!(out.result, Some("stop_init_gate_readiness".to_string()));
        assert_eq!(out.ts, Some("2026-03-01T00:01:00.000Z".to_string()));
        assert_eq!(out.hold_reason, Some("gate_manual".to_string()));
    }

    #[test]
    fn autoscale_json_policy_hold_latest_event_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_latest_event",
            "policy_hold_latest_event_input": {
                "events": [
                    { "event_type": "autonomy_run", "result": "executed", "policy_hold": false, "ts_ms": 1000000.0, "ts": "2026-03-01T00:00:00.000Z" },
                    { "event_type": "autonomy_run", "result": "stop_init_gate_budget_autopause", "policy_hold": false, "ts_ms": 1100000.0, "ts": "2026-03-01T00:01:00.000Z" }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_latest_event");
        assert!(out.contains("\"mode\":\"policy_hold_latest_event\""));
    }

    #[test]
    fn policy_hold_cooldown_escalates_for_cap_until_next_day() {
        let out = compute_policy_hold_cooldown(&PolicyHoldCooldownInput {
            base_minutes: Some(15.0),
            pressure_level: Some("warn".to_string()),
            pressure_applicable: Some(true),
            last_result: Some("no_candidates_policy_daily_cap".to_string()),
            now_ms: Some(1_700_000_000_000.0),
            cooldown_warn_minutes: Some(30.0),
            cooldown_hard_minutes: Some(60.0),
            cooldown_cap_minutes: Some(180.0),
            cooldown_manual_review_minutes: Some(90.0),
            cooldown_unchanged_state_minutes: Some(90.0),
            readiness_retry_minutes: Some(120.0),
            until_next_day_caps: Some(true),
        });
        assert!(out.cooldown_minutes >= 30);
    }

    #[test]
    fn autoscale_json_policy_hold_cooldown_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_cooldown",
            "policy_hold_cooldown_input": {
                "base_minutes": 15,
                "pressure_level": "hard",
                "pressure_applicable": true,
                "last_result": "stop_init_gate_readiness",
                "now_ms": 1700000000000i64,
                "cooldown_warn_minutes": 30,
                "cooldown_hard_minutes": 60,
                "cooldown_cap_minutes": 180,
                "cooldown_manual_review_minutes": 90,
                "cooldown_unchanged_state_minutes": 90,
                "readiness_retry_minutes": 120,
                "until_next_day_caps": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_cooldown");
        assert!(out.contains("\"mode\":\"policy_hold_cooldown\""));
    }

    #[test]
    fn receipt_verdict_reverts_when_exec_fails() {
        let out = compute_receipt_verdict(&ReceiptVerdictInput {
            decision: "ACTUATE".to_string(),
            exec_ok: false,
            postconditions_ok: true,
            dod_passed: true,
            success_criteria_required: true,
            success_criteria_passed: true,
            queue_outcome_logged: true,
            route_attestation_status: "ok".to_string(),
            route_attestation_expected_model: "gpt-5".to_string(),
            success_criteria_primary_failure: None,
        });
        assert_eq!(out.exec_check_name, "actuation_execute_ok");
        assert_eq!(out.outcome, "reverted");
        assert!(!out.passed);
        assert!(out.failed.iter().any(|f| f == "actuation_execute_ok"));
    }

    #[test]
    fn receipt_verdict_uses_criteria_primary_failure_when_present() {
        let out = compute_receipt_verdict(&ReceiptVerdictInput {
            decision: "ROUTE".to_string(),
            exec_ok: true,
            postconditions_ok: true,
            dod_passed: true,
            success_criteria_required: true,
            success_criteria_passed: false,
            queue_outcome_logged: true,
            route_attestation_status: "ok".to_string(),
            route_attestation_expected_model: "gpt-5".to_string(),
            success_criteria_primary_failure: Some("insufficient_supported_metrics".to_string()),
        });
        assert_eq!(out.outcome, "no_change");
        assert_eq!(
            out.primary_failure,
            Some("insufficient_supported_metrics".to_string())
        );
    }

    #[test]
    fn build_overlay_keeps_latest_decision_and_outcome() {
        let out = compute_build_overlay(&BuildOverlayInput {
            events: vec![
                BuildOverlayEventInput {
                    proposal_id: Some("p-1".to_string()),
                    event_type: Some("decision".to_string()),
                    decision: Some("accept".to_string()),
                    ts: Some("2026-03-04T00:00:00.000Z".to_string()),
                    reason: Some("first".to_string()),
                    outcome: None,
                    evidence_ref: None,
                },
                BuildOverlayEventInput {
                    proposal_id: Some("p-1".to_string()),
                    event_type: Some("outcome".to_string()),
                    decision: None,
                    ts: Some("2026-03-04T00:05:00.000Z".to_string()),
                    reason: None,
                    outcome: Some("shipped".to_string()),
                    evidence_ref: Some("eye:test".to_string()),
                },
                BuildOverlayEventInput {
                    proposal_id: Some("p-1".to_string()),
                    event_type: Some("decision".to_string()),
                    decision: Some("reject".to_string()),
                    ts: Some("2026-03-04T00:10:00.000Z".to_string()),
                    reason: Some("latest".to_string()),
                    outcome: None,
                    evidence_ref: None,
                },
            ],
        });
        assert_eq!(out.entries.len(), 1);
        let row = &out.entries[0];
        assert_eq!(row.proposal_id, "p-1");
        assert_eq!(row.decision.as_deref(), Some("reject"));
        assert_eq!(row.decision_reason.as_deref(), Some("latest"));
        assert_eq!(row.last_outcome.as_deref(), Some("shipped"));
        assert_eq!(row.outcomes.shipped, 1);
    }

    #[test]
    fn has_adaptive_mutation_signal_detects_blob_markers() {
        let out = compute_has_adaptive_mutation_signal(&HasAdaptiveMutationSignalInput {
            proposal_type: Some("improvement".to_string()),
            adaptive_mutation: false,
            mutation_proposal: false,
            topology_mutation: false,
            self_improvement_change: false,
            signal_blob: Some("run mutation_guard with rollback receipt".to_string()),
        });
        assert!(out.has_signal);
    }

    #[test]
    fn adaptive_mutation_execution_guard_requires_receipts() {
        let out = compute_adaptive_mutation_execution_guard(&AdaptiveMutationExecutionGuardInput {
            guard_required: true,
            applies: true,
            metadata_applies: true,
            guard_pass: true,
            guard_reason: None,
            safety_attestation: None,
            rollback_receipt: None,
            guard_receipt_id: None,
            mutation_kernel_applies: false,
            mutation_kernel_pass: true,
        });
        assert!(!out.pass);
        assert!(out
            .reasons
            .contains(&"adaptive_mutation_missing_safety_attestation".to_string()));
    }

    #[test]
    fn autoscale_json_adaptive_mutation_guard_path_works() {
        let payload = serde_json::json!({
            "mode": "adaptive_mutation_execution_guard",
            "adaptive_mutation_execution_guard_input": {
                "guard_required": true,
                "applies": true,
                "metadata_applies": false,
                "guard_pass": false,
                "guard_reason": "failed",
                "safety_attestation": "safe-1",
                "rollback_receipt": "roll-1",
                "guard_receipt_id": "guard-1",
                "mutation_kernel_applies": true,
                "mutation_kernel_pass": false
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale adaptive_mutation_execution_guard");
        assert!(out.contains("\"mode\":\"adaptive_mutation_execution_guard\""));
    }

    #[test]
    fn strategy_selection_chooses_primary_when_canary_not_due() {
        let out = compute_strategy_selection(&StrategySelectionInput {
            date_str: Some("2026-03-04".to_string()),
            attempt_index: 1.0,
            canary_enabled: true,
            canary_allow_execute: false,
            canary_fraction: 0.25,
            max_active: 3.0,
            fallback_strategy_id: Some("fallback".to_string()),
            variants: vec![
                StrategySelectionVariantInput {
                    strategy_id: Some("s-main".to_string()),
                    score: 0.9,
                    confidence: 0.8,
                    stage: Some("stable".to_string()),
                    execution_mode: Some("execute".to_string()),
                },
                StrategySelectionVariantInput {
                    strategy_id: Some("s-canary".to_string()),
                    score: 0.8,
                    confidence: 0.7,
                    stage: Some("trial".to_string()),
                    execution_mode: Some("score_only".to_string()),
                },
            ],
        });
        assert_eq!(out.mode, "primary_best");
        assert_eq!(out.selected_strategy_id.as_deref(), Some("s-main"));
        assert_eq!(out.active_count, 2);
    }

    #[test]
    fn autoscale_json_strategy_selection_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_selection",
            "strategy_selection_input": {
                "date_str": "2026-03-04",
                "attempt_index": 4,
                "canary_enabled": true,
                "canary_allow_execute": false,
                "canary_fraction": 0.25,
                "max_active": 3,
                "fallback_strategy_id": "fallback",
                "variants": [
                    {
                        "strategy_id": "s-main",
                        "score": 0.9,
                        "confidence": 0.8,
                        "stage": "stable",
                        "execution_mode": "execute"
                    },
                    {
                        "strategy_id": "s-canary",
                        "score": 0.8,
                        "confidence": 0.7,
                        "stage": "trial",
                        "execution_mode": "score_only"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_selection");
        assert!(out.contains("\"mode\":\"strategy_selection\""));
    }

    #[test]
    fn calibration_deltas_loosen_when_exhausted_and_low_ship_rate() {
        let out = compute_calibration_deltas(&CalibrationDeltasInput {
            executed_count: 8.0,
            shipped_rate: 0.05,
            no_change_rate: 0.7,
            reverted_rate: 0.1,
            exhausted: 4.0,
            min_executed: 6.0,
            tighten_min_executed: 10.0,
            loosen_low_shipped_rate: 0.2,
            loosen_exhausted_threshold: 3.0,
            tighten_min_shipped_rate: 0.2,
            max_delta: 6.0,
        });
        assert_eq!(out.min_signal_quality, -3.0);
        assert_eq!(out.min_directive_fit, -3.0);
        assert_eq!(out.min_actionability_score, -2.0);
        assert_eq!(out.min_sensory_relevance_score, -1.0);
    }

    #[test]
    fn autoscale_json_calibration_deltas_path_works() {
        let payload = serde_json::json!({
            "mode": "calibration_deltas",
            "calibration_deltas_input": {
                "executed_count": 12,
                "shipped_rate": 0.5,
                "no_change_rate": 0.65,
                "reverted_rate": 0.2,
                "exhausted": 3,
                "min_executed": 6,
                "tighten_min_executed": 10,
                "loosen_low_shipped_rate": 0.2,
                "loosen_exhausted_threshold": 3,
                "tighten_min_shipped_rate": 0.2,
                "max_delta": 6
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale calibration_deltas");
        assert!(out.contains("\"mode\":\"calibration_deltas\""));
    }

    #[test]
    fn strategy_admission_decision_blocks_duplicate_window() {
        let out = compute_strategy_admission_decision(&StrategyAdmissionDecisionInput {
            require_admission_preview: false,
            preview_eligible: true,
            preview_blocked_by: vec![],
            mutation_guard: None,
            strategy_type_allowed: true,
            max_risk_per_action: Some(0.8),
            strategy_max_risk_per_action: Some(0.8),
            hard_max_risk_per_action: None,
            risk_score: Some(0.2),
            remediation_check_required: false,
            remediation_depth: None,
            remediation_max_depth: None,
            dedup_key: Some("proposal:key".to_string()),
            duplicate_window_hours: Some(24.0),
            recent_count: Some(2.0),
        });
        assert!(!out.allow);
        assert_eq!(out.reason.as_deref(), Some("strategy_duplicate_window"));
        assert_eq!(out.recent_count, Some(2.0));
    }

    #[test]
    fn autoscale_json_strategy_admission_decision_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_admission_decision",
            "strategy_admission_decision_input": {
                "require_admission_preview": true,
                "preview_eligible": false,
                "preview_blocked_by": ["preview_gate"],
                "mutation_guard": {
                    "applies": false,
                    "pass": true,
                    "reason": null,
                    "reasons": [],
                    "controls": {}
                },
                "strategy_type_allowed": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_admission_decision");
        assert!(out.contains("\"mode\":\"strategy_admission_decision\""));
    }

    #[test]
    fn expected_value_score_returns_input_score() {
        let out = compute_expected_value_score(&ExpectedValueScoreInput { score: 42.5 });
        assert_eq!(out.score, 42.5);
    }

    #[test]
    fn autoscale_json_expected_value_score_path_works() {
        let payload = serde_json::json!({
            "mode": "expected_value_score",
            "expected_value_score_input": {
                "score": 77.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale expected_value_score");
        assert!(out.contains("\"mode\":\"expected_value_score\""));
    }

    #[test]
    fn suggest_run_batch_max_normalizes_values() {
        let out = compute_suggest_run_batch_max(&SuggestRunBatchMaxInput {
            enabled: true,
            batch_max: 3.8,
            batch_reason: Some(" backlog_autoscale ".to_string()),
            daily_remaining: 4.2,
            autoscale_hint: serde_json::json!({"current_cells": 2}),
        });
        assert!(out.enabled);
        assert_eq!(out.max, 3.0);
        assert_eq!(out.reason, "backlog_autoscale");
        assert_eq!(out.daily_remaining, 4.0);
    }

    #[test]
    fn autoscale_json_suggest_run_batch_max_path_works() {
        let payload = serde_json::json!({
            "mode": "suggest_run_batch_max",
            "suggest_run_batch_max_input": {
                "enabled": true,
                "batch_max": 2,
                "batch_reason": "no_pressure",
                "daily_remaining": 6,
                "autoscale_hint": {"state": {"current_cells": 1}}
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale suggest_run_batch_max");
        assert!(out.contains("\"mode\":\"suggest_run_batch_max\""));
    }

    #[test]
    fn backlog_autoscale_snapshot_normalizes_payload() {
        let out = compute_backlog_autoscale_snapshot(&BacklogAutoscaleSnapshotInput {
            enabled: true,
            module: Some(" autonomy_backlog_autoscale ".to_string()),
            state: serde_json::json!({"current_cells": 2}),
            queue: serde_json::json!({"pressure": "warning"}),
            current_cells: 3.8,
            plan: serde_json::json!({"action": "scale_up"}),
            trit_productivity: serde_json::json!({"hold": false}),
        });
        assert!(out.enabled);
        assert_eq!(out.module, "autonomy_backlog_autoscale");
        assert_eq!(out.current_cells, 3.8);
    }

    #[test]
    fn autoscale_json_backlog_autoscale_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "backlog_autoscale_snapshot",
            "backlog_autoscale_snapshot_input": {
                "enabled": true,
                "module": "autonomy_backlog_autoscale",
                "state": {"current_cells": 1},
                "queue": {"pressure": "normal"},
                "current_cells": 1,
                "plan": {"action": "hold"},
                "trit_productivity": {"hold": false}
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale backlog_autoscale_snapshot");
        assert!(out.contains("\"mode\":\"backlog_autoscale_snapshot\""));
    }

    #[test]
    fn admission_summary_tallies_blocked_reasons() {
        let out = compute_admission_summary(&AdmissionSummaryInput {
            proposals: vec![
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(true),
                    blocked_by: vec![],
                },
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(false),
                    blocked_by: vec!["policy_hold".to_string(), "risk".to_string()],
                },
                AdmissionSummaryProposalInput {
                    preview_eligible: Some(false),
                    blocked_by: vec![],
                },
            ],
        });
        assert_eq!(out.total, 3);
        assert_eq!(out.eligible, 1);
        assert_eq!(out.blocked, 2);
        assert_eq!(out.blocked_by_reason.get("policy_hold"), Some(&1));
        assert_eq!(out.blocked_by_reason.get("risk"), Some(&1));
        assert_eq!(out.blocked_by_reason.get("unknown"), Some(&1));
    }

    #[test]
    fn autoscale_json_admission_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "admission_summary",
            "admission_summary_input": {
                "proposals": [
                    {"preview_eligible": true, "blocked_by": []},
                    {"preview_eligible": false, "blocked_by": ["manual_gate"]}
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale admission_summary");
        assert!(out.contains("\"mode\":\"admission_summary\""));
    }

    #[test]
    fn unknown_type_quarantine_decision_blocks_unknown_type() {
        let out = compute_unknown_type_quarantine_decision(&UnknownTypeQuarantineDecisionInput {
            enabled: true,
            proposal_type: Some("unknown_type".to_string()),
            type_in_quarantine_set: true,
            allow_directive: true,
            allow_tier1: true,
            objective_id: Some("T1_OBJ".to_string()),
            tier1_objective: false,
        });
        assert!(out.block);
        assert_eq!(out.reason.as_deref(), Some("unknown_type_quarantine"));
        assert_eq!(out.proposal_type.as_deref(), Some("unknown_type"));
    }

    #[test]
    fn autoscale_json_unknown_type_quarantine_decision_path_works() {
        let payload = serde_json::json!({
            "mode": "unknown_type_quarantine_decision",
            "unknown_type_quarantine_decision_input": {
                "enabled": true,
                "proposal_type": "directive_decomposition",
                "type_in_quarantine_set": true,
                "allow_directive": true,
                "allow_tier1": true,
                "objective_id": "T1_demo",
                "tier1_objective": false
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale unknown_type_quarantine_decision");
        assert!(out.contains("\"mode\":\"unknown_type_quarantine_decision\""));
    }

    #[test]
    fn infer_optimization_delta_prefers_direct_meta_field() {
        let out = compute_infer_optimization_delta(&InferOptimizationDeltaInput {
            optimization_delta_percent: None,
            expected_optimization_percent: Some(12.75),
            expected_delta_percent: Some(9.0),
            estimated_improvement_percent: None,
            target_improvement_percent: None,
            performance_gain_percent: None,
            text_blob: Some("fallback 30%".to_string()),
        });
        assert_eq!(out.delta_percent, Some(12.75));
        assert_eq!(
            out.delta_source.as_deref(),
            Some("meta:expected_optimization_percent")
        );
    }

    #[test]
    fn autoscale_json_infer_optimization_delta_path_works() {
        let payload = serde_json::json!({
            "mode": "infer_optimization_delta",
            "infer_optimization_delta_input": {
                "optimization_delta_percent": null,
                "expected_optimization_percent": null,
                "expected_delta_percent": null,
                "estimated_improvement_percent": null,
                "target_improvement_percent": null,
                "performance_gain_percent": null,
                "text_blob": "target +18% reduction"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale infer_optimization_delta");
        assert!(out.contains("\"mode\":\"infer_optimization_delta\""));
    }

    #[test]
    fn optimization_intent_proposal_detects_expected_terms() {
        let out = compute_optimization_intent_proposal(&OptimizationIntentProposalInput {
            proposal_type: Some("automation".to_string()),
            blob: Some("optimize latency and throughput".to_string()),
            has_actuation_meta: false,
        });
        assert!(out.intent);
    }

    #[test]
    fn autoscale_json_optimization_intent_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_intent_proposal",
            "optimization_intent_proposal_input": {
                "proposal_type": "actuation",
                "blob": "canary smoke test rollout",
                "has_actuation_meta": true
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_intent_proposal");
        assert!(out.contains("\"mode\":\"optimization_intent_proposal\""));
    }

    #[test]
    fn unlinked_optimization_admission_blocks_high_risk_when_unlinked() {
        let out = compute_unlinked_optimization_admission(&UnlinkedOptimizationAdmissionInput {
            optimization_intent: true,
            proposal_type: Some("optimization".to_string()),
            exempt_types: vec!["directive_clarification".to_string()],
            linked: false,
            normalized_risk: Some("high".to_string()),
            hard_block_high_risk: true,
            penalty: 8.0,
        });
        assert!(out.applies);
        assert!(!out.linked);
        assert!(out.block);
        assert_eq!(
            out.reason.as_deref(),
            Some("optimization_unlinked_objective_high_risk_block")
        );
    }

    #[test]
    fn autoscale_json_unlinked_optimization_admission_path_works() {
        let payload = serde_json::json!({
            "mode": "unlinked_optimization_admission",
            "unlinked_optimization_admission_input": {
                "optimization_intent": true,
                "proposal_type": "optimization",
                "exempt_types": ["directive_clarification"],
                "linked": false,
                "normalized_risk": "low",
                "hard_block_high_risk": true,
                "penalty": 12
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale unlinked_optimization_admission");
        assert!(out.contains("\"mode\":\"unlinked_optimization_admission\""));
    }

    #[test]
    fn optimization_good_enough_fails_when_delta_below_min() {
        let out = compute_optimization_good_enough(&OptimizationGoodEnoughInput {
            applies: true,
            min_delta_percent: 10.0,
            require_delta: true,
            high_accuracy_mode: false,
            normalized_risk: Some("medium".to_string()),
            delta_percent: Some(4.0),
            delta_source: Some("text:%".to_string()),
        });
        assert!(out.applies);
        assert!(!out.pass);
        assert_eq!(out.reason.as_deref(), Some("optimization_good_enough"));
    }

    #[test]
    fn autoscale_json_optimization_good_enough_path_works() {
        let payload = serde_json::json!({
            "mode": "optimization_good_enough",
            "optimization_good_enough_input": {
                "applies": true,
                "min_delta_percent": 8,
                "require_delta": true,
                "high_accuracy_mode": false,
                "normalized_risk": "low",
                "delta_percent": 12,
                "delta_source": "meta:expected_delta_percent"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale optimization_good_enough");
        assert!(out.contains("\"mode\":\"optimization_good_enough\""));
    }

    #[test]
    fn proposal_dependency_summary_builds_chain_and_edges() {
        let out = compute_proposal_dependency_summary(&ProposalDependencySummaryInput {
            proposal_id: Some("p-1".to_string()),
            decision: Some("accept".to_string()),
            source: Some("directive_decomposition".to_string()),
            parent_objective_id: Some("T1_parent".to_string()),
            created_ids: vec!["T1_child_a".to_string(), "T1_child_b".to_string()],
            dry_run: false,
            created_count: Some(2.0),
            quality_ok: true,
            reason: None,
        });
        assert_eq!(out.decision, "ACCEPT");
        assert_eq!(out.edge_count, 2);
        assert_eq!(out.chain.len(), 3);
        assert_eq!(out.child_objective_ids.len(), 2);
    }

    #[test]
    fn autoscale_json_proposal_dependency_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_dependency_summary",
            "proposal_dependency_summary_input": {
                "proposal_id": "p-2",
                "decision": "accept",
                "source": "directive_decomposition",
                "parent_objective_id": "T1_parent",
                "created_ids": ["T1_child_a"],
                "dry_run": false,
                "created_count": 1,
                "quality_ok": true,
                "reason": null
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_dependency_summary");
        assert!(out.contains("\"mode\":\"proposal_dependency_summary\""));
    }

    #[test]
    fn choose_selection_mode_switches_to_explore_on_cadence() {
        let out = compute_choose_selection_mode(&ChooseSelectionModeInput {
            eligible_len: 6,
            executed_count: 4,
            explore_used: 1,
            exploit_used: 3,
            explore_quota: 5,
            every_n: 2,
            min_eligible: 2,
        });
        assert_eq!(out.mode, "explore");
        assert!(out.index >= 1);
    }

    #[test]
    fn autoscale_json_choose_selection_mode_path_works() {
        let payload = serde_json::json!({
            "mode": "choose_selection_mode",
            "choose_selection_mode_input": {
                "eligible_len": 3,
                "executed_count": 1,
                "explore_used": 0,
                "exploit_used": 1,
                "explore_quota": 2,
                "every_n": 1,
                "min_eligible": 2
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale choose_selection_mode");
        assert!(out.contains("\"mode\":\"choose_selection_mode\""));
    }

    #[test]
    fn explore_quota_for_day_clamps_fraction_and_floor() {
        let out = compute_explore_quota_for_day(&ExploreQuotaForDayInput {
            daily_runs_cap: Some(12.0),
            explore_fraction: Some(0.25),
            default_max_runs: 8.0,
        });
        assert_eq!(out.quota, 3);
    }

    #[test]
    fn autoscale_json_explore_quota_for_day_path_works() {
        let payload = serde_json::json!({
            "mode": "explore_quota_for_day",
            "explore_quota_for_day_input": {
                "daily_runs_cap": 10,
                "explore_fraction": 0.2,
                "default_max_runs": 8
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale explore_quota_for_day");
        assert!(out.contains("\"mode\":\"explore_quota_for_day\""));
    }

    #[test]
    fn medium_risk_thresholds_derives_bounds() {
        let out = compute_medium_risk_thresholds(&MediumRiskThresholdsInput {
            base_min_directive_fit: 40.0,
            base_min_actionability_score: 45.0,
            medium_risk_min_composite_eligibility: 70.0,
            min_composite_eligibility: 68.0,
            medium_risk_min_directive_fit: 50.0,
            default_min_directive_fit: 45.0,
            medium_risk_min_actionability: 52.0,
            default_min_actionability: 46.0,
        });
        assert_eq!(out.composite_min, 74.0);
        assert_eq!(out.directive_fit_min, 50.0);
        assert_eq!(out.actionability_min, 52.0);
    }

    #[test]
    fn autoscale_json_medium_risk_thresholds_path_works() {
        let payload = serde_json::json!({
            "mode": "medium_risk_thresholds",
            "medium_risk_thresholds_input": {
                "base_min_directive_fit": 40,
                "base_min_actionability_score": 45,
                "medium_risk_min_composite_eligibility": 70,
                "min_composite_eligibility": 68,
                "medium_risk_min_directive_fit": 50,
                "default_min_directive_fit": 45,
                "medium_risk_min_actionability": 52,
                "default_min_actionability": 46
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale medium_risk_thresholds");
        assert!(out.contains("\"mode\":\"medium_risk_thresholds\""));
    }

    #[test]
    fn medium_risk_gate_decision_flags_low_scores() {
        let out = compute_medium_risk_gate_decision(&MediumRiskGateDecisionInput {
            risk: Some("medium".to_string()),
            composite_score: 60.0,
            directive_fit_score: 55.0,
            actionability_score: 54.0,
            composite_min: 70.0,
            directive_fit_min: 60.0,
            actionability_min: 62.0,
        });
        assert!(!out.pass);
        assert_eq!(out.risk, "medium");
        assert!(out.reasons.contains(&"medium_composite_low".to_string()));
        assert!(out.required.is_some());
    }

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

    #[test]
    fn top_biases_summary_sorts_by_abs_bias_then_total() {
        let out = compute_top_biases_summary(&TopBiasesSummaryInput {
            entries: vec![
                TopBiasSummaryEntryInput {
                    key: Some("a".to_string()),
                    bias: 2.0,
                    total: 10.0,
                    shipped: 3.0,
                    no_change: 4.0,
                    reverted: 3.0,
                },
                TopBiasSummaryEntryInput {
                    key: Some("b".to_string()),
                    bias: -5.0,
                    total: 2.0,
                    shipped: 1.0,
                    no_change: 1.0,
                    reverted: 0.0,
                },
            ],
            limit: 2,
        });
        assert_eq!(out.rows.len(), 2);
        assert_eq!(out.rows[0].key, "b");
    }

    #[test]
    fn autoscale_json_top_biases_summary_path_works() {
        let payload = serde_json::json!({
            "mode": "top_biases_summary",
            "top_biases_summary_input": {
                "entries": [
                    {"key":"x","bias":3,"total":5,"shipped":2,"no_change":2,"reverted":1},
                    {"key":"y","bias":1,"total":8,"shipped":4,"no_change":3,"reverted":1}
                ],
                "limit": 1
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale top_biases_summary");
        assert!(out.contains("\"mode\":\"top_biases_summary\""));
    }

    #[test]
    fn criteria_pattern_penalty_accumulates_hits() {
        let out = compute_criteria_pattern_penalty(&CriteriaPatternPenaltyInput {
            keys: vec!["cap|metric".to_string()],
            patterns: vec![CriteriaPatternPenaltyPatternInput {
                key: "cap|metric".to_string(),
                failures: 4.0,
                passes: 0.0,
                last_failure_ts: Some("2026-03-04T00:00:00.000Z".to_string()),
            }],
            fail_threshold: 2.0,
            penalty_per_hit: 3.0,
            max_penalty: 20.0,
            window_days: 365.0,
            now_ms: chrono::DateTime::parse_from_rfc3339("2026-03-04T06:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc)
                .timestamp_millis() as f64,
        });
        assert_eq!(out.penalty, 9.0);
        assert_eq!(out.hit_patterns.len(), 1);
    }

    #[test]
    fn autoscale_json_criteria_pattern_penalty_path_works() {
        let payload = serde_json::json!({
            "mode": "criteria_pattern_penalty",
            "criteria_pattern_penalty_input": {
                "keys": ["cap|metric"],
                "patterns": [{"key":"cap|metric","failures":4,"passes":1,"last_failure_ts":"2026-03-04T00:00:00.000Z"}],
                "fail_threshold": 2,
                "penalty_per_hit": 3,
                "max_penalty": 20,
                "window_days": 365,
                "now_ms": 1772600000000.0
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria_pattern_penalty");
        assert!(out.contains("\"mode\":\"criteria_pattern_penalty\""));
    }

    #[test]
    fn strategy_threshold_overrides_prefers_override_values() {
        let out = compute_strategy_threshold_overrides(&StrategyThresholdOverridesInput {
            min_signal_quality: Some(55.0),
            min_sensory_signal_score: Some(60.0),
            min_sensory_relevance_score: Some(62.0),
            min_directive_fit: Some(45.0),
            min_actionability_score: Some(50.0),
            min_eye_score_ema: Some(48.0),
            override_min_signal_quality: Some(70.0),
            override_min_sensory_signal_score: None,
            override_min_sensory_relevance_score: None,
            override_min_directive_fit: Some(52.0),
            override_min_actionability_score: None,
            override_min_eye_score_ema: None,
        });
        assert_eq!(out.min_signal_quality, 70.0);
        assert_eq!(out.min_directive_fit, 52.0);
        assert_eq!(out.min_sensory_signal_score, 60.0);
    }

    #[test]
    fn autoscale_json_strategy_threshold_overrides_path_works() {
        let payload = serde_json::json!({
            "mode": "strategy_threshold_overrides",
            "strategy_threshold_overrides_input": {
                "min_signal_quality": 55,
                "min_sensory_signal_score": 60,
                "min_sensory_relevance_score": 62,
                "min_directive_fit": 45,
                "min_actionability_score": 50,
                "min_eye_score_ema": 48,
                "override_min_signal_quality": 70
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale strategy_threshold_overrides");
        assert!(out.contains("\"mode\":\"strategy_threshold_overrides\""));
    }

    #[test]
    fn effective_allowed_risks_prefers_strategy_list() {
        let out = compute_effective_allowed_risks(&EffectiveAllowedRisksInput {
            default_risks: vec!["low".to_string(), "medium".to_string()],
            strategy_allowed_risks: vec!["high".to_string()],
        });
        assert_eq!(out.risks, vec!["high".to_string()]);
    }

    #[test]
    fn autoscale_json_effective_allowed_risks_path_works() {
        let payload = serde_json::json!({
            "mode": "effective_allowed_risks",
            "effective_allowed_risks_input": {
                "default_risks": ["low","medium"],
                "strategy_allowed_risks": ["high"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale effective_allowed_risks");
        assert!(out.contains("\"mode\":\"effective_allowed_risks\""));
    }

    #[test]
    fn directive_pulse_context_clamps_and_normalizes_fields() {
        let out = compute_directive_pulse_context(&DirectivePulseContextInput {
            enabled: true,
            available: true,
            objectives: vec![serde_json::json!({"id":"t1","tier":1})],
            error: Some("  ".to_string()),
            window_days: 99.0,
            urgency_hours: -1.0,
            no_progress_limit: 0.0,
            cooldown_hours: 300.0,
            tier_attempts_today: std::collections::BTreeMap::from([
                ("1".to_string(), 2.0),
                ("2".to_string(), -1.0),
            ]),
            attempts_today: 4.2,
            objective_stats: vec![DirectivePulseContextObjectiveStatInput {
                objective_id: Some(" obj_a ".to_string()),
                tier: Some(1.0),
                attempts: Some(3.0),
                shipped: Some(1.0),
                no_change: Some(1.0),
                reverted: Some(1.0),
                no_progress_streak: Some(2.0),
                last_attempt_ts: Some(" 2026-03-04T00:00:00.000Z ".to_string()),
                last_shipped_ts: Some("".to_string()),
            }],
        });
        assert_eq!(out.window_days, 60.0);
        assert_eq!(out.urgency_hours, 1.0);
        assert_eq!(out.no_progress_limit, 1.0);
        assert_eq!(out.cooldown_hours, 168.0);
        assert_eq!(out.attempts_today, 4.0);
        assert_eq!(
            out.tier_attempts_today.get("1").copied().unwrap_or(0.0),
            2.0
        );
        assert_eq!(
            out.tier_attempts_today.get("2").copied().unwrap_or(0.0),
            0.0
        );
        assert!(out.error.is_none());
        assert_eq!(out.objective_stats.len(), 1);
        assert_eq!(out.objective_stats[0].objective_id, "obj_a");
        assert_eq!(
            out.objective_stats[0].last_attempt_ts.as_deref(),
            Some("2026-03-04T00:00:00.000Z")
        );
        assert_eq!(out.objective_stats[0].last_shipped_ts, None);
    }

    #[test]
    fn autoscale_json_directive_pulse_context_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_context",
            "directive_pulse_context_input": {
                "enabled": true,
                "available": true,
                "objectives": [{"id":"t1","tier":1}],
                "window_days": 14,
                "urgency_hours": 24,
                "no_progress_limit": 3,
                "cooldown_hours": 6,
                "tier_attempts_today": {"1": 1},
                "attempts_today": 1,
                "objective_stats": [
                    {
                        "objective_id": "obj_a",
                        "tier": 1,
                        "attempts": 1,
                        "shipped": 1,
                        "no_change": 0,
                        "reverted": 0,
                        "no_progress_streak": 0
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_pulse_context");
        assert!(out.contains("\"mode\":\"directive_pulse_context\""));
    }

    #[test]
    fn directive_pulse_stats_aggregates_attempts_and_outcomes() {
        let out = compute_directive_pulse_stats(&DirectivePulseStatsInput {
            date_str: Some("2026-03-04".to_string()),
            window_days: Some(14.0),
            events: vec![
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-04".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("shipped".to_string()),
                    objective_id: Some("obj_a".to_string()),
                    tier: Some(1.0),
                    ts: Some("2026-03-04T01:00:00.000Z".to_string()),
                },
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-04".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("no_change".to_string()),
                    objective_id: Some("obj_a".to_string()),
                    tier: Some(1.0),
                    ts: Some("2026-03-04T02:00:00.000Z".to_string()),
                },
                DirectivePulseStatsEventInput {
                    day: Some("2026-03-03".to_string()),
                    event_type: Some("autonomy_run".to_string()),
                    result: Some("executed".to_string()),
                    outcome: Some("reverted".to_string()),
                    objective_id: Some("obj_b".to_string()),
                    tier: Some(2.0),
                    ts: Some("2026-03-03T03:00:00.000Z".to_string()),
                },
            ],
        });
        assert_eq!(out.attempts_today, 2.0);
        assert_eq!(
            out.tier_attempts_today.get("1").copied().unwrap_or(0.0),
            2.0
        );
        assert_eq!(out.objective_stats.len(), 2);
        let a = out
            .objective_stats
            .iter()
            .find(|row| row.objective_id == "obj_a")
            .expect("obj_a");
        assert_eq!(a.attempts, 2);
        assert_eq!(a.shipped, 1);
        assert_eq!(a.no_change, 1);
        assert_eq!(a.reverted, 0);
    }

    #[test]
    fn autoscale_json_directive_pulse_stats_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_stats",
            "directive_pulse_stats_input": {
                "date_str": "2026-03-04",
                "window_days": 14,
                "events": [
                    {
                        "day": "2026-03-04",
                        "event_type": "autonomy_run",
                        "result": "executed",
                        "outcome": "shipped",
                        "objective_id": "obj_a",
                        "tier": 1,
                        "ts": "2026-03-04T01:00:00.000Z"
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale directive_pulse_stats");
        assert!(out.contains("\"mode\":\"directive_pulse_stats\""));
    }

    #[test]
    fn compile_directive_pulse_objectives_filters_and_derives_rows() {
        let out = compute_compile_directive_pulse_objectives(
            &CompileDirectivePulseObjectivesInput {
                directives: vec![
                    serde_json::json!({
                        "id": "T0_FOUNDATION",
                        "data": {
                            "metadata": { "id": "T0_FOUNDATION", "description": "ignore me", "tier": 1 }
                        }
                    }),
                    serde_json::json!({
                        "id": "T1_MEMORY",
                        "tier": 1,
                        "data": {
                            "metadata": {
                                "id": "T1_MEMORY",
                                "description": "Improve memory durability and recall quality",
                                "value_currency": "quality"
                            },
                            "intent": {
                                "primary": "Improve memory durability",
                                "value_currency": "time_savings"
                            },
                            "scope": {
                                "included": ["durability guardrails", "recall quality"]
                            },
                            "success_metrics": {
                                "leading": ["reduced regressions"],
                                "lagging": ["higher recall score"]
                            }
                        }
                    }),
                ],
                stopwords: vec!["the".to_string(), "and".to_string()],
                allowed_value_keys: vec![
                    "revenue".to_string(),
                    "delivery".to_string(),
                    "user_value".to_string(),
                    "quality".to_string(),
                    "time_savings".to_string(),
                    "learning".to_string(),
                ],
                t1_min_share: Some(0.5),
                t2_min_share: Some(0.25),
            },
        );
        assert_eq!(out.objectives.len(), 1);
        let row = &out.objectives[0];
        assert_eq!(row.id, "T1_MEMORY");
        assert_eq!(row.tier, 1);
        assert!(!row.phrases.is_empty());
        assert!(!row.tokens.is_empty());
        assert_eq!(row.primary_currency.as_deref(), Some("quality"));
    }

    #[test]
    fn autoscale_json_compile_directive_pulse_objectives_path_works() {
        let payload = serde_json::json!({
            "mode": "compile_directive_pulse_objectives",
            "compile_directive_pulse_objectives_input": {
                "directives": [
                    {
                        "id": "T1_MEMORY",
                        "tier": 1,
                        "data": {
                            "metadata": { "id": "T1_MEMORY", "description": "memory durability" },
                            "intent": { "primary": "Improve memory durability" }
                        }
                    }
                ],
                "stopwords": ["the", "and"],
                "allowed_value_keys": ["quality", "time_savings", "learning", "user_value", "delivery", "revenue"],
                "t1_min_share": 0.5,
                "t2_min_share": 0.25
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale compile_directive_pulse_objectives");
        assert!(out.contains("\"mode\":\"compile_directive_pulse_objectives\""));
    }

    #[test]
    fn directive_pulse_objectives_profile_handles_disabled_and_error() {
        let disabled =
            compute_directive_pulse_objectives_profile(&DirectivePulseObjectivesProfileInput {
                enabled: false,
                load_error: None,
                objectives: vec![],
            });
        assert!(!disabled.enabled);
        assert!(!disabled.available);
        assert_eq!(disabled.error.as_deref(), Some("directive_pulse_disabled"));

        let errored =
            compute_directive_pulse_objectives_profile(&DirectivePulseObjectivesProfileInput {
                enabled: true,
                load_error: Some(" boom ".to_string()),
                objectives: vec![serde_json::json!({"id":"T1"})],
            });
        assert!(errored.enabled);
        assert!(!errored.available);
        assert_eq!(errored.objectives.len(), 0);
        assert_eq!(errored.error.as_deref(), Some("boom"));
    }

    #[test]
    fn autoscale_json_directive_pulse_objectives_profile_path_works() {
        let payload = serde_json::json!({
            "mode": "directive_pulse_objectives_profile",
            "directive_pulse_objectives_profile_input": {
                "enabled": true,
                "load_error": null,
                "objectives": [{"id":"T1_MEMORY"}]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale directive_pulse_objectives_profile");
        assert!(out.contains("\"mode\":\"directive_pulse_objectives_profile\""));
    }

    #[test]
    fn recent_directive_pulse_cooldown_count_matches_objective_and_window() {
        let now_ms = chrono::DateTime::parse_from_rfc3339("2026-03-04T12:00:00.000Z")
            .unwrap()
            .with_timezone(&Utc)
            .timestamp_millis() as f64;
        let out = compute_recent_directive_pulse_cooldown_count(
            &RecentDirectivePulseCooldownCountInput {
                objective_id: Some("OBJ-1".to_string()),
                hours: Some(24.0),
                now_ms: Some(now_ms),
                events: vec![
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-04T10:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-1".to_string()),
                        sample_objective_id: None,
                    },
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-03T08:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-1".to_string()),
                        sample_objective_id: None,
                    },
                    RecentDirectivePulseCooldownEventInput {
                        event_type: Some("autonomy_run".to_string()),
                        result: Some("stop_repeat_gate_directive_pulse_cooldown".to_string()),
                        ts: Some("2026-03-04T11:00:00.000Z".to_string()),
                        objective_id: Some("OBJ-2".to_string()),
                        sample_objective_id: None,
                    },
                ],
            },
        );
        assert_eq!(out.count, 1);
    }

    #[test]
    fn autoscale_json_recent_directive_pulse_cooldown_count_path_works() {
        let payload = serde_json::json!({
            "mode": "recent_directive_pulse_cooldown_count",
            "recent_directive_pulse_cooldown_count_input": {
                "objective_id": "OBJ-1",
                "hours": 24,
                "now_ms": 1772625600000.0,
                "events": [
                    {
                        "event_type": "autonomy_run",
                        "result": "stop_repeat_gate_directive_pulse_cooldown",
                        "ts": "2026-03-04T10:00:00.000Z",
                        "objective_id": "OBJ-1"
                    }
                ]
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale recent_directive_pulse_cooldown_count");
        assert!(out.contains("\"mode\":\"recent_directive_pulse_cooldown_count\""));
    }

    #[test]
    fn proposal_directive_text_matches_expected_normalization() {
        let out = compute_proposal_directive_text(&ProposalDirectiveTextInput {
            proposal: Some(serde_json::json!({
                "title": "Directive fit improve",
                "type": "directive_clarification",
                "summary": "Improve objective focus",
                "meta": {
                    "normalized_hint_tokens": ["memory", "durability"],
                    "topics": ["alignment", "metrics"]
                },
                "validation": ["one metric"],
                "evidence": [{"match":"directive", "evidence_ref":"eye:directive/1"}]
            })),
        });
        assert!(out.text.contains("directive"));
        assert!(out.text.contains("memory"));
        assert!(out.text.contains("alignment"));
    }

    #[test]
    fn autoscale_json_proposal_directive_text_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_directive_text",
            "proposal_directive_text_input": {
                "proposal": {
                    "title": "Directive fit improve",
                    "type": "directive_clarification",
                    "summary": "Improve objective focus"
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_directive_text");
        assert!(out.contains("\"mode\":\"proposal_directive_text\""));
    }

    #[test]
    fn objective_ids_from_pulse_context_prefers_objectives_then_fallback() {
        let out = compute_objective_ids_from_pulse_context(&ObjectiveIdsFromPulseContextInput {
            objectives: vec![
                serde_json::json!({"id":"OBJ-A"}),
                serde_json::json!({"id":"OBJ-B"}),
                serde_json::json!({"id":"OBJ-A"}),
            ],
            fallback_enabled: true,
            fallback_ids: vec!["OBJ-C".to_string()],
        });
        assert_eq!(out.ids, vec!["OBJ-A".to_string(), "OBJ-B".to_string()]);

        let fallback =
            compute_objective_ids_from_pulse_context(&ObjectiveIdsFromPulseContextInput {
                objectives: vec![],
                fallback_enabled: true,
                fallback_ids: vec![
                    "OBJ-C".to_string(),
                    "OBJ-C".to_string(),
                    "OBJ-D".to_string(),
                ],
            });
        assert_eq!(fallback.ids, vec!["OBJ-C".to_string(), "OBJ-D".to_string()]);
    }

    #[test]
    fn autoscale_json_objective_ids_from_pulse_context_path_works() {
        let payload = serde_json::json!({
            "mode": "objective_ids_from_pulse_context",
            "objective_ids_from_pulse_context_input": {
                "objectives": [{"id":"OBJ-A"}, {"id":"OBJ-B"}],
                "fallback_enabled": true,
                "fallback_ids": ["OBJ-C"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale objective_ids_from_pulse_context");
        assert!(out.contains("\"mode\":\"objective_ids_from_pulse_context\""));
    }

    #[test]
    fn policy_hold_objective_context_prefers_candidate_then_dominant() {
        let out = compute_policy_hold_objective_context(&PolicyHoldObjectiveContextInput {
            candidate_objective_ids: vec![
                "T1_OBJ_A".to_string(),
                " T1_OBJ_A ".to_string(),
                "T2_OBJ_B".to_string(),
            ],
            pool_objective_ids: vec!["T3_OBJ_C".to_string()],
            dominant_objective_id: Some("T4_OBJ_Z".to_string()),
        });
        assert_eq!(out.objective_id.as_deref(), Some("T4_OBJ_Z"));
        assert_eq!(
            out.objective_source.as_deref(),
            Some("directive_pulse_dominant")
        );
        assert_eq!(
            out.objective_ids.unwrap_or_default(),
            vec!["T1_OBJ_A".to_string(), "T2_OBJ_B".to_string()]
        );
    }

    #[test]
    fn autoscale_json_policy_hold_objective_context_path_works() {
        let payload = serde_json::json!({
            "mode": "policy_hold_objective_context",
            "policy_hold_objective_context_input": {
                "candidate_objective_ids": ["OBJ_A"],
                "pool_objective_ids": ["OBJ_B"],
                "dominant_objective_id": "OBJ_A"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale policy_hold_objective_context");
        assert!(out.contains("\"mode\":\"policy_hold_objective_context\""));
    }

    #[test]
    fn proposal_semantic_objective_id_prefers_meta_then_command() {
        let out = compute_proposal_semantic_objective_id(&ProposalSemanticObjectiveIdInput {
            proposal: Some(serde_json::json!({
                "meta": {
                    "objective_id": "",
                    "directive_objective_id": "T1_PRIMARY",
                    "linked_objective_id": "T2_SECONDARY"
                },
                "suggested_next_command": "node x --id=T3_CMD"
            })),
        });
        assert_eq!(out.objective_id, "T1_PRIMARY");
    }

    #[test]
    fn autoscale_json_proposal_semantic_objective_id_path_works() {
        let payload = serde_json::json!({
            "mode": "proposal_semantic_objective_id",
            "proposal_semantic_objective_id_input": {
                "proposal": {
                    "meta": { "objective_id": "T1_OBJ" }
                }
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale proposal_semantic_objective_id");
        assert!(out.contains("\"mode\":\"proposal_semantic_objective_id\""));
    }

    #[test]
    fn criteria_pattern_keys_normalizes_and_sorts_unique() {
        let out = compute_criteria_pattern_keys(&CriteriaPatternKeysInput {
            capability_key_hint: Some("".to_string()),
            capability_descriptor_key: Some("actuation:run".to_string()),
            rows: vec![
                CriteriaPatternKeysRowInput {
                    metric: Some("latency_ms".to_string()),
                },
                CriteriaPatternKeysRowInput {
                    metric: Some("Latency Ms".to_string()),
                },
                CriteriaPatternKeysRowInput { metric: None },
            ],
        });
        assert_eq!(out.keys, vec!["actuation:run|latency_ms".to_string()]);
    }

    #[test]
    fn autoscale_json_criteria_pattern_keys_path_works() {
        let payload = serde_json::json!({
            "mode": "criteria_pattern_keys",
            "criteria_pattern_keys_input": {
                "capability_key_hint": "actuation:run",
                "rows": [{"metric":"latency_ms"}]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale criteria_pattern_keys");
        assert!(out.contains("\"mode\":\"criteria_pattern_keys\""));
    }

    #[test]
    fn success_criteria_requirement_merges_exempt_types() {
        let out = compute_success_criteria_requirement(&SuccessCriteriaRequirementInput {
            require_success_criteria: Some(true),
            min_success_criteria_count: Some(2.0),
            policy_exempt_types: vec!["directive_clarification".to_string()],
            env_exempt_types: vec![
                "directive_clarification".to_string(),
                "remediation".to_string(),
            ],
        });
        assert!(out.required);
        assert_eq!(out.min_count, 2.0);
        assert_eq!(
            out.exempt_types,
            vec![
                "directive_clarification".to_string(),
                "remediation".to_string()
            ]
        );
    }

    #[test]
    fn autoscale_json_success_criteria_requirement_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_requirement",
            "success_criteria_requirement_input": {
                "require_success_criteria": true,
                "min_success_criteria_count": 1,
                "policy_exempt_types": ["directive_clarification"],
                "env_exempt_types": ["remediation"]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale success_criteria_requirement");
        assert!(out.contains("\"mode\":\"success_criteria_requirement\""));
    }

    #[test]
    fn success_criteria_policy_for_proposal_applies_exemptions() {
        let out =
            compute_success_criteria_policy_for_proposal(&SuccessCriteriaPolicyForProposalInput {
                base_required: true,
                base_min_count: 1.0,
                base_exempt_types: vec!["directive_clarification".to_string()],
                proposal_type: Some("directive_clarification".to_string()),
            });
        assert!(!out.required);
        assert!(out.exempt);
        assert_eq!(out.min_count, 1.0);
    }

    #[test]
    fn autoscale_json_success_criteria_policy_for_proposal_path_works() {
        let payload = serde_json::json!({
            "mode": "success_criteria_policy_for_proposal",
            "success_criteria_policy_for_proposal_input": {
                "base_required": true,
                "base_min_count": 1,
                "base_exempt_types": ["directive_clarification"],
                "proposal_type": "directive_clarification"
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale success_criteria_policy_for_proposal");
        assert!(out.contains("\"mode\":\"success_criteria_policy_for_proposal\""));
    }

    #[test]
    fn capability_descriptor_prefers_actuation_kind() {
        let out = compute_capability_descriptor(&CapabilityDescriptorInput {
            actuation_kind: Some("route_execute".to_string()),
            proposal_type: Some("optimization".to_string()),
        });
        assert_eq!(out.key, "actuation:route_execute");
        assert_eq!(out.aliases, vec!["actuation".to_string()]);
    }

    #[test]
    fn autoscale_json_capability_descriptor_path_works() {
        let payload = serde_json::json!({
            "mode": "capability_descriptor",
            "capability_descriptor_input": {
                "actuation_kind": null,
                "proposal_type": "optimization"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale capability_descriptor");
        assert!(out.contains("\"mode\":\"capability_descriptor\""));
    }

    #[test]
    fn normalize_token_usage_shape_uses_fallback_fields() {
        let out = compute_normalize_token_usage_shape(&NormalizeTokenUsageShapeInput {
            prompt_tokens: None,
            input_tokens: Some(11.0),
            completion_tokens: None,
            output_tokens: Some(5.0),
            total_tokens: None,
            tokens_used: None,
            source: Some("route_execute_metrics".to_string()),
        });
        assert!(out.has_value);
        let usage = out.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, Some(11.0));
        assert_eq!(usage.completion_tokens, Some(5.0));
        assert_eq!(usage.total_tokens, Some(16.0));
    }

    #[test]
    fn autoscale_json_normalize_token_usage_shape_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_token_usage_shape",
            "normalize_token_usage_shape_input": {
                "prompt_tokens": 10,
                "completion_tokens": 4,
                "source": "route_execute_metrics"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale normalize_token_usage_shape");
        assert!(out.contains("\"mode\":\"normalize_token_usage_shape\""));
    }

    #[test]
    fn default_backlog_autoscale_state_uses_input_module() {
        let out = compute_default_backlog_autoscale_state(&DefaultBacklogAutoscaleStateInput {
            module: "autonomy_spawn".to_string(),
        });
        assert_eq!(out.schema_id, "autonomy_backlog_autoscale");
        assert_eq!(out.schema_version, "1.0.0");
        assert_eq!(out.module, "autonomy_spawn");
        assert_eq!(out.current_cells, 0.0);
        assert_eq!(out.target_cells, 0.0);
        assert_eq!(out.last_run_ts, None);
    }

    #[test]
    fn autoscale_json_default_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "default_backlog_autoscale_state",
            "default_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale"
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale default_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"default_backlog_autoscale_state\""));
    }

    #[test]
    fn normalize_backlog_autoscale_state_normalizes_cells_and_strings() {
        let out = compute_normalize_backlog_autoscale_state(&NormalizeBacklogAutoscaleStateInput {
            module: "autonomy_backlog_autoscale".to_string(),
            src: Some(serde_json::json!({
                "module": " autonomy_backlog_autoscale ",
                "current_cells": 2.8,
                "target_cells": "5",
                "last_run_ts": " 2026-03-04T00:00:00.000Z ",
                "last_high_pressure_ts": "",
                "last_action": " scale_up ",
                "updated_at": null
            })),
        });
        assert_eq!(out.module, "autonomy_backlog_autoscale");
        assert_eq!(out.current_cells, 2.8);
        assert_eq!(out.target_cells, 5.0);
        assert_eq!(
            out.last_run_ts,
            Some("2026-03-04T00:00:00.000Z".to_string())
        );
        assert_eq!(out.last_high_pressure_ts, None);
        assert_eq!(out.last_action, Some("scale_up".to_string()));
        assert_eq!(out.updated_at, None);
    }

    #[test]
    fn autoscale_json_normalize_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_backlog_autoscale_state",
            "normalize_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale",
                "src": {
                    "current_cells": 1,
                    "target_cells": 3
                }
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale normalize_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"normalize_backlog_autoscale_state\""));
    }

    #[test]
    fn spawn_allocated_cells_prefers_active_then_current_then_allocated() {
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: Some(4.2),
            current_cells: Some(7.0),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(4));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: Some(7.8),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(7));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: None,
            allocated_cells: Some(2.0),
        });
        assert_eq!(out.active_cells, Some(2));
    }

    #[test]
    fn autoscale_json_spawn_allocated_cells_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_allocated_cells",
            "spawn_allocated_cells_input": {
                "active_cells": null,
                "current_cells": 3.4,
                "allocated_cells": 8
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_allocated_cells");
        assert!(out.contains("\"mode\":\"spawn_allocated_cells\""));
    }

    #[test]
    fn spawn_capacity_boost_snapshot_counts_recent_spawn_grants() {
        let out = compute_spawn_capacity_boost_snapshot(&SpawnCapacityBoostSnapshotInput {
            enabled: true,
            lookback_minutes: 30.0,
            min_granted_cells: 1.0,
            now_ms: 1_700_000_000_000.0,
            rows: vec![
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:13:20.000Z".to_string()),
                    granted_cells: Some(2.0),
                },
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:12:20.000Z".to_string()),
                    granted_cells: Some(1.0),
                },
            ],
        });
        assert!(out.active);
        assert_eq!(out.grant_count, 2);
        assert_eq!(out.granted_cells, 3.0);
        assert_eq!(out.latest_ts, Some("2023-11-14T22:12:20.000Z".to_string()));
    }

    #[test]
    fn autoscale_json_spawn_capacity_boost_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_capacity_boost_snapshot",
            "spawn_capacity_boost_snapshot_input": {
                "enabled": true,
                "lookback_minutes": 30,
                "min_granted_cells": 1,
                "now_ms": 1700000000000i64,
                "rows": [
                    {
                        "type": "spawn_request",
                        "ts": "2023-11-14T22:13:20.000Z",
                        "granted_cells": 1
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_capacity_boost_snapshot");
        assert!(out.contains("\"mode\":\"spawn_capacity_boost_snapshot\""));
    }

    #[test]
    fn inversion_maturity_score_matches_expected_banding() {
        let out = compute_inversion_maturity_score(&InversionMaturityScoreInput {
            total_tests: 40.0,
            passed_tests: 32.0,
            destructive_failures: 1.0,
            target_test_count: 40.0,
            weight_pass_rate: 0.5,
            weight_non_destructive_rate: 0.3,
            weight_experience: 0.2,
            band_novice: 0.25,
            band_developing: 0.45,
            band_mature: 0.65,
            band_seasoned: 0.82,
        });
        assert_eq!(out.band, "legendary");
        assert!(out.score >= 0.82);
        assert!(out.pass_rate >= 0.8 && out.pass_rate <= 0.81);
        assert!(out.non_destructive_rate >= 0.97 && out.non_destructive_rate <= 0.98);
    }

    #[test]
    fn autoscale_json_inversion_maturity_score_path_works() {
        let payload = serde_json::json!({
            "mode": "inversion_maturity_score",
            "inversion_maturity_score_input": {
                "total_tests": 10,
                "passed_tests": 6,
                "destructive_failures": 1,
                "target_test_count": 40,
                "weight_pass_rate": 0.5,
                "weight_non_destructive_rate": 0.3,
                "weight_experience": 0.2,
                "band_novice": 0.25,
                "band_developing": 0.45,
                "band_mature": 0.65,
                "band_seasoned": 0.82
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale inversion_maturity_score");
        assert!(out.contains("\"mode\":\"inversion_maturity_score\""));
    }

    fn extract_mode_literals(text: &str, call_name: &str) -> std::collections::BTreeSet<String> {
        let pattern = format!(r#"{}\s*\(\s*['"`]([^'"`]+)['"`]"#, regex::escape(call_name));
        let re = Regex::new(&pattern).expect("valid call regex");
        let static_mode_re =
            Regex::new(r"^[a-zA-Z0-9_-]+$").expect("valid static mode token regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty() && static_mode_re.is_match(mode))
            .collect()
    }

    fn extract_bridge_modes(text: &str, fn_name: &str) -> std::collections::BTreeSet<String> {
        let section_re = Regex::new(&format!(
            r#"(?s)function {}\s*\([^)]*\)\s*\{{.*?const fieldByMode:\s*AnyObj\s*=\s*\{{(.*?)\}}\s*(?:;|\r?\n)?"#,
            regex::escape(fn_name)
        ))
        .expect("valid section regex");
        let keys_re = Regex::new(r#"(?m)^\s*(?:([a-zA-Z0-9_]+)|['"]([^'"]+)['"])\s*:"#)
            .expect("valid key regex");
        let Some(section) = section_re
            .captures(text)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        else {
            return std::collections::BTreeSet::new();
        };
        keys_re
            .captures_iter(section)
            .filter_map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().trim().to_string())
            })
            .filter(|key| !key.is_empty())
            .collect()
    }

    fn extract_dispatch_modes(text: &str) -> std::collections::BTreeSet<String> {
        let re = Regex::new(r#"(?m)^\s*(?:if|else if) mode == "([^"]+)""#)
            .expect("valid dispatch regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty())
            .collect()
    }

    #[test]
    fn extract_mode_literals_accepts_all_quote_styles() {
        let text = r#"
const a = runBacklogAutoscalePrimitive("alpha", {});
const b = runBacklogAutoscalePrimitive('beta', {});
const c = runBacklogAutoscalePrimitive(`gamma`, {});
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta", "gamma"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_accepts_quoted_and_unquoted_keys() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    "beta-mode": "payload_beta",
    'gamma_mode': "payload_gamma"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_allows_non_string_values() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: payloadAlpha,
    "beta-mode": payloadBeta,
    'gamma_mode': payloadGamma
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_selects_requested_function_section() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha"
  };
}
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed_backlog = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected_backlog = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_backlog, expected_backlog);

        let parsed_other = extract_bridge_modes(bridge, "runOtherPrimitive");
        let expected_other = ["rogue"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_other, expected_other);
    }

    #[test]
    fn extract_bridge_modes_allows_missing_trailing_semicolon() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    beta: "payload_beta"
  }
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_returns_empty_when_function_missing() {
        let bridge = r#"
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        assert!(parsed.is_empty());
    }

    #[test]
    fn extract_bridge_modes_supports_crlf_lines() {
        let bridge = "function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {\r\n  const fieldByMode: AnyObj = {\r\n    alpha: \"payload_alpha\",\r\n    beta: \"payload_beta\"\r\n  }\r\n}\r\n";
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_dynamic_template_modes() {
        let text = r#"
const a = runBacklogAutoscalePrimitive("alpha", {});
const b = runBacklogAutoscalePrimitive(`beta_${suffix}`, {});
const c = runBacklogAutoscalePrimitive(modeName, {});
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_commented_calls() {
        let text = r#"
// runBacklogAutoscalePrimitive("ignored_line", {});
/* runBacklogAutoscalePrimitive("ignored_block", {}); */
const a = runBacklogAutoscalePrimitive(
  "alpha",
  {}
);
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_accepts_if_and_else_if() {
        let text = r#"
if mode == "alpha" {
}
else if mode == "beta" {
}
if another == "gamma" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_ignores_commented_branches() {
        let text = r#"
// if mode == "ignored_line" {
// }
/* else if mode == "ignored_block" {
} */
if mode == "alpha" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    fn read_optional_autonomy_surface(rel: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_default()
    }

    #[test]
    fn backlog_bridge_is_wrapper_only_in_coreized_layout() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let mut called = extract_mode_literals(&ts_autonomy, "runBacklogAutoscalePrimitive");
        called.extend(extract_mode_literals(
            &ts_inversion,
            "runBacklogAutoscalePrimitive",
        ));
        if bridge.is_empty() {
            assert!(
                called.is_empty(),
                "coreized wrappers should not carry backlog autoscale mode calls"
            );
            return;
        }
        assert!(
            bridge.contains("createLegacyRetiredModule"),
            "backlog_autoscale_rust_bridge.js must remain a thin wrapper"
        );
        assert!(
            !bridge.contains("fieldByMode"),
            "wrapper-only bridge must not contain legacy mode maps"
        );
        assert!(
            called.is_empty(),
            "coreized wrappers should not carry backlog autoscale mode calls"
        );
    }

    #[test]
    fn controller_callsite_modes_are_dispatched_by_rust_autoscale_json() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let rust_src = include_str!("../autoscale.rs");
        let mut called = extract_mode_literals(&ts_autonomy, "runBacklogAutoscalePrimitive");
        called.extend(extract_mode_literals(
            &ts_inversion,
            "runBacklogAutoscalePrimitive",
        ));
        if !(ts_autonomy.is_empty() && ts_inversion.is_empty()) {
            assert!(
                ts_autonomy.contains("createOpsLaneBridge")
                    || ts_inversion.contains("createLegacyRetiredModule"),
                "expected thin-wrapper bridge markers in autonomy wrappers"
            );
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = called.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "controller TS sources use autoscale modes not dispatched by Rust autoscale_json: {:?}",
            missing
        );
    }

    #[test]
    fn rust_dispatch_covers_all_backlog_bridge_modes() {
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let rust_src = include_str!("../autoscale.rs");
        if bridge.is_empty() {
            return;
        }
        let mapped = extract_bridge_modes(&bridge, "runBacklogAutoscalePrimitive");
        if mapped.is_empty() {
            assert!(
                bridge.contains("createLegacyRetiredModule"),
                "wrapper-only bridge expected when map literals are retired"
            );
            return;
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = mapped.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "backlog bridge maps modes not dispatched by Rust autoscale_json: {:?}",
            missing
        );
    }
}
