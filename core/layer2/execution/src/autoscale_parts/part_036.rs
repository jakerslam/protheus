pub fn compute_execute_confidence_policy(
    input: &ExecuteConfidencePolicyInput,
) -> ExecuteConfidencePolicyOutput {
    let history_obj = input
        .history
        .as_ref()
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let history_executed = history_obj
        .get("executed")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let history_shipped = history_obj
        .get("shipped")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let history_reverted = history_obj
        .get("reverted")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let history_no_change_rate = history_obj
        .get("no_change_rate")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let history_confidence_fallback = history_obj
        .get("confidence_fallback")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);

    let mut composite_margin = input.base_composite_margin.max(0.0);
    let mut value_margin = input.base_value_margin.max(0.0);
    let mut reasons = Vec::<String>::new();

    let risk = input
        .risk
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| value == "low" || value == "medium" || value == "high")
        .unwrap_or_else(|| "low".to_string());
    let execution_mode = input
        .execution_mode
        .as_ref()
        .map(|value| value.trim().to_string())
        .unwrap_or_default();

    if input.adaptive_enabled && execution_mode == "canary_execute" && risk == "low" {
        composite_margin = (composite_margin - input.low_risk_relax_composite.max(0.0)).max(0.0);
        value_margin = (value_margin - input.low_risk_relax_value.max(0.0)).max(0.0);
        reasons.push("low_risk_canary_relax".to_string());
    }

    if input.adaptive_enabled
        && history_reverted <= 0.0
        && history_confidence_fallback >= input.fallback_relax_every.max(1.0)
    {
        let ship_rate = if history_executed > 0.0 {
            history_shipped / history_executed.max(1.0)
        } else {
            0.0
        };
        let relax_eligible = history_executed >= input.fallback_relax_min_executed
            && history_shipped >= input.fallback_relax_min_shipped
            && ship_rate >= input.fallback_relax_min_ship_rate;
        if relax_eligible {
            let relax_steps =
                (history_confidence_fallback / input.fallback_relax_every.max(1.0)).floor();
            let relax_raw = relax_steps * input.fallback_relax_step.max(0.0);
            let relax = relax_raw.clamp(0.0, input.fallback_relax_max.max(0.0));
            if relax > 0.0 {
                composite_margin = (composite_margin - relax).max(0.0);
                value_margin = (value_margin - relax).max(0.0);
                reasons.push("fallback_churn_relax".to_string());
            }
        } else {
            reasons.push("fallback_churn_relax_blocked_low_success".to_string());
        }
    }

    if input.adaptive_enabled
        && history_executed >= input.no_change_tighten_min_executed
        && history_no_change_rate >= input.no_change_tighten_threshold
    {
        composite_margin += input.no_change_tighten_step.max(0.0);
        value_margin += input.no_change_tighten_step.max(0.0);
        reasons.push("high_no_change_tighten".to_string());
    }

    if history_reverted > 0.0 {
        composite_margin = composite_margin.max(input.base_composite_margin.max(0.0));
        value_margin = value_margin.max(input.base_value_margin.max(0.0));
        reasons.push("reverted_restore_base".to_string());
    }

    let ship_rate = if history_executed > 0.0 {
        ((history_shipped / history_executed.max(1.0)) * 1000.0).round() / 1000.0
    } else {
        0.0
    };
    let policy = serde_json::json!({
        "adaptive_enabled": input.adaptive_enabled,
        "proposal_type": input.proposal_type.as_ref().map(|v| v.trim().to_ascii_lowercase()).filter(|v| !v.is_empty()),
        "capability_key": input.capability_key.as_ref().map(|v| v.trim().to_ascii_lowercase()).filter(|v| !v.is_empty()),
        "risk": risk,
        "execution_mode": execution_mode,
        "base": {
            "composite_margin": input.base_composite_margin.max(0.0),
            "value_margin": input.base_value_margin.max(0.0)
        },
        "applied": {
            "composite_margin": composite_margin.max(0.0),
            "value_margin": value_margin.max(0.0)
        },
        "history": history_obj,
        "fallback_relax_eligibility": {
            "min_executed": input.fallback_relax_min_executed,
            "min_shipped": input.fallback_relax_min_shipped,
            "min_ship_rate": input.fallback_relax_min_ship_rate,
            "ship_rate": ship_rate
        },
        "reasons": reasons
    });
    ExecuteConfidencePolicyOutput { policy }
}

