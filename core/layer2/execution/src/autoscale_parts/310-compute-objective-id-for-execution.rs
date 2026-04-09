pub fn compute_objective_id_for_execution(
    input: &ObjectiveIdForExecutionInput,
) -> ObjectiveIdForExecutionOutput {
    let candidates = [
        input.objective_binding_id.as_deref().unwrap_or(""),
        input.directive_pulse_id.as_deref().unwrap_or(""),
        input.directive_action_id.as_deref().unwrap_or(""),
        input.meta_objective_id.as_deref().unwrap_or(""),
        input.meta_directive_objective_id.as_deref().unwrap_or(""),
        input.action_spec_objective_id.as_deref().unwrap_or(""),
    ];
    for candidate in candidates {
        let sanitized =
            compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
                value: Some(candidate.to_string()),
            });
        if !sanitized.objective_id.is_empty() {
            return ObjectiveIdForExecutionOutput {
                objective_id: Some(sanitized.objective_id),
            };
        }
    }
    ObjectiveIdForExecutionOutput { objective_id: None }
}

pub fn compute_short_text(input: &ShortTextInput) -> ShortTextOutput {
    let text = input.value.as_deref().unwrap_or("").to_string();
    let max = input
        .max_len
        .and_then(|v| {
            if v.is_finite() && v >= 0.0 {
                Some(v as usize)
            } else {
                None
            }
        })
        .unwrap_or(220usize);
    if text.chars().count() <= max {
        return ShortTextOutput { text };
    }
    let truncated: String = text.chars().take(max).collect();
    ShortTextOutput {
        text: format!("{truncated}..."),
    }
}

pub fn compute_normalized_signal_status(
    input: &NormalizedSignalStatusInput,
) -> NormalizedSignalStatusOutput {
    let raw = normalize_spaces(input.value.as_deref().unwrap_or("")).to_ascii_lowercase();
    if raw == "pass" || raw == "warn" || raw == "fail" {
        return NormalizedSignalStatusOutput { status: raw };
    }
    let fallback = input.fallback.as_deref().unwrap_or("unknown").to_string();
    NormalizedSignalStatusOutput { status: fallback }
}

pub fn compute_execution_reserve_snapshot(
    input: &ExecutionReserveSnapshotInput,
) -> ExecutionReserveSnapshotOutput {
    let token_cap = input.cap.max(0.0);
    let used_est = input.used.max(0.0);
    let reserve_target = if input.reserve_enabled {
        (token_cap * input.reserve_ratio)
            .round()
            .max(input.reserve_min_tokens)
    } else {
        0.0
    };
    let reserve_tokens = reserve_target.max(0.0).min(token_cap);
    let spend_beyond_non_reserve = (used_est - (token_cap - reserve_tokens).max(0.0)).max(0.0);
    let reserve_remaining = (reserve_tokens - spend_beyond_non_reserve).max(0.0);
    ExecutionReserveSnapshotOutput {
        enabled: input.reserve_enabled,
        reserve_tokens,
        reserve_remaining,
    }
}

pub fn compute_budget_pacing_gate(input: &BudgetPacingGateInput) -> BudgetPacingGateOutput {
    if !input.budget_pacing_enabled {
        return BudgetPacingGateOutput {
            pass: true,
            reason: None,
            execution_reserve_bypass: false,
        };
    }
    if !input.snapshot_tight {
        return BudgetPacingGateOutput {
            pass: true,
            reason: None,
            execution_reserve_bypass: false,
        };
    }
    let value_score = if !input.value_signal_score.is_finite() {
        0.0
    } else {
        input.value_signal_score.clamp(0.0, 100.0)
    };
    let est_tokens = if !input.est_tokens.is_finite() {
        0.0
    } else {
        input.est_tokens.max(0.0)
    };
    let normalized_risk = input
        .risk
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let high_value_escape = value_score >= (input.min_value_signal_score + 20.0).max(85.0);
    if high_value_escape {
        return BudgetPacingGateOutput {
            pass: true,
            reason: None,
            execution_reserve_bypass: false,
        };
    }
    let reserve_bypass_allowed = input.execution_reserve_enabled
        && input.execution_floor_deficit
        && normalized_risk == "low"
        && value_score >= input.execution_reserve_min_value_signal
        && input.execution_reserve_remaining >= est_tokens;
    if reserve_bypass_allowed {
        return BudgetPacingGateOutput {
            pass: true,
            reason: Some("execution_floor_reserve_bypass".to_string()),
            execution_reserve_bypass: true,
        };
    }
    if input.snapshot_autopause_active && normalized_risk != "low" {
        return BudgetPacingGateOutput {
            pass: false,
            reason: Some("budget_pacing_autopause_risk_guard".to_string()),
            execution_reserve_bypass: false,
        };
    }
    if est_tokens >= input.high_token_threshold && value_score < input.min_value_signal_score {
        return BudgetPacingGateOutput {
            pass: false,
            reason: Some("budget_pacing_high_token_low_value".to_string()),
            execution_reserve_bypass: false,
        };
    }
    if input.snapshot_remaining_ratio <= input.min_remaining_ratio
        && value_score < input.min_value_signal_score
    {
        return BudgetPacingGateOutput {
            pass: false,
            reason: Some("budget_pacing_low_remaining_ratio".to_string()),
            execution_reserve_bypass: false,
        };
    }
    BudgetPacingGateOutput {
        pass: true,
        reason: None,
        execution_reserve_bypass: false,
    }
}

