        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_intent_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_intent_proposal_encode_failed:{e}"));
    }
    if mode == "unlinked_optimization_admission" {
        let input = request
            .unlinked_optimization_admission_input
            .ok_or_else(|| "autoscale_missing_unlinked_optimization_admission_input".to_string())?;
        let out = compute_unlinked_optimization_admission(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "unlinked_optimization_admission",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_unlinked_optimization_admission_encode_failed:{e}"));
    }
    if mode == "optimization_good_enough" {
        let input = request
            .optimization_good_enough_input
            .ok_or_else(|| "autoscale_missing_optimization_good_enough_input".to_string())?;
        let out = compute_optimization_good_enough(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_good_enough",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_good_enough_encode_failed:{e}"));
    }
    if mode == "proposal_dependency_summary" {
        let input = request
            .proposal_dependency_summary_input
            .ok_or_else(|| "autoscale_missing_proposal_dependency_summary_input".to_string())?;
        let out = compute_proposal_dependency_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_dependency_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_dependency_summary_encode_failed:{e}"));
    }
    if mode == "choose_selection_mode" {
        let input = request
            .choose_selection_mode_input
            .ok_or_else(|| "autoscale_missing_choose_selection_mode_input".to_string())?;
        let out = compute_choose_selection_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "choose_selection_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_choose_selection_mode_encode_failed:{e}"));
    }
    if mode == "explore_quota_for_day" {
        let input = request
            .explore_quota_for_day_input
            .ok_or_else(|| "autoscale_missing_explore_quota_for_day_input".to_string())?;
        let out = compute_explore_quota_for_day(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "explore_quota_for_day",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_explore_quota_for_day_encode_failed:{e}"));
    }
    if mode == "medium_risk_thresholds" {
        let input = request
            .medium_risk_thresholds_input
            .ok_or_else(|| "autoscale_missing_medium_risk_thresholds_input".to_string())?;
        let out = compute_medium_risk_thresholds(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "medium_risk_thresholds",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_medium_risk_thresholds_encode_failed:{e}"));
    }
    if mode == "medium_risk_gate_decision" {
        let input = request
            .medium_risk_gate_decision_input
            .ok_or_else(|| "autoscale_missing_medium_risk_gate_decision_input".to_string())?;
        let out = compute_medium_risk_gate_decision(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "medium_risk_gate_decision",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_medium_risk_gate_decision_encode_failed:{e}"));
    }
    if mode == "route_block_prefilter" {
        let input = request
            .route_block_prefilter_input
            .ok_or_else(|| "autoscale_missing_route_block_prefilter_input".to_string())?;
        let out = compute_route_block_prefilter(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "route_block_prefilter",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_route_block_prefilter_encode_failed:{e}"));
    }
    if mode == "route_execution_sample_event" {
        let input = request
            .route_execution_sample_event_input
            .ok_or_else(|| "autoscale_missing_route_execution_sample_event_input".to_string())?;
        let out = compute_route_execution_sample_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "route_execution_sample_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_route_execution_sample_event_encode_failed:{e}"));
    }
    if mode == "route_block_telemetry_summary" {
        let input = request
            .route_block_telemetry_summary_input
            .ok_or_else(|| "autoscale_missing_route_block_telemetry_summary_input".to_string())?;
        let out = compute_route_block_telemetry_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "route_block_telemetry_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_route_block_telemetry_summary_encode_failed:{e}"));
    }
    if mode == "is_stub_proposal" {
        let input = request
            .is_stub_proposal_input
            .ok_or_else(|| "autoscale_missing_is_stub_proposal_input".to_string())?;
        let out = compute_is_stub_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_stub_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_stub_proposal_encode_failed:{e}"));
    }
    if mode == "recent_autonomy_run_events" {
        let input = request
            .recent_autonomy_run_events_input
            .ok_or_else(|| "autoscale_missing_recent_autonomy_run_events_input".to_string())?;
        let out = compute_recent_autonomy_run_events(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "recent_autonomy_run_events",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_recent_autonomy_run_events_encode_failed:{e}"));
    }
    if mode == "proposal_meta_index" {
        let input = request
            .proposal_meta_index_input
            .ok_or_else(|| "autoscale_missing_proposal_meta_index_input".to_string())?;
        let out = compute_proposal_meta_index(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_meta_index",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_meta_index_encode_failed:{e}"));
    }
    if mode == "new_log_events" {
        let input = request
            .new_log_events_input
            .ok_or_else(|| "autoscale_missing_new_log_events_input".to_string())?;
        let out = compute_new_log_events(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "new_log_events",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_new_log_events_encode_failed:{e}"));
    }
    if mode == "outcome_buckets" {
        let input = request
            .outcome_buckets_input
            .unwrap_or(OutcomeBucketsInput {});
        let out = compute_outcome_buckets(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "outcome_buckets",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_outcome_buckets_encode_failed:{e}"));
    }
    if mode == "recent_run_events" {
        let input = request
            .recent_run_events_input
            .ok_or_else(|| "autoscale_missing_recent_run_events_input".to_string())?;
        let out = compute_recent_run_events(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "recent_run_events",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_recent_run_events_encode_failed:{e}"));
    }
    if mode == "all_decision_events" {
        let input = request
            .all_decision_events_input
            .ok_or_else(|| "autoscale_missing_all_decision_events_input".to_string())?;
        let out = compute_all_decision_events(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "all_decision_events",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_all_decision_events_encode_failed:{e}"));
    }
    if mode == "cooldown_active_state" {
        let input = request
            .cooldown_active_state_input
            .ok_or_else(|| "autoscale_missing_cooldown_active_state_input".to_string())?;
        let out = compute_cooldown_active_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "cooldown_active_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_cooldown_active_state_encode_failed:{e}"));
    }
    if mode == "bump_count" {
        let input = request
            .bump_count_input
            .ok_or_else(|| "autoscale_missing_bump_count_input".to_string())?;
        let out = compute_bump_count(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "bump_count",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_bump_count_encode_failed:{e}"));
    }
    if mode == "lock_age_minutes" {
        let input = request
            .lock_age_minutes_input
            .ok_or_else(|| "autoscale_missing_lock_age_minutes_input".to_string())?;
        let out = compute_lock_age_minutes(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "lock_age_minutes",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_lock_age_minutes_encode_failed:{e}"));
    }
    if mode == "hash_obj" {
        let input = request
            .hash_obj_input
            .ok_or_else(|| "autoscale_missing_hash_obj_input".to_string())?;
        let out = compute_hash_obj(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "hash_obj",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_hash_obj_encode_failed:{e}"));
    }
    if mode == "assess_success_criteria_quality" {
        let input = request
            .assess_success_criteria_quality_input
            .ok_or_else(|| "autoscale_missing_assess_success_criteria_quality_input".to_string())?;
        let out = compute_assess_success_criteria_quality(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "assess_success_criteria_quality",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_assess_success_criteria_quality_encode_failed:{e}"));
    }
    if mode == "manual_gate_prefilter" {
        let input = request
            .manual_gate_prefilter_input
            .ok_or_else(|| "autoscale_missing_manual_gate_prefilter_input".to_string())?;
        let out = compute_manual_gate_prefilter(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "manual_gate_prefilter",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_manual_gate_prefilter_encode_failed:{e}"));
    }
    if mode == "execute_confidence_cooldown_active" {
        let input = request
            .execute_confidence_cooldown_active_input
            .ok_or_else(|| {
                "autoscale_missing_execute_confidence_cooldown_active_input".to_string()
            })?;
        let out = compute_execute_confidence_cooldown_active(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execute_confidence_cooldown_active",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execute_confidence_cooldown_active_encode_failed:{e}"));
    }
    if mode == "top_biases_summary" {
        let input = request
            .top_biases_summary_input
            .ok_or_else(|| "autoscale_missing_top_biases_summary_input".to_string())?;
        let out = compute_top_biases_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "top_biases_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_top_biases_summary_encode_failed:{e}"));
    }
    if mode == "criteria_pattern_penalty" {
        let input = request
            .criteria_pattern_penalty_input
            .ok_or_else(|| "autoscale_missing_criteria_pattern_penalty_input".to_string())?;
        let out = compute_criteria_pattern_penalty(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "criteria_pattern_penalty",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_criteria_pattern_penalty_encode_failed:{e}"));
    }
    if mode == "strategy_threshold_overrides" {
        let input = request
            .strategy_threshold_overrides_input
            .ok_or_else(|| "autoscale_missing_strategy_threshold_overrides_input".to_string())?;
        let out = compute_strategy_threshold_overrides(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_threshold_overrides",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_threshold_overrides_encode_failed:{e}"));
    }
    if mode == "effective_allowed_risks" {
        let input = request
            .effective_allowed_risks_input
            .ok_or_else(|| "autoscale_missing_effective_allowed_risks_input".to_string())?;
        let out = compute_effective_allowed_risks(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "effective_allowed_risks",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_effective_allowed_risks_encode_failed:{e}"));
    }
    if mode == "directive_pulse_stats" {
        let input = request
            .directive_pulse_stats_input
            .ok_or_else(|| "autoscale_missing_directive_pulse_stats_input".to_string())?;
        let out = compute_directive_pulse_stats(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_pulse_stats",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_pulse_stats_encode_failed:{e}"));
    }
    if mode == "compile_directive_pulse_objectives" {
        let input = request
            .compile_directive_pulse_objectives_input
            .ok_or_else(|| {
                "autoscale_missing_compile_directive_pulse_objectives_input".to_string()
            })?;
        let out = compute_compile_directive_pulse_objectives(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "compile_directive_pulse_objectives",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_compile_directive_pulse_objectives_encode_failed:{e}"));
    }
    if mode == "directive_pulse_objectives_profile" {
        let input = request
            .directive_pulse_objectives_profile_input
            .ok_or_else(|| {
                "autoscale_missing_directive_pulse_objectives_profile_input".to_string()
            })?;
        let out = compute_directive_pulse_objectives_profile(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_pulse_objectives_profile",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_pulse_objectives_profile_encode_failed:{e}"));
    }
    if mode == "recent_directive_pulse_cooldown_count" {
        let input = request
            .recent_directive_pulse_cooldown_count_input
            .ok_or_else(|| {
                "autoscale_missing_recent_directive_pulse_cooldown_count_input".to_string()
            })?;
        let out = compute_recent_directive_pulse_cooldown_count(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "recent_directive_pulse_cooldown_count",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_recent_directive_pulse_cooldown_count_encode_failed:{e}"));
    }
    if mode == "proposal_directive_text" {
        let input = request
            .proposal_directive_text_input
            .ok_or_else(|| "autoscale_missing_proposal_directive_text_input".to_string())?;
        let out = compute_proposal_directive_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_directive_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_directive_text_encode_failed:{e}"));
    }
    if mode == "objective_ids_from_pulse_context" {
        let input = request
            .objective_ids_from_pulse_context_input
            .ok_or_else(|| {
                "autoscale_missing_objective_ids_from_pulse_context_input".to_string()
            })?;
        let out = compute_objective_ids_from_pulse_context(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "objective_ids_from_pulse_context",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_objective_ids_from_pulse_context_encode_failed:{e}"));
    }
    if mode == "policy_hold_objective_context" {
        let input = request
            .policy_hold_objective_context_input
            .ok_or_else(|| "autoscale_missing_policy_hold_objective_context_input".to_string())?;
        let out = compute_policy_hold_objective_context(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_objective_context",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_objective_context_encode_failed:{e}"));
    }
    if mode == "proposal_semantic_objective_id" {
        let input = request
            .proposal_semantic_objective_id_input
            .ok_or_else(|| "autoscale_missing_proposal_semantic_objective_id_input".to_string())?;
        let out = compute_proposal_semantic_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_semantic_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_semantic_objective_id_encode_failed:{e}"));
    }
    if mode == "criteria_pattern_keys" {
