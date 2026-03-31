pub fn compute_non_yield_category(input: &NonYieldCategoryInput) -> NonYieldCategoryOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return NonYieldCategoryOutput { category: None };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if result.is_empty() || result == "lock_busy" || result == "stop_repeat_gate_interval" {
        return NonYieldCategoryOutput { category: None };
    }

    if input.policy_hold.unwrap_or(false) || is_policy_hold_result(&result) {
        let reason_raw = input
            .hold_reason
            .as_ref()
            .or(input.route_block_reason.as_ref())
            .map(|v| v.as_str())
            .unwrap_or(result.as_str());
        let reason = normalize_spaces(reason_raw).to_ascii_lowercase();
        let result_lc = result.to_ascii_lowercase();
        if result_lc.contains("budget") || reason.contains("budget") || reason.contains("autopause")
        {
            return NonYieldCategoryOutput {
                category: Some("budget_hold".to_string()),
            };
        }
        return NonYieldCategoryOutput {
            category: Some("policy_hold".to_string()),
        };
    }

    let safety = compute_safety_stop_run_event(&SafetyStopRunEventInput {
        event_type: input.event_type.clone(),
        result: input.result.clone(),
    });
    if safety.is_safety_stop {
        return NonYieldCategoryOutput {
            category: Some("safety_stop".to_string()),
        };
    }

    let no_progress = compute_no_progress_result(&NoProgressResultInput {
        event_type: input.event_type.clone(),
        result: input.result.clone(),
        outcome: input.outcome.clone(),
    });
    if no_progress.is_no_progress {
        return NonYieldCategoryOutput {
            category: Some("no_progress".to_string()),
        };
    }

    NonYieldCategoryOutput { category: None }
}

pub fn compute_non_yield_reason(input: &NonYieldReasonInput) -> NonYieldReasonOutput {
    let explicit_raw = input
        .hold_reason
        .as_ref()
        .or(input.route_block_reason.as_ref())
        .or(input.reason.as_ref())
        .map(|v| v.as_str())
        .unwrap_or("");
    let explicit = normalize_spaces(explicit_raw).to_ascii_lowercase();
    if !explicit.is_empty() {
        return NonYieldReasonOutput { reason: explicit };
    }

    let result = normalize_spaces(input.result.as_deref().unwrap_or("")).to_ascii_lowercase();
    let outcome = normalize_spaces(input.outcome.as_deref().unwrap_or("")).to_ascii_lowercase();
    let category = normalize_spaces(input.category.as_deref().unwrap_or("")).to_ascii_lowercase();

    if category == "no_progress" && result == "executed" {
        return NonYieldReasonOutput {
            reason: if outcome.is_empty() {
                "executed_no_progress".to_string()
            } else {
                format!("executed_{outcome}")
            },
        };
    }

    if !result.is_empty() {
        return NonYieldReasonOutput { reason: result };
    }

    NonYieldReasonOutput {
        reason: format!(
            "{}_unknown",
            if category.is_empty() {
                "non_yield".to_string()
            } else {
                category
            }
        ),
    }
}

pub fn compute_proposal_type_from_run_event(
    input: &ProposalTypeFromRunEventInput,
) -> ProposalTypeFromRunEventOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return ProposalTypeFromRunEventOutput {
            proposal_type: String::new(),
        };
    }

    let direct = input
        .proposal_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !direct.is_empty() {
        return ProposalTypeFromRunEventOutput {
            proposal_type: direct,
        };
    }

    let capability = input
        .capability_key
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if capability.starts_with("proposal:") && capability.len() > "proposal:".len() {
        return ProposalTypeFromRunEventOutput {
            proposal_type: capability["proposal:".len()..].to_string(),
        };
    }

    ProposalTypeFromRunEventOutput {
        proposal_type: String::new(),
    }
}

pub fn compute_run_event_objective_id(
    input: &RunEventObjectiveIdInput,
) -> RunEventObjectiveIdOutput {
    let selected = if input.directive_pulse_present.unwrap_or(false) {
        input.directive_pulse_objective_id.as_deref().unwrap_or("")
    } else if input.objective_id_present.unwrap_or(false) {
        input.objective_id.as_deref().unwrap_or("")
    } else if input.objective_binding_present.unwrap_or(false) {
        input
            .objective_binding_objective_id
            .as_deref()
            .unwrap_or("")
    } else if input.top_escalation_present.unwrap_or(false) {
        input.top_escalation_objective_id.as_deref().unwrap_or("")
    } else {
        ""
    };

    RunEventObjectiveIdOutput {
        objective_id: sanitize_directive_objective_id(selected),
    }
}

