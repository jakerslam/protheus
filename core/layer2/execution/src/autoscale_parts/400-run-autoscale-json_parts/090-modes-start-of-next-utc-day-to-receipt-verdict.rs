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
