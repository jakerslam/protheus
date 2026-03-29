pub fn compute_medium_risk_thresholds(
    input: &MediumRiskThresholdsInput,
) -> MediumRiskThresholdsOutput {
    let composite_min = input
        .medium_risk_min_composite_eligibility
        .max(input.min_composite_eligibility + 6.0);
    let directive_base = if input.base_min_directive_fit.is_finite() {
        input.base_min_directive_fit
    } else {
        input.default_min_directive_fit
    };
    let actionability_base = if input.base_min_actionability_score.is_finite() {
        input.base_min_actionability_score
    } else {
        input.default_min_actionability
    };
    let directive_fit_min = input
        .medium_risk_min_directive_fit
        .max(directive_base + 5.0);
    let actionability_min = input
        .medium_risk_min_actionability
        .max(actionability_base + 6.0);
    MediumRiskThresholdsOutput {
        composite_min,
        directive_fit_min,
        actionability_min,
    }
}

pub fn compute_medium_risk_gate_decision(
    input: &MediumRiskGateDecisionInput,
) -> MediumRiskGateDecisionOutput {
    let risk = input
        .risk
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| v == "low" || v == "medium" || v == "high")
        .unwrap_or_else(|| "low".to_string());
    if risk != "medium" {
        return MediumRiskGateDecisionOutput {
            pass: true,
            risk,
            reasons: Vec::new(),
            required: None,
        };
    }
    let required = MediumRiskThresholdsOutput {
        composite_min: input.composite_min,
        directive_fit_min: input.directive_fit_min,
        actionability_min: input.actionability_min,
    };
    let mut reasons = Vec::<String>::new();
    if input.composite_score < required.composite_min {
        reasons.push("medium_composite_low".to_string());
    }
    if input.directive_fit_score < required.directive_fit_min {
        reasons.push("medium_directive_fit_low".to_string());
    }
    if input.actionability_score < required.actionability_min {
        reasons.push("medium_actionability_low".to_string());
    }
    MediumRiskGateDecisionOutput {
        pass: reasons.is_empty(),
        risk,
        reasons,
        required: Some(required),
    }
}

pub fn compute_route_block_prefilter(
    input: &RouteBlockPrefilterInput,
) -> RouteBlockPrefilterOutput {
    let key = input
        .capability_key
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());
    let mut out = RouteBlockPrefilterOutput {
        enabled: input.enabled,
        applicable: false,
        pass: true,
        reason: "disabled".to_string(),
        capability_key: key.clone(),
        window_hours: input.window_hours,
        min_observations: input.min_observations,
        max_block_rate: input.max_block_rate,
        attempts: 0.0,
        route_blocked: 0.0,
        route_block_rate: 0.0,
    };
    if !input.enabled {
        return out;
    }
    out.reason = "missing_capability_key".to_string();
    if key.is_none() {
        return out;
    }
    out.applicable = true;
    out.reason = "no_recent_route_samples".to_string();
    if !input.row_present {
        return out;
    }
    out.attempts = input.attempts.max(0.0);
    out.route_blocked = input.route_blocked.max(0.0);
    out.route_block_rate = input.route_block_rate.clamp(0.0, 1.0);
    if out.attempts < input.min_observations {
        out.reason = "insufficient_observations".to_string();
        return out;
    }
    if out.route_block_rate >= input.max_block_rate {
        out.pass = false;
        out.reason = "route_block_rate_exceeded".to_string();
        return out;
    }
    out.reason = "pass".to_string();
    out
}

pub fn compute_route_execution_sample_event(
    input: &RouteExecutionSampleEventInput,
) -> RouteExecutionSampleEventOutput {
    let event_type = input.event_type.as_deref().unwrap_or("");
    if event_type != "autonomy_run" {
        return RouteExecutionSampleEventOutput {
            is_sample_event: false,
        };
    }
    let result = input.result.as_deref().unwrap_or("").trim();
    if result.is_empty() {
        return RouteExecutionSampleEventOutput {
            is_sample_event: false,
        };
    }
    if result == "score_only_fallback_route_block" || result == "init_gate_blocked_route" {
        return RouteExecutionSampleEventOutput {
            is_sample_event: true,
        };
    }
    let target = input
        .execution_target
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if target == "route" {
        return RouteExecutionSampleEventOutput {
            is_sample_event: result == "executed",
        };
    }
    RouteExecutionSampleEventOutput {
        is_sample_event: result == "executed" && input.route_summary_present,
    }
}