pub fn compute_run_event_proposal_id(input: &RunEventProposalIdInput) -> RunEventProposalIdOutput {
    let selected = if input.proposal_id_present.unwrap_or(false) {
        input.proposal_id.as_deref().unwrap_or("")
    } else if input.selected_proposal_id_present.unwrap_or(false) {
        input.selected_proposal_id.as_deref().unwrap_or("")
    } else if input.top_escalation_present.unwrap_or(false) {
        input.top_escalation_proposal_id.as_deref().unwrap_or("")
    } else {
        ""
    };

    RunEventProposalIdOutput {
        proposal_id: normalize_spaces(selected),
    }
}

fn is_score_only_result_for_capacity(result: &str) -> bool {
    compute_score_only_result(&ScoreOnlyResultInput {
        result: Some(result.to_string()),
    })
    .is_score_only
}

pub fn compute_capacity_counted_attempt_event(
    input: &CapacityCountedAttemptEventInput,
) -> CapacityCountedAttemptEventOutput {
    let event_type = input
        .event_type
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if event_type != "autonomy_run" {
        return CapacityCountedAttemptEventOutput {
            capacity_counted: false,
        };
    }

    let result = input
        .result
        .as_ref()
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if result.is_empty() {
        return CapacityCountedAttemptEventOutput {
            capacity_counted: false,
        };
    }
    if input.policy_hold.unwrap_or(false) {
        return CapacityCountedAttemptEventOutput {
            capacity_counted: false,
        };
    }
    if is_policy_hold_result(&result)
        || result == "lock_busy"
        || result == "stop_repeat_gate_interval"
        || is_score_only_result_for_capacity(&result)
    {
        return CapacityCountedAttemptEventOutput {
            capacity_counted: false,
        };
    }
    if result == "executed" {
        return CapacityCountedAttemptEventOutput {
            capacity_counted: true,
        };
    }

    let is_attempt = compute_attempt_run_event(&AttemptRunEventInput {
        event_type: Some(event_type),
        result: Some(result),
    })
    .is_attempt;
    let proposal_id = normalize_spaces(input.proposal_id.as_deref().unwrap_or(""));
    CapacityCountedAttemptEventOutput {
        capacity_counted: is_attempt && !proposal_id.is_empty(),
    }
}

pub fn compute_repeat_gate_anchor(input: &RepeatGateAnchorInput) -> RepeatGateAnchorOutput {
    let proposal_id = normalize_spaces(input.proposal_id.as_deref().unwrap_or(""));
    let objective_id = normalize_spaces(input.objective_id.as_deref().unwrap_or(""));
    let objective_binding = if input.objective_binding_present.unwrap_or(false)
        && !objective_id.is_empty()
    {
        let source_raw = normalize_spaces(input.objective_binding_source.as_deref().unwrap_or(""));
        Some(RepeatGateAnchorBindingOutput {
            pass: input.objective_binding_pass.unwrap_or(true),
            required: input.objective_binding_required.unwrap_or(false),
            objective_id: objective_id.clone(),
            source: if source_raw.is_empty() {
                "repeat_gate_anchor".to_string()
            } else {
                source_raw
            },
            valid: input.objective_binding_valid.unwrap_or(true),
        })
    } else {
        None
    };

    RepeatGateAnchorOutput {
        proposal_id: if proposal_id.is_empty() {
            None
        } else {
            Some(proposal_id)
        },
        objective_id: if objective_id.is_empty() {
            None
        } else {
            Some(objective_id)
        },
        objective_binding,
    }
}

