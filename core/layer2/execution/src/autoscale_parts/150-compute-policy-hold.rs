
fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn sanitize_directive_objective_id(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        return String::new();
    }
    let bytes = value.as_bytes();
    if bytes.first().copied() != Some(b'T') {
        return String::new();
    }
    let mut idx: usize = 1;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == 1 || idx >= bytes.len() || bytes[idx] != b'_' {
        return String::new();
    }
    idx += 1;
    if idx >= bytes.len() {
        return String::new();
    }
    if !bytes[idx..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return String::new();
    }
    value.to_string()
}

fn sanitize_directive_objective_id_single_digit(raw: &str) -> String {
    let value = raw.trim();
    let bytes = value.as_bytes();
    if bytes.len() < 4 {
        return String::new();
    }
    if bytes[0] != b'T' || !bytes[1].is_ascii_digit() || bytes[2] != b'_' {
        return String::new();
    }
    if !bytes[3..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return String::new();
    }
    value.to_string()
}

fn sanitize_cooldown_fragment(raw: &str) -> String {
    normalize_spaces(raw)
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == ':' || ch == '_' || ch == '-'
            {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}

pub fn compute_policy_hold(input: &PolicyHoldInput) -> PolicyHoldOutput {
    let target = input.target.trim().to_ascii_lowercase();
    if target != "route" {
        return PolicyHoldOutput {
            hold: false,
            hold_scope: None,
            hold_reason: None,
            route_block_reason: None,
        };
    }

    let budget_reason = normalize_spaces(&input.budget_reason);
    let route_reason = normalize_spaces(&input.route_reason);
    let budget_signal_text =
        normalize_spaces(&format!("{budget_reason} {route_reason}")).to_ascii_lowercase();
    let budget_blocked_by_reason = budget_signal_text.contains("burn_rate_exceeded")
        || budget_signal_text.contains("budget_autopause")
        || budget_signal_text.contains("budget guard blocked")
        || budget_signal_text.contains("budget_deferred")
        || budget_signal_text.contains("budget_blocked");
    let budget_blocked = input.budget_blocked_flag
        || input.budget_global_blocked
        || input.budget_enforcement_blocked
        || budget_blocked_by_reason;

    if budget_blocked {
        let reason = if budget_reason.trim().is_empty() {
            "budget_guard_blocked".to_string()
        } else {
            budget_reason
        };
        return PolicyHoldOutput {
            hold: true,
            hold_scope: Some("budget".to_string()),
            hold_reason: Some(reason.clone()),
            route_block_reason: Some(reason),
        };
    }

    let gate_decision = input.gate_decision.trim().to_ascii_uppercase();
    let route_decision = input.route_decision.trim().to_ascii_uppercase();
    let manual_blocked =
        gate_decision == "MANUAL" || route_decision == "MANUAL" || input.needs_manual_review;
    if manual_blocked && !input.executable {
        return PolicyHoldOutput {
            hold: true,
            hold_scope: Some("proposal".to_string()),
            hold_reason: Some("gate_manual".to_string()),
            route_block_reason: Some("gate_manual".to_string()),
        };
    }

    PolicyHoldOutput {
        hold: false,
        hold_scope: None,
        hold_reason: None,
        route_block_reason: None,
    }
}

pub fn compute_route_execution_policy_hold(
    input: &RouteExecutionPolicyHoldInput,
) -> PolicyHoldOutput {
    let target = normalize_spaces(input.target.as_deref().unwrap_or("route")).to_ascii_lowercase();
    let gate_decision = normalize_spaces(input.gate_decision.as_deref().unwrap_or(""));
    let route_decision = {
        let raw = normalize_spaces(input.route_decision_raw.as_deref().unwrap_or(""));
        if !raw.is_empty() {
            raw
        } else {
            normalize_spaces(input.decision.as_deref().unwrap_or(""))
        }
    };
    let needs_manual_review = input.needs_manual_review.unwrap_or(false);
    let executable = input.executable.unwrap_or(true);

    let budget_reason = {
        let direct = normalize_spaces(input.budget_block_reason.as_deref().unwrap_or(""));
        if !direct.is_empty() {
            direct
        } else {
            let enforced =
                normalize_spaces(input.budget_enforcement_reason.as_deref().unwrap_or(""));
            if !enforced.is_empty() {
                enforced
            } else {
                normalize_spaces(input.budget_global_reason.as_deref().unwrap_or(""))
            }
        }
    };

    let route_reason = {
        let summary = normalize_spaces(input.summary_reason.as_deref().unwrap_or(""));
        if !summary.is_empty() {
            summary
        } else {
            normalize_spaces(input.route_reason.as_deref().unwrap_or(""))
        }
    };

    let normalized = PolicyHoldInput {
        target,
        gate_decision,
        route_decision,
        needs_manual_review,
        executable,
        budget_reason,
        route_reason,
        budget_blocked_flag: input.budget_blocked.unwrap_or(false),
        budget_global_blocked: input.budget_global_blocked.unwrap_or(false),
        budget_enforcement_blocked: input.budget_enforcement_blocked.unwrap_or(false),
    };
    compute_policy_hold(&normalized)
}

fn round3(v: f64) -> f64 {
    (v * 1_000.0).round() / 1_000.0
}

fn is_policy_hold_result(result: &str) -> bool {
    !result.is_empty()
        && (result.starts_with("no_candidates_policy_")
            || result == "stop_init_gate_budget_autopause"
            || result == "stop_init_gate_readiness"
            || result == "stop_init_gate_readiness_blocked"
            || result == "stop_init_gate_criteria_quality_insufficient"
            || result == "stop_repeat_gate_mutation_guard"
            || result == "score_only_fallback_route_block"
            || result == "score_only_fallback_low_execution_confidence")
}

pub fn compute_policy_hold_result(input: &PolicyHoldResultInput) -> PolicyHoldResultOutput {
    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    PolicyHoldResultOutput {
        is_policy_hold: is_policy_hold_result(&result),
    }
}

pub fn compute_policy_hold_run_event(input: &PolicyHoldRunEventInput) -> PolicyHoldRunEventOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    PolicyHoldRunEventOutput {
        is_policy_hold_run_event: event_type == "autonomy_run"
            && (input.policy_hold.unwrap_or(false) || is_policy_hold_result(&result)),
    }
}

