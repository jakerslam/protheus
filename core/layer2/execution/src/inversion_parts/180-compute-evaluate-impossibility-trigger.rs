pub fn compute_evaluate_impossibility_trigger(
    input: &EvaluateImpossibilityTriggerInput,
) -> EvaluateImpossibilityTriggerOutput {
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let signals = input
        .signals
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let cfg = value_path(Some(&policy), &["organ", "trigger_detection"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let force = input.force.unwrap_or(false);
    let threshold = clamp_number(
        js_number_for_extract(cfg.get("min_impossibility_score")).unwrap_or(0.58),
        0.0,
        1.0,
    );
    let min_signal_count = clamp_int_value(cfg.get("min_signal_count"), 1, 12, 2);
    let enabled = to_bool_like(cfg.get("enabled"), false);
    if !enabled && !force {
        return EvaluateImpossibilityTriggerOutput {
            triggered: false,
            forced: false,
            enabled: false,
            score: 0.0,
            threshold: (threshold * 1_000_000.0).round() / 1_000_000.0,
            signal_count: 0,
            min_signal_count,
            reasons: vec!["trigger_detection_disabled".to_string()],
            components: json!({}),
        };
    }
    let weights = cfg
        .get("weights")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let thresholds = cfg
        .get("thresholds")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let trit = normalize_trit_value(
        value_path(Some(&Value::Object(signals.clone())), &["trit", "value"])
            .unwrap_or(&Value::Null),
    );
    let trit_pain_signal = if trit < 0 {
        1.0
    } else if trit == 0 {
        0.5
    } else {
        0.0
    };
    let mirror_pressure = clamp_number(
        js_number_for_extract(value_path(
            Some(&Value::Object(signals.clone())),
            &["mirror", "pressure_score"],
        ))
        .unwrap_or(0.0),
        0.0,
        1.0,
    );
    let predicted_drift = clamp_number(
        js_number_for_extract(value_path(
            Some(&Value::Object(signals.clone())),
            &["simulation", "predicted_drift"],
        ))
        .unwrap_or(0.0),
        0.0,
        1.0,
    );
    let predicted_yield = clamp_number(
        js_number_for_extract(value_path(
            Some(&Value::Object(signals.clone())),
            &["simulation", "predicted_yield"],
        ))
        .unwrap_or(0.0),
        0.0,
        1.0,
    );
    let drift_warn = clamp_number(
        js_number_for_extract(thresholds.get("predicted_drift_warn")).unwrap_or(0.03),
        0.0,
        1.0,
    );
    let yield_warn = clamp_number(
        js_number_for_extract(thresholds.get("predicted_yield_warn")).unwrap_or(0.68),
        0.0,
        1.0,
    );
    let drift_score = if predicted_drift <= drift_warn {
        0.0
    } else {
        clamp_number(
            (predicted_drift - drift_warn) / (1.0 - drift_warn).max(0.0001),
            0.0,
            1.0,
        )
    };
    let yield_gap_score = if predicted_yield >= yield_warn {
        0.0
    } else {
        clamp_number(
            (yield_warn - predicted_yield) / yield_warn.max(0.0001),
            0.0,
            1.0,
        )
    };
    let red_team_critical = if clamp_int_value(
        value_path(
            Some(&Value::Object(signals.clone())),
            &["red_team", "critical_fail_cases"],
        ),
        0,
        100000,
        0,
    ) > 0
    {
        1.0
    } else {
        0.0
    };
    let regime_constrained = if to_bool_like(
        value_path(
            Some(&Value::Object(signals.clone())),
            &["regime", "constrained"],
        ),
        false,
    ) {
        1.0
    } else {
        0.0
    };
    let w_trit = js_number_for_extract(weights.get("trit_pain")).unwrap_or(0.2);
    let w_mirror = js_number_for_extract(weights.get("mirror_pressure")).unwrap_or(0.2);
    let w_drift = js_number_for_extract(weights.get("predicted_drift")).unwrap_or(0.18);
    let w_yield = js_number_for_extract(weights.get("predicted_yield_gap")).unwrap_or(0.18);
    let w_red = js_number_for_extract(weights.get("red_team_critical")).unwrap_or(0.14);
    let w_regime = js_number_for_extract(weights.get("regime_constrained")).unwrap_or(0.1);
    let weight_total = (w_trit + w_mirror + w_drift + w_yield + w_red + w_regime).max(0.0001);
    let score = clamp_number(
        ((trit_pain_signal * w_trit)
            + (mirror_pressure * w_mirror)
            + (drift_score * w_drift)
            + (yield_gap_score * w_yield)
            + (red_team_critical * w_red)
            + (regime_constrained * w_regime))
            / weight_total,
        0.0,
        1.0,
    );
    let signal_count = [
        trit_pain_signal,
        mirror_pressure,
        drift_score,
        yield_gap_score,
        red_team_critical,
        regime_constrained,
    ]
    .iter()
    .map(|v| if *v > 0.0 { 1 } else { 0 })
    .sum::<i32>() as i64;
    let mut reasons = Vec::new();
    if force {
        reasons.push("forced".to_string());
    }
    if trit_pain_signal > 0.0 {
        reasons.push("trit_pain_or_uncertain".to_string());
    }
    if mirror_pressure > 0.0 {
        reasons.push("mirror_pressure_signal".to_string());
    }
    if drift_score > 0.0 {
        reasons.push("predicted_drift_above_warn".to_string());
    }
    if yield_gap_score > 0.0 {
        reasons.push("predicted_yield_below_warn".to_string());
    }
    if red_team_critical > 0.0 {
        reasons.push("red_team_critical_present".to_string());
    }
    if regime_constrained > 0.0 {
        reasons.push("regime_constrained".to_string());
    }
    let triggered = force || (score >= threshold && signal_count >= min_signal_count);
    EvaluateImpossibilityTriggerOutput {
        triggered,
        forced: force,
        enabled,
        score: (score * 1_000_000.0).round() / 1_000_000.0,
        threshold: (threshold * 1_000_000.0).round() / 1_000_000.0,
        signal_count,
        min_signal_count,
        reasons: reasons.into_iter().take(12).collect::<Vec<_>>(),
        components: json!({
            "trit_pain": (trit_pain_signal * 1_000_000.0).round() / 1_000_000.0,
            "mirror_pressure": (mirror_pressure * 1_000_000.0).round() / 1_000_000.0,
            "predicted_drift": (drift_score * 1_000_000.0).round() / 1_000_000.0,
            "predicted_yield_gap": (yield_gap_score * 1_000_000.0).round() / 1_000_000.0,
            "red_team_critical": red_team_critical,
            "regime_constrained": regime_constrained
        }),
    }
}

pub fn compute_extract_first_principle(
    input: &ExtractFirstPrincipleInput,
) -> ExtractFirstPrincipleOutput {
    let policy = input.policy.as_ref();
    if value_path(policy, &["first_principles", "enabled"])
        .map(|v| !to_bool_like(Some(v), false))
        .unwrap_or(false)
    {
        return ExtractFirstPrincipleOutput { principle: None };
    }
    if clean_text_runtime(input.result.as_deref().unwrap_or(""), 24) != "success" {
        return ExtractFirstPrincipleOutput { principle: None };
    }
    let session = input
        .session
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let args = input
        .args
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let principle_text = clean_text_runtime(
        &value_to_string(
            args.get("principle")
                .or_else(|| args.get("first-principle")),
        ),
        360,
    );
    let auto_extract = to_bool_like(
        value_path(policy, &["first_principles", "auto_extract_on_success"]),
        false,
    );
    let text = if !principle_text.is_empty() {
        principle_text
    } else if auto_extract {
        let objective = clean_text_runtime(&value_to_string(session.get("objective")), 180);
        let filters = compute_normalize_list(&NormalizeListInput {
            value: Some(
                session
                    .get("filter_stack")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
            ),
            max_len: Some(120),
        })
        .items
        .join(", ");
        let target = compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(session.get("target"))),
        })
        .value;
        clean_text_runtime(
            &format!(
                "For {}, use inversion filters ({}) with a guarded {} lane, then revert to baseline paradigm.",
                if objective.is_empty() {
                    "objective".to_string()
                } else {
                    objective
                },
                if filters.is_empty() {
                    "none".to_string()
                } else {
                    filters
                },
                target
            ),
            360,
        )
    } else {
        String::new()
    };
    if text.is_empty() {
        return ExtractFirstPrincipleOutput { principle: None };
    }
    let certainty = clamp_number(
        js_number_for_extract(session.get("certainty")).unwrap_or(0.0),
        0.0,
        1.0,
    );
    let confidence = clamp_number(
        (certainty * 0.7)
            + if value_to_string(session.get("fallback_entry_id")).is_empty() {
                0.05
            } else {
                0.15
            },
        0.0,
        1.0,
    );
    let max_bonus = js_number_for_extract(value_path(
        policy,
        &["first_principles", "max_strategy_bonus"],
    ))
    .unwrap_or(0.12);
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let id_seed = format!("{}|{}", value_to_string(session.get("session_id")), text);
    let objective_id_value = {
        let v = clean_text_runtime(&value_to_string(session.get("objective_id")), 140);
        if v.is_empty() {
            Value::Null
        } else {
            Value::String(v)
        }
    };
    let suggested_bonus = {
        let bonus = clamp_number(confidence * max_bonus, 0.0, max_bonus.max(0.0));
        (bonus * 1_000_000.0).round() / 1_000_000.0
    };
    let principle = json!({
        "id": stable_id_runtime(&id_seed, "ifp"),
        "ts": now_iso.clone(),
        "source": "inversion_controller",
        "objective": clean_text_runtime(&value_to_string(session.get("objective")), 240),
        "objective_id": objective_id_value,
        "statement": text,
        "target": compute_normalize_target(&NormalizeTargetInput { value: Some(value_to_string(session.get("target"))) }).value,
        "confidence": (confidence * 1_000_000.0).round() / 1_000_000.0,
        "strategy_feedback": {
            "enabled": true,
            "suggested_bonus": suggested_bonus
        },
        "session_id": clean_text_runtime(&value_to_string(session.get("session_id")), 80)
    });
    ExtractFirstPrincipleOutput {
        principle: Some(principle),
    }
}
