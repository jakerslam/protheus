pub fn compute_execute_confidence_cooldown_key(
    input: &ExecuteConfidenceCooldownKeyInput,
) -> ExecuteConfidenceCooldownKeyOutput {
    let objective =
        sanitize_directive_objective_id_single_digit(input.objective_id.as_deref().unwrap_or(""));
    if !objective.is_empty() {
        let token = sanitize_cooldown_fragment(&objective);
        if !token.is_empty() {
            return ExecuteConfidenceCooldownKeyOutput {
                cooldown_key: format!("exec_confidence:objective:{token}"),
            };
        }
    }

    let capability = sanitize_cooldown_fragment(input.capability_key.as_deref().unwrap_or(""));
    if !capability.is_empty() {
        return ExecuteConfidenceCooldownKeyOutput {
            cooldown_key: format!("exec_confidence:capability:{capability}"),
        };
    }

    let proposal_type = sanitize_cooldown_fragment(input.proposal_type.as_deref().unwrap_or(""));
    if !proposal_type.is_empty() {
        return ExecuteConfidenceCooldownKeyOutput {
            cooldown_key: format!("exec_confidence:type:{proposal_type}"),
        };
    }

    ExecuteConfidenceCooldownKeyOutput {
        cooldown_key: String::new(),
    }
}

pub fn compute_qos_lane_weights(input: &QosLaneWeightsInput) -> QosLaneWeightsOutput {
    let pressure = input
        .pressure
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "normal".to_string());
    let mut out = QosLaneWeightsOutput {
        critical: input.critical_weight,
        standard: input.standard_weight,
        explore: input.explore_weight,
        quarantine: input.quarantine_weight,
    };
    if pressure == "warning" {
        out.explore = round6(out.explore * 0.75);
        out.quarantine = round6(out.quarantine * 0.35);
    } else if pressure == "critical" {
        out.critical = round6(out.critical * 1.2);
        out.standard = round6(out.standard * 1.1);
        out.explore = round6(out.explore * 0.3);
        out.quarantine = round6(out.quarantine * 0.1);
    }
    out
}

pub fn compute_qos_lane_usage(input: &QosLaneUsageInput) -> QosLaneUsageOutput {
    let mut out = QosLaneUsageOutput {
        critical: 0,
        standard: 0,
        explore: 0,
        quarantine: 0,
    };
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" || result != "executed" {
            continue;
        }
        let mode = evt
            .selection_mode
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if mode.contains("qos_critical_") {
            out.critical += 1;
        } else if mode.contains("qos_standard_") {
            out.standard += 1;
        } else if mode.contains("qos_explore_") {
            out.explore += 1;
        } else if mode.contains("qos_quarantine_") {
            out.quarantine += 1;
        }
    }
    out
}

pub fn compute_qos_lane_share_cap_exceeded(
    input: &QosLaneShareCapExceededInput,
) -> QosLaneShareCapExceededOutput {
    if input.executed_count <= 0.0 {
        return QosLaneShareCapExceededOutput { exceeded: false };
    }
    let lane = input
        .lane
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let exceeded = if lane == "explore" {
        (input.explore_usage / input.executed_count) >= input.explore_max_share
    } else if lane == "quarantine" {
        (input.quarantine_usage / input.executed_count) >= input.quarantine_max_share
    } else {
        false
    };
    QosLaneShareCapExceededOutput { exceeded }
}

pub fn compute_qos_lane_from_candidate(
    input: &QosLaneFromCandidateInput,
) -> QosLaneFromCandidateOutput {
    if input.queue_underflow_backfill {
        return QosLaneFromCandidateOutput {
            lane: "quarantine".to_string(),
        };
    }
    if input.pulse_tier <= 1 {
        return QosLaneFromCandidateOutput {
            lane: "critical".to_string(),
        };
    }
    let proposal_type = input
        .proposal_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if proposal_type == "directive_clarification" || proposal_type == "directive_decomposition" {
        return QosLaneFromCandidateOutput {
            lane: "critical".to_string(),
        };
    }
    if input.deprioritized_source {
        return QosLaneFromCandidateOutput {
            lane: "quarantine".to_string(),
        };
    }
    let risk = normalize_risk_level(input.risk.as_deref().unwrap_or(""));
    if risk == "medium" {
        return QosLaneFromCandidateOutput {
            lane: "explore".to_string(),
        };
    }
    QosLaneFromCandidateOutput {
        lane: "standard".to_string(),
    }
}