pub fn compute_dod_evidence_diff(input: &DodEvidenceDiffInput) -> DodEvidenceDiffOutput {
    let before_artifacts = input.before_artifacts.unwrap_or(0.0);
    let before_entries = input.before_entries.unwrap_or(0.0);
    let before_revenue_actions = input.before_revenue_actions.unwrap_or(0.0);
    let before_registry_total = input.before_registry_total.unwrap_or(0.0);
    let before_registry_active = input.before_registry_active.unwrap_or(0.0);
    let before_registry_candidate = input.before_registry_candidate.unwrap_or(0.0);
    let before_habit_runs = input.before_habit_runs.unwrap_or(0.0);
    let before_habit_errors = input.before_habit_errors.unwrap_or(0.0);

    let after_artifacts = input.after_artifacts.unwrap_or(0.0);
    let after_entries = input.after_entries.unwrap_or(0.0);
    let after_revenue_actions = input.after_revenue_actions.unwrap_or(0.0);
    let after_registry_total = input.after_registry_total.unwrap_or(0.0);
    let after_registry_active = input.after_registry_active.unwrap_or(0.0);
    let after_registry_candidate = input.after_registry_candidate.unwrap_or(0.0);
    let after_habit_runs = input.after_habit_runs.unwrap_or(0.0);
    let after_habit_errors = input.after_habit_errors.unwrap_or(0.0);

    DodEvidenceDiffOutput {
        artifacts_delta: after_artifacts - before_artifacts,
        entries_delta: after_entries - before_entries,
        revenue_actions_delta: after_revenue_actions - before_revenue_actions,
        registry_total_delta: after_registry_total - before_registry_total,
        registry_active_delta: after_registry_active - before_registry_active,
        registry_candidate_delta: after_registry_candidate - before_registry_candidate,
        habit_runs_delta: after_habit_runs - before_habit_runs,
        habit_errors_delta: after_habit_errors - before_habit_errors,
    }
}

pub fn compute_score_only_result(input: &ScoreOnlyResultInput) -> ScoreOnlyResultOutput {
    let result = input.result.as_ref().map(|v| v.trim()).unwrap_or_default();
    ScoreOnlyResultOutput {
        is_score_only: result == "score_only_preview"
            || result == "score_only_evidence"
            || result == "stop_repeat_gate_preview_structural_cooldown"
            || result == "stop_repeat_gate_preview_churn_cooldown",
    }
}

