pub fn compute_strategy_selection(input: &StrategySelectionInput) -> StrategySelectionOutput {
    let attempt_index = input.attempt_index.max(1.0).round() as u32;
    let mut variants: Vec<StrategySelectionRankedOutput> = input
        .variants
        .iter()
        .map(|row| StrategySelectionRankedOutput {
            strategy_id: normalize_spaces(row.strategy_id.as_deref().unwrap_or("")),
            score: if row.score.is_finite() {
                row.score
            } else {
                0.0
            },
            confidence: if row.confidence.is_finite() {
                row.confidence
            } else {
                0.0
            },
            stage: row
                .stage
                .as_ref()
                .map(|v| normalize_spaces(v))
                .filter(|v| !v.is_empty()),
            execution_mode: normalize_spaces(row.execution_mode.as_deref().unwrap_or("")),
        })
        .filter(|row| !row.strategy_id.is_empty())
        .collect();

    variants.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.strategy_id.cmp(&b.strategy_id))
    });
    let max_active = input.max_active.max(1.0).round() as usize;
    if variants.len() > max_active {
        variants.truncate(max_active);
    }

    let fallback_id = normalize_spaces(input.fallback_strategy_id.as_deref().unwrap_or(""));
    if variants.is_empty() {
        return StrategySelectionOutput {
            selected_strategy_id: if fallback_id.is_empty() {
                None
            } else {
                Some(fallback_id)
            },
            mode: "none".to_string(),
            canary_enabled: input.canary_enabled,
            canary_due: false,
            canary_every: None,
            attempt_index,
            active_count: 0,
            ranked: Vec::new(),
        };
    }

    let default_id = variants
        .first()
        .map(|row| row.strategy_id.clone())
        .filter(|id| !id.is_empty())
        .or_else(|| {
            if fallback_id.is_empty() {
                None
            } else {
                Some(fallback_id.clone())
            }
        });

    let canary_pool: Vec<StrategySelectionRankedOutput> = variants
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            if idx == 0 {
                return None;
            }
            if input.canary_allow_execute {
                return Some(row.clone());
            }
            if row.execution_mode != "execute" {
                return Some(row.clone());
            }
            None
        })
        .collect();

    let canary_every = if input.canary_fraction.is_finite() && input.canary_fraction > 0.0 {
        Some((1.0 / input.canary_fraction).round().max(2.0) as u32)
    } else {
        None
    };
    let canary_due = input.canary_enabled
        && !canary_pool.is_empty()
        && canary_every
            .map(|every| attempt_index.is_multiple_of(every))
            .unwrap_or(false);
    let selected_strategy_id = if canary_due {
        let pool_ids: Vec<String> = canary_pool
            .iter()
            .map(|row| row.strategy_id.clone())
            .collect();
        let seed = format!(
            "{}|{}|{}",
            normalize_spaces(input.date_str.as_deref().unwrap_or("")),
            attempt_index,
            pool_ids.join(",")
        );
        let idx = stable_selection_index(&seed, canary_pool.len());
        canary_pool
            .get(idx)
            .map(|row| row.strategy_id.clone())
            .filter(|row| !row.is_empty())
            .or_else(|| default_id.clone())
    } else {
        default_id.clone()
    };

    StrategySelectionOutput {
        selected_strategy_id,
        mode: if canary_due {
            "canary_variant".to_string()
        } else {
            "primary_best".to_string()
        },
        canary_enabled: input.canary_enabled,
        canary_due,
        canary_every,
        attempt_index,
        active_count: variants.len() as u32,
        ranked: variants,
    }
}
pub fn compute_calibration_deltas(input: &CalibrationDeltasInput) -> CalibrationDeltasOutput {
    let mut out = CalibrationDeltasOutput {
        min_signal_quality: 0.0,
        min_sensory_signal_score: 0.0,
        min_sensory_relevance_score: 0.0,
        min_directive_fit: 0.0,
        min_actionability_score: 0.0,
        min_eye_score_ema: 0.0,
    };
    let executed_count = input.executed_count.max(0.0);
    let shipped_rate = input.shipped_rate;
    let no_change_rate = input.no_change_rate;
    let reverted_rate = input.reverted_rate;
    let exhausted = input.exhausted.max(0.0);
    let min_executed = input.min_executed.max(0.0);
    let tighten_min_executed = input.tighten_min_executed.max(0.0);
    let loosen_low_shipped_rate = input.loosen_low_shipped_rate;
    let loosen_exhausted_threshold = input.loosen_exhausted_threshold.max(0.0);
    let tighten_min_shipped_rate = input.tighten_min_shipped_rate;
    let max_delta = input.max_delta.max(0.0);

    let tighten_eligible = executed_count >= min_executed.max(tighten_min_executed);
    let loosen_eligible = executed_count >= min_executed;
    let low_ship_high_exhaustion = loosen_eligible
        && shipped_rate < loosen_low_shipped_rate
        && exhausted >= loosen_exhausted_threshold;

    if low_ship_high_exhaustion {
        out.min_signal_quality -= 3.0;
        out.min_directive_fit -= 3.0;
        out.min_actionability_score -= 2.0;
        out.min_sensory_relevance_score -= 1.0;
    } else if tighten_eligible {
        if no_change_rate >= 0.6 && shipped_rate >= tighten_min_shipped_rate {
            out.min_signal_quality += 3.0;
            out.min_directive_fit += 3.0;
            out.min_actionability_score += 2.0;
            out.min_sensory_relevance_score += 2.0;
        }
        if reverted_rate >= 0.15 {
            out.min_signal_quality += 2.0;
            out.min_actionability_score += 2.0;
        }
        if shipped_rate >= 0.45 && exhausted >= 2.0 {
            out.min_signal_quality -= 2.0;
            out.min_directive_fit -= 2.0;
            out.min_actionability_score -= 1.0;
        }
    } else if exhausted >= 3.0 {
        out.min_signal_quality -= 1.0;
        out.min_directive_fit -= 1.0;
    }

    out.min_signal_quality = out.min_signal_quality.clamp(-max_delta, max_delta);
    out.min_sensory_signal_score = out.min_sensory_signal_score.clamp(-max_delta, max_delta);
    out.min_sensory_relevance_score = out.min_sensory_relevance_score.clamp(-max_delta, max_delta);
    out.min_directive_fit = out.min_directive_fit.clamp(-max_delta, max_delta);
    out.min_actionability_score = out.min_actionability_score.clamp(-max_delta, max_delta);
    out.min_eye_score_ema = out.min_eye_score_ema.clamp(-max_delta, max_delta);
    out
}

