// FILE_SIZE_EXCEPTION: reason=Single dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-12
pub fn run_autoscale_json(payload_json: &str) -> Result<String, String> {
    let request: AutoscaleRequest = serde_json::from_str(payload_json)
        .map_err(|e| format!("autoscale_request_parse_failed:{e}"))?;
    let mode_raw = request.mode.trim().to_ascii_lowercase();
    if mode_raw.is_empty() {
        return Err("autoscale_mode_missing".to_string());
    }
    let mode = mode_raw.replace('-', "_").replace(' ', "_");
    let mode = match mode.as_str() {
        "scale_plan" | "plan_scale" => "plan".to_string(),
        "batchmax" => "batch_max".to_string(),
        "dynamiccaps" => "dynamic_caps".to_string(),
        "normalizequeue" => "normalize_queue".to_string(),
        "tokenusage" => "token_usage".to_string(),
        "criteria" | "criteria_check" => "criteria_gate".to_string(),
        _ => mode,
    };
    if mode == "default_criteria_pattern_memory" {
        let input = request
            .default_criteria_pattern_memory_input
            .ok_or_else(|| "autoscale_missing_default_criteria_pattern_memory_input".to_string())?;
        let out = compute_default_criteria_pattern_memory(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "default_criteria_pattern_memory",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_default_criteria_pattern_memory_encode_failed:{e}"));
    }
    if mode == "strategy_execution_mode_effective" {
        let input = request
            .strategy_execution_mode_effective_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_execution_mode_effective_input".to_string()
            })?;
        let out = compute_strategy_execution_mode_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_execution_mode_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_execution_mode_effective_encode_failed:{e}"));
    }
    if mode == "strategy_canary_exec_limit_effective" {
        let input = request
            .strategy_canary_exec_limit_effective_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_canary_exec_limit_effective_input".to_string()
            })?;
        let out = compute_strategy_canary_exec_limit_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_canary_exec_limit_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_canary_exec_limit_effective_encode_failed:{e}"));
    }
    if mode == "strategy_exploration_effective" {
        let input = request
            .strategy_exploration_effective_input
            .ok_or_else(|| "autoscale_missing_strategy_exploration_effective_input".to_string())?;
        let out = compute_strategy_exploration_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_exploration_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_exploration_effective_encode_failed:{e}"));
    }
    if mode == "strategy_budget_effective" {
        let input = request
            .strategy_budget_effective_input
            .ok_or_else(|| "autoscale_missing_strategy_budget_effective_input".to_string())?;
        let out = compute_strategy_budget_effective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_budget_effective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_budget_effective_encode_failed:{e}"));
    }
    if mode == "preexec_verdict_from_signals" {
        let input = request
            .preexec_verdict_from_signals_input
            .ok_or_else(|| "autoscale_missing_preexec_verdict_from_signals_input".to_string())?;
        let out = compute_preexec_verdict_from_signals(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "preexec_verdict_from_signals",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_preexec_verdict_from_signals_encode_failed:{e}"));
    }
    if mode == "score_only_proposal_churn" {
        let input = request
            .score_only_proposal_churn_input
            .ok_or_else(|| "autoscale_missing_score_only_proposal_churn_input".to_string())?;
        let out = compute_score_only_proposal_churn(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_proposal_churn",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_proposal_churn_encode_failed:{e}"));
    }
    if mode == "success_criteria_quality_audit" {
        let input = request
            .success_criteria_quality_audit_input
            .ok_or_else(|| "autoscale_missing_success_criteria_quality_audit_input".to_string())?;
        let out = compute_success_criteria_quality_audit(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_quality_audit",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_quality_audit_encode_failed:{e}"));
    }
    if mode == "detect_eyes_terminology_drift" {
        let input = request
            .detect_eyes_terminology_drift_input
            .ok_or_else(|| "autoscale_missing_detect_eyes_terminology_drift_input".to_string())?;
        let out = compute_detect_eyes_terminology_drift(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "detect_eyes_terminology_drift",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_detect_eyes_terminology_drift_encode_failed:{e}"));
    }
    if mode == "normalize_stored_proposal_row" {
        let input = request
            .normalize_stored_proposal_row_input
            .ok_or_else(|| "autoscale_missing_normalize_stored_proposal_row_input".to_string())?;
        let out = compute_normalize_stored_proposal_row(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_stored_proposal_row",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_stored_proposal_row_encode_failed:{e}"));
    }
    if mode == "default_backlog_autoscale_state" {
        let input = request
            .default_backlog_autoscale_state_input
            .ok_or_else(|| "autoscale_missing_default_backlog_autoscale_state_input".to_string())?;
        let out = compute_default_backlog_autoscale_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "default_backlog_autoscale_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_default_backlog_autoscale_state_encode_failed:{e}"));
    }
    if mode == "normalize_backlog_autoscale_state" {
        let input = request
            .normalize_backlog_autoscale_state_input
            .ok_or_else(|| {
                "autoscale_missing_normalize_backlog_autoscale_state_input".to_string()
            })?;
        let out = compute_normalize_backlog_autoscale_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_backlog_autoscale_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_backlog_autoscale_state_encode_failed:{e}"));
    }
    if mode == "spawn_allocated_cells" {
        let input = request
            .spawn_allocated_cells_input
            .ok_or_else(|| "autoscale_missing_spawn_allocated_cells_input".to_string())?;
        let out = compute_spawn_allocated_cells(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "spawn_allocated_cells",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_spawn_allocated_cells_encode_failed:{e}"));
    }
    if mode == "spawn_capacity_boost_snapshot" {
        let input = request
            .spawn_capacity_boost_snapshot_input
            .ok_or_else(|| "autoscale_missing_spawn_capacity_boost_snapshot_input".to_string())?;
        let out = compute_spawn_capacity_boost_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "spawn_capacity_boost_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_spawn_capacity_boost_snapshot_encode_failed:{e}"));
    }
    if mode == "inversion_maturity_score" {
        let input = request
            .inversion_maturity_score_input
            .ok_or_else(|| "autoscale_missing_inversion_maturity_score_input".to_string())?;
        let out = compute_inversion_maturity_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "inversion_maturity_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_inversion_maturity_score_encode_failed:{e}"));
    }
    if mode == "plan" {
        let input = request
            .plan_input
            .ok_or_else(|| "autoscale_missing_plan_input".to_string())?;
        let out = compute_plan(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "plan",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_plan_encode_failed:{e}"));
    }
    if mode == "batch_max" {
        let input = request
            .batch_input
            .ok_or_else(|| "autoscale_missing_batch_input".to_string())?;
        let out = compute_batch_max(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "batch_max",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_batch_encode_failed:{e}"));
    }
    if mode == "dynamic_caps" {
        let input = request
            .dynamic_caps_input
            .ok_or_else(|| "autoscale_missing_dynamic_caps_input".to_string())?;
        let out = compute_dynamic_caps(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "dynamic_caps",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_dynamic_caps_encode_failed:{e}"));
    }
    if mode == "token_usage" {
        let input = request
            .token_usage_input
            .ok_or_else(|| "autoscale_missing_token_usage_input".to_string())?;
        let out = compute_token_usage(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "token_usage",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_token_usage_encode_failed:{e}"));
    }
    if mode == "normalize_queue" {
        let input = request
            .normalize_queue_input
            .ok_or_else(|| "autoscale_missing_normalize_queue_input".to_string())?;
        let out = compute_normalize_queue(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_queue",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_queue_encode_failed:{e}"));
    }
    if mode == "criteria_gate" {
        let input = request
            .criteria_gate_input
            .ok_or_else(|| "autoscale_missing_criteria_gate_input".to_string())?;
        let out = compute_criteria_gate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "criteria_gate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_criteria_gate_encode_failed:{e}"));
    }
    if mode == "structural_preview_criteria_failure" {
        let input = request
            .structural_preview_criteria_failure_input
            .ok_or_else(|| {
                "autoscale_missing_structural_preview_criteria_failure_input".to_string()
            })?;
        let out = compute_structural_preview_criteria_failure(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "structural_preview_criteria_failure",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_structural_preview_criteria_failure_encode_failed:{e}"));
    }
    if mode == "policy_hold" {
        let input = request
            .policy_hold_input
            .ok_or_else(|| "autoscale_missing_policy_hold_input".to_string())?;
        let out = compute_policy_hold(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_encode_failed:{e}"));
    }
    if mode == "policy_hold_result" {
        let input = request
            .policy_hold_result_input
            .ok_or_else(|| "autoscale_missing_policy_hold_result_input".to_string())?;
        let out = compute_policy_hold_result(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_result",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_result_encode_failed:{e}"));
    }
    if mode == "policy_hold_run_event" {
        let input = request
            .policy_hold_run_event_input
            .ok_or_else(|| "autoscale_missing_policy_hold_run_event_input".to_string())?;
        let out = compute_policy_hold_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_run_event_encode_failed:{e}"));
    }
    if mode == "dod_evidence_diff" {
        let input = request
            .dod_evidence_diff_input
            .ok_or_else(|| "autoscale_missing_dod_evidence_diff_input".to_string())?;
        let out = compute_dod_evidence_diff(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "dod_evidence_diff",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_dod_evidence_diff_encode_failed:{e}"));
    }
    if mode == "score_only_result" {
        let input = request
            .score_only_result_input
            .ok_or_else(|| "autoscale_missing_score_only_result_input".to_string())?;
        let out = compute_score_only_result(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_result",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_result_encode_failed:{e}"));
    }
    if mode == "score_only_failure_like" {
        let input = request
            .score_only_failure_like_input
            .ok_or_else(|| "autoscale_missing_score_only_failure_like_input".to_string())?;
        let out = compute_score_only_failure_like(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "score_only_failure_like",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_score_only_failure_like_encode_failed:{e}"));
    }
    if mode == "gate_exhausted_attempt" {
        let input = request
            .gate_exhausted_attempt_input
            .ok_or_else(|| "autoscale_missing_gate_exhausted_attempt_input".to_string())?;
        let out = compute_gate_exhausted_attempt(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "gate_exhausted_attempt",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_gate_exhausted_attempt_encode_failed:{e}"));
    }
    if mode == "consecutive_gate_exhausted_attempts" {
        let input = request
            .consecutive_gate_exhausted_attempts_input
            .ok_or_else(|| {
                "autoscale_missing_consecutive_gate_exhausted_attempts_input".to_string()
            })?;
        let out = compute_consecutive_gate_exhausted_attempts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "consecutive_gate_exhausted_attempts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_consecutive_gate_exhausted_attempts_encode_failed:{e}"));
    }
    if mode == "runs_since_reset_index" {
        let input = request
            .runs_since_reset_index_input
            .ok_or_else(|| "autoscale_missing_runs_since_reset_index_input".to_string())?;
        let out = compute_runs_since_reset_index(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "runs_since_reset_index",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_runs_since_reset_index_encode_failed:{e}"));
    }
    if mode == "attempt_event_indices" {
        let input = request
            .attempt_event_indices_input
            .ok_or_else(|| "autoscale_missing_attempt_event_indices_input".to_string())?;
        let out = compute_attempt_event_indices(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "attempt_event_indices",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_attempt_event_indices_encode_failed:{e}"));
    }
    if mode == "capacity_counted_attempt_indices" {
        let input = request
            .capacity_counted_attempt_indices_input
            .ok_or_else(|| {
                "autoscale_missing_capacity_counted_attempt_indices_input".to_string()
            })?;
        let out = compute_capacity_counted_attempt_indices(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capacity_counted_attempt_indices",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capacity_counted_attempt_indices_encode_failed:{e}"));
    }
    if mode == "consecutive_no_progress_runs" {
        let input = request
            .consecutive_no_progress_runs_input
            .ok_or_else(|| "autoscale_missing_consecutive_no_progress_runs_input".to_string())?;
        let out = compute_consecutive_no_progress_runs(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "consecutive_no_progress_runs",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_consecutive_no_progress_runs_encode_failed:{e}"));
    }
    if mode == "shipped_count" {
        let input = request
            .shipped_count_input
            .ok_or_else(|| "autoscale_missing_shipped_count_input".to_string())?;
        let out = compute_shipped_count(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "shipped_count",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_shipped_count_encode_failed:{e}"));
    }
    if mode == "executed_count_by_risk" {
        let input = request
            .executed_count_by_risk_input
            .ok_or_else(|| "autoscale_missing_executed_count_by_risk_input".to_string())?;
        let out = compute_executed_count_by_risk(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "executed_count_by_risk",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_executed_count_by_risk_encode_failed:{e}"));
    }
    if mode == "run_result_tally" {
        let input = request
            .run_result_tally_input
            .ok_or_else(|| "autoscale_missing_run_result_tally_input".to_string())?;
        let out = compute_run_result_tally(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "run_result_tally",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_run_result_tally_encode_failed:{e}"));
    }
    if mode == "qos_lane_weights" {
        let input = request
            .qos_lane_weights_input
            .ok_or_else(|| "autoscale_missing_qos_lane_weights_input".to_string())?;
        let out = compute_qos_lane_weights(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_weights",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_weights_encode_failed:{e}"));
    }
    if mode == "qos_lane_usage" {
        let input = request
            .qos_lane_usage_input
            .ok_or_else(|| "autoscale_missing_qos_lane_usage_input".to_string())?;
        let out = compute_qos_lane_usage(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_usage",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_usage_encode_failed:{e}"));
    }
    if mode == "qos_lane_share_cap_exceeded" {
        let input = request
            .qos_lane_share_cap_exceeded_input
            .ok_or_else(|| "autoscale_missing_qos_lane_share_cap_exceeded_input".to_string())?;
        let out = compute_qos_lane_share_cap_exceeded(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_share_cap_exceeded",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_share_cap_exceeded_encode_failed:{e}"));
    }
    if mode == "qos_lane_from_candidate" {
        let input = request
            .qos_lane_from_candidate_input
            .ok_or_else(|| "autoscale_missing_qos_lane_from_candidate_input".to_string())?;
        let out = compute_qos_lane_from_candidate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "qos_lane_from_candidate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_qos_lane_from_candidate_encode_failed:{e}"));
    }
    if mode == "eye_outcome_count_window" {
        let input = request
            .eye_outcome_count_window_input
            .ok_or_else(|| "autoscale_missing_eye_outcome_count_window_input".to_string())?;
        let out = compute_eye_outcome_count_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "eye_outcome_count_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_eye_outcome_count_window_encode_failed:{e}"));
    }
    if mode == "eye_outcome_count_last_hours" {
        let input = request
            .eye_outcome_count_last_hours_input
            .ok_or_else(|| "autoscale_missing_eye_outcome_count_last_hours_input".to_string())?;
        let out = compute_eye_outcome_count_last_hours(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "eye_outcome_count_last_hours",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_eye_outcome_count_last_hours_encode_failed:{e}"));
    }
    if mode == "sorted_counts" {
        let input = request
            .sorted_counts_input
            .ok_or_else(|| "autoscale_missing_sorted_counts_input".to_string())?;
        let out = compute_sorted_counts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sorted_counts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sorted_counts_encode_failed:{e}"));
    }
    if mode == "normalize_proposal_status" {
        let input = request
            .normalize_proposal_status_input
            .ok_or_else(|| "autoscale_missing_normalize_proposal_status_input".to_string())?;
        let out = compute_normalize_proposal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_proposal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_proposal_status_encode_failed:{e}"));
    }
    if mode == "proposal_status" {
        let input = request
            .proposal_status_input
            .ok_or_else(|| "autoscale_missing_proposal_status_input".to_string())?;
        let out = compute_proposal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_status_encode_failed:{e}"));
    }
    if mode == "proposal_outcome_status" {
        let input = request
            .proposal_outcome_status_input
            .ok_or_else(|| "autoscale_missing_proposal_outcome_status_input".to_string())?;
        let out = compute_proposal_outcome_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_outcome_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_outcome_status_encode_failed:{e}"));
    }
    if mode == "queue_underflow_backfill" {
        let input = request
            .queue_underflow_backfill_input
            .ok_or_else(|| "autoscale_missing_queue_underflow_backfill_input".to_string())?;
        let out = compute_queue_underflow_backfill(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "queue_underflow_backfill",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_queue_underflow_backfill_encode_failed:{e}"));
    }
    if mode == "proposal_risk_score" {
        let input = request
            .proposal_risk_score_input
            .ok_or_else(|| "autoscale_missing_proposal_risk_score_input".to_string())?;
        let out = compute_proposal_risk_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_risk_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_risk_score_encode_failed:{e}"));
    }
    if mode == "proposal_score" {
        let input = request
            .proposal_score_input
            .ok_or_else(|| "autoscale_missing_proposal_score_input".to_string())?;
        let out = compute_proposal_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_score_encode_failed:{e}"));
    }
    if mode == "proposal_admission_preview" {
        let input = request
            .proposal_admission_preview_input
            .ok_or_else(|| "autoscale_missing_proposal_admission_preview_input".to_string())?;
        let out = compute_proposal_admission_preview(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_admission_preview",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_admission_preview_encode_failed:{e}"));
    }
    if mode == "impact_weight" {
        let input = request
            .impact_weight_input
            .ok_or_else(|| "autoscale_missing_impact_weight_input".to_string())?;
        let out = compute_impact_weight(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "impact_weight",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_impact_weight_encode_failed:{e}"));
    }
    if mode == "risk_penalty" {
        let input = request
            .risk_penalty_input
            .ok_or_else(|| "autoscale_missing_risk_penalty_input".to_string())?;
        let out = compute_risk_penalty(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "risk_penalty",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_risk_penalty_encode_failed:{e}"));
    }
    if mode == "estimate_tokens" {
        let input = request
            .estimate_tokens_input
            .ok_or_else(|| "autoscale_missing_estimate_tokens_input".to_string())?;
        let out = compute_estimate_tokens(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "estimate_tokens",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_estimate_tokens_encode_failed:{e}"));
    }
    if mode == "proposal_remediation_depth" {
        let input = request
            .proposal_remediation_depth_input
            .ok_or_else(|| "autoscale_missing_proposal_remediation_depth_input".to_string())?;
        let out = compute_proposal_remediation_depth(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_remediation_depth",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_remediation_depth_encode_failed:{e}"));
    }
    if mode == "proposal_dedup_key" {
        let input = request
            .proposal_dedup_key_input
            .ok_or_else(|| "autoscale_missing_proposal_dedup_key_input".to_string())?;
        let out = compute_proposal_dedup_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_dedup_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_dedup_key_encode_failed:{e}"));
    }
    if mode == "proposal_semantic_fingerprint" {
        let input = request
            .proposal_semantic_fingerprint_input
            .ok_or_else(|| "autoscale_missing_proposal_semantic_fingerprint_input".to_string())?;
        let out = compute_proposal_semantic_fingerprint(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_semantic_fingerprint",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_semantic_fingerprint_encode_failed:{e}"));
    }
    if mode == "semantic_token_similarity" {
        let input = request
            .semantic_token_similarity_input
            .ok_or_else(|| "autoscale_missing_semantic_token_similarity_input".to_string())?;
        let out = compute_semantic_token_similarity(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_token_similarity",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_token_similarity_encode_failed:{e}"));
    }
    if mode == "semantic_context_comparable" {
        let input = request
            .semantic_context_comparable_input
            .ok_or_else(|| "autoscale_missing_semantic_context_comparable_input".to_string())?;
        let out = compute_semantic_context_comparable(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_context_comparable",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_context_comparable_encode_failed:{e}"));
    }
    if mode == "semantic_near_duplicate_match" {
        let input = request
            .semantic_near_duplicate_match_input
            .ok_or_else(|| "autoscale_missing_semantic_near_duplicate_match_input".to_string())?;
        let out = compute_semantic_near_duplicate_match(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "semantic_near_duplicate_match",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_semantic_near_duplicate_match_encode_failed:{e}"));
    }
    if mode == "strategy_rank_score" {
        let input = request
            .strategy_rank_score_input
            .ok_or_else(|| "autoscale_missing_strategy_rank_score_input".to_string())?;
        let out = compute_strategy_rank_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_rank_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_rank_score_encode_failed:{e}"));
    }
    if mode == "strategy_rank_adjusted" {
        let input = request
            .strategy_rank_adjusted_input
            .ok_or_else(|| "autoscale_missing_strategy_rank_adjusted_input".to_string())?;
        let out = compute_strategy_rank_adjusted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_rank_adjusted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_rank_adjusted_encode_failed:{e}"));
    }
    if mode == "trit_shadow_rank_score" {
        let input = request
            .trit_shadow_rank_score_input
            .ok_or_else(|| "autoscale_missing_trit_shadow_rank_score_input".to_string())?;
        let out = compute_trit_shadow_rank_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "trit_shadow_rank_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_trit_shadow_rank_score_encode_failed:{e}"));
    }
    if mode == "strategy_circuit_cooldown" {
        let input = request
            .strategy_circuit_cooldown_input
            .ok_or_else(|| "autoscale_missing_strategy_circuit_cooldown_input".to_string())?;
        let out = compute_strategy_circuit_cooldown(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_circuit_cooldown",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_circuit_cooldown_encode_failed:{e}"));
    }
    if mode == "strategy_trit_shadow_adjusted" {
        let input = request
            .strategy_trit_shadow_adjusted_input
            .ok_or_else(|| "autoscale_missing_strategy_trit_shadow_adjusted_input".to_string())?;
        let out = compute_strategy_trit_shadow_adjusted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_trit_shadow_adjusted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_trit_shadow_adjusted_encode_failed:{e}"));
    }
    if mode == "non_yield_penalty_score" {
        let input = request
            .non_yield_penalty_score_input
            .ok_or_else(|| "autoscale_missing_non_yield_penalty_score_input".to_string())?;
        let out = compute_non_yield_penalty_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "non_yield_penalty_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_non_yield_penalty_score_encode_failed:{e}"));
    }
    if mode == "collective_shadow_adjustments" {
        let input = request
            .collective_shadow_adjustments_input
            .ok_or_else(|| "autoscale_missing_collective_shadow_adjustments_input".to_string())?;
        let out = compute_collective_shadow_adjustments(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collective_shadow_adjustments",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collective_shadow_adjustments_encode_failed:{e}"));
    }
    if mode == "strategy_trit_shadow_ranking_summary" {
        let input = request
            .strategy_trit_shadow_ranking_summary_input
            .ok_or_else(|| {
                "autoscale_missing_strategy_trit_shadow_ranking_summary_input".to_string()
            })?;
        let out = compute_strategy_trit_shadow_ranking_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_trit_shadow_ranking_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_trit_shadow_ranking_summary_encode_failed:{e}"));
    }
    if mode == "shadow_scope_matches" {
        let input = request
            .shadow_scope_matches_input
            .ok_or_else(|| "autoscale_missing_shadow_scope_matches_input".to_string())?;
        let out = compute_shadow_scope_matches(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "shadow_scope_matches",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_shadow_scope_matches_encode_failed:{e}"));
    }
    if mode == "collective_shadow_aggregate" {
        let input = request
            .collective_shadow_aggregate_input
            .ok_or_else(|| "autoscale_missing_collective_shadow_aggregate_input".to_string())?;
        let out = compute_collective_shadow_aggregate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collective_shadow_aggregate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collective_shadow_aggregate_encode_failed:{e}"));
    }
    if mode == "expected_value_signal" {
        let input = request
            .expected_value_signal_input
            .ok_or_else(|| "autoscale_missing_expected_value_signal_input".to_string())?;
        let out = compute_expected_value_signal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "expected_value_signal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_expected_value_signal_encode_failed:{e}"));
    }
    if mode == "value_signal_score" {
        let input = request
            .value_signal_score_input
            .ok_or_else(|| "autoscale_missing_value_signal_score_input".to_string())?;
        let out = compute_value_signal_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "value_signal_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_value_signal_score_encode_failed:{e}"));
    }
    if mode == "composite_eligibility_score" {
        let input = request
            .composite_eligibility_score_input
            .ok_or_else(|| "autoscale_missing_composite_eligibility_score_input".to_string())?;
        let out = compute_composite_eligibility_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "composite_eligibility_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_composite_eligibility_score_encode_failed:{e}"));
    }
    if mode == "time_to_value_score" {
        let input = request
            .time_to_value_score_input
            .ok_or_else(|| "autoscale_missing_time_to_value_score_input".to_string())?;
        let out = compute_time_to_value_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "time_to_value_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_time_to_value_score_encode_failed:{e}"));
    }
    if mode == "value_density_score" {
        let input = request
            .value_density_score_input
            .ok_or_else(|| "autoscale_missing_value_density_score_input".to_string())?;
        let out = compute_value_density_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "value_density_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_value_density_score_encode_failed:{e}"));
    }
    if mode == "normalize_directive_tier" {
        let input = request
            .normalize_directive_tier_input
            .ok_or_else(|| "autoscale_missing_normalize_directive_tier_input".to_string())?;
        let out = compute_normalize_directive_tier(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_directive_tier",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_directive_tier_encode_failed:{e}"));
    }
    if mode == "directive_tier_weight" {
        let input = request
            .directive_tier_weight_input
            .ok_or_else(|| "autoscale_missing_directive_tier_weight_input".to_string())?;
        let out = compute_directive_tier_weight(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_weight",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_weight_encode_failed:{e}"));
    }
    if mode == "directive_tier_min_share" {
        let input = request
            .directive_tier_min_share_input
            .ok_or_else(|| "autoscale_missing_directive_tier_min_share_input".to_string())?;
        let out = compute_directive_tier_min_share(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_min_share",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_min_share_encode_failed:{e}"));
    }
    if mode == "directive_tier_coverage_bonus" {
        let input = request
            .directive_tier_coverage_bonus_input
            .ok_or_else(|| "autoscale_missing_directive_tier_coverage_bonus_input".to_string())?;
        let out = compute_directive_tier_coverage_bonus(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_coverage_bonus",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_coverage_bonus_encode_failed:{e}"));
    }
    if mode == "directive_tier_reservation_need" {
        let input = request
            .directive_tier_reservation_need_input
            .ok_or_else(|| "autoscale_missing_directive_tier_reservation_need_input".to_string())?;
        let out = compute_directive_tier_reservation_need(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_tier_reservation_need",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_tier_reservation_need_encode_failed:{e}"));
    }
    if mode == "pulse_objective_cooldown_active" {
        let input = request
            .pulse_objective_cooldown_active_input
            .ok_or_else(|| "autoscale_missing_pulse_objective_cooldown_active_input".to_string())?;
        let out = compute_pulse_objective_cooldown_active(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "pulse_objective_cooldown_active",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_pulse_objective_cooldown_active_encode_failed:{e}"));
    }
    if mode == "directive_token_hits" {
        let input = request
            .directive_token_hits_input
            .ok_or_else(|| "autoscale_missing_directive_token_hits_input".to_string())?;
        let out = compute_directive_token_hits(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_token_hits",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_token_hits_encode_failed:{e}"));
    }
    if mode == "to_stem" {
        let input = request
            .to_stem_input
            .ok_or_else(|| "autoscale_missing_to_stem_input".to_string())?;
        let out = compute_to_stem(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "to_stem",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_to_stem_encode_failed:{e}"));
    }
    if mode == "normalize_directive_text" {
        let input = request
            .normalize_directive_text_input
            .ok_or_else(|| "autoscale_missing_normalize_directive_text_input".to_string())?;
        let out = compute_normalize_directive_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_directive_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_directive_text_encode_failed:{e}"));
    }
    if mode == "tokenize_directive_text" {
        let input = request
            .tokenize_directive_text_input
            .ok_or_else(|| "autoscale_missing_tokenize_directive_text_input".to_string())?;
        let out = compute_tokenize_directive_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "tokenize_directive_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_tokenize_directive_text_encode_failed:{e}"));
    }
    if mode == "normalize_spaces" {
        let input = request
            .normalize_spaces_input
            .ok_or_else(|| "autoscale_missing_normalize_spaces_input".to_string())?;
        let out = compute_normalize_spaces(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_spaces",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_spaces_encode_failed:{e}"));
    }
    if mode == "parse_lower_list" {
        let input = request
            .parse_lower_list_input
            .ok_or_else(|| "autoscale_missing_parse_lower_list_input".to_string())?;
        let out = compute_parse_lower_list(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_lower_list",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_lower_list_encode_failed:{e}"));
    }
    if mode == "canary_failed_checks_allowed" {
        let input = request
            .canary_failed_checks_allowed_input
            .ok_or_else(|| "autoscale_missing_canary_failed_checks_allowed_input".to_string())?;
        let out = compute_canary_failed_checks_allowed(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "canary_failed_checks_allowed",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_canary_failed_checks_allowed_encode_failed:{e}"));
    }
    if mode == "proposal_text_blob" {
        let input = request
            .proposal_text_blob_input
            .ok_or_else(|| "autoscale_missing_proposal_text_blob_input".to_string())?;
        let out = compute_proposal_text_blob(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_text_blob",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_text_blob_encode_failed:{e}"));
    }
    if mode == "percent_mentions_from_text" {
        let input = request
            .percent_mentions_from_text_input
            .ok_or_else(|| "autoscale_missing_percent_mentions_from_text_input".to_string())?;
        let out = compute_percent_mentions_from_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "percent_mentions_from_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_percent_mentions_from_text_encode_failed:{e}"));
    }
    if mode == "optimization_min_delta_percent" {
        let input = request
            .optimization_min_delta_percent_input
            .ok_or_else(|| "autoscale_missing_optimization_min_delta_percent_input".to_string())?;
        let out = compute_optimization_min_delta_percent(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "optimization_min_delta_percent",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_optimization_min_delta_percent_encode_failed:{e}"));
    }
    if mode == "source_eye_ref" {
        let input = request
            .source_eye_ref_input
            .ok_or_else(|| "autoscale_missing_source_eye_ref_input".to_string())?;
        let out = compute_source_eye_ref(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "source_eye_ref",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_source_eye_ref_encode_failed:{e}"));
    }
    if mode == "normalized_risk" {
        let input = request
            .normalized_risk_input
            .ok_or_else(|| "autoscale_missing_normalized_risk_input".to_string())?;
        let out = compute_normalized_risk(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalized_risk",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalized_risk_encode_failed:{e}"));
    }
    if mode == "parse_iso_ts" {
        let input = request
            .parse_iso_ts_input
            .ok_or_else(|| "autoscale_missing_parse_iso_ts_input".to_string())?;
        let out = compute_parse_iso_ts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_iso_ts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_iso_ts_encode_failed:{e}"));
    }
    if mode == "extract_objective_id_token" {
        let input = request
            .extract_objective_id_token_input
            .ok_or_else(|| "autoscale_missing_extract_objective_id_token_input".to_string())?;
        let out = compute_extract_objective_id_token(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "extract_objective_id_token",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_extract_objective_id_token_encode_failed:{e}"));
    }
    if mode == "normalize_value_currency_token" {
        let input = request
            .normalize_value_currency_token_input
            .ok_or_else(|| "autoscale_missing_normalize_value_currency_token_input".to_string())?;
        let out = compute_normalize_value_currency_token(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_value_currency_token",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_value_currency_token_encode_failed:{e}"));
    }
    if mode == "list_value_currencies" {
        let input = request
            .list_value_currencies_input
            .ok_or_else(|| "autoscale_missing_list_value_currencies_input".to_string())?;
        let out = compute_list_value_currencies(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "list_value_currencies",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_list_value_currencies_encode_failed:{e}"));
    }
    if mode == "infer_value_currencies_from_directive_bits" {
        let input = request
            .infer_value_currencies_from_directive_bits_input
            .ok_or_else(|| {
                "autoscale_missing_infer_value_currencies_from_directive_bits_input".to_string()
            })?;
        let out = compute_infer_value_currencies_from_directive_bits(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "infer_value_currencies_from_directive_bits",
            "payload": out
        }))
        .map_err(|e| {
            format!("autoscale_infer_value_currencies_from_directive_bits_encode_failed:{e}")
        });
    }
    if mode == "has_linked_objective_entry" {
        let input = request
            .has_linked_objective_entry_input
            .ok_or_else(|| "autoscale_missing_has_linked_objective_entry_input".to_string())?;
        let out = compute_has_linked_objective_entry(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_linked_objective_entry",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_linked_objective_entry_encode_failed:{e}"));
    }
    if mode == "verified_entry_outcome" {
        let input = request
            .verified_entry_outcome_input
            .ok_or_else(|| "autoscale_missing_verified_entry_outcome_input".to_string())?;
        let out = compute_verified_entry_outcome(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "verified_entry_outcome",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_verified_entry_outcome_encode_failed:{e}"));
    }
    if mode == "verified_revenue_action" {
        let input = request
            .verified_revenue_action_input
            .ok_or_else(|| "autoscale_missing_verified_revenue_action_input".to_string())?;
        let out = compute_verified_revenue_action(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "verified_revenue_action",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_verified_revenue_action_encode_failed:{e}"));
    }
    if mode == "minutes_until_next_utc_day" {
        let input = request
            .minutes_until_next_utc_day_input
            .ok_or_else(|| "autoscale_missing_minutes_until_next_utc_day_input".to_string())?;
        let out = compute_minutes_until_next_utc_day(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "minutes_until_next_utc_day",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_minutes_until_next_utc_day_encode_failed:{e}"));
    }
    if mode == "age_hours" {
        let input = request
            .age_hours_input
            .ok_or_else(|| "autoscale_missing_age_hours_input".to_string())?;
        let out = compute_age_hours(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "age_hours",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_age_hours_encode_failed:{e}"));
    }
    if mode == "url_domain" {
        let input = request
            .url_domain_input
            .ok_or_else(|| "autoscale_missing_url_domain_input".to_string())?;
        let out = compute_url_domain(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "url_domain",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_url_domain_encode_failed:{e}"));
    }
    if mode == "domain_allowed" {
        let input = request
            .domain_allowed_input
            .ok_or_else(|| "autoscale_missing_domain_allowed_input".to_string())?;
        let out = compute_domain_allowed(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "domain_allowed",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_domain_allowed_encode_failed:{e}"));
    }
    if mode == "is_execute_mode" {
        let input = request
            .is_execute_mode_input
            .ok_or_else(|| "autoscale_missing_is_execute_mode_input".to_string())?;
        let out = compute_is_execute_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_execute_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_execute_mode_encode_failed:{e}"));
    }
    if mode == "execution_allowed_by_feature_flag" {
        let input = request
            .execution_allowed_by_feature_flag_input
            .ok_or_else(|| {
                "autoscale_missing_execution_allowed_by_feature_flag_input".to_string()
            })?;
        let out = compute_execution_allowed_by_feature_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execution_allowed_by_feature_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execution_allowed_by_feature_flag_encode_failed:{e}"));
    }
    if mode == "is_tier1_objective_id" {
        let input = request
            .is_tier1_objective_id_input
            .ok_or_else(|| "autoscale_missing_is_tier1_objective_id_input".to_string())?;
        let out = compute_is_tier1_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_tier1_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_tier1_objective_id_encode_failed:{e}"));
    }
    if mode == "is_tier1_candidate_objective" {
        let input = request
            .is_tier1_candidate_objective_input
            .ok_or_else(|| "autoscale_missing_is_tier1_candidate_objective_input".to_string())?;
        let out = compute_is_tier1_candidate_objective(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_tier1_candidate_objective",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_tier1_candidate_objective_encode_failed:{e}"));
    }
    if mode == "needs_execution_quota" {
        let input = request
            .needs_execution_quota_input
            .ok_or_else(|| "autoscale_missing_needs_execution_quota_input".to_string())?;
        let out = compute_needs_execution_quota(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "needs_execution_quota",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_needs_execution_quota_encode_failed:{e}"));
    }
    if mode == "normalize_criteria_metric" {
        let input = request
            .normalize_criteria_metric_input
            .ok_or_else(|| "autoscale_missing_normalize_criteria_metric_input".to_string())?;
        let out = compute_normalize_criteria_metric(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_criteria_metric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_criteria_metric_encode_failed:{e}"));
    }
    if mode == "escape_reg_exp" {
        let input = request
            .escape_reg_exp_input
            .ok_or_else(|| "autoscale_missing_escape_reg_exp_input".to_string())?;
        let out = compute_escape_reg_exp(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "escape_reg_exp",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_escape_reg_exp_encode_failed:{e}"));
    }
    if mode == "tool_token_mentioned" {
        let input = request
            .tool_token_mentioned_input
            .ok_or_else(|| "autoscale_missing_tool_token_mentioned_input".to_string())?;
        let out = compute_tool_token_mentioned(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "tool_token_mentioned",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_tool_token_mentioned_encode_failed:{e}"));
    }
    if mode == "policy_hold_reason_from_event" {
        let input = request
            .policy_hold_reason_from_event_input
            .ok_or_else(|| "autoscale_missing_policy_hold_reason_from_event_input".to_string())?;
        let out = compute_policy_hold_reason_from_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_reason_from_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_reason_from_event_encode_failed:{e}"));
    }
    if mode == "strategy_marker_tokens" {
        let input = request
            .strategy_marker_tokens_input
            .ok_or_else(|| "autoscale_missing_strategy_marker_tokens_input".to_string())?;
        let out = compute_strategy_marker_tokens(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_marker_tokens",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_marker_tokens_encode_failed:{e}"));
    }
    if mode == "capability_cooldown_key" {
        let input = request
            .capability_cooldown_key_input
            .ok_or_else(|| "autoscale_missing_capability_cooldown_key_input".to_string())?;
        let out = compute_capability_cooldown_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_cooldown_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_cooldown_key_encode_failed:{e}"));
    }
    if mode == "readiness_retry_cooldown_key" {
        let input = request
            .readiness_retry_cooldown_key_input
            .ok_or_else(|| "autoscale_missing_readiness_retry_cooldown_key_input".to_string())?;
        let out = compute_readiness_retry_cooldown_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "readiness_retry_cooldown_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_readiness_retry_cooldown_key_encode_failed:{e}"));
    }
    if mode == "source_eye_id" {
        let input = request
            .source_eye_id_input
            .ok_or_else(|| "autoscale_missing_source_eye_id_input".to_string())?;
        let out = compute_source_eye_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "source_eye_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_source_eye_id_encode_failed:{e}"));
    }
    if mode == "deprioritized_source_proposal" {
        let input = request
            .deprioritized_source_proposal_input
            .ok_or_else(|| "autoscale_missing_deprioritized_source_proposal_input".to_string())?;
        let out = compute_deprioritized_source_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "deprioritized_source_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_deprioritized_source_proposal_encode_failed:{e}"));
    }
    if mode == "composite_eligibility_min" {
        let input = request
            .composite_eligibility_min_input
            .ok_or_else(|| "autoscale_missing_composite_eligibility_min_input".to_string())?;
        let out = compute_composite_eligibility_min(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "composite_eligibility_min",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_composite_eligibility_min_encode_failed:{e}"));
    }
    if mode == "clamp_threshold" {
        let input = request
            .clamp_threshold_input
            .ok_or_else(|| "autoscale_missing_clamp_threshold_input".to_string())?;
        let out = compute_clamp_threshold(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "clamp_threshold",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_clamp_threshold_encode_failed:{e}"));
    }
    if mode == "applied_thresholds" {
        let input = request
            .applied_thresholds_input
            .ok_or_else(|| "autoscale_missing_applied_thresholds_input".to_string())?;
        let out = compute_applied_thresholds(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "applied_thresholds",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_applied_thresholds_encode_failed:{e}"));
    }
    if mode == "extract_eye_from_evidence_ref" {
        let input = request
            .extract_eye_from_evidence_ref_input
            .ok_or_else(|| "autoscale_missing_extract_eye_from_evidence_ref_input".to_string())?;
        let out = compute_extract_eye_from_evidence_ref(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "extract_eye_from_evidence_ref",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_extract_eye_from_evidence_ref_encode_failed:{e}"));
    }
    if mode == "total_outcomes" {
        let input = request
            .total_outcomes_input
            .ok_or_else(|| "autoscale_missing_total_outcomes_input".to_string())?;
        let out = compute_total_outcomes(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "total_outcomes",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_total_outcomes_encode_failed:{e}"));
    }
    if mode == "derive_entity_bias" {
        let input = request
            .derive_entity_bias_input
            .ok_or_else(|| "autoscale_missing_derive_entity_bias_input".to_string())?;
        let out = compute_derive_entity_bias(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "derive_entity_bias",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_derive_entity_bias_encode_failed:{e}"));
    }
    if mode == "strategy_profile" {
        let input = request
            .strategy_profile_input
            .ok_or_else(|| "autoscale_missing_strategy_profile_input".to_string())?;
        let out = compute_strategy_profile(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_profile",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_profile_encode_failed:{e}"));
    }
    if mode == "active_strategy_variants" {
        let input = request
            .active_strategy_variants_input
            .ok_or_else(|| "autoscale_missing_active_strategy_variants_input".to_string())?;
        let out = compute_active_strategy_variants(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "active_strategy_variants",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_active_strategy_variants_encode_failed:{e}"));
    }
    if mode == "strategy_scorecard_summaries" {
        let input = request
            .strategy_scorecard_summaries_input
            .ok_or_else(|| "autoscale_missing_strategy_scorecard_summaries_input".to_string())?;
        let out = compute_strategy_scorecard_summaries(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_scorecard_summaries",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_scorecard_summaries_encode_failed:{e}"));
    }
    if mode == "outcome_fitness_policy" {
        let input = request
            .outcome_fitness_policy_input
            .ok_or_else(|| "autoscale_missing_outcome_fitness_policy_input".to_string())?;
        let out = compute_outcome_fitness_policy(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "outcome_fitness_policy",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_outcome_fitness_policy_encode_failed:{e}"));
    }
    if mode == "load_eyes_map" {
        let input = request
            .load_eyes_map_input
            .ok_or_else(|| "autoscale_missing_load_eyes_map_input".to_string())?;
        let out = compute_load_eyes_map(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "load_eyes_map",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_load_eyes_map_encode_failed:{e}"));
    }
    if mode == "fallback_directive_objective_ids" {
        let input = request
            .fallback_directive_objective_ids_input
            .ok_or_else(|| {
                "autoscale_missing_fallback_directive_objective_ids_input".to_string()
            })?;
        let out = compute_fallback_directive_objective_ids(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "fallback_directive_objective_ids",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_fallback_directive_objective_ids_encode_failed:{e}"));
    }
    if mode == "queue_pressure_snapshot" {
        let input = request
            .queue_pressure_snapshot_input
            .ok_or_else(|| "autoscale_missing_queue_pressure_snapshot_input".to_string())?;
        let out = compute_queue_pressure_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "queue_pressure_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_queue_pressure_snapshot_encode_failed:{e}"));
    }
    if mode == "parse_success_criteria_rows" {
        let input = request
            .parse_success_criteria_rows_input
            .ok_or_else(|| "autoscale_missing_parse_success_criteria_rows_input".to_string())?;
        let out = compute_parse_success_criteria_rows(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_success_criteria_rows",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_success_criteria_rows_encode_failed:{e}"));
    }
    if mode == "collect_outcome_stats" {
        let input = request
            .collect_outcome_stats_input
            .ok_or_else(|| "autoscale_missing_collect_outcome_stats_input".to_string())?;
        let out = compute_collect_outcome_stats(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "collect_outcome_stats",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_collect_outcome_stats_encode_failed:{e}"));
    }
    if mode == "subdirective_v2_signals" {
        let input = request
            .subdirective_v2_signals_input
            .ok_or_else(|| "autoscale_missing_subdirective_v2_signals_input".to_string())?;
        let out = compute_subdirective_v2_signals(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "subdirective_v2_signals",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_subdirective_v2_signals_encode_failed:{e}"));
    }
    if mode == "build_overlay" {
        let input = request
            .build_overlay_input
            .ok_or_else(|| "autoscale_missing_build_overlay_input".to_string())?;
        let out = compute_build_overlay(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "build_overlay",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_build_overlay_encode_failed:{e}"));
    }
    if mode == "has_adaptive_mutation_signal" {
        let input = request
            .has_adaptive_mutation_signal_input
            .ok_or_else(|| "autoscale_missing_has_adaptive_mutation_signal_input".to_string())?;
        let out = compute_has_adaptive_mutation_signal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_adaptive_mutation_signal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_adaptive_mutation_signal_encode_failed:{e}"));
    }
    if mode == "adaptive_mutation_execution_guard" {
        let input = request
            .adaptive_mutation_execution_guard_input
            .ok_or_else(|| {
                "autoscale_missing_adaptive_mutation_execution_guard_input".to_string()
            })?;
        let out = compute_adaptive_mutation_execution_guard(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "adaptive_mutation_execution_guard",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_adaptive_mutation_execution_guard_encode_failed:{e}"));
    }
    if mode == "strategy_selection" {
        let input = request
            .strategy_selection_input
            .ok_or_else(|| "autoscale_missing_strategy_selection_input".to_string())?;
        let out = compute_strategy_selection(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_selection",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_selection_encode_failed:{e}"));
    }
    if mode == "calibration_deltas" {
        let input = request
            .calibration_deltas_input
            .ok_or_else(|| "autoscale_missing_calibration_deltas_input".to_string())?;
        let out = compute_calibration_deltas(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "calibration_deltas",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_calibration_deltas_encode_failed:{e}"));
    }
    if mode == "strategy_admission_decision" {
        let input = request
            .strategy_admission_decision_input
            .ok_or_else(|| "autoscale_missing_strategy_admission_decision_input".to_string())?;
        let out = compute_strategy_admission_decision(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "strategy_admission_decision",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_strategy_admission_decision_encode_failed:{e}"));
    }
    if mode == "expected_value_score" {
        let input = request
            .expected_value_score_input
            .ok_or_else(|| "autoscale_missing_expected_value_score_input".to_string())?;
        let out = compute_expected_value_score(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "expected_value_score",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_expected_value_score_encode_failed:{e}"));
    }
    if mode == "suggest_run_batch_max" {
        let input = request
            .suggest_run_batch_max_input
            .ok_or_else(|| "autoscale_missing_suggest_run_batch_max_input".to_string())?;
        let out = compute_suggest_run_batch_max(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "suggest_run_batch_max",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_suggest_run_batch_max_encode_failed:{e}"));
    }
    if mode == "backlog_autoscale_snapshot" {
        let input = request
            .backlog_autoscale_snapshot_input
            .ok_or_else(|| "autoscale_missing_backlog_autoscale_snapshot_input".to_string())?;
        let out = compute_backlog_autoscale_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "backlog_autoscale_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_backlog_autoscale_snapshot_encode_failed:{e}"));
    }
    if mode == "admission_summary" {
        let input = request
            .admission_summary_input
            .ok_or_else(|| "autoscale_missing_admission_summary_input".to_string())?;
        let out = compute_admission_summary(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "admission_summary",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_admission_summary_encode_failed:{e}"));
    }
    if mode == "unknown_type_quarantine_decision" {
        let input = request
            .unknown_type_quarantine_decision_input
            .ok_or_else(|| {
                "autoscale_missing_unknown_type_quarantine_decision_input".to_string()
            })?;
        let out = compute_unknown_type_quarantine_decision(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "unknown_type_quarantine_decision",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_unknown_type_quarantine_decision_encode_failed:{e}"));
    }
    if mode == "infer_optimization_delta" {
        let input = request
            .infer_optimization_delta_input
            .ok_or_else(|| "autoscale_missing_infer_optimization_delta_input".to_string())?;
        let out = compute_infer_optimization_delta(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "infer_optimization_delta",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_infer_optimization_delta_encode_failed:{e}"));
    }
    if mode == "optimization_intent_proposal" {
        let input = request
            .optimization_intent_proposal_input
            .ok_or_else(|| "autoscale_missing_optimization_intent_proposal_input".to_string())?;
        let out = compute_optimization_intent_proposal(&input);
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
        let input = request
            .criteria_pattern_keys_input
            .ok_or_else(|| "autoscale_missing_criteria_pattern_keys_input".to_string())?;
        let out = compute_criteria_pattern_keys(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "criteria_pattern_keys",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_criteria_pattern_keys_encode_failed:{e}"));
    }
    if mode == "success_criteria_requirement" {
        let input = request
            .success_criteria_requirement_input
            .ok_or_else(|| "autoscale_missing_success_criteria_requirement_input".to_string())?;
        let out = compute_success_criteria_requirement(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_requirement",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_requirement_encode_failed:{e}"));
    }
    if mode == "success_criteria_policy_for_proposal" {
        let input = request
            .success_criteria_policy_for_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_success_criteria_policy_for_proposal_input".to_string()
            })?;
        let out = compute_success_criteria_policy_for_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "success_criteria_policy_for_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_success_criteria_policy_for_proposal_encode_failed:{e}"));
    }
    if mode == "capability_descriptor" {
        let input = request
            .capability_descriptor_input
            .ok_or_else(|| "autoscale_missing_capability_descriptor_input".to_string())?;
        let out = compute_capability_descriptor(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_descriptor",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_descriptor_encode_failed:{e}"));
    }
    if mode == "normalize_token_usage_shape" {
        let input = request
            .normalize_token_usage_shape_input
            .ok_or_else(|| "autoscale_missing_normalize_token_usage_shape_input".to_string())?;
        let out = compute_normalize_token_usage_shape(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_token_usage_shape",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_token_usage_shape_encode_failed:{e}"));
    }
    if mode == "directive_pulse_context" {
        let input = request
            .directive_pulse_context_input
            .ok_or_else(|| "autoscale_missing_directive_pulse_context_input".to_string())?;
        let out = compute_directive_pulse_context(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_pulse_context",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_pulse_context_encode_failed:{e}"));
    }
    if mode == "is_directive_clarification_proposal" {
        let input = request
            .is_directive_clarification_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_is_directive_clarification_proposal_input".to_string()
            })?;
        let out = compute_is_directive_clarification_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_directive_clarification_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_directive_clarification_proposal_encode_failed:{e}"));
    }
    if mode == "is_directive_decomposition_proposal" {
        let input = request
            .is_directive_decomposition_proposal_input
            .ok_or_else(|| {
                "autoscale_missing_is_directive_decomposition_proposal_input".to_string()
            })?;
        let out = compute_is_directive_decomposition_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "is_directive_decomposition_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_is_directive_decomposition_proposal_encode_failed:{e}"));
    }
    if mode == "sanitize_directive_objective_id" {
        let input = request
            .sanitize_directive_objective_id_input
            .ok_or_else(|| "autoscale_missing_sanitize_directive_objective_id_input".to_string())?;
        let out = compute_sanitize_directive_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sanitize_directive_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sanitize_directive_objective_id_encode_failed:{e}"));
    }
    if mode == "sanitized_directive_id_list" {
        let input = request
            .sanitized_directive_id_list_input
            .ok_or_else(|| "autoscale_missing_sanitized_directive_id_list_input".to_string())?;
        let out = compute_sanitized_directive_id_list(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "sanitized_directive_id_list",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_sanitized_directive_id_list_encode_failed:{e}"));
    }
    if mode == "parse_first_json_line" {
        let input = request
            .parse_first_json_line_input
            .ok_or_else(|| "autoscale_missing_parse_first_json_line_input".to_string())?;
        let out = compute_parse_first_json_line(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_first_json_line",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_first_json_line_encode_failed:{e}"));
    }
    if mode == "parse_json_objects_from_text" {
        let input = request
            .parse_json_objects_from_text_input
            .ok_or_else(|| "autoscale_missing_parse_json_objects_from_text_input".to_string())?;
        let out = compute_parse_json_objects_from_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_json_objects_from_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_json_objects_from_text_encode_failed:{e}"));
    }
    if mode == "read_path_value" {
        let input = request
            .read_path_value_input
            .ok_or_else(|| "autoscale_missing_read_path_value_input".to_string())?;
        let out = compute_read_path_value(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "read_path_value",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_read_path_value_encode_failed:{e}"));
    }
    if mode == "number_or_null" {
        let input = request
            .number_or_null_input
            .ok_or_else(|| "autoscale_missing_number_or_null_input".to_string())?;
        let out = compute_number_or_null(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "number_or_null",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_number_or_null_encode_failed:{e}"));
    }
    if mode == "choose_evidence_selection_mode" {
        let input = request
            .choose_evidence_selection_mode_input
            .ok_or_else(|| "autoscale_missing_choose_evidence_selection_mode_input".to_string())?;
        let out = compute_choose_evidence_selection_mode(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "choose_evidence_selection_mode",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_choose_evidence_selection_mode_encode_failed:{e}"));
    }
    if mode == "truthy_flag" {
        let input = request
            .truthy_flag_input
            .ok_or_else(|| "autoscale_missing_truthy_flag_input".to_string())?;
        let out = compute_truthy_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "truthy_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_truthy_flag_encode_failed:{e}"));
    }
    if mode == "falsey_flag" {
        let input = request
            .falsey_flag_input
            .ok_or_else(|| "autoscale_missing_falsey_flag_input".to_string())?;
        let out = compute_falsey_flag(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "falsey_flag",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_falsey_flag_encode_failed:{e}"));
    }
    if mode == "stable_selection_index" {
        let input = request
            .stable_selection_index_input
            .ok_or_else(|| "autoscale_missing_stable_selection_index_input".to_string())?;
        let out = compute_stable_selection_index(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "stable_selection_index",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_stable_selection_index_encode_failed:{e}"));
    }
    if mode == "as_string_array" {
        let input = request
            .as_string_array_input
            .ok_or_else(|| "autoscale_missing_as_string_array_input".to_string())?;
        let out = compute_as_string_array(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "as_string_array",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_as_string_array_encode_failed:{e}"));
    }
    if mode == "uniq_sorted" {
        let input = request
            .uniq_sorted_input
            .ok_or_else(|| "autoscale_missing_uniq_sorted_input".to_string())?;
        let out = compute_uniq_sorted(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "uniq_sorted",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_uniq_sorted_encode_failed:{e}"));
    }
    if mode == "normalize_model_ids" {
        let input = request
            .normalize_model_ids_input
            .ok_or_else(|| "autoscale_missing_normalize_model_ids_input".to_string())?;
        let out = compute_normalize_model_ids(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalize_model_ids",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalize_model_ids_encode_failed:{e}"));
    }
    if mode == "selected_model_from_run_event" {
        let input = request
            .selected_model_from_run_event_input
            .ok_or_else(|| "autoscale_missing_selected_model_from_run_event_input".to_string())?;
        let out = compute_selected_model_from_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "selected_model_from_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_selected_model_from_run_event_encode_failed:{e}"));
    }
    if mode == "read_first_numeric_metric" {
        let input = request
            .read_first_numeric_metric_input
            .ok_or_else(|| "autoscale_missing_read_first_numeric_metric_input".to_string())?;
        let out = compute_read_first_numeric_metric(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "read_first_numeric_metric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_read_first_numeric_metric_encode_failed:{e}"));
    }
    if mode == "parse_arg" {
        let input = request
            .parse_arg_input
            .ok_or_else(|| "autoscale_missing_parse_arg_input".to_string())?;
        let out = compute_parse_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_arg_encode_failed:{e}"));
    }
    if mode == "date_arg_or_today" {
        let input = request
            .date_arg_or_today_input
            .ok_or_else(|| "autoscale_missing_date_arg_or_today_input".to_string())?;
        let out = compute_date_arg_or_today(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "date_arg_or_today",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_date_arg_or_today_encode_failed:{e}"));
    }
    if mode == "has_env_numeric_override" {
        let input = request
            .has_env_numeric_override_input
            .ok_or_else(|| "autoscale_missing_has_env_numeric_override_input".to_string())?;
        let out = compute_has_env_numeric_override(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "has_env_numeric_override",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_has_env_numeric_override_encode_failed:{e}"));
    }
    if mode == "coalesce_numeric" {
        let input = request
            .coalesce_numeric_input
            .ok_or_else(|| "autoscale_missing_coalesce_numeric_input".to_string())?;
        let out = compute_coalesce_numeric(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "coalesce_numeric",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_coalesce_numeric_encode_failed:{e}"));
    }
    if mode == "clamp_number" {
        let input = request
            .clamp_number_input
            .ok_or_else(|| "autoscale_missing_clamp_number_input".to_string())?;
        let out = compute_clamp_number(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "clamp_number",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_clamp_number_encode_failed:{e}"));
    }
    if mode == "list_proposal_files" {
        let input = request
            .list_proposal_files_input
            .ok_or_else(|| "autoscale_missing_list_proposal_files_input".to_string())?;
        let out = compute_list_proposal_files(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "list_proposal_files",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_list_proposal_files_encode_failed:{e}"));
    }
    if mode == "latest_proposal_date" {
        let input = request
            .latest_proposal_date_input
            .ok_or_else(|| "autoscale_missing_latest_proposal_date_input".to_string())?;
        let out = compute_latest_proposal_date(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "latest_proposal_date",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_latest_proposal_date_encode_failed:{e}"));
    }
    if mode == "now_iso" {
        let input = request
            .now_iso_input
            .ok_or_else(|| "autoscale_missing_now_iso_input".to_string())?;
        let out = compute_now_iso(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "now_iso",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_now_iso_encode_failed:{e}"));
    }
    if mode == "today_str" {
        let input = request
            .today_str_input
            .ok_or_else(|| "autoscale_missing_today_str_input".to_string())?;
        let out = compute_today_str(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "today_str",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_today_str_encode_failed:{e}"));
    }
    if mode == "human_canary_override_approval_phrase" {
        let input = request
            .human_canary_override_approval_phrase_input
            .ok_or_else(|| {
                "autoscale_missing_human_canary_override_approval_phrase_input".to_string()
            })?;
        let out = compute_human_canary_override_approval_phrase(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "human_canary_override_approval_phrase",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_human_canary_override_approval_phrase_encode_failed:{e}"));
    }
    if mode == "parse_human_canary_override_state" {
        let input = request
            .parse_human_canary_override_state_input
            .ok_or_else(|| {
                "autoscale_missing_parse_human_canary_override_state_input".to_string()
            })?;
        let out = compute_parse_human_canary_override_state(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_human_canary_override_state",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_human_canary_override_state_encode_failed:{e}"));
    }
    if mode == "daily_budget_path" {
        let input = request
            .daily_budget_path_input
            .ok_or_else(|| "autoscale_missing_daily_budget_path_input".to_string())?;
        let out = compute_daily_budget_path(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "daily_budget_path",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_daily_budget_path_encode_failed:{e}"));
    }
    if mode == "runs_path_for" {
        let input = request
            .runs_path_for_input
            .ok_or_else(|| "autoscale_missing_runs_path_for_input".to_string())?;
        let out = compute_runs_path_for(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "runs_path_for",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_runs_path_for_encode_failed:{e}"));
    }
    if mode == "effective_tier1_policy" {
        let input = request
            .effective_tier1_policy_input
            .ok_or_else(|| "autoscale_missing_effective_tier1_policy_input".to_string())?;
        let out = compute_effective_tier1_policy(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "effective_tier1_policy",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_effective_tier1_policy_encode_failed:{e}"));
    }
    if mode == "compact_tier1_exception" {
        let input = request
            .compact_tier1_exception_input
            .ok_or_else(|| "autoscale_missing_compact_tier1_exception_input".to_string())?;
        let out = compute_compact_tier1_exception(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "compact_tier1_exception",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_compact_tier1_exception_encode_failed:{e}"));
    }
    if mode == "next_human_escalation_clear_at" {
        let input = request
            .next_human_escalation_clear_at_input
            .ok_or_else(|| "autoscale_missing_next_human_escalation_clear_at_input".to_string())?;
        let out = compute_next_human_escalation_clear_at(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "next_human_escalation_clear_at",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_next_human_escalation_clear_at_encode_failed:{e}"));
    }
    if mode == "model_catalog_canary_thresholds" {
        let input = request
            .model_catalog_canary_thresholds_input
            .ok_or_else(|| "autoscale_missing_model_catalog_canary_thresholds_input".to_string())?;
        let out = compute_model_catalog_canary_thresholds(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "model_catalog_canary_thresholds",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_model_catalog_canary_thresholds_encode_failed:{e}"));
    }
    if mode == "parse_directive_file_arg" {
        let input = request
            .parse_directive_file_arg_input
            .ok_or_else(|| "autoscale_missing_parse_directive_file_arg_input".to_string())?;
        let out = compute_parse_directive_file_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_directive_file_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_directive_file_arg_encode_failed:{e}"));
    }
    if mode == "parse_directive_objective_arg" {
        let input = request
            .parse_directive_objective_arg_input
            .ok_or_else(|| "autoscale_missing_parse_directive_objective_arg_input".to_string())?;
        let out = compute_parse_directive_objective_arg(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_directive_objective_arg",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_directive_objective_arg_encode_failed:{e}"));
    }
    if mode == "directive_clarification_exec_spec" {
        let input = request
            .directive_clarification_exec_spec_input
            .ok_or_else(|| {
                "autoscale_missing_directive_clarification_exec_spec_input".to_string()
            })?;
        let out = compute_directive_clarification_exec_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_clarification_exec_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_clarification_exec_spec_encode_failed:{e}"));
    }
    if mode == "directive_decomposition_exec_spec" {
        let input = request
            .directive_decomposition_exec_spec_input
            .ok_or_else(|| {
                "autoscale_missing_directive_decomposition_exec_spec_input".to_string()
            })?;
        let out = compute_directive_decomposition_exec_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_decomposition_exec_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_decomposition_exec_spec_encode_failed:{e}"));
    }
    if mode == "parse_actuation_spec" {
        let input = request
            .parse_actuation_spec_input
            .ok_or_else(|| "autoscale_missing_parse_actuation_spec_input".to_string())?;
        let out = compute_parse_actuation_spec(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_actuation_spec",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_actuation_spec_encode_failed:{e}"));
    }
    if mode == "task_from_proposal" {
        let input = request
            .task_from_proposal_input
            .ok_or_else(|| "autoscale_missing_task_from_proposal_input".to_string())?;
        let out = compute_task_from_proposal(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "task_from_proposal",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_task_from_proposal_encode_failed:{e}"));
    }
    if mode == "parse_objective_id_from_evidence_refs" {
        let input = request
            .parse_objective_id_from_evidence_refs_input
            .ok_or_else(|| {
                "autoscale_missing_parse_objective_id_from_evidence_refs_input".to_string()
            })?;
        let out = compute_parse_objective_id_from_evidence_refs(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_objective_id_from_evidence_refs",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_objective_id_from_evidence_refs_encode_failed:{e}"));
    }
    if mode == "parse_objective_id_from_command" {
        let input = request
            .parse_objective_id_from_command_input
            .ok_or_else(|| "autoscale_missing_parse_objective_id_from_command_input".to_string())?;
        let out = compute_parse_objective_id_from_command(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "parse_objective_id_from_command",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_parse_objective_id_from_command_encode_failed:{e}"));
    }
    if mode == "objective_id_for_execution" {
        let input = request
            .objective_id_for_execution_input
            .ok_or_else(|| "autoscale_missing_objective_id_for_execution_input".to_string())?;
        let out = compute_objective_id_for_execution(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "objective_id_for_execution",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_objective_id_for_execution_encode_failed:{e}"));
    }
    if mode == "short_text" {
        let input = request
            .short_text_input
            .ok_or_else(|| "autoscale_missing_short_text_input".to_string())?;
        let out = compute_short_text(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "short_text",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_short_text_encode_failed:{e}"));
    }
    if mode == "normalized_signal_status" {
        let input = request
            .normalized_signal_status_input
            .ok_or_else(|| "autoscale_missing_normalized_signal_status_input".to_string())?;
        let out = compute_normalized_signal_status(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "normalized_signal_status",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_normalized_signal_status_encode_failed:{e}"));
    }
    if mode == "execution_reserve_snapshot" {
        let input = request
            .execution_reserve_snapshot_input
            .ok_or_else(|| "autoscale_missing_execution_reserve_snapshot_input".to_string())?;
        let out = compute_execution_reserve_snapshot(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execution_reserve_snapshot",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execution_reserve_snapshot_encode_failed:{e}"));
    }
    if mode == "budget_pacing_gate" {
        let input = request
            .budget_pacing_gate_input
            .ok_or_else(|| "autoscale_missing_budget_pacing_gate_input".to_string())?;
        let out = compute_budget_pacing_gate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "budget_pacing_gate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_budget_pacing_gate_encode_failed:{e}"));
    }
    if mode == "capability_cap" {
        let input = request
            .capability_cap_input
            .ok_or_else(|| "autoscale_missing_capability_cap_input".to_string())?;
        let out = compute_capability_cap(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_cap",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_cap_encode_failed:{e}"));
    }
    if mode == "estimate_tokens_for_candidate" {
        let input = request
            .estimate_tokens_for_candidate_input
            .ok_or_else(|| "autoscale_missing_estimate_tokens_for_candidate_input".to_string())?;
        let out = compute_estimate_tokens_for_candidate(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "estimate_tokens_for_candidate",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_estimate_tokens_for_candidate_encode_failed:{e}"));
    }
    if mode == "proposal_status_for_queue_pressure" {
        let input = request
            .proposal_status_for_queue_pressure_input
            .ok_or_else(|| {
                "autoscale_missing_proposal_status_for_queue_pressure_input".to_string()
            })?;
        let out = compute_proposal_status_for_queue_pressure(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_status_for_queue_pressure",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_status_for_queue_pressure_encode_failed:{e}"));
    }
    if mode == "minutes_since_ts" {
        let input = request
            .minutes_since_ts_input
            .ok_or_else(|| "autoscale_missing_minutes_since_ts_input".to_string())?;
        let out = compute_minutes_since_ts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "minutes_since_ts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_minutes_since_ts_encode_failed:{e}"));
    }
    if mode == "date_window" {
        let input = request
            .date_window_input
            .ok_or_else(|| "autoscale_missing_date_window_input".to_string())?;
        let out = compute_date_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "date_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_date_window_encode_failed:{e}"));
    }
    if mode == "in_window" {
        let input = request
            .in_window_input
            .ok_or_else(|| "autoscale_missing_in_window_input".to_string())?;
        let out = compute_in_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "in_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_in_window_encode_failed:{e}"));
    }
    if mode == "exec_window_match" {
        let input = request
            .exec_window_match_input
            .ok_or_else(|| "autoscale_missing_exec_window_match_input".to_string())?;
        let out = compute_exec_window_match(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "exec_window_match",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_exec_window_match_encode_failed:{e}"));
    }
    if mode == "start_of_next_utc_day" {
        let input = request
            .start_of_next_utc_day_input
            .ok_or_else(|| "autoscale_missing_start_of_next_utc_day_input".to_string())?;
        let out = compute_start_of_next_utc_day(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "start_of_next_utc_day",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_start_of_next_utc_day_encode_failed:{e}"));
    }
    if mode == "iso_after_minutes" {
        let input = request
            .iso_after_minutes_input
            .ok_or_else(|| "autoscale_missing_iso_after_minutes_input".to_string())?;
        let out = compute_iso_after_minutes(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "iso_after_minutes",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_iso_after_minutes_encode_failed:{e}"));
    }
    if mode == "execute_confidence_history_match" {
        let input = request
            .execute_confidence_history_match_input
            .ok_or_else(|| {
                "autoscale_missing_execute_confidence_history_match_input".to_string()
            })?;
        let out = compute_execute_confidence_history_match(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execute_confidence_history_match",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execute_confidence_history_match_encode_failed:{e}"));
    }
    if mode == "execute_confidence_cooldown_key" {
        let input = request
            .execute_confidence_cooldown_key_input
            .ok_or_else(|| "autoscale_missing_execute_confidence_cooldown_key_input".to_string())?;
        let out = compute_execute_confidence_cooldown_key(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execute_confidence_cooldown_key",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execute_confidence_cooldown_key_encode_failed:{e}"));
    }
    if mode == "recent_proposal_key_counts" {
        let input = request
            .recent_proposal_key_counts_input
            .ok_or_else(|| "autoscale_missing_recent_proposal_key_counts_input".to_string())?;
        let out = compute_recent_proposal_key_counts(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "recent_proposal_key_counts",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_recent_proposal_key_counts_encode_failed:{e}"));
    }
    if mode == "capability_attempt_count_for_date" {
        let input = request
            .capability_attempt_count_for_date_input
            .ok_or_else(|| {
                "autoscale_missing_capability_attempt_count_for_date_input".to_string()
            })?;
        let out = compute_capability_attempt_count_for_date(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_attempt_count_for_date",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_attempt_count_for_date_encode_failed:{e}"));
    }
    if mode == "capability_outcome_stats_in_window" {
        let input = request
            .capability_outcome_stats_in_window_input
            .ok_or_else(|| {
                "autoscale_missing_capability_outcome_stats_in_window_input".to_string()
            })?;
        let out = compute_capability_outcome_stats_in_window(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capability_outcome_stats_in_window",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capability_outcome_stats_in_window_encode_failed:{e}"));
    }
    if mode == "execute_confidence_history" {
        let input = request
            .execute_confidence_history_input
            .ok_or_else(|| "autoscale_missing_execute_confidence_history_input".to_string())?;
        let out = compute_execute_confidence_history(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execute_confidence_history",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execute_confidence_history_encode_failed:{e}"));
    }
    if mode == "execute_confidence_policy" {
        let input = request
            .execute_confidence_policy_input
            .ok_or_else(|| "autoscale_missing_execute_confidence_policy_input".to_string())?;
        let out = compute_execute_confidence_policy(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "execute_confidence_policy",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_execute_confidence_policy_encode_failed:{e}"));
    }
    if mode == "directive_fit_assessment" {
        let input = request
            .directive_fit_assessment_input
            .ok_or_else(|| "autoscale_missing_directive_fit_assessment_input".to_string())?;
        let out = compute_directive_fit_assessment(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "directive_fit_assessment",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_directive_fit_assessment_encode_failed:{e}"));
    }
    if mode == "signal_quality_assessment" {
        let input = request
            .signal_quality_assessment_input
            .ok_or_else(|| "autoscale_missing_signal_quality_assessment_input".to_string())?;
        let out = compute_signal_quality_assessment(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "signal_quality_assessment",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_signal_quality_assessment_encode_failed:{e}"));
    }
    if mode == "actionability_assessment" {
        let input = request
            .actionability_assessment_input
            .ok_or_else(|| "autoscale_missing_actionability_assessment_input".to_string())?;
        let out = compute_actionability_assessment(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "actionability_assessment",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_actionability_assessment_encode_failed:{e}"));
    }
    if mode == "no_progress_result" {
        let input = request
            .no_progress_result_input
            .ok_or_else(|| "autoscale_missing_no_progress_result_input".to_string())?;
        let out = compute_no_progress_result(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "no_progress_result",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_no_progress_result_encode_failed:{e}"));
    }
    if mode == "attempt_run_event" {
        let input = request
            .attempt_run_event_input
            .ok_or_else(|| "autoscale_missing_attempt_run_event_input".to_string())?;
        let out = compute_attempt_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "attempt_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_attempt_run_event_encode_failed:{e}"));
    }
    if mode == "safety_stop_run_event" {
        let input = request
            .safety_stop_run_event_input
            .ok_or_else(|| "autoscale_missing_safety_stop_run_event_input".to_string())?;
        let out = compute_safety_stop_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "safety_stop_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_safety_stop_run_event_encode_failed:{e}"));
    }
    if mode == "non_yield_category" {
        let input = request
            .non_yield_category_input
            .ok_or_else(|| "autoscale_missing_non_yield_category_input".to_string())?;
        let out = compute_non_yield_category(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "non_yield_category",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_non_yield_category_encode_failed:{e}"));
    }
    if mode == "non_yield_reason" {
        let input = request
            .non_yield_reason_input
            .ok_or_else(|| "autoscale_missing_non_yield_reason_input".to_string())?;
        let out = compute_non_yield_reason(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "non_yield_reason",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_non_yield_reason_encode_failed:{e}"));
    }
    if mode == "proposal_type_from_run_event" {
        let input = request
            .proposal_type_from_run_event_input
            .ok_or_else(|| "autoscale_missing_proposal_type_from_run_event_input".to_string())?;
        let out = compute_proposal_type_from_run_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "proposal_type_from_run_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_proposal_type_from_run_event_encode_failed:{e}"));
    }
    if mode == "run_event_objective_id" {
        let input = request
            .run_event_objective_id_input
            .ok_or_else(|| "autoscale_missing_run_event_objective_id_input".to_string())?;
        let out = compute_run_event_objective_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "run_event_objective_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_run_event_objective_id_encode_failed:{e}"));
    }
    if mode == "run_event_proposal_id" {
        let input = request
            .run_event_proposal_id_input
            .ok_or_else(|| "autoscale_missing_run_event_proposal_id_input".to_string())?;
        let out = compute_run_event_proposal_id(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "run_event_proposal_id",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_run_event_proposal_id_encode_failed:{e}"));
    }
    if mode == "capacity_counted_attempt_event" {
        let input = request
            .capacity_counted_attempt_event_input
            .ok_or_else(|| "autoscale_missing_capacity_counted_attempt_event_input".to_string())?;
        let out = compute_capacity_counted_attempt_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "capacity_counted_attempt_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_capacity_counted_attempt_event_encode_failed:{e}"));
    }
    if mode == "repeat_gate_anchor" {
        let input = request
            .repeat_gate_anchor_input
            .ok_or_else(|| "autoscale_missing_repeat_gate_anchor_input".to_string())?;
        let out = compute_repeat_gate_anchor(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "repeat_gate_anchor",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_repeat_gate_anchor_encode_failed:{e}"));
    }
    if mode == "route_execution_policy_hold" {
        let input = request
            .route_execution_policy_hold_input
            .ok_or_else(|| "autoscale_missing_route_execution_policy_hold_input".to_string())?;
        let out = compute_route_execution_policy_hold(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "route_execution_policy_hold",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_route_execution_policy_hold_encode_failed:{e}"));
    }
    if mode == "policy_hold_pressure" {
        let input = request
            .policy_hold_pressure_input
            .ok_or_else(|| "autoscale_missing_policy_hold_pressure_input".to_string())?;
        let out = compute_policy_hold_pressure(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_pressure",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_pressure_encode_failed:{e}"));
    }
    if mode == "policy_hold_pattern" {
        let input = request
            .policy_hold_pattern_input
            .ok_or_else(|| "autoscale_missing_policy_hold_pattern_input".to_string())?;
        let out = compute_policy_hold_pattern(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_pattern",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_pattern_encode_failed:{e}"));
    }
    if mode == "policy_hold_latest_event" {
        let input = request
            .policy_hold_latest_event_input
            .ok_or_else(|| "autoscale_missing_policy_hold_latest_event_input".to_string())?;
        let out = compute_policy_hold_latest_event(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_latest_event",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_latest_event_encode_failed:{e}"));
    }
    if mode == "policy_hold_cooldown" {
        let input = request
            .policy_hold_cooldown_input
            .ok_or_else(|| "autoscale_missing_policy_hold_cooldown_input".to_string())?;
        let out = compute_policy_hold_cooldown(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "policy_hold_cooldown",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_policy_hold_cooldown_encode_failed:{e}"));
    }
    if mode == "receipt_verdict" {
        let input = request
            .receipt_verdict_input
            .ok_or_else(|| "autoscale_missing_receipt_verdict_input".to_string())?;
        let out = compute_receipt_verdict(&input);
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "mode": "receipt_verdict",
            "payload": out
        }))
        .map_err(|e| format!("autoscale_receipt_verdict_encode_failed:{e}"));
    }
    Err(format!(
        "autoscale_mode_unsupported:raw={mode_raw}:normalized={mode}"
    ))
}
