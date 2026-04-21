
pub fn evaluate_trace_window(
    profile: &EmbeddedObservabilityProfile,
    events: &[TraceEvent],
) -> TraceWindowReport {
    let accepted = capped_events(profile, events);
    let dropped_events = events.len().saturating_sub(accepted.len());

    let high_severity_events = accepted
        .iter()
        .filter(|event| {
            let sev = event.severity.to_ascii_lowercase();
            sev == "critical" || sev == "high"
        })
        .count();

    let red_legion_channels_triggered = profile
        .red_legion_trace_channels
        .iter()
        .filter(|channel| channel_triggered(channel, &accepted))
        .cloned()
        .collect::<Vec<_>>();

    let event_digest = digest_lines(
        &accepted
            .iter()
            .map(event_fingerprint)
            .collect::<Vec<String>>(),
    );

    let drift_weight_sum = accepted
        .iter()
        .filter(|event| {
            event
                .tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase().contains("drift"))
        })
        .map(|event| severity_weight(&event.severity))
        .sum::<f64>();

    let drift_score_pct = if accepted.is_empty() {
        0.0
    } else {
        ((drift_weight_sum / accepted.len() as f64) * 100.0).clamp(0.0, 100.0)
    };

    TraceWindowReport {
        accepted_events: accepted.len(),
        dropped_events,
        high_severity_events,
        red_legion_channels_triggered,
        event_digest,
        drift_score_pct: round3(drift_score_pct),
    }
}

pub fn compute_sovereignty_index(
    profile: &EmbeddedObservabilityProfile,
    events: &[TraceEvent],
    trace_report: &TraceWindowReport,
    inject_fault_every: u32,
    enforce_fail_closed: bool,
) -> SovereigntyIndex {
    let accepted_events = capped_events(profile, events);

    let integrity_component_pct = if accepted_events.is_empty() {
        100.0
    } else {
        let signed = accepted_events.iter().filter(|event| event.signed).count();
        (signed as f64 / accepted_events.len() as f64) * 100.0
    };

    let continuity_component_pct =
        continuity_component(&accepted_events, profile.stream_policy.trace_window_ms);
    let reliability_component_pct = reliability_component(events, trace_report.accepted_events);

    let fault_penalty = if inject_fault_every == 0 {
        0.0
    } else {
        (100.0 / inject_fault_every as f64).clamp(0.0, 40.0)
    };
    let drift_penalty = (trace_report.drift_score_pct * 0.25).clamp(0.0, 25.0);
    let chaos_penalty_pct = (fault_penalty + drift_penalty).clamp(0.0, 100.0);

    let weights = &profile.sovereignty_scorer;
    let weighted_score = ((integrity_component_pct * weights.integrity_weight_pct as f64)
        + (continuity_component_pct * weights.continuity_weight_pct as f64)
        + (reliability_component_pct * weights.reliability_weight_pct as f64))
        / 100.0
        - ((chaos_penalty_pct * weights.chaos_penalty_pct as f64) / 100.0);

    let score_pct = round3(weighted_score.clamp(0.0, 100.0));

    let mut reasons: Vec<String> = Vec::new();
    if integrity_component_pct < 70.0 {
        reasons.push("integrity_component_below_70".to_string());
    }
    if continuity_component_pct < 70.0 {
        reasons.push("continuity_component_below_70".to_string());
    }
    if reliability_component_pct < 70.0 {
        reasons.push("reliability_component_below_70".to_string());
    }
    if chaos_penalty_pct > 15.0 {
        reasons.push("chaos_penalty_above_15".to_string());
    }

    let tamper_critical = accepted_events.iter().any(|event| {
        event.severity.eq_ignore_ascii_case("critical")
            && event
                .tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase().contains("tamper"))
    });

    if tamper_critical {
        reasons.push("critical_tamper_detected".to_string());
    }

    let threshold = profile.sovereignty_scorer.fail_closed_threshold_pct as f64;
    let fail_closed =
        (score_pct < threshold && enforce_fail_closed) || (tamper_critical && enforce_fail_closed);
    let status = if fail_closed {
        "fail_closed".to_string()
    } else if score_pct < threshold {
        "degraded".to_string()
    } else {
        "stable".to_string()
    };

    SovereigntyIndex {
        score_pct,
        fail_closed,
        status,
        reasons,
        integrity_component_pct: round3(integrity_component_pct),
        continuity_component_pct: round3(continuity_component_pct),
        reliability_component_pct: round3(reliability_component_pct),
        chaos_penalty_pct: round3(chaos_penalty_pct),
    }
}