pub fn compute_strategy_admission_decision(
    input: &StrategyAdmissionDecisionInput,
) -> StrategyAdmissionDecisionOutput {
    let preview_blocked: Vec<String> = input
        .preview_blocked_by
        .iter()
        .map(|row| normalize_spaces(row))
        .filter(|row| !row.is_empty())
        .take(6)
        .collect();
    if input.require_admission_preview && !input.preview_eligible {
        return StrategyAdmissionDecisionOutput {
            allow: false,
            reason: Some(
                preview_blocked
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "admission_preview_blocked".to_string()),
            ),
            admission_preview: Some(StrategyAdmissionPreviewOutput {
                eligible: false,
                blocked_by: preview_blocked,
            }),
            mutation_guard: None,
            risk_score: None,
            max_risk_per_action: None,
            strategy_max_risk_per_action: None,
            hard_max_risk_per_action: None,
            duplicate_window_hours: None,
            recent_count: None,
        };
    }

    if let Some(guard) = input.mutation_guard.as_ref() {
        if guard.applies && !guard.pass {
            return StrategyAdmissionDecisionOutput {
                allow: false,
                reason: Some(
                    normalize_spaces(guard.reason.as_deref().unwrap_or(""))
                        .chars()
                        .collect::<String>(),
                )
                .filter(|row| !row.is_empty())
                .or_else(|| Some("adaptive_mutation_execution_guard_blocked".to_string())),
                admission_preview: None,
                mutation_guard: Some(guard.clone()),
                risk_score: None,
                max_risk_per_action: None,
                strategy_max_risk_per_action: None,
                hard_max_risk_per_action: None,
                duplicate_window_hours: None,
                recent_count: None,
            };
        }
    }

    if !input.strategy_type_allowed {
        return StrategyAdmissionDecisionOutput {
            allow: false,
            reason: Some("strategy_type_filtered".to_string()),
            admission_preview: None,
            mutation_guard: None,
            risk_score: None,
            max_risk_per_action: None,
            strategy_max_risk_per_action: None,
            hard_max_risk_per_action: None,
            duplicate_window_hours: None,
            recent_count: None,
        };
    }

    if let Some(max_risk) = input.max_risk_per_action {
        let risk_score = input.risk_score.unwrap_or(0.0);
        if risk_score > max_risk {
            return StrategyAdmissionDecisionOutput {
                allow: false,
                reason: Some("strategy_risk_cap_exceeded".to_string()),
                admission_preview: None,
                mutation_guard: None,
                risk_score: Some(risk_score),
                max_risk_per_action: Some(max_risk),
                strategy_max_risk_per_action: input.strategy_max_risk_per_action,
                hard_max_risk_per_action: input.hard_max_risk_per_action,
                duplicate_window_hours: None,
                recent_count: None,
            };
        }
    }

    if input.remediation_check_required {
        let depth = input.remediation_depth.unwrap_or(0.0);
        let max_depth = input.remediation_max_depth.unwrap_or(f64::INFINITY);
        if depth > max_depth {
            return StrategyAdmissionDecisionOutput {
                allow: false,
                reason: Some("strategy_remediation_depth_exceeded".to_string()),
                admission_preview: None,
                mutation_guard: None,
                risk_score: None,
                max_risk_per_action: None,
                strategy_max_risk_per_action: None,
                hard_max_risk_per_action: None,
                duplicate_window_hours: None,
                recent_count: None,
            };
        }
    }

    let dedup_key = normalize_spaces(input.dedup_key.as_deref().unwrap_or(""));
    let recent_count = input.recent_count.unwrap_or(0.0);
    if !dedup_key.is_empty() && recent_count > 0.0 {
        return StrategyAdmissionDecisionOutput {
            allow: false,
            reason: Some("strategy_duplicate_window".to_string()),
            admission_preview: None,
            mutation_guard: None,
            risk_score: None,
            max_risk_per_action: None,
            strategy_max_risk_per_action: None,
            hard_max_risk_per_action: None,
            duplicate_window_hours: input.duplicate_window_hours,
            recent_count: Some(recent_count),
        };
    }

    StrategyAdmissionDecisionOutput {
        allow: true,
        reason: None,
        admission_preview: None,
        mutation_guard: None,
        risk_score: None,
        max_risk_per_action: None,
        strategy_max_risk_per_action: None,
        hard_max_risk_per_action: None,
        duplicate_window_hours: None,
        recent_count: None,
    }
}

