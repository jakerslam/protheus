
fn clamp_ratio(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

pub fn compute_plan(input: &PlanInput) -> PlanOutput {
    let pressure = input.queue_pressure.pressure.to_ascii_lowercase();
    let pending = input.queue_pressure.pending.max(0.0);
    let pending_ratio = clamp_ratio(input.queue_pressure.pending_ratio);
    let min_cells = input.min_cells;
    let max_cells = input.max_cells.max(min_cells);
    let current_cells = input.current_cells.max(min_cells).min(max_cells);
    let run_interval_minutes = input.run_interval_minutes.max(1.0);
    let idle_release_minutes = input.idle_release_minutes.max(run_interval_minutes);
    let high_pressure = pressure == "critical";
    let warning_pressure = pressure == "warning";
    let pressure_active = high_pressure || warning_pressure;

    let cooldown_active = input
        .last_run_minutes_ago
        .map(|v| v.is_finite() && v < run_interval_minutes)
        .unwrap_or(false);
    let idle_release_ready = current_cells > min_cells
        && !pressure_active
        && input
            .last_high_pressure_minutes_ago
            .map(|v| v.is_finite() && v >= idle_release_minutes)
            .unwrap_or(false);

    if input.trit_shadow_blocked {
        return PlanOutput {
            action: "hold".to_string(),
            reason: "shadow_hold".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: current_cells,
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active,
            idle_release_ready,
            budget_blocked: input.autopause_active,
            trit_shadow_blocked: true,
        };
    }

    if input.autopause_active && pressure_active {
        return PlanOutput {
            action: "hold".to_string(),
            reason: "budget_autopause_active".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: current_cells,
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active,
            idle_release_ready,
            budget_blocked: true,
            trit_shadow_blocked: false,
        };
    }

    if pressure_active && cooldown_active {
        return PlanOutput {
            action: "cooldown_hold".to_string(),
            reason: "cooldown_hold".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: current_cells,
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active: true,
            idle_release_ready,
            budget_blocked: input.autopause_active,
            trit_shadow_blocked: false,
        };
    }

    if high_pressure && current_cells < max_cells {
        return PlanOutput {
            action: "scale_up".to_string(),
            reason: "backlog_critical".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: max_cells,
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active,
            idle_release_ready,
            budget_blocked: input.autopause_active,
            trit_shadow_blocked: false,
        };
    }

    if warning_pressure && current_cells < max_cells {
        return PlanOutput {
            action: "scale_up".to_string(),
            reason: "backlog_warning".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: (current_cells + 1).max(min_cells).min(max_cells),
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active,
            idle_release_ready,
            budget_blocked: input.autopause_active,
            trit_shadow_blocked: false,
        };
    }

    if idle_release_ready {
        return PlanOutput {
            action: "scale_down".to_string(),
            reason: "idle_release_ready".to_string(),
            pressure,
            pending,
            pending_ratio,
            current_cells,
            target_cells: min_cells,
            warning_pressure,
            high_pressure,
            pressure_active,
            cooldown_active,
            idle_release_ready: true,
            budget_blocked: input.autopause_active,
            trit_shadow_blocked: false,
        };
    }

    PlanOutput {
        action: "hold".to_string(),
        reason: if current_cells > min_cells && !pressure_active {
            "idle_hold".to_string()
        } else {
            "no_pressure".to_string()
        },
        pressure,
        pending,
        pending_ratio,
        current_cells,
        target_cells: current_cells,
        warning_pressure,
        high_pressure,
        pressure_active,
        cooldown_active,
        idle_release_ready,
        budget_blocked: input.autopause_active,
        trit_shadow_blocked: false,
    }
}

pub fn compute_batch_max(input: &BatchMaxInput) -> BatchMaxOutput {
    let pressure = input.pressure.to_ascii_lowercase();
    let max_batch = input.max_batch.max(1);
    let current_cells = input.current_cells;

    if !input.enabled {
        return BatchMaxOutput {
            max: 1,
            reason: "disabled".to_string(),
            pressure,
            current_cells,
            budget_blocked: input.budget_blocked,
            trit_shadow_blocked: input.trit_shadow_blocked,
        };
    }
    if input.budget_blocked {
        return BatchMaxOutput {
            max: 1,
            reason: "budget_blocked".to_string(),
            pressure,
            current_cells,
            budget_blocked: true,
            trit_shadow_blocked: input.trit_shadow_blocked,
        };
    }
    if input.trit_shadow_blocked {
        return BatchMaxOutput {
            max: 1,
            reason: "shadow_hold".to_string(),
            pressure,
            current_cells,
            budget_blocked: input.budget_blocked,
            trit_shadow_blocked: true,
        };
    }

    let mut suggested = 1_u32;
    if pressure == "critical" {
        suggested = max_batch.min((current_cells + 1).max(1));
    } else if pressure == "warning" {
        suggested = max_batch.min(2);
    }
    let mut reason = if suggested > 1 {
        "backlog_autoscale".to_string()
    } else {
        "no_pressure".to_string()
    };
    if let Some(remaining) = input.daily_remaining {
        if remaining < suggested {
            suggested = remaining.max(1);
            reason = "daily_cap_limited".to_string();
        }
    }

    BatchMaxOutput {
        max: suggested.max(1),
        reason,
        pressure,
        current_cells,
        budget_blocked: input.budget_blocked,
        trit_shadow_blocked: input.trit_shadow_blocked,
    }
}

pub fn compute_dynamic_caps(input: &DynamicCapsInput) -> DynamicCapsOutput {
    let mut out = DynamicCapsOutput {
        enabled: input.enabled,
        daily_runs_cap: input.base_daily_cap.max(1),
        canary_daily_exec_cap: input.base_canary_cap,
        input_candidates_cap: None,
        input_candidate_cap_alias: None,
        low_yield: false,
        high_yield: false,
        spawn_reset_active: false,
        queue_pressure: input.queue_pressure.to_ascii_lowercase(),
        policy_hold_level: input.policy_hold_level.to_ascii_lowercase(),
        reasons: Vec::new(),
    };

    let mark_low_yield = |reason: &str, out: &mut DynamicCapsOutput| {
        out.low_yield = true;
        if !out.reasons.iter().any(|r| r == reason) {
            out.reasons.push(reason.to_string());
        }
    };

    if input.enabled {
        let pressure = out.queue_pressure.as_str();
        let mut factor = 1.0_f64;
        if pressure == "critical" {
            factor = input.critical_factor;
            mark_low_yield("downshift_queue_backlog_critical", &mut out);
        } else if pressure == "warning" {
            factor = input.warn_factor;
            mark_low_yield("downshift_queue_backlog_warning", &mut out);
        }
        if factor < 1.0 {
            let lowered_runs = ((input.base_daily_cap as f64) * factor).floor() as u32;
            out.daily_runs_cap = out.daily_runs_cap.min(lowered_runs.max(1));
            if input.candidate_pool_size > 0 {
                let lowered_pool = ((input.candidate_pool_size as f64) * factor).floor() as u32;
                let lowered_pool = lowered_pool.max(input.min_input_pool.max(1));
                if lowered_pool < input.candidate_pool_size {
                    out.input_candidates_cap = Some(lowered_pool);
                    out.input_candidate_cap_alias = Some(lowered_pool);
                }
            }
        }
    }

    let hold_level = out.policy_hold_level.as_str();
    if input.policy_hold_applicable && hold_level == "hard" {
        out.daily_runs_cap = out.daily_runs_cap.min(1);
        out.high_yield = false;
        mark_low_yield("downshift_policy_hold_hard", &mut out);
    } else if input.policy_hold_applicable && hold_level == "warn" {
        let warn_cap = ((input.base_daily_cap as f64) * 0.6).floor() as u32;
        out.daily_runs_cap = out.daily_runs_cap.min(warn_cap.max(1));
        out.high_yield = false;
        mark_low_yield("downshift_policy_hold_warn", &mut out);
    }

    if input.spawn_boost_enabled && input.spawn_boost_active {
        out.daily_runs_cap = input.base_daily_cap.max(1);
        out.input_candidates_cap = None;
        out.input_candidate_cap_alias = None;
        out.low_yield = false;
        out.spawn_reset_active = true;
        if !out.reasons.iter().any(|r| r == "reset_caps_spawn_capacity") {
            out.reasons.push("reset_caps_spawn_capacity".to_string());
        }
    }

    if !out.low_yield {
        out.high_yield = input.shipped_today > 0.0
            && input.no_progress_streak <= 0.0
            && input.gate_exhaustion_streak <= 0.0;
    }

    out
}

fn non_negative_number(v: Option<f64>) -> Option<f64> {
    let n = v?;
    if n.is_finite() && n >= 0.0 {
        Some(n)
    } else {
        None
    }
}

pub fn compute_token_usage(input: &TokenUsageInput) -> TokenUsageOutput {
    let est_selected = non_negative_number(input.selected_model_tokens_est)
        .or_else(|| non_negative_number(input.route_budget_request_tokens_est))
        .or_else(|| non_negative_number(input.route_tokens_est))
        .or_else(|| non_negative_number(input.fallback_est_tokens))
        .unwrap_or(0.0);

    let prompt = non_negative_number(input.metrics_prompt_tokens)
        .or_else(|| non_negative_number(input.metrics_input_tokens));
    let completion = non_negative_number(input.metrics_completion_tokens)
        .or_else(|| non_negative_number(input.metrics_output_tokens));
    let total_direct = non_negative_number(input.metrics_total_tokens)
        .or_else(|| non_negative_number(input.metrics_tokens_used));

    let actual_total = if let Some(total) = total_direct {
        Some(total)
    } else if prompt.is_some() || completion.is_some() {
        Some(prompt.unwrap_or(0.0) + completion.unwrap_or(0.0))
    } else {
        None
    };

    let effective_tokens = actual_total.unwrap_or(est_selected);
    let source = if actual_total.is_some() {
        let raw = input
            .metrics_source
            .clone()
            .unwrap_or_else(|| "route_execute_metrics".to_string());
        let normalized = raw.trim();
        if normalized.is_empty() {
            "route_execute_metrics".to_string()
        } else {
            normalized.to_string()
        }
    } else {
        "estimated_fallback".to_string()
    };

    TokenUsageOutput {
        available: actual_total.is_some(),
        source,
        actual_prompt_tokens: prompt,
        actual_completion_tokens: completion,
        actual_total_tokens: actual_total,
        estimated_tokens: est_selected,
        effective_tokens,
    }
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

pub fn compute_normalize_queue(input: &NormalizeQueueInput) -> NormalizeQueueOutput {
    let pending = input.pending.unwrap_or(0.0).max(0.0);
    let total = input.total.unwrap_or(0.0).max(0.0);
    let pending_ratio = non_negative_number(input.pending_ratio)
        .map(clamp_ratio)
        .unwrap_or_else(|| {
            if total > 0.0 {
                clamp_ratio(pending / total)
            } else {
                0.0
            }
        });

    let mut pressure = input
        .pressure
        .clone()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if pressure != "critical" && pressure != "warning" && pressure != "normal" {
        pressure = "normal".to_string();
        if pending >= input.critical_pending_count || pending_ratio >= input.critical_pending_ratio
        {
            pressure = "critical".to_string();
        } else if pending >= input.warn_pending_count || pending_ratio >= input.warn_pending_ratio {
            pressure = "warning".to_string();
        }
    }

    NormalizeQueueOutput {
        pressure,
        pending,
        total,
        pending_ratio: round6(pending_ratio),
    }
}

pub fn compute_criteria_gate(input: &CriteriaGateInput) -> CriteriaGateOutput {
    let min_count = input.min_count.unwrap_or(0.0).max(0.0);
    let total_count = input.total_count.unwrap_or(0.0).max(0.0);
    let unsupported_count = (input.contract_not_allowed_count.unwrap_or(0.0).max(0.0)
        + input.unsupported_count.unwrap_or(0.0).max(0.0))
    .max(0.0);
    let supported_count = input
        .structurally_supported_count
        .unwrap_or(total_count - unsupported_count)
        .max(0.0);
    let contract_violation_count = input.contract_violation_count.unwrap_or(0.0).max(0.0);

    let mut reasons: Vec<String> = Vec::new();
    if total_count < min_count {
        reasons.push("criteria_count_below_min".to_string());
    }
    if contract_violation_count > 0.0 {
        reasons.push("criteria_contract_violation".to_string());
    }
    if supported_count < min_count {
        reasons.push("criteria_supported_count_below_min".to_string());
    }

    CriteriaGateOutput {
        pass: reasons.is_empty(),
        reasons,
        min_count,
        total_count,
        supported_count,
        unsupported_count,
        contract_violation_count,
    }
}

pub fn compute_structural_preview_criteria_failure(
    input: &StructuralPreviewCriteriaFailureInput,
) -> StructuralPreviewCriteriaFailureOutput {
    let primary = input
        .primary_failure
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if primary.contains("metric_not_allowed_for_capability")
        || primary.contains("insufficient_supported_metrics")
    {
        return StructuralPreviewCriteriaFailureOutput { has_failure: true };
    }

    let not_allowed = input.contract_not_allowed_count.unwrap_or(0.0).max(0.0);
    let unsupported = input.unsupported_count.unwrap_or(0.0).max(0.0);
    let total = input.total_count.unwrap_or(0.0).max(1.0);
    let has_failure = not_allowed > 0.0 || (unsupported > 0.0 && (unsupported / total) >= 0.5);
    StructuralPreviewCriteriaFailureOutput { has_failure }
}
