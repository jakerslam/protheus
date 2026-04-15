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