pub fn compute_score_only_failure_like(
    input: &ScoreOnlyFailureLikeInput,
) -> ScoreOnlyFailureLikeOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return ScoreOnlyFailureLikeOutput {
            is_failure_like: false,
        };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if !compute_score_only_result(&ScoreOnlyResultInput {
        result: Some(result.clone()),
    })
    .is_score_only
    {
        return ScoreOnlyFailureLikeOutput {
            is_failure_like: false,
        };
    }

    if result == "stop_repeat_gate_preview_structural_cooldown"
        || result == "stop_repeat_gate_preview_churn_cooldown"
    {
        return ScoreOnlyFailureLikeOutput {
            is_failure_like: true,
        };
    }

    if !input.preview_verification_present.unwrap_or(false) {
        return ScoreOnlyFailureLikeOutput {
            is_failure_like: false,
        };
    }
    if input.preview_verification_passed == Some(false) {
        return ScoreOnlyFailureLikeOutput {
            is_failure_like: true,
        };
    }
    let outcome = input
        .preview_verification_outcome
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    ScoreOnlyFailureLikeOutput {
        is_failure_like: outcome == "no_change",
    }
}

pub fn compute_gate_exhausted_attempt(
    input: &GateExhaustedAttemptInput,
) -> GateExhaustedAttemptOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return GateExhaustedAttemptOutput {
            is_gate_exhausted: false,
        };
    }

    let result = input.result.as_ref().map(|v| v.trim()).unwrap_or_default();
    GateExhaustedAttemptOutput {
        is_gate_exhausted: result == "stop_repeat_gate_stale_signal"
            || result == "stop_repeat_gate_capability_cap"
            || result == "stop_repeat_gate_directive_pulse_cooldown"
            || result == "stop_repeat_gate_directive_pulse_tier_reservation"
            || result == "stop_repeat_gate_human_escalation_pending"
            || result == "init_gate_blocked_route"
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
            || result == "stop_repeat_gate_preview_churn_cooldown"
            || result == "stop_repeat_gate_medium_canary_cap"
            || result == "stop_repeat_gate_candidate_exhausted",
    }
}

pub fn compute_consecutive_gate_exhausted_attempts(
    input: &ConsecutiveGateExhaustedAttemptsInput,
) -> ConsecutiveGateExhaustedAttemptsOutput {
    let mut count: u32 = 0;
    for evt in input.events.iter().rev() {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }

        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let is_attempt = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: Some(event_type),
            result: Some(result.clone()),
        })
        .is_attempt;
        if !is_attempt {
            continue;
        }

        let is_gate_exhausted = compute_gate_exhausted_attempt(&GateExhaustedAttemptInput {
            event_type: Some("autonomy_run".to_string()),
            result: Some(result),
        })
        .is_gate_exhausted;
        if !is_gate_exhausted {
            break;
        }
        count += 1;
    }
    ConsecutiveGateExhaustedAttemptsOutput { count }
}

pub fn compute_runs_since_reset_index(
    input: &RunsSinceResetIndexInput,
) -> RunsSinceResetIndexOutput {
    let mut start_index: usize = 0;
    for (idx, evt) in input.events.iter().enumerate() {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type == "autonomy_reset" {
            start_index = idx + 1;
        }
    }
    RunsSinceResetIndexOutput {
        start_index: start_index as u32,
    }
}

pub fn compute_attempt_event_indices(
    input: &AttemptEventIndicesInput,
) -> AttemptEventIndicesOutput {
    let mut indices: Vec<u32> = Vec::new();
    for (idx, evt) in input.events.iter().enumerate() {
        let is_attempt = compute_attempt_run_event(&AttemptRunEventInput {
            event_type: evt.event_type.clone(),
            result: evt.result.clone(),
        })
        .is_attempt;
        if is_attempt {
            indices.push(idx as u32);
        }
    }
    AttemptEventIndicesOutput { indices }
}

pub fn compute_capacity_counted_attempt_indices(
    input: &CapacityCountedAttemptIndicesInput,
) -> CapacityCountedAttemptIndicesOutput {
    let mut indices: Vec<u32> = Vec::new();
    for (idx, evt) in input.events.iter().enumerate() {
        let counted = compute_capacity_counted_attempt_event(&CapacityCountedAttemptEventInput {
            event_type: evt.event_type.clone(),
            result: evt.result.clone(),
            policy_hold: evt.policy_hold,
            proposal_id: evt.proposal_id.clone(),
        })
        .capacity_counted;
        if counted {
            indices.push(idx as u32);
        }
    }
    CapacityCountedAttemptIndicesOutput { indices }
}