fn parse_rfc3339_ts_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
}

pub fn compute_eye_outcome_count_window(
    input: &EyeOutcomeWindowCountInput,
) -> EyeOutcomeWindowCountOutput {
    let eye_ref = input
        .eye_ref
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if eye_ref.is_empty() {
        return EyeOutcomeWindowCountOutput { count: 0 };
    }
    let outcome = input
        .outcome
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    let end_date_raw = input
        .end_date_str
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    let end_date = NaiveDate::parse_from_str(&end_date_raw, "%Y-%m-%d").ok();
    let Some(end_date) = end_date else {
        return EyeOutcomeWindowCountOutput { count: 0 };
    };
    let days = input.days.unwrap_or(1).max(1);
    let end_dt = end_date.and_hms_milli_opt(23, 59, 59, 999);
    let start_date = end_date - Duration::days(days - 1);
    let start_dt = start_date.and_hms_milli_opt(0, 0, 0, 0);
    let (Some(end_dt), Some(start_dt)) = (end_dt, start_dt) else {
        return EyeOutcomeWindowCountOutput { count: 0 };
    };
    let end_ms = end_dt.and_utc().timestamp_millis();
    let start_ms = start_dt.and_utc().timestamp_millis();

    let mut count: u32 = 0;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "outcome" {
            continue;
        }
        let event_outcome = evt
            .outcome
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if event_outcome != outcome {
            continue;
        }
        let evidence_ref = evt
            .evidence_ref
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        if !evidence_ref.contains(&eye_ref) {
            continue;
        }
        let ts_ms = evt.ts.as_ref().and_then(|v| parse_rfc3339_ts_ms(v.trim()));
        let Some(ts_ms) = ts_ms else {
            continue;
        };
        if ts_ms < start_ms || ts_ms > end_ms {
            continue;
        }
        count += 1;
    }
    EyeOutcomeWindowCountOutput { count }
}

pub fn compute_eye_outcome_count_last_hours(
    input: &EyeOutcomeLastHoursCountInput,
) -> EyeOutcomeLastHoursCountOutput {
    let eye_ref = input
        .eye_ref
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    let hours = input.hours.unwrap_or(0.0);
    if eye_ref.is_empty() || !hours.is_finite() || hours <= 0.0 {
        return EyeOutcomeLastHoursCountOutput { count: 0 };
    }
    let outcome = input
        .outcome
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    let now_ms = if let Some(v) = input.now_ms {
        if v.is_finite() {
            v
        } else {
            0.0
        }
    } else {
        Utc::now().timestamp_millis() as f64
    };
    let cutoff = now_ms - (hours * 3_600_000.0);

    let mut count: u32 = 0;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "outcome" {
            continue;
        }
        let event_outcome = evt
            .outcome
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if event_outcome != outcome {
            continue;
        }
        let evidence_ref = evt
            .evidence_ref
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        if !evidence_ref.contains(&eye_ref) {
            continue;
        }
        let ts_ms = evt.ts.as_ref().and_then(|v| parse_rfc3339_ts_ms(v.trim()));
        let Some(ts_ms) = ts_ms else {
            continue;
        };
        if (ts_ms as f64) < cutoff {
            continue;
        }
        count += 1;
    }
    EyeOutcomeLastHoursCountOutput { count }
}

