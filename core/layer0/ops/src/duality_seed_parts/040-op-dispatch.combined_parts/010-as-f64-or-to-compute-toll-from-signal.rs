// SRS coverage marker: V4-DUAL-VOI-001

fn as_f64_or(value: Option<&Value>, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback)
}

fn evaluate_dual_voice_signal(
    policy: &Value,
    codex: &Value,
    state: &Value,
    context: &Value,
    left: &Value,
    right: &Value,
    opts: &Value,
) -> Value {
    let mut left_context = context.as_object().cloned().unwrap_or_default();
    left_context.insert("voice".to_string(), Value::String("left".to_string()));
    left_context.insert(
        "voice_role".to_string(),
        Value::String("structured_reasoning".to_string()),
    );
    if let Some(obj) = left.as_object() {
        for (k, v) in obj {
            left_context.insert(k.clone(), v.clone());
        }
    }
    let mut right_context = context.as_object().cloned().unwrap_or_default();
    right_context.insert("voice".to_string(), Value::String("right".to_string()));
    right_context.insert(
        "voice_role".to_string(),
        Value::String("creative_inversion".to_string()),
    );
    if let Some(obj) = right.as_object() {
        for (k, v) in obj {
            right_context.insert(k.clone(), v.clone());
        }
    }

    let left_signal = evaluate_signal(
        policy,
        codex,
        state,
        &Value::Object(left_context),
        &json!({
            "lane": "belief_formation",
            "source": "dual_voice_left",
            "run_id": as_str(context.get("run_id"))
        }),
    );
    let right_signal = evaluate_signal(
        policy,
        codex,
        state,
        &Value::Object(right_context),
        &json!({
            "lane": "weaver_arbitration",
            "source": "dual_voice_right",
            "run_id": as_str(context.get("run_id"))
        }),
    );

    let left_trit = normalize_trit(left_signal.get("score_trit"));
    let right_trit = normalize_trit(right_signal.get("score_trit"));
    let left_confidence = clamp_f64(as_f64_or(left_signal.get("confidence"), 0.0), 0.0, 1.0);
    let right_confidence = clamp_f64(as_f64_or(right_signal.get("confidence"), 0.0), 0.0, 1.0);
    let left_harmony = clamp_f64(
        as_f64_or(left_signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let right_harmony = clamp_f64(
        as_f64_or(right_signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );

    let voice_alignment = if left_trit != TRIT_UNKNOWN && left_trit == right_trit {
        1.0
    } else if left_trit == TRIT_UNKNOWN || right_trit == TRIT_UNKNOWN {
        0.5
    } else {
        0.0
    };
    let harmony = clamp_f64(
        (((left_harmony + right_harmony) * 0.5) * 0.7) + (voice_alignment * 0.3),
        0.0,
        1.0,
    );
    let min_harmony = clamp_f64(
        as_f64_or(
            policy
                .get("dual_voice")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("min_harmony")),
            0.42,
        ),
        0.0,
        1.0,
    );
    let min_voice_conf = clamp_f64(
        as_f64_or(
            policy
                .get("dual_voice")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("minimum_voice_confidence")),
            0.3,
        ),
        0.0,
        1.0,
    );
    let dual_voice_enabled = as_bool(
        policy
            .get("dual_voice")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("enabled")),
        true,
    );
    let min_confidence = left_confidence.min(right_confidence);
    let pass = dual_voice_enabled && harmony >= min_harmony && min_confidence >= min_voice_conf;
    let score_trit = if pass {
        TRIT_OK
    } else if harmony < (min_harmony * 0.65) {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    };

    let mut out = json!({
        "type": "duality_dual_voice_evaluation",
        "enabled": dual_voice_enabled,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "harmony": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "voice_alignment": (voice_alignment * 1_000_000.0).round() / 1_000_000.0,
        "minimum_harmony": min_harmony,
        "minimum_voice_confidence": min_voice_conf,
        "minimum_observed_confidence": (min_confidence * 1_000_000.0).round() / 1_000_000.0,
        "pass": pass,
        "recommended_adjustment": if pass {
            "hold_balance_near_zero_point"
        } else if left_trit == TRIT_PAIN || right_trit == TRIT_PAIN {
            "decrease_extreme_voice_and_rebalance"
        } else {
            "increase_cross_voice_harmony"
        },
        "left_voice": left_signal,
        "right_voice": right_signal
    });

    if let Some(run_id) = context.get("run_id").and_then(Value::as_str) {
        if !run_id.trim().is_empty() {
            out["run_id"] = Value::String(clean_text(run_id, 160));
        }
    }
    if let Some(source) = opts.get("source").and_then(Value::as_str) {
        if !source.trim().is_empty() {
            out["source"] = Value::String(normalize_token(source, 120));
        }
    }
    out
}

fn compute_toll_from_signal(policy: &Value, state: &Value, signal: &Value) -> Value {
    let toll_enabled = as_bool(policy.get("toll_enabled"), true);
    let debt_before = clamp_f64(as_f64_or(state.get("toll_debt"), 0.0), 0.0, 100.0);
    let score_trit = normalize_trit(signal.get("score_trit"));
    let balance_score = as_f64_or(signal.get("balance_score"), 0.0);
    let harmony = clamp_f64(
        as_f64_or(signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let trigger_negative = clamp_f64(
        as_f64_or(policy.get("toll_trigger_negative_threshold"), -0.2),
        -1.0,
        1.0,
    );
    let debt_step = clamp_f64(as_f64_or(policy.get("toll_debt_step"), 0.2), 0.0001, 10.0);
    let recovery_step = clamp_f64(
        as_f64_or(policy.get("toll_recovery_step"), 0.08),
        0.0001,
        10.0,
    );
    let hard_block_threshold = clamp_f64(
        as_f64_or(policy.get("toll_hard_block_threshold"), 1.0),
        0.1,
        100.0,
    );

    let mut debt_after = debt_before;
    if toll_enabled {
        if score_trit == TRIT_PAIN || balance_score <= trigger_negative {
            let severity = (balance_score.abs() + (1.0 - harmony)).max(0.15);
            debt_after = (debt_before + (debt_step * severity)).min(100.0);
        } else {
            let recovery = recovery_step * (1.0 + harmony);
            debt_after = (debt_before - recovery).max(0.0);
        }
    }
    let hard_block = toll_enabled && debt_after >= hard_block_threshold;

    json!({
        "enabled": toll_enabled,
        "debt_before": (debt_before * 1_000_000.0).round() / 1_000_000.0,
        "debt_after": (debt_after * 1_000_000.0).round() / 1_000_000.0,
        "trigger_negative_threshold": trigger_negative,
        "hard_block_threshold": hard_block_threshold,
        "hard_block": hard_block,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "harmony": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "balance_score": (balance_score * 1_000_000.0).round() / 1_000_000.0
    })
}