pub fn run_chaos_resilience(
    request: &ChaosScenarioRequest,
) -> Result<ChaosResilienceReport, ObservabilityError> {
    if request.events.is_empty() {
        return Err(ObservabilityError::InvalidRequest(
            "events_required".to_string(),
        ));
    }
    let profile = load_embedded_observability_profile()?;
    let runtime_envelope = load_embedded_observability_runtime_envelope().ok();

    let trace_report = evaluate_trace_window(&profile, &request.events);
    let sovereignty = compute_sovereignty_index(
        &profile,
        &request.events,
        &trace_report,
        request.inject_fault_every,
        request.enforce_fail_closed,
    );

    let accepted_events = capped_events(&profile, &request.events);

    let hooks_fired = profile
        .chaos_hooks
        .iter()
        .filter(|hook| hook_triggered(hook, &trace_report, &accepted_events))
        .map(|hook| hook.id.clone())
        .collect::<Vec<_>>();

    let telemetry_overhead_ms = round3(
        (trace_report.accepted_events as f64 * 0.00045)
            + (trace_report.red_legion_channels_triggered.len() as f64 * 0.08)
            + 0.12,
    );

    let inject_factor = if request.inject_fault_every == 0 {
        0.0
    } else {
        (250.0 / request.inject_fault_every as f64).clamp(0.05, 2.5)
    };

    let chaos_battery_pct_24h = round3(
        (request.cycles as f64 / 200000.0) * 1.2
            + (trace_report.high_severity_events as f64 * 0.01)
            + inject_factor
            + 0.25,
    );

    let telemetry_cap = runtime_envelope
        .as_ref()
        .map(|v| v.max_telemetry_overhead_ms)
        .unwrap_or(1.0);
    let battery_cap = runtime_envelope
        .as_ref()
        .map(|v| v.max_battery_pct_24h)
        .unwrap_or(3.0);
    let drift_cap = runtime_envelope
        .as_ref()
        .map(|v| v.max_drift_pct)
        .unwrap_or(2.0);

    let drift_exceeded = trace_report.drift_score_pct > drift_cap;
    let envelope_fail_closed = runtime_envelope
        .as_ref()
        .map(|v| {
            v.enforce_fail_closed
                && (drift_exceeded
                    || telemetry_overhead_ms > telemetry_cap
                    || chaos_battery_pct_24h > battery_cap)
        })
        .unwrap_or(false);

    let resilient = !sovereignty.fail_closed
        && !envelope_fail_closed
        && !drift_exceeded
        && telemetry_overhead_ms <= telemetry_cap
        && chaos_battery_pct_24h <= battery_cap;

    Ok(ChaosResilienceReport {
        profile_id: profile.profile_id,
        scenario_id: normalize_text(&request.scenario_id, 160),
        hooks_fired,
        trace_report,
        sovereignty,
        telemetry_overhead_ms,
        chaos_battery_pct_24h,
        resilient,
    })
}

pub fn run_chaos_resilience_json(request_json: &str) -> Result<String, ObservabilityError> {
    let request: ChaosScenarioRequest = serde_json::from_str(request_json)
        .map_err(|err| ObservabilityError::InvalidRequest(format!("request_parse_failed:{err}")))?;
    let report = run_chaos_resilience(&request)?;
    serde_json::to_string(&report).map_err(|err| ObservabilityError::EncodeFailed(err.to_string()))
}

pub fn load_embedded_observability_profile_json() -> Result<String, ObservabilityError> {
    let profile = load_embedded_observability_profile()?;
    serde_json::to_string(&profile).map_err(|err| ObservabilityError::EncodeFailed(err.to_string()))
}

fn c_str_to_string(ptr: *const c_char) -> Result<String, ObservabilityError> {
    if ptr.is_null() {
        return Err(ObservabilityError::InvalidRequest(
            "null_pointer".to_string(),
        ));
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| ObservabilityError::InvalidRequest("invalid_utf8".to_string()))?;
    Ok(s.to_string())
}
