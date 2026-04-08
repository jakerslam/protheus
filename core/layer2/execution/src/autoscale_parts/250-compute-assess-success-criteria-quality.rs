pub fn compute_assess_success_criteria_quality(
    input: &AssessSuccessCriteriaQualityInput,
) -> AssessSuccessCriteriaQualityOutput {
    let checks = &input.checks;
    let total_count = input.total_count;
    let unknown_exempt_reasons = [
        "artifact_delta_unavailable",
        "entry_delta_unavailable",
        "revenue_delta_unavailable",
        "outreach_artifact_unavailable",
        "reply_or_interview_count_unavailable",
        "deferred_pending_window",
    ];
    let unknown_exempt_count = checks
        .iter()
        .filter(|row| {
            if row.evaluated {
                return false;
            }
            let reason = row.reason.as_deref().unwrap_or("").trim();
            unknown_exempt_reasons.contains(&reason)
        })
        .count() as f64;

    let unknown_count_raw = input.unknown_count;
    let unknown_count = (unknown_count_raw - unknown_exempt_count).max(0.0);
    let unknown_rate = if total_count > 0.0 {
        unknown_count / total_count
    } else if !checks.is_empty() {
        let unevaluated = checks.iter().filter(|row| !row.evaluated).count() as f64;
        (unevaluated - unknown_exempt_count).max(0.0) / (checks.len() as f64)
    } else {
        1.0
    };

    let unsupported_count = checks
        .iter()
        .filter(|row| {
            let reason = row.reason.as_deref().unwrap_or("").trim();
            reason == "unsupported_metric" || reason == "metric_not_allowed_for_capability"
        })
        .count() as f64;
    let unsupported_rate = if checks.is_empty() {
        0.0
    } else {
        unsupported_count / (checks.len() as f64)
    };

    let synthesized = input.synthesized;
    let mut reasons = Vec::<String>::new();
    if synthesized {
        reasons.push("synthesized_criteria".to_string());
    }
    if unknown_rate > 0.4 {
        reasons.push("high_unknown_rate".to_string());
    }
    if unsupported_rate > 0.5 {
        reasons.push("high_unsupported_rate".to_string());
    }

    AssessSuccessCriteriaQualityOutput {
        insufficient: !reasons.is_empty(),
        reasons,
        total_count,
        unknown_count_raw,
        unknown_exempt_count,
        unknown_count,
        unknown_rate: round4(unknown_rate),
        unsupported_count,
        unsupported_rate: round4(unsupported_rate),
        synthesized,
    }
}

pub fn compute_manual_gate_prefilter(
    input: &ManualGatePrefilterInput,
) -> ManualGatePrefilterOutput {
    let key = input
        .capability_key
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());
    let mut out = ManualGatePrefilterOutput {
        enabled: input.enabled,
        applicable: false,
        pass: true,
        reason: "disabled".to_string(),
        capability_key: key.clone(),
        window_hours: input.window_hours,
        min_observations: input.min_observations,
        max_manual_block_rate: input.max_manual_block_rate,
        attempts: 0.0,
        manual_blocked: 0.0,
        manual_block_rate: 0.0,
    };
    if !input.enabled {
        return out;
    }
    out.reason = "missing_capability_key".to_string();
    if key.is_none() {
        return out;
    }
    out.applicable = true;
    out.reason = "no_recent_manual_gate_samples".to_string();
    if !input.row_present {
        return out;
    }
    out.attempts = input.attempts.max(0.0);
    out.manual_blocked = input.manual_blocked.max(0.0);
    out.manual_block_rate = input.manual_block_rate.clamp(0.0, 1.0);
    if out.attempts < input.min_observations {
        out.reason = "insufficient_observations".to_string();
        return out;
    }
    if out.manual_block_rate >= input.max_manual_block_rate {
        out.pass = false;
        out.reason = "manual_gate_rate_exceeded".to_string();
        return out;
    }
    out.reason = "pass".to_string();
    out
}