pub fn compute_capability_cap(input: &CapabilityCapInput) -> CapabilityCapOutput {
    let mut keys: Vec<String> = Vec::new();
    if let Some(primary) = input.primary_key.as_deref() {
        let key = primary.trim();
        if !key.is_empty() {
            keys.push(key.to_string());
        }
    }
    for alias in &input.aliases {
        let key = alias.trim();
        if key.is_empty() {
            continue;
        }
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
    }
    for key in keys {
        if let Some(raw) = input.caps.get(&key) {
            if raw.is_finite() && *raw >= 0.0 {
                return CapabilityCapOutput {
                    cap: Some(raw.round().clamp(0.0, u32::MAX as f64) as u32),
                };
            }
        }
    }
    CapabilityCapOutput { cap: None }
}

pub fn compute_estimate_tokens_for_candidate(
    input: &EstimateTokensForCandidateInput,
) -> EstimateTokensForCandidateOutput {
    let clamp = |v: f64| -> u32 {
        let rounded = if v.is_finite() { v.round() } else { 80.0 };
        if rounded <= 80.0 {
            80
        } else if rounded >= 12000.0 {
            12000
        } else {
            rounded as u32
        }
    };
    if input.direct_est_tokens.is_finite() && input.direct_est_tokens > 0.0 {
        return EstimateTokensForCandidateOutput {
            est_tokens: clamp(input.direct_est_tokens),
        };
    }
    if input.route_tokens_est.is_finite() && input.route_tokens_est > 0.0 {
        return EstimateTokensForCandidateOutput {
            est_tokens: clamp(input.route_tokens_est),
        };
    }
    EstimateTokensForCandidateOutput {
        est_tokens: clamp(input.fallback_estimate),
    }
}

pub fn compute_proposal_status_for_queue_pressure(
    input: &ProposalStatusForQueuePressureInput,
) -> ProposalStatusForQueuePressureOutput {
    let has_overlay_decision = input
        .overlay_decision
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let mut status = compute_proposal_status(&ProposalStatusInput {
        overlay_decision: input.overlay_decision.clone(),
    })
    .status;
    if has_overlay_decision {
        return ProposalStatusForQueuePressureOutput { status };
    }

    let explicit = compute_normalize_proposal_status(&NormalizeProposalStatusInput {
        raw_status: input.proposal_status.clone(),
        fallback: Some("pending".to_string()),
    })
    .normalized_status;
    if explicit == "accepted"
        || explicit == "closed"
        || explicit == "rejected"
        || explicit == "parked"
    {
        status = explicit;
    }
    ProposalStatusForQueuePressureOutput { status }
}