pub fn compute_policy_hold_pressure(input: &PolicyHoldPressureInput) -> PolicyHoldPressureOutput {
    let window_hours = input.window_hours.unwrap_or(24.0).max(1.0);
    let min_samples = input.min_samples.unwrap_or(1.0).max(1.0);
    let now_ms = non_negative_number(input.now_ms).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_millis() as f64)
            .unwrap_or(0.0)
    });
    let cutoff_ms = now_ms - (window_hours * 3_600_000.0);

    let mut attempts: u32 = 0;
    let mut policy_holds: u32 = 0;
    for evt in &input.events {
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
        if result.is_empty() || result == "lock_busy" || result == "stop_repeat_gate_interval" {
            continue;
        }
        if let Some(ts_ms) = non_negative_number(evt.ts_ms) {
            if ts_ms < cutoff_ms {
                continue;
            }
        }
        attempts += 1;
        if evt.policy_hold.unwrap_or(false) || is_policy_hold_result(&result) {
            policy_holds += 1;
        }
    }

    let rate = if attempts > 0 {
        clamp_ratio((policy_holds as f64) / (attempts as f64))
    } else {
        0.0
    };
    let applicable = (attempts as f64) >= min_samples;
    let warn_rate = clamp_ratio(input.warn_rate.unwrap_or(0.25));
    let hard_rate = clamp_ratio(input.hard_rate.unwrap_or(0.4).max(warn_rate));
    let level = if !applicable {
        "normal".to_string()
    } else if rate >= hard_rate {
        "hard".to_string()
    } else if rate >= warn_rate {
        "warn".to_string()
    } else {
        "normal".to_string()
    };

    PolicyHoldPressureOutput {
        window_hours: round3(window_hours),
        min_samples: round3(min_samples),
        samples: attempts,
        policy_holds,
        rate: round3(rate),
        level,
        applicable,
    }
}

fn policy_hold_reason_from_event_input(evt: &PolicyHoldPatternEventInput) -> String {
    let explicit = normalize_spaces(
        evt.hold_reason
            .as_ref()
            .or(evt.route_block_reason.as_ref())
            .map(|v| v.as_str())
            .unwrap_or(""),
    )
    .to_ascii_lowercase();
    if !explicit.is_empty() {
        return explicit;
    }
    normalize_spaces(evt.result.as_deref().unwrap_or("")).to_ascii_lowercase()
}

pub fn compute_policy_hold_pattern(input: &PolicyHoldPatternInput) -> PolicyHoldPatternOutput {
    let objective_id = normalize_spaces(input.objective_id.as_deref().unwrap_or(""));
    let window_hours = input.window_hours.unwrap_or(24.0).max(1.0);
    let repeat_threshold = input.repeat_threshold.unwrap_or(2.0).max(2.0);
    let now_ms = non_negative_number(input.now_ms).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_millis() as f64)
            .unwrap_or(0.0)
    });
    let cutoff_ms = now_ms - (window_hours * 3_600_000.0);

    let mut total_holds: u32 = 0;
    let mut by_reason: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for evt in &input.events {
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
        let evt_objective = normalize_spaces(evt.objective_id.as_deref().unwrap_or(""));
        if objective_id.is_empty() || evt_objective != objective_id {
            continue;
        }
        if !evt.policy_hold.unwrap_or(false) && !is_policy_hold_result(&result) {
            continue;
        }
        if let Some(ts_ms) = non_negative_number(evt.ts_ms) {
            if ts_ms < cutoff_ms {
                continue;
            }
        }
        let reason = policy_hold_reason_from_event_input(evt);
        let key = if reason.is_empty() {
            "policy_hold_unknown".to_string()
        } else {
            reason
        };
        let current = by_reason.get(&key).copied().unwrap_or(0);
        by_reason.insert(key, current + 1);
        total_holds += 1;
    }

    let mut top_reason: Option<String> = None;
    let mut top_count: u32 = 0;
    for (reason, count) in &by_reason {
        if *count > top_count {
            top_reason = Some(reason.clone());
            top_count = *count;
        }
    }
    let should_dampen = (top_count as f64) >= repeat_threshold;

    PolicyHoldPatternOutput {
        objective_id: if objective_id.is_empty() {
            None
        } else {
            Some(objective_id)
        },
        window_hours: round3(window_hours),
        repeat_threshold: round3(repeat_threshold),
        total_holds,
        top_reason,
        top_count,
        by_reason,
        should_dampen,
    }
}
fn policy_hold_reason_from_latest_entry(evt: &PolicyHoldLatestEventEntryInput) -> Option<String> {
    let hold_reason = evt
        .hold_reason
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if hold_reason.is_some() {
        return hold_reason;
    }
    evt.route_block_reason
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