pub fn compute_route_block_telemetry_summary(
    input: &RouteBlockTelemetrySummaryInput,
) -> RouteBlockTelemetrySummaryOutput {
    let mut rows = std::collections::HashMap::<String, RouteBlockTelemetryCapabilityOutput>::new();
    for evt in input.events.iter() {
        let sample = compute_route_execution_sample_event(&RouteExecutionSampleEventInput {
            event_type: evt.event_type.clone(),
            result: evt.result.clone(),
            execution_target: evt.execution_target.clone(),
            route_summary_present: evt.route_summary_present,
        });
        if !sample.is_sample_event {
            continue;
        }
        let key = evt
            .capability_key
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if key.is_empty() {
            continue;
        }
        let row = rows
            .entry(key.clone())
            .or_insert_with(|| RouteBlockTelemetryCapabilityOutput {
                key: key.clone(),
                attempts: 0.0,
                route_blocked: 0.0,
                route_block_rate: 0.0,
            });
        row.attempts += 1.0;
        let result = evt.result.as_deref().unwrap_or("").trim();
        if result == "score_only_fallback_route_block" || result == "init_gate_blocked_route" {
            row.route_blocked += 1.0;
        }
    }

    let mut by_capability = rows.into_values().collect::<Vec<_>>();
    by_capability.sort_by(|a, b| a.key.cmp(&b.key));
    for row in by_capability.iter_mut() {
        row.route_block_rate = if row.attempts > 0.0 {
            ((row.route_blocked / row.attempts) * 1000.0).round() / 1000.0
        } else {
            0.0
        };
    }

    RouteBlockTelemetrySummaryOutput {
        window_hours: input.window_hours.max(1.0),
        sample_events: input.events.len() as f64,
        by_capability,
    }
}

pub fn compute_is_stub_proposal(input: &IsStubProposalInput) -> IsStubProposalOutput {
    let title = input.title.as_deref().unwrap_or("");
    IsStubProposalOutput {
        is_stub: title.to_uppercase().contains("[STUB]"),
    }
}

pub fn compute_recent_autonomy_run_events(
    input: &RecentAutonomyRunEventsInput,
) -> RecentAutonomyRunEventsOutput {
    let cutoff_ms = if input.cutoff_ms.is_finite() {
        input.cutoff_ms
    } else {
        0.0
    };
    let mut cap = input.cap;
    if !cap.is_finite() {
        cap = 800.0;
    }
    cap = cap.max(50.0);

    let mut out = Vec::<serde_json::Value>::new();
    for evt in input.events.iter() {
        if (out.len() as f64) >= cap {
            break;
        }
        let event_type = evt
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if event_type != "autonomy_run" {
            continue;
        }
        let ts_raw = evt.get("ts").and_then(|v| v.as_str()).unwrap_or("").trim();
        if ts_raw.is_empty() {
            continue;
        }
        let Some(ts_ms) = parse_rfc3339_ts_ms(ts_raw) else {
            continue;
        };
        if (ts_ms as f64) < cutoff_ms {
            continue;
        }
        out.push(evt.clone());
    }

    RecentAutonomyRunEventsOutput { events: out }
}

pub fn compute_proposal_meta_index(input: &ProposalMetaIndexInput) -> ProposalMetaIndexOutput {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::<ProposalMetaIndexEntryOutput>::new();
    for row in input.entries.iter() {
        let proposal_id = row.proposal_id.as_deref().unwrap_or("").trim().to_string();
        if proposal_id.is_empty() || seen.contains(&proposal_id) {
            continue;
        }
        seen.insert(proposal_id.clone());
        let eye_id = row.eye_id.as_deref().unwrap_or("").trim().to_string();
        let topics = row
            .topics
            .iter()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<String>>();
        out.push(ProposalMetaIndexEntryOutput {
            proposal_id,
            eye_id,
            topics,
        });
    }
    ProposalMetaIndexOutput { entries: out }
}