pub fn compute_no_progress_result(input: &NoProgressResultInput) -> NoProgressResultOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return NoProgressResultOutput {
            is_no_progress: false,
        };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if result == "executed" {
        let outcome = input
            .outcome
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        return NoProgressResultOutput {
            is_no_progress: outcome != "shipped" && outcome != "success" && outcome != "applied",
        };
    }

    let is_no_progress = result == "init_gate_stub"
        || result == "init_gate_low_score"
        || result == "init_gate_blocked_route"
        || result == "score_only_preview"
        || result == "score_only_fallback_route_block"
        || result == "score_only_fallback_low_execution_confidence"
        || result == "stop_repeat_gate_capability_cap"
        || result == "stop_repeat_gate_directive_pulse_cooldown"
        || result == "stop_repeat_gate_directive_pulse_tier_reservation"
        || result == "stop_repeat_gate_human_escalation_pending"
        || result == "stop_repeat_gate_stale_signal"
        || result == "stop_repeat_gate_circuit_breaker"
        || result == "stop_init_gate_quality_exhausted"
        || result == "stop_init_gate_directive_fit_exhausted"
        || result == "stop_init_gate_actionability_exhausted"
        || result == "stop_init_gate_optimization_good_enough"
        || result == "stop_init_gate_value_signal_exhausted"
        || result == "stop_init_gate_tier1_governance"
        || result == "stop_init_gate_medium_risk_guard"
        || result == "stop_init_gate_medium_requires_canary"
        || result == "stop_init_gate_composite_exhausted"
        || result == "stop_repeat_gate_capability_cooldown"
        || result == "stop_repeat_gate_capability_no_change_cooldown"
        || result == "stop_repeat_gate_medium_canary_cap"
        || result == "stop_repeat_gate_candidate_exhausted"
        || result == "stop_repeat_gate_preview_churn_cooldown"
        || result == "stop_repeat_gate_exhaustion_cooldown"
        || result == "stop_repeat_gate_no_progress"
        || result == "stop_repeat_gate_dopamine";
    NoProgressResultOutput { is_no_progress }
}

pub fn compute_attempt_run_event(input: &AttemptRunEventInput) -> AttemptRunEventOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return AttemptRunEventOutput { is_attempt: false };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let is_attempt = result == "executed"
        || result == "init_gate_stub"
        || result == "init_gate_low_score"
        || result == "init_gate_blocked_route"
        || result == "score_only_preview"
        || result == "score_only_fallback_route_block"
        || result == "score_only_fallback_low_execution_confidence"
        || result == "stop_repeat_gate_directive_pulse_cooldown"
        || result == "stop_repeat_gate_directive_pulse_tier_reservation"
        || result == "stop_repeat_gate_human_escalation_pending"
        || result == "stop_repeat_gate_capability_cap"
        || result == "stop_repeat_gate_stale_signal"
        || result == "stop_repeat_gate_circuit_breaker"
        || result == "stop_init_gate_quality_exhausted"
        || result == "stop_init_gate_directive_fit_exhausted"
        || result == "stop_init_gate_actionability_exhausted"
        || result == "stop_init_gate_optimization_good_enough"
        || result == "stop_init_gate_value_signal_exhausted"
        || result == "stop_init_gate_tier1_governance"
        || result == "stop_init_gate_composite_exhausted"
        || result == "stop_repeat_gate_capability_cooldown"
        || result == "stop_repeat_gate_capability_no_change_cooldown"
        || result == "stop_repeat_gate_preview_churn_cooldown"
        || result == "stop_repeat_gate_exhaustion_cooldown"
        || result == "stop_repeat_gate_candidate_exhausted";
    AttemptRunEventOutput { is_attempt }
}
pub fn compute_safety_stop_run_event(input: &SafetyStopRunEventInput) -> SafetyStopRunEventOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return SafetyStopRunEventOutput {
            is_safety_stop: false,
        };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let is_safety_stop = result.contains("human_escalation")
        || result.contains("tier1_governance")
        || result.contains("medium_risk_guard")
        || result.contains("capability_cooldown")
        || result.contains("directive_pulse_tier_reservation");
    SafetyStopRunEventOutput { is_safety_stop }
}