pub fn compute_execute_confidence_cooldown_active(
    input: &ExecuteConfidenceCooldownActiveInput,
) -> ExecuteConfidenceCooldownActiveOutput {
    let key_present = input
        .cooldown_key
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    ExecuteConfidenceCooldownActiveOutput {
        active: key_present && input.cooldown_active,
    }
}

pub fn compute_top_biases_summary(input: &TopBiasesSummaryInput) -> TopBiasesSummaryOutput {
    let mut rows = input
        .entries
        .iter()
        .map(|row| TopBiasSummaryEntryOutput {
            key: row
                .key
                .as_ref()
                .map(|v| v.trim().to_string())
                .unwrap_or_default(),
            bias: row.bias,
            total: row.total,
            shipped: row.shipped,
            no_change: row.no_change,
            reverted: row.reverted,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.bias
            .abs()
            .partial_cmp(&a.bias.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.total
                    .partial_cmp(&a.total)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.key.cmp(&b.key))
    });
    let limit = input.limit.max(1) as usize;
    rows.truncate(limit);
    TopBiasesSummaryOutput { rows }
}

pub fn compute_criteria_pattern_penalty(
    input: &CriteriaPatternPenaltyInput,
) -> CriteriaPatternPenaltyOutput {
    if input.keys.is_empty() {
        return CriteriaPatternPenaltyOutput {
            penalty: 0.0,
            hit_patterns: Vec::new(),
            threshold: input.fail_threshold,
        };
    }
    let mut pattern_map =
        std::collections::BTreeMap::<String, &CriteriaPatternPenaltyPatternInput>::new();
    for row in &input.patterns {
        let key = row.key.trim().to_string();
        if key.is_empty() {
            continue;
        }
        pattern_map.insert(key, row);
    }
    let window_ms = input.window_days.max(0.0) * 24.0 * 3600.0 * 1000.0;
    let now_ms = if input.now_ms.is_finite() {
        input.now_ms
    } else {
        0.0
    };
    let mut penalty = 0.0_f64;
    let mut hits = Vec::<CriteriaPatternPenaltyHitOutput>::new();
    for key in &input.keys {
        let k = key.trim().to_string();
        if k.is_empty() {
            continue;
        }
        let Some(row) = pattern_map.get(&k) else {
            continue;
        };
        if let Some(ts) = &row.last_failure_ts {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts.trim()) {
                let fail_ms = dt.with_timezone(&Utc).timestamp_millis() as f64;
                if window_ms > 0.0 && (now_ms - fail_ms) > window_ms {
                    continue;
                }
            }
        }
        let failures = row.failures.max(0.0);
        let passes = row.passes.max(0.0);
        let effective_failures = (failures - (passes * 0.5).floor()).max(0.0);
        if effective_failures < input.fail_threshold {
            continue;
        }
        let over = effective_failures - input.fail_threshold + 1.0;
        let row_penalty = over * input.penalty_per_hit;
        penalty += row_penalty;
        hits.push(CriteriaPatternPenaltyHitOutput {
            key: k,
            failures,
            passes,
            effective_failures,
            penalty: row_penalty,
        });
    }
    CriteriaPatternPenaltyOutput {
        penalty: penalty.round().clamp(0.0, input.max_penalty.max(0.0)),
        hit_patterns: hits.into_iter().take(4).collect(),
        threshold: input.fail_threshold,
    }
}