fn js_slice_start(len: usize, raw: Option<f64>) -> usize {
    let Some(raw) = raw else {
        return 0;
    };
    if !raw.is_finite() {
        return 0;
    }
    let trunc = raw.trunc() as i64;
    if trunc >= 0 {
        (trunc as usize).min(len)
    } else {
        let idx = (len as i64) + trunc;
        if idx <= 0 {
            0
        } else {
            idx as usize
        }
    }
}

pub fn compute_new_log_events(input: &NewLogEventsInput) -> NewLogEventsOutput {
    let run_start = js_slice_start(input.after_runs.len(), input.before_run_len);
    let err_start = js_slice_start(input.after_errors.len(), input.before_error_len);
    NewLogEventsOutput {
        runs: input.after_runs[run_start..].to_vec(),
        errors: input.after_errors[err_start..].to_vec(),
    }
}

pub fn compute_outcome_buckets(_input: &OutcomeBucketsInput) -> OutcomeBucketsOutput {
    OutcomeBucketsOutput {
        shipped: 0.0,
        no_change: 0.0,
        reverted: 0.0,
    }
}

pub fn compute_recent_run_events(input: &RecentRunEventsInput) -> RecentRunEventsOutput {
    let mut events = Vec::<serde_json::Value>::new();
    for bucket in input.day_events.iter() {
        for evt in bucket.iter() {
            events.push(evt.clone());
        }
    }
    RecentRunEventsOutput { events }
}

pub fn compute_all_decision_events(input: &AllDecisionEventsInput) -> AllDecisionEventsOutput {
    let mut events = Vec::<serde_json::Value>::new();
    for bucket in input.day_events.iter() {
        for evt in bucket.iter() {
            events.push(evt.clone());
        }
    }
    AllDecisionEventsOutput { events }
}

pub fn compute_cooldown_active_state(
    input: &CooldownActiveStateInput,
) -> CooldownActiveStateOutput {
    let now_ms = input.now_ms.unwrap_or(0.0);
    let until_ms = input.until_ms.unwrap_or(f64::NAN);
    if !until_ms.is_finite() || until_ms <= 0.0 || !now_ms.is_finite() {
        return CooldownActiveStateOutput {
            active: false,
            expired: true,
        };
    }
    if now_ms > until_ms {
        return CooldownActiveStateOutput {
            active: false,
            expired: true,
        };
    }
    CooldownActiveStateOutput {
        active: true,
        expired: false,
    }
}

pub fn compute_bump_count(input: &BumpCountInput) -> BumpCountOutput {
    let current = input.current_count.unwrap_or(0.0);
    let base = if current.is_finite() { current } else { 0.0 };
    BumpCountOutput { count: base + 1.0 }
}

pub fn compute_lock_age_minutes(input: &LockAgeMinutesInput) -> LockAgeMinutesOutput {
    let ts_raw = input.lock_ts.as_deref().unwrap_or("").trim();
    if ts_raw.is_empty() {
        return LockAgeMinutesOutput { age_minutes: None };
    }
    let parsed = DateTime::parse_from_rfc3339(ts_raw)
        .map(|v| v.with_timezone(&Utc))
        .ok();
    let Some(parsed) = parsed else {
        return LockAgeMinutesOutput { age_minutes: None };
    };
    let now_ms = input
        .now_ms
        .unwrap_or_else(|| Utc::now().timestamp_millis() as f64);
    if !now_ms.is_finite() {
        return LockAgeMinutesOutput { age_minutes: None };
    }
    let diff_ms = (now_ms - parsed.timestamp_millis() as f64).max(0.0);
    LockAgeMinutesOutput {
        age_minutes: Some(diff_ms / 60_000.0),
    }
}

pub fn compute_hash_obj(input: &HashObjInput) -> HashObjOutput {
    let json = input.json.as_deref().unwrap_or("");
    if json.is_empty() {
        return HashObjOutput { hash: None };
    }
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    HashObjOutput {
        hash: Some(format!("{:x}", digest)),
    }
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}