pub fn compute_directive_fit_assessment(
    input: &DirectiveFitAssessmentInput,
) -> DirectiveFitAssessmentOutput {
    let active_directive_ids = input
        .active_directive_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !input.profile_available {
        return DirectiveFitAssessmentOutput {
            pass: true,
            score: 100.0,
            profile_available: false,
            active_directive_ids,
            reasons: vec!["directive_profile_unavailable".to_string()],
            matched_positive: Vec::new(),
            matched_negative: Vec::new(),
        };
    }

    let positive_phrase_hits = input
        .positive_phrase_hits
        .iter()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let positive_token_hits = input
        .positive_token_hits
        .iter()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let strategy_hits = input
        .strategy_hits
        .iter()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let negative_phrase_hits = input
        .negative_phrase_hits
        .iter()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let negative_token_hits = input
        .negative_token_hits
        .iter()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    let mut score = 30.0;
    score += positive_phrase_hits.len() as f64 * 18.0;
    score += ((positive_token_hits.len() as f64) * 5.0).min(30.0);
    score += ((strategy_hits.len() as f64) * 4.0).min(12.0);
    score -= negative_phrase_hits.len() as f64 * 20.0;
    score -= ((negative_token_hits.len() as f64) * 6.0).min(24.0);

    let impact = input
        .impact
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if impact == "high" {
        score += 6.0;
    } else if impact == "medium" {
        score += 3.0;
    }

    let final_score = score.round().clamp(0.0, 100.0);
    let mut reasons = Vec::<String>::new();
    if positive_phrase_hits.is_empty() && positive_token_hits.is_empty() && strategy_hits.is_empty()
    {
        reasons.push("no_directive_alignment".to_string());
    }
    if input.strategy_token_count > 0.0 && strategy_hits.is_empty() {
        reasons.push("no_strategy_marker".to_string());
    }
    if !negative_phrase_hits.is_empty() || !negative_token_hits.is_empty() {
        reasons.push("matches_excluded_scope".to_string());
    }
    let pass = final_score >= input.min_directive_fit;
    if !pass {
        reasons.push("below_min_directive_fit".to_string());
    }

    let mut pos_set = std::collections::BTreeSet::<String>::new();
    for value in positive_phrase_hits
        .iter()
        .chain(positive_token_hits.iter())
        .chain(strategy_hits.iter())
    {
        if !value.trim().is_empty() {
            pos_set.insert(value.trim().to_string());
        }
    }
    let matched_positive = pos_set.into_iter().take(5).collect::<Vec<_>>();

    let mut neg_set = std::collections::BTreeSet::<String>::new();
    for value in negative_phrase_hits
        .iter()
        .chain(negative_token_hits.iter())
    {
        if !value.trim().is_empty() {
            neg_set.insert(value.trim().to_string());
        }
    }
    let matched_negative = neg_set.into_iter().take(5).collect::<Vec<_>>();

    DirectiveFitAssessmentOutput {
        pass,
        score: final_score,
        profile_available: true,
        active_directive_ids,
        reasons,
        matched_positive,
        matched_negative,
    }
}