pub fn compute_expected_value_score(input: &ExpectedValueScoreInput) -> ExpectedValueScoreOutput {
    let score = if input.score.is_finite() {
        input.score
    } else {
        0.0
    };
    ExpectedValueScoreOutput { score }
}

pub fn compute_suggest_run_batch_max(input: &SuggestRunBatchMaxInput) -> SuggestRunBatchMaxOutput {
    SuggestRunBatchMaxOutput {
        enabled: input.enabled,
        max: if input.batch_max.is_finite() {
            input.batch_max.max(1.0).floor()
        } else {
            1.0
        },
        reason: normalize_spaces(input.batch_reason.as_deref().unwrap_or("no_pressure")),
        daily_remaining: if input.daily_remaining.is_finite() {
            input.daily_remaining.max(0.0).floor()
        } else {
            0.0
        },
        autoscale_hint: input.autoscale_hint.clone(),
    }
}

pub fn compute_backlog_autoscale_snapshot(
    input: &BacklogAutoscaleSnapshotInput,
) -> BacklogAutoscaleSnapshotOutput {
    BacklogAutoscaleSnapshotOutput {
        enabled: input.enabled,
        module: normalize_spaces(input.module.as_deref().unwrap_or("")),
        state: input.state.clone(),
        queue: input.queue.clone(),
        current_cells: if input.current_cells.is_finite() {
            input.current_cells
        } else {
            0.0
        },
        plan: input.plan.clone(),
        trit_productivity: input.trit_productivity.clone(),
    }
}

pub fn compute_admission_summary(input: &AdmissionSummaryInput) -> AdmissionSummaryOutput {
    let mut eligible: u32 = 0;
    let mut blocked: u32 = 0;
    let mut blocked_by_reason = std::collections::BTreeMap::<String, u32>::new();
    for row in &input.proposals {
        let is_eligible = row.preview_eligible.unwrap_or(true);
        if is_eligible {
            eligible = eligible.saturating_add(1);
            continue;
        }
        blocked = blocked.saturating_add(1);
        if row.blocked_by.is_empty() {
            *blocked_by_reason.entry("unknown".to_string()).or_insert(0) += 1;
            continue;
        }
        for reason in &row.blocked_by {
            let key = reason.split_whitespace().collect::<Vec<_>>().join(" ");
            let normalized = if key.is_empty() {
                "unknown".to_string()
            } else {
                key
            };
            *blocked_by_reason.entry(normalized).or_insert(0) += 1;
        }
    }
    AdmissionSummaryOutput {
        total: input.proposals.len() as u32,
        eligible,
        blocked,
        blocked_by_reason,
    }
}
