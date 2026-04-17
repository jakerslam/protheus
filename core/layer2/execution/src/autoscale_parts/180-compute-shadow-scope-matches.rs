fn normalize_shadow_scope_token(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else if ch.is_ascii_whitespace() || matches!(ch, '-' | '.' | ':' | '/') {
            '_'
        } else {
            continue;
        };
        if mapped == '_' {
            if prev_sep || out.is_empty() {
                continue;
            }
            prev_sep = true;
            out.push('_');
        } else {
            prev_sep = false;
            out.push(mapped);
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_scope_values(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in raw.split([',', '|']) {
        let normalized = normalize_shadow_scope_token(token);
        if normalized.is_empty() || out.contains(&normalized) {
            continue;
        }
        out.push(normalized);
    }
    out
}

fn canonical_scope_type(raw: &str) -> String {
    let token = normalize_shadow_scope_token(raw);
    match token.as_str() {
        "proposal" => "proposal_type".to_string(),
        "capability" => "capability_key".to_string(),
        "objective" => "objective_id".to_string(),
        "risk" | "risk_level" => "global".to_string(),
        _ => token,
    }
}

pub fn compute_shadow_scope_matches(input: &ShadowScopeMatchesInput) -> ShadowScopeMatchesOutput {
    let scope_type = canonical_scope_type(input.scope_type.as_deref().unwrap_or(""));
    let scope_values = parse_scope_values(input.scope_value.as_deref().unwrap_or(""));
    let risk_levels = input
        .risk_levels
        .iter()
        .map(|v| normalize_shadow_scope_token(v))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let risk = normalize_shadow_scope_token(input.risk.as_deref().unwrap_or(""));
    let proposal_type = normalize_shadow_scope_token(input.proposal_type.as_deref().unwrap_or(""));
    let capability_key =
        normalize_shadow_scope_token(input.capability_key.as_deref().unwrap_or(""));
    let objective_id = normalize_shadow_scope_token(input.objective_id.as_deref().unwrap_or(""));

    let matched = match scope_type.as_str() {
        "proposal_type" => !proposal_type.is_empty() && scope_values.iter().any(|v| v == &proposal_type),
        "capability_key" => !capability_key.is_empty() && scope_values.iter().any(|v| v == &capability_key),
        "objective_id" => !objective_id.is_empty() && scope_values.iter().any(|v| v == &objective_id),
        "global" => {
            let mut levels = risk_levels;
            if !scope_values.is_empty() {
                for row in scope_values {
                    if !levels.contains(&row) {
                        levels.push(row);
                    }
                }
            }
            if levels.is_empty() {
                true
            } else {
                !risk.is_empty() && levels.iter().any(|v| v == &risk)
            }
        }
        _ => false,
    };
    ShadowScopeMatchesOutput { matched }
}

pub fn compute_collective_shadow_aggregate(
    input: &CollectiveShadowAggregateInput,
) -> CollectiveShadowAggregateOutput {
    let to_fixed4 = |value: f64| -> f64 { format!("{value:.4}").parse::<f64>().unwrap_or(value) };
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let matches = input.entries.len() as u32;
    if matches == 0 {
        return CollectiveShadowAggregateOutput {
            matches: 0,
            confidence_avg: 0.0,
            penalty_raw: 0.0,
            bonus_raw: 0.0,
        };
    }

    let confidence_sum = input
        .entries
        .iter()
        .map(|row| row.confidence.clamp(0.0, 1.0))
        .sum::<f64>();
    let confidence_avg = to_fixed4(confidence_sum / (matches as f64));

    let penalty_raw = input
        .entries
        .iter()
        .filter(|row| {
            row.kind
                .as_deref()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("avoid")
        })
        .map(|row| row.score_impact.max(0.0) * row.confidence.clamp(0.0, 1.0))
        .sum::<f64>();
    let bonus_raw = input
        .entries
        .iter()
        .filter(|row| {
            row.kind
                .as_deref()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("reinforce")
        })
        .map(|row| row.score_impact.max(0.0) * row.confidence.clamp(0.0, 1.0))
        .sum::<f64>();

    CollectiveShadowAggregateOutput {
        matches,
        confidence_avg,
        penalty_raw: to_fixed3(penalty_raw),
        bonus_raw: to_fixed3(bonus_raw),
    }
}

pub fn compute_expected_value_signal(
    input: &ExpectedValueSignalInput,
) -> ExpectedValueSignalOutput {
    let clamp_score = |value: f64| -> f64 {
        if !value.is_finite() {
            0.0
        } else {
            value.clamp(0.0, 100.0)
        }
    };
    let round_score = |value: f64| -> f64 { clamp_score(value.round()) };
    let to_fixed3 = |value: f64| -> f64 { format!("{value:.3}").parse::<f64>().unwrap_or(value) };
    let selected_currency = input.selected_currency.as_deref().unwrap_or("").trim();
    let oracle_priority = input
        .oracle_priority_score
        .filter(|value| value.is_finite())
        .map(round_score);

    let (base_score, source) = if let Some(explicit) = input.explicit_score {
        if explicit.is_finite() {
            (round_score(explicit), "expected_value_score".to_string())
        } else if let Some(usd) = input.expected_value_usd {
            if usd.is_finite() && usd > 0.0 {
                (
                    round_score((usd.max(1.0).log10()) * 30.0),
                    "expected_value_usd".to_string(),
                )
            } else if let Some(priority) = oracle_priority {
                (priority, "value_oracle_priority_score".to_string())
            } else {
                (
                    round_score(clamp_score(input.impact_weight) * 20.0),
                    "impact_weight_fallback".to_string(),
                )
            }
        } else if let Some(priority) = oracle_priority {
            (priority, "value_oracle_priority_score".to_string())
        } else {
            (
                round_score(clamp_score(input.impact_weight) * 20.0),
                "impact_weight_fallback".to_string(),
            )
        }
    } else if let Some(usd) = input.expected_value_usd {
        if usd.is_finite() && usd > 0.0 {
            (
                round_score((usd.max(1.0).log10()) * 30.0),
                "expected_value_usd".to_string(),
            )
        } else if let Some(priority) = oracle_priority {
            (priority, "value_oracle_priority_score".to_string())
        } else {
            (
                round_score(clamp_score(input.impact_weight) * 20.0),
                "impact_weight_fallback".to_string(),
            )
        }
    } else if let Some(priority) = oracle_priority {
        (priority, "value_oracle_priority_score".to_string())
    } else {
        (
            round_score(clamp_score(input.impact_weight) * 20.0),
            "impact_weight_fallback".to_string(),
        )
    };

    let currency_adjusted_score = oracle_priority.map(|priority| {
        round_score(
            priority
                * if input.currency_multiplier.is_finite() {
                    input.currency_multiplier.max(0.0)
                } else {
                    1.0
                },
        )
    });
    let apply_currency_rank = input.currency_ranking_enabled
        && input.oracle_applies
        && input.oracle_pass
        && currency_adjusted_score.is_some();
    let first_sentence_bonus = if apply_currency_rank
        && !selected_currency.is_empty()
        && input.matched_first_sentence_contains_selected
    {
        2.0
    } else {
        0.0
    };

    let delta = if apply_currency_rank {
        let adjusted = currency_adjusted_score.unwrap_or(0.0);
        let blend = if input.rank_blend.is_finite() {
            input.rank_blend.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let blended = (base_score * (1.0 - blend)) + (adjusted * blend) + first_sentence_bonus;
        let cap = if input.bonus_cap.is_finite() {
            input.bonus_cap.max(0.0)
        } else {
            0.0
        };
        (blended - base_score).clamp(-cap, cap)
    } else {
        0.0
    };
    let score = round_score(base_score + delta);

    ExpectedValueSignalOutput {
        score,
        base_score,
        source,
        value_oracle_priority: oracle_priority,
        currency_adjusted_score,
        currency_delta: to_fixed3(delta),
        oracle_applies: input.oracle_applies,
        oracle_pass: input.oracle_pass,
    }
}

pub fn compute_value_signal_score(input: &ValueSignalScoreInput) -> ValueSignalScoreOutput {
    let raw = (input.expected_value * 0.52)
        + (input.time_to_value * 0.22)
        + (input.actionability * 0.18)
        + (input.directive_fit * 0.08);
    ValueSignalScoreOutput {
        score: (raw * 1000.0).round() / 1000.0,
    }
}

pub fn compute_composite_eligibility_score(
    input: &CompositeEligibilityScoreInput,
) -> CompositeEligibilityScoreOutput {
    let clamp = |v: f64| -> f64 {
        if !v.is_finite() || v < 0.0 {
            0.0
        } else if v > 100.0 {
            100.0
        } else {
            v
        }
    };
    let q = clamp(input.quality_score);
    let d = clamp(input.directive_fit_score);
    let a = clamp(input.actionability_score);
    let weighted = (q * 0.42) + (d * 0.26) + (a * 0.32);
    let rounded = weighted.round();
    let score = if rounded <= 0.0 {
        0
    } else if rounded >= 100.0 {
        100
    } else {
        rounded as u32
    };
    CompositeEligibilityScoreOutput { score }
}

pub fn compute_time_to_value_score(input: &TimeToValueScoreInput) -> TimeToValueScoreOutput {
    if let Some(hours) = input.time_to_cash_hours {
        if hours.is_finite() && hours >= 0.0 {
            let score = 100.0 - (hours.min(168.0) / 168.0) * 100.0;
            let rounded = score.round();
            let clamped = if rounded <= 0.0 {
                0
            } else if rounded >= 100.0 {
                100
            } else {
                rounded as u32
            };
            return TimeToValueScoreOutput { score: clamped };
        }
    }
    let impact = input
        .expected_impact
        .as_ref()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let score = if impact == "high" {
        40
    } else if impact == "medium" {
        55
    } else {
        70
    };
    TimeToValueScoreOutput { score }
}

pub fn compute_value_density_score(input: &ValueDensityScoreInput) -> ValueDensityScoreOutput {
    let value = if !input.expected_value.is_finite() || input.expected_value < 0.0 {
        0.0
    } else if input.expected_value > 100.0 {
        100.0
    } else {
        input.expected_value
    };
    let tokens = if !input.est_tokens.is_finite() {
        80.0
    } else {
        input.est_tokens.clamp(80.0, 12000.0)
    };
    if value <= 0.0 {
        return ValueDensityScoreOutput { score: 0 };
    }
    let score = (value * 1000.0) / tokens.max(80.0);
    let rounded = score.round();
    let clamped = if rounded <= 0.0 {
        0
    } else if rounded >= 100.0 {
        100
    } else {
        rounded as u32
    };
    ValueDensityScoreOutput { score: clamped }
}

pub fn compute_normalize_directive_tier(
    input: &NormalizeDirectiveTierInput,
) -> NormalizeDirectiveTierOutput {
    let fallback = input.fallback.filter(|v| v.is_finite()).unwrap_or(3.0);
    let raw = input.raw_tier.filter(|v| v.is_finite()).unwrap_or(fallback);
    let tier = raw.round().max(1.0);
    NormalizeDirectiveTierOutput { tier: tier as u32 }
}

pub fn compute_directive_tier_weight(
    input: &DirectiveTierWeightInput,
) -> DirectiveTierWeightOutput {
    let fallback = input.fallback.filter(|v| v.is_finite()).unwrap_or(3.0);
    let raw = input.tier.filter(|v| v.is_finite()).unwrap_or(fallback);
    let normalized_tier = raw.round().max(1.0);
    let weight = if normalized_tier <= 1.0 {
        1.3
    } else if normalized_tier <= 2.0 {
        1.0
    } else if normalized_tier <= 3.0 {
        0.82
    } else {
        0.7
    };
    DirectiveTierWeightOutput { weight }
}

pub fn compute_directive_tier_min_share(
    input: &DirectiveTierMinShareInput,
) -> DirectiveTierMinShareOutput {
    let fallback = input.fallback.filter(|v| v.is_finite()).unwrap_or(3.0);
    let raw = input.tier.filter(|v| v.is_finite()).unwrap_or(fallback);
    let normalized_tier = raw.round().max(1.0);
    let clamp_ratio = |value: f64| -> f64 {
        if !value.is_finite() {
            0.0
        } else {
            value.clamp(0.0, 1.0)
        }
    };
    let min_share = if normalized_tier <= 1.0 {
        clamp_ratio(input.t1_min_share)
    } else if normalized_tier <= 2.0 {
        clamp_ratio(input.t2_min_share)
    } else {
        0.0
    };
    DirectiveTierMinShareOutput { min_share }
}

pub fn compute_directive_tier_coverage_bonus(
    input: &DirectiveTierCoverageBonusInput,
) -> DirectiveTierCoverageBonusOutput {
    let fallback = input.fallback.filter(|v| v.is_finite()).unwrap_or(3.0);
    let raw = input.tier.filter(|v| v.is_finite()).unwrap_or(fallback);
    let normalized_tier = raw.round().max(1.0);
    let attempts_today = if input.attempts_today.is_finite() {
        input.attempts_today.max(0.0)
    } else {
        0.0
    };
    let current_for_tier = if input.current_for_tier.is_finite() {
        input.current_for_tier.max(0.0)
    } else {
        0.0
    };

    if attempts_today <= 0.0 {
        let bonus = if normalized_tier <= 1.0 {
            8.0
        } else if normalized_tier <= 2.0 {
            4.0
        } else {
            0.0
        };
        return DirectiveTierCoverageBonusOutput { bonus };
    }

    let min_share = compute_directive_tier_min_share(&DirectiveTierMinShareInput {
        tier: Some(normalized_tier),
        fallback: Some(3.0),
        t1_min_share: input.t1_min_share,
        t2_min_share: input.t2_min_share,
    })
    .min_share;
    if min_share <= 0.0 {
        return DirectiveTierCoverageBonusOutput { bonus: 0.0 };
    }

    let expected = (attempts_today * min_share).ceil();
    let deficit = (expected - current_for_tier).max(0.0);
    let bonus = (deficit * 6.0).min(18.0);
    DirectiveTierCoverageBonusOutput { bonus }
}