pub fn compute_minutes_since_ts(input: &MinutesSinceTsInput) -> MinutesSinceTsOutput {
    let ts_text = input
        .ts
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let Some(ts_text) = ts_text else {
        return MinutesSinceTsOutput {
            minutes_since: None,
        };
    };
    let parsed = DateTime::parse_from_rfc3339(ts_text)
        .ok()
        .map(|dt| dt.with_timezone(&Utc));
    let Some(parsed) = parsed else {
        return MinutesSinceTsOutput {
            minutes_since: None,
        };
    };
    let now_ms = input
        .now_ms
        .filter(|v| v.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let ts_ms = parsed.timestamp_millis() as f64;
    let minutes_since = (now_ms - ts_ms) / 60000.0;
    MinutesSinceTsOutput {
        minutes_since: Some(minutes_since),
    }
}

pub fn compute_date_window(input: &DateWindowInput) -> DateWindowOutput {
    let end_date_str = input
        .end_date_str
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let Some(end_date_str) = end_date_str else {
        return DateWindowOutput { dates: Vec::new() };
    };
    let end = NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d").ok();
    let Some(end) = end else {
        return DateWindowOutput { dates: Vec::new() };
    };
    let days = input.days.filter(|v| v.is_finite()).unwrap_or(0.0);
    if days <= 0.0 {
        return DateWindowOutput { dates: Vec::new() };
    }
    let mut dates: Vec<String> = Vec::new();
    let mut i = 0.0_f64;
    while i < days {
        let d = end - Duration::days(i as i64);
        dates.push(d.format("%Y-%m-%d").to_string());
        i += 1.0;
    }
    DateWindowOutput { dates }
}

pub fn compute_in_window(input: &InWindowInput) -> InWindowOutput {
    let ts = input
        .ts
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let Some(ts) = ts else {
        return InWindowOutput { in_window: false };
    };
    let end_date = input
        .end_date_str
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
    let Some(end_date) = end_date else {
        return InWindowOutput { in_window: false };
    };
    let end_naive = end_date
        .and_hms_milli_opt(23, 59, 59, 999)
        .unwrap_or_else(|| end_date.and_hms_opt(23, 59, 59).expect("valid hms"));
    let end = DateTime::<Utc>::from_naive_utc_and_offset(end_naive, Utc);
    let days = input.days.filter(|v| v.is_finite()).unwrap_or(0.0);
    if days <= 0.0 {
        return InWindowOutput { in_window: false };
    }
    let start_offset_days = (days - 1.0).floor() as i64;
    let start_date = end_date - Duration::days(start_offset_days);
    let start_naive = start_date
        .and_hms_milli_opt(0, 0, 0, 0)
        .unwrap_or_else(|| start_date.and_hms_opt(0, 0, 0).expect("valid hms"));
    let start = DateTime::<Utc>::from_naive_utc_and_offset(start_naive, Utc);
    InWindowOutput {
        in_window: ts >= start && ts <= end,
    }
}

pub fn compute_exec_window_match(input: &ExecWindowMatchInput) -> ExecWindowMatchOutput {
    let ts_ms = input.ts_ms.unwrap_or(f64::NAN);
    let start_ms = input.start_ms.unwrap_or(f64::NAN);
    let end_ms = input.end_ms.unwrap_or(f64::NAN);
    if !ts_ms.is_finite() || !start_ms.is_finite() || !end_ms.is_finite() {
        return ExecWindowMatchOutput { in_window: false };
    }
    if start_ms == 0.0 || end_ms == 0.0 {
        return ExecWindowMatchOutput { in_window: false };
    }
    ExecWindowMatchOutput {
        in_window: ts_ms >= start_ms && ts_ms <= end_ms,
    }
}

pub fn compute_start_of_next_utc_day(input: &StartOfNextUtcDayInput) -> StartOfNextUtcDayOutput {
    let date_str = input
        .date_str
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let Some(date_str) = date_str else {
        return StartOfNextUtcDayOutput { iso_ts: None };
    };
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
    let Some(date) = date else {
        return StartOfNextUtcDayOutput { iso_ts: None };
    };
    let next = date + Duration::days(1);
    StartOfNextUtcDayOutput {
        iso_ts: Some(format!("{}T00:00:00.000Z", next.format("%Y-%m-%d"))),
    }
}
pub fn compute_iso_after_minutes(input: &IsoAfterMinutesInput) -> IsoAfterMinutesOutput {
    let minutes = input.minutes.filter(|v| v.is_finite());
    let Some(minutes) = minutes else {
        return IsoAfterMinutesOutput { iso_ts: None };
    };
    let safe_minutes = if minutes < 0.0 { 0.0 } else { minutes };
    let now_ms = input
        .now_ms
        .filter(|v| v.is_finite())
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    let target_ms = now_ms + (safe_minutes * 60_000.0);
    if !target_ms.is_finite() || target_ms < i64::MIN as f64 || target_ms > i64::MAX as f64 {
        return IsoAfterMinutesOutput { iso_ts: None };
    }
    let target = DateTime::<Utc>::from_timestamp_millis(target_ms as i64);
    let Some(target) = target else {
        return IsoAfterMinutesOutput { iso_ts: None };
    };
    IsoAfterMinutesOutput {
        iso_ts: Some(target.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
    }
}

pub fn compute_execute_confidence_history_match(
    input: &ExecuteConfidenceHistoryMatchInput,
) -> ExecuteConfidenceHistoryMatchOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return ExecuteConfidenceHistoryMatchOutput { matched: false };
    }
    let capability_key = input
        .capability_key
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let event_capability_key = input
        .event_capability_key
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !capability_key.is_empty() && !event_capability_key.is_empty() {
        return ExecuteConfidenceHistoryMatchOutput {
            matched: event_capability_key == capability_key,
        };
    }
    let proposal_type = input
        .proposal_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let event_proposal_type = input
        .event_proposal_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !proposal_type.is_empty() && !event_proposal_type.is_empty() {
        return ExecuteConfidenceHistoryMatchOutput {
            matched: event_proposal_type == proposal_type,
        };
    }
    ExecuteConfidenceHistoryMatchOutput { matched: false }
}