pub fn compute_signal_quality_assessment(
    input: &SignalQualityAssessmentInput,
) -> SignalQualityAssessmentOutput {
    let eye_id = input
        .eye_id
        .as_ref()
        .map(|value| normalize_spaces(value))
        .unwrap_or_default();
    let score_source = input
        .score_source
        .as_ref()
        .map(|value| normalize_spaces(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "fallback_default".to_string());
    let impact = input
        .impact
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let risk = input
        .risk
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let domain = input
        .domain
        .as_ref()
        .map(|value| normalize_spaces(value).to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let url_scheme = input
        .url_scheme
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let sensory_relevance_tier = input
        .sensory_relevance_tier
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let sensory_quality_tier = input
        .sensory_quality_tier
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let eye_status = input
        .eye_status
        .as_ref()
        .map(|value| normalize_spaces(value).to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let parser_type = input
        .parser_type
        .as_ref()
        .map(|value| normalize_spaces(value).to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    let mut reasons = Vec::<String>::new();
    let mut hard_block = false;
    let mut score = 0.0;

    if let Some(raw) = input.combined_item_score {
        if raw.is_finite() {
            score += raw.clamp(0.0, 100.0);
        } else {
            score += 18.0;
            reasons.push("missing_meta_score".to_string());
        }
    } else {
        score += 18.0;
        reasons.push("missing_meta_score".to_string());
    }

    if let Some(raw) = input.sensory_relevance_score {
        if raw.is_finite() && raw < input.min_sensory_relevance {
            hard_block = true;
            reasons.push("sensory_relevance_low".to_string());
        }
    }
    if let Some(raw) = input.sensory_quality_score {
        if raw.is_finite() && raw < input.min_sensory_signal {
            hard_block = true;
            reasons.push("sensory_quality_low".to_string());
        }
    }

    if sensory_relevance_tier.as_deref() == Some("low") {
        score -= 8.0;
    }
    if sensory_quality_tier.as_deref() == Some("low") {
        score -= 8.0;
    }

    if impact == "high" {
        score += 12.0;
    } else if impact == "medium" {
        score += 6.0;
    }

    if risk == "high" {
        score -= 12.0;
    } else if risk == "medium" {
        score -= 6.0;
    }

    if url_scheme == "https" {
        score += 6.0;
    } else if url_scheme == "http" {
        score += 2.0;
    } else {
        score -= 8.0;
    }

    if input.title_has_stub {
        score -= 40.0;
        hard_block = true;
        reasons.push("stub_title".to_string());
    }

    if input.eye_known {
        if let Some(eye_score_ema) = input.eye_score_ema {
            if eye_score_ema.is_finite() {
                score += (eye_score_ema - 50.0) * 0.35;
                if eye_score_ema < input.min_eye_score_ema {
                    hard_block = true;
                    reasons.push("eye_score_ema_low".to_string());
                }
            }
        }

        if let Some(status) = eye_status.as_deref() {
            if status == "active" {
                score += 4.0;
            } else if status == "probation" {
                score -= 6.0;
            } else if status == "dormant" {
                score -= 18.0;
                hard_block = true;
                reasons.push("eye_dormant".to_string());
            }
        }

        if input.parser_disallowed {
            score -= 30.0;
            hard_block = true;
            let parser_label = parser_type.as_deref().unwrap_or("unknown");
            reasons.push(format!("parser_disallowed:{parser_label}"));
        }

        if domain.is_some() && input.domain_allowlist_enforced && !input.domain_allowed {
            score -= 3.0;
            reasons.push("domain_outside_allowlist".to_string());
        }

        let proposed_total = input.eye_proposed_total.unwrap_or(0.0);
        if proposed_total >= 3.0 {
            if let Some(yield_rate) = input.eye_yield_rate {
                if yield_rate.is_finite() {
                    score += (yield_rate * 15.0) - 5.0;
                    if yield_rate < 0.1 {
                        reasons.push("eye_yield_low".to_string());
                    }
                }
            }
        }
    } else {
        reasons.push("eye_unknown".to_string());
    }

    let total_bias = input.calibration_eye_bias + input.calibration_topic_bias;
    if total_bias.is_finite() && total_bias != 0.0 {
        score -= total_bias;
        reasons.push(if total_bias > 0.0 {
            "calibration_penalty".to_string()
        } else {
            "calibration_bonus".to_string()
        });
    }

    let final_score = score.round().clamp(0.0, 100.0);
    let pass = !hard_block && final_score >= input.min_signal_quality;
    if !pass && final_score < input.min_signal_quality {
        reasons.push("below_min_signal_quality".to_string());
    }

    SignalQualityAssessmentOutput {
        pass,
        score: final_score,
        score_source,
        eye_id,
        sensory_relevance_score: input.sensory_relevance_score,
        sensory_relevance_tier,
        sensory_quality_score: input.sensory_quality_score,
        sensory_quality_tier,
        eye_status,
        eye_score_ema: input.eye_score_ema,
        parser_type,
        domain,
        calibration_eye_bias: input.calibration_eye_bias,
        calibration_topic_bias: ((input.calibration_topic_bias * 1000.0).round()) / 1000.0,
        calibration_total_bias: ((total_bias * 1000.0).round()) / 1000.0,
        reasons,
    }
}
