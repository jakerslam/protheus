    let response_gate_contract_consistent = response_gate_blocker_count_matches
        && response_gate_primary_blocker_matches
        && response_gate_blocker_priority_consistent
        && response_gate_blocker_set_consistent
        && response_gate_blocker_set_key_consistent
        && response_gate_blocker_count_key_consistent
        && response_gate_expected_blocker_count_matches
        && response_gate_blocker_vector_consistent
        && response_gate_signature_consistent
        && response_gate_score_consistent
        && response_gate_score_band_consistent
        && response_gate_score_band_known
        && response_gate_score_vector_consistent
        && response_gate_score_band_vector_consistent
        && response_gate_score_band_severity_consistent
        && response_gate_score_band_severity_bucket_consistent
        && response_gate_score_band_severity_bucket_known
        && response_gate_blocker_flags_consistent
        && response_gate_escalation_signature_consistent
        && response_gate_decision_signature_consistent
        && response_gate_blocker_budget_consistent
        && response_gate_manual_review_signature_consistent
        && response_gate_manual_review_reason_consistent
        && response_gate_manual_review_reason_known
        && response_gate_manual_review_vector_consistent
        && response_gate_manual_review_vector_known
        && response_gate_primary_blocker_known
        && response_gate_blockers_consistent
        && response_gate_severity_consistent
        && response_gate_manual_review_consistent
        && response_gate_escalation_contract_ok
        && response_gate_escalation_lane_known
        && response_gate_escalation_reason_known
        && response_gate_escalation_vector_known
        && response_gate_decision_vector_known
        && response_gate_next_action_command_consistent
        && response_gate_next_action_command_known
        && response_gate_next_action_lane_consistent
        && response_gate_retry_class_consistent
        && response_gate_retry_class_known
        && response_gate_retry_command_consistent
        && response_gate_retry_window_consistent
        && response_gate_retry_signature_consistent
        && response_gate_retry_signature_known
        && response_gate_lane_retry_window_consistent
        && response_gate_retry_band_consistent
        && response_gate_retry_contract_after_seconds_class_consistent
        && response_gate_retry_contract_after_seconds_score_band_consistent
        && response_gate_retry_contract_after_seconds_lane_consistent
        && response_gate_retry_contract_after_seconds_next_action_window_consistent
        && response_gate_retry_contract_lane_command_consistent
        && response_gate_retry_contract_after_seconds_lane_command_consistent
        && response_gate_retry_contract_after_seconds_command_consistent
        && response_gate_retry_mode_consistent
        && response_gate_retry_mode_known
        && response_gate_retry_contract_lane_mode_consistent
        && response_gate_retry_contract_after_seconds_mode_consistent
        && response_gate_retry_contract_after_seconds_lane_mode_consistent
        && response_gate_retry_contract_lane_command_mode_consistent
        && response_gate_retry_contract_after_seconds_lane_command_mode_consistent
        && response_gate_retry_contract_after_seconds_command_mode_consistent
        && response_gate_retry_action_vector_consistent
        && response_gate_retry_budget_consistent
        && response_gate_retry_budget_non_negative
        && response_gate_retry_budget_band_consistent
        && response_gate_retry_budget_expected_band_consistent
        && response_gate_retry_budget_range_consistent
        && response_gate_retry_budget_mode_consistent
        && response_gate_retry_contract_after_seconds_budget_consistent
        && response_gate_retry_pressure_tier_consistent
        && response_gate_retry_pressure_tier_known
        && response_gate_retry_contract_after_seconds_pressure_consistent
        && response_gate_retry_budget_vector_consistent
        && response_gate_retry_budget_vector_known
        && response_gate_retry_tier_window_consistent
        && response_gate_retry_tier_mode_consistent
        && response_gate_retry_tier_vector_consistent
        && response_gate_retry_tier_vector_known
        && response_gate_retry_contract_vector_consistent
        && response_gate_retry_contract_vector_known
        && response_gate_retry_contract_family_consistent
        && response_gate_retry_contract_severity_consistent
        && response_gate_retry_contract_coherence_consistent
        && response_gate_retry_contract_lane_class_consistent
        && response_gate_retry_contract_command_class_consistent
        && response_gate_retry_contract_expected_class_consistent
        && response_gate_retry_contract_pressure_class_consistent
        && response_gate_retry_contract_expected_pressure_class_consistent
        && response_gate_retry_contract_expected_command_class_consistent
        && response_gate_retry_contract_expected_mode_class_consistent
        && response_gate_retry_contract_expected_after_seconds_class_consistent
        && response_gate_retry_contract_expected_after_seconds_band_class_consistent
        && response_gate_retry_contract_expected_after_seconds_pressure_consistent
        && response_gate_retry_contract_expected_mode_pressure_consistent
        && response_gate_retry_contract_expected_lane_pressure_consistent
        && response_gate_retry_contract_expected_command_pressure_consistent
        && response_gate_retry_contract_expected_pressure_class_inverse_consistent
        && response_gate_retry_contract_expected_command_mode_consistent
        && response_gate_retry_contract_expected_after_seconds_mode_consistent
        && response_gate_retry_contract_expected_lane_after_seconds_consistent
        && response_gate_retry_contract_expected_command_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_pressure_consistent
        && response_gate_retry_contract_expected_lane_mode_pressure_consistent
        && response_gate_retry_contract_expected_lane_command_mode_consistent
        && response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent
        && response_gate_retry_contract_expected_lane_command_consistent
        && response_gate_retry_contract_expected_lane_command_class_consistent
        && response_gate_retry_contract_expected_lane_mode_class_consistent
        && response_gate_retry_contract_expected_lane_class_consistent
        && response_gate_retry_after_seconds_consistent
        && response_gate_retry_after_seconds_non_negative;
    let gate_health_ok = unknown_provider_count == 0
        && watchdog_critical_count == 0
        && (tool_lane_rows == 0 || execution_attempted_count >= execution_skipped_count);
    let manual_intervention_required =
        watchdog_triggered || policy_block_count > 0 || unknown_provider_count > 0;
    let provider_quality_tier = if tool_lane_rows == 0 || provider_missing_count == 0 {
        "good"
    } else if provider_resolved_count >= provider_missing_count {
        "mixed"
    } else {
        "poor"
    };
    let decision_confidence = if tool_lane_rows == 0 {
        1.0
    } else {
        let rows = tool_lane_rows as f64;
        let execution_ratio = (execution_attempted_count as f64 / rows).clamp(0.0, 1.0);
        let provider_ratio = (provider_resolved_count as f64 / rows).clamp(0.0, 1.0);
        let watchdog_ratio = if watchdog_triggered { 0.0 } else { 1.0 };
        let completion_ratio = if completion_signal_ok { 1.0 } else { 0.0 };
        ((execution_ratio * 0.40)
            + (provider_ratio * 0.30)
            + (watchdog_ratio * 0.20)
            + (completion_ratio * 0.10))
        .clamp(0.0, 1.0)
    };
    let decision_confidence = (decision_confidence * 10_000.0).round() / 10_000.0;
    let decision_confidence_label = if decision_confidence >= 0.85 {
        "high_confidence"
    } else if decision_confidence >= 0.60 {
        "medium_confidence"
    } else {
        "low_confidence"
    };
    let decision_rationale_blurb = if !answer_contract_ok {
        "Output contract failed; hold response and repair before surfacing."
    } else if !final_response_contract_ok {
        "Final response contract is incomplete; retry with full contract coverage."
    } else if watchdog_triggered || unknown_provider_count > 0 {
        "Runtime health is degraded; route through troubleshooting before user-facing claims."
    } else if decision_confidence_label == "high_confidence" {
        "Signals are aligned across execution, provider resolution, and completion checks."
    } else if decision_confidence_label == "medium_confidence" {
        "Core checks passed, but some lanes need follow-up monitoring."
    } else {
        "Confidence is limited; collect more evidence before asserting strong conclusions."
    };
    let requires_snapshot = manual_intervention_required
        || !completion_signal_ok
        || provider_quality_tier == "poor"
        || llm_reliability_tier == "low"
        || !final_response_contract_ok
        || !answer_contract_ok;
    let next_action = if !answer_contract_ok {
        "enforce_final_response_contract_and_emit_nonempty_answer"
    } else if hallucination_pattern_detected {
        "capture_workflow_snapshot_and_run_eval_audit"
    } else if placeholder_output_detected {
        "enforce_final_response_contract_and_retry_once"
    } else if watchdog_triggered {
        "inspect_recent_signature_loops_and_pause_auto_retry"
    } else if !completion_signal_ok {
        "verify_workflow_completion_signal_contract_before_retry"
    } else if unknown_provider_count > 0 {
        "refresh_provider_config_and_retry"
    } else if policy_block_count > 0 {
        "resolve_policy_or_auth_and_retry"
    } else if meta_block_count > 0 {
        "narrow_to_explicit_web_query_or_use_override"
    } else {
        "continue_normal_observation"
    };
    let next_action_lane = if !answer_contract_ok {
        "dashboard.troubleshooting.recent.state"
    } else if hallucination_pattern_detected {
        "dashboard.troubleshooting.snapshot.capture"
    } else if placeholder_output_detected {
        "dashboard.troubleshooting.summary"
    } else if watchdog_triggered {
        "dashboard.troubleshooting.summary"
    } else if !completion_signal_ok {
        "dashboard.troubleshooting.recent.state"
    } else if unknown_provider_count > 0 {
        "dashboard.troubleshooting.summary.queue_health"
    } else if policy_block_count > 0 {
        "dashboard.troubleshooting.outbox.state"
    } else if meta_block_count > 0 {
        "dashboard.troubleshooting.summary"
    } else {
        "none"
    };
    let next_action_routable = next_action_lane != "none";