pub fn compute_strategy_threshold_overrides(
    input: &StrategyThresholdOverridesInput,
) -> StrategyThresholdOverridesOutput {
    let choose = |base: Option<f64>, override_val: Option<f64>| -> f64 {
        if let Some(v) = override_val {
            if v.is_finite() {
                return v;
            }
        }
        base.filter(|v| v.is_finite()).unwrap_or(0.0)
    };
    StrategyThresholdOverridesOutput {
        min_signal_quality: choose(input.min_signal_quality, input.override_min_signal_quality),
        min_sensory_signal_score: choose(
            input.min_sensory_signal_score,
            input.override_min_sensory_signal_score,
        ),
        min_sensory_relevance_score: choose(
            input.min_sensory_relevance_score,
            input.override_min_sensory_relevance_score,
        ),
        min_directive_fit: choose(input.min_directive_fit, input.override_min_directive_fit),
        min_actionability_score: choose(
            input.min_actionability_score,
            input.override_min_actionability_score,
        ),
        min_eye_score_ema: choose(input.min_eye_score_ema, input.override_min_eye_score_ema),
    }
}

pub fn compute_effective_allowed_risks(
    input: &EffectiveAllowedRisksInput,
) -> EffectiveAllowedRisksOutput {
    let normalize = |rows: &[String]| -> Vec<String> {
        let mut out = Vec::<String>::new();
        let mut seen = std::collections::BTreeSet::<String>::new();
        for row in rows {
            let v = row.trim().to_lowercase();
            if v.is_empty() || !seen.insert(v.clone()) {
                continue;
            }
            out.push(v);
        }
        out
    };
    let defaults = normalize(&input.default_risks);
    let from_strategy = normalize(&input.strategy_allowed_risks);
    EffectiveAllowedRisksOutput {
        risks: if from_strategy.is_empty() {
            defaults
        } else {
            from_strategy
        },
    }
}

pub fn compute_directive_pulse_context(
    input: &DirectivePulseContextInput,
) -> DirectivePulseContextOutput {
    let clamp_number = |value: f64, min: f64, max: f64| -> f64 {
        if !value.is_finite() {
            min
        } else {
            value.clamp(min, max)
        }
    };
    let to_count = |value: Option<f64>| -> u32 {
        let v = value.unwrap_or(0.0);
        if !v.is_finite() || v <= 0.0 {
            0
        } else {
            v.round() as u32
        }
    };
    let clean_optional = |value: &Option<String>| -> Option<String> {
        value
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };

    let mut tier_attempts_today = std::collections::BTreeMap::<String, f64>::new();
    for (k, v) in &input.tier_attempts_today {
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        let count = if v.is_finite() && *v > 0.0 { *v } else { 0.0 };
        tier_attempts_today.insert(key.to_string(), count.round());
    }

    let mut objective_stats = Vec::<DirectivePulseContextObjectiveStatOutput>::new();
    for row in &input.objective_stats {
        let objective_id = row
            .objective_id
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if objective_id.is_empty() {
            continue;
        }
        let tier_raw = row.tier.unwrap_or(3.0);
        let tier = if tier_raw.is_finite() {
            tier_raw.round().clamp(1.0, 9.0) as u32
        } else {
            3
        };
        objective_stats.push(DirectivePulseContextObjectiveStatOutput {
            objective_id,
            tier,
            attempts: to_count(row.attempts),
            shipped: to_count(row.shipped),
            no_change: to_count(row.no_change),
            reverted: to_count(row.reverted),
            no_progress_streak: to_count(row.no_progress_streak),
            last_attempt_ts: clean_optional(&row.last_attempt_ts),
            last_shipped_ts: clean_optional(&row.last_shipped_ts),
        });
    }

    DirectivePulseContextOutput {
        enabled: input.enabled,
        available: input.available,
        objectives: input.objectives.clone(),
        error: clean_optional(&input.error),
        window_days: clamp_number(input.window_days, 1.0, 60.0),
        urgency_hours: clamp_number(input.urgency_hours, 1.0, 240.0),
        no_progress_limit: clamp_number(input.no_progress_limit, 1.0, 12.0),
        cooldown_hours: clamp_number(input.cooldown_hours, 1.0, 168.0),
        tier_attempts_today,
        attempts_today: if input.attempts_today.is_finite() && input.attempts_today > 0.0 {
            input.attempts_today.round()
        } else {
            0.0
        },
        objective_stats,
    }
}
