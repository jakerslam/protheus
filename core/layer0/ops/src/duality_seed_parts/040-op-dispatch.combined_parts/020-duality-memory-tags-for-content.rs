
fn duality_memory_tags_for_content(policy: &Value, signal: &Value) -> Value {
    let tagging_enabled = as_bool(
        policy
            .get("memory")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("tagging_enabled")),
        true,
    );
    let high_recall_threshold = clamp_f64(
        as_f64_or(
            policy
                .get("memory")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("high_recall_threshold")),
            0.65,
        ),
        0.0,
        1.0,
    );
    let inversion_flag_threshold = clamp_f64(
        as_f64_or(
            policy
                .get("memory")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("inversion_flag_threshold")),
            0.35,
        ),
        0.0,
        1.0,
    );
    let score_trit = normalize_trit(signal.get("score_trit"));
    let balance_score = as_f64_or(signal.get("balance_score"), 0.0);
    let harmony = clamp_f64(
        as_f64_or(signal.get("zero_point_harmony_potential"), 0.0),
        0.0,
        1.0,
    );
    let high_recall_priority =
        tagging_enabled && score_trit == TRIT_OK && harmony >= high_recall_threshold;
    let inversion_review_flag = tagging_enabled
        && (score_trit == TRIT_PAIN
            || harmony <= inversion_flag_threshold
            || balance_score <= -inversion_flag_threshold);
    json!({
        "enabled": tagging_enabled,
        "score_trit": score_trit,
        "score_label": trit_label(score_trit),
        "balance_score": (balance_score * 1_000_000.0).round() / 1_000_000.0,
        "zero_point_harmony_potential": (harmony * 1_000_000.0).round() / 1_000_000.0,
        "high_recall_priority": high_recall_priority,
        "inversion_review_flag": inversion_review_flag,
        "recommended_adjustment": signal.get("recommended_adjustment").cloned().unwrap_or(Value::Null)
    })
}
