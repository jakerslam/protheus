    let provider_resolution_ok =
        tool_lane_rows == 0 || provider_resolved_count >= provider_missing_count;
    let completion_signal_ok =
        tool_lane_rows == 0 || completion_signal_missing_count.saturating_mul(2) < tool_lane_rows;
    let watchdog_triggered = watchdog_critical_count > 0 || watchdog_warning_count >= 2;
    let hallucination_pattern_detected = hallucination_pattern_count > 0 || invalid_draft_count > 0;
    let placeholder_output_detected = placeholder_output_count > 0;
    let no_result_pattern_detected = no_result_pattern_count > 0;
    let llm_reliability_tier = if hallucination_pattern_detected
        || placeholder_output_count >= 2
        || context_mismatch_count >= 2
        || no_result_pattern_count >= 2
    {
        "low"
    } else if context_mismatch_count > 0 || placeholder_output_detected || no_result_pattern_detected
    {
        "medium"
    } else {
        "high"
    };
    let llm_reliability_not_low = llm_reliability_tier != "low";
    let answer_contract_ok = answer_missing_after_completion_count == 0;
    let answer_signal_coverage = if rows.is_empty() {
        1.0
    } else {
        (answer_emitted_count as f64 / rows.len() as f64).clamp(0.0, 1.0)
    };
    let answer_signal_coverage = (answer_signal_coverage * 10_000.0).round() / 10_000.0;
    let final_response_contract_violation_count = completion_signal_missing_count
        .saturating_add(invalid_draft_count)
        .saturating_add(hallucination_pattern_count)
        .saturating_add(placeholder_output_count)
        .saturating_add(no_result_pattern_count)
        .saturating_add(answer_missing_after_completion_count);
    let final_response_contract_ok = final_response_contract_violation_count == 0;
    let response_gate_ready = final_response_contract_ok
        && answer_contract_ok
        && llm_reliability_not_low
        && !watchdog_triggered;
    let response_gate_score = dashboard_response_gate_score_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_expected_score = dashboard_response_gate_score_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_score_consistent =
        (response_gate_score - response_gate_expected_score).abs() <= 0.0001;
    let response_gate_severity =
        dashboard_response_gate_severity_from_state(response_gate_ready, response_gate_score);
    let response_gate_expected_severity = dashboard_response_gate_severity_from_state(
        response_gate_ready,
        response_gate_score,
    );
    let response_gate_score_band =
        dashboard_response_gate_score_band_from_state(response_gate_ready, response_gate_score);
    let response_gate_expected_score_band = dashboard_response_gate_score_band_from_state(
        response_gate_expected_severity == "ready",
        response_gate_expected_score,
    );
    let response_gate_score_band_consistent =
        response_gate_score_band == response_gate_expected_score_band;
    let response_gate_score_band_known = matches!(
        response_gate_score_band,
        "ready" | "strong" | "watch" | "weak" | "critical"
    );
    let response_gate_score_vector_key = format!(
        "score={:.4};severity={}",
        response_gate_score, response_gate_severity
    );
    let response_gate_expected_score_vector_key = format!(
        "score={:.4};severity={}",
        response_gate_expected_score, response_gate_expected_severity
    );
    let response_gate_score_vector_consistent =
        response_gate_score_vector_key == response_gate_expected_score_vector_key;
    let response_gate_score_band_vector_key = format!(
        "band={};score={:.4}",
        response_gate_score_band, response_gate_score
    );
    let response_gate_expected_score_band_vector_key = format!(
        "band={};score={:.4}",
        response_gate_expected_score_band, response_gate_expected_score
    );
    let response_gate_score_band_vector_consistent =
        response_gate_score_band_vector_key == response_gate_expected_score_band_vector_key;
    let response_gate_expected_severity_from_score_band = if response_gate_score_band == "ready" {
        "ready"
    } else if matches!(response_gate_score_band, "strong" | "watch") {
        "degraded"
    } else {
        "blocked"
    };
    let response_gate_score_band_severity_consistent =
        response_gate_severity == response_gate_expected_severity_from_score_band;
    let response_gate_score_band_severity_bucket_consistent =
        dashboard_response_gate_score_band_severity_bucket_consistent(
            response_gate_severity,
            response_gate_score_band,
        );
    let response_gate_score_band_severity_bucket_known =
        dashboard_response_gate_score_band_severity_bucket_known(
            response_gate_severity,
            response_gate_score_band,
        );
