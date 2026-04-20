fn chat_ui_apply_loop_risk_to_retry(
    retry_recommended: bool,
    retry_strategy: &'static str,
    retry_lane: &'static str,
    loop_risk: &Value,
) -> (bool, &'static str, &'static str) {
    let loop_risk_detected = loop_risk
        .get("detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if loop_risk_detected && retry_recommended {
        return (false, "halt_on_loop_risk", "manual_intervention");
    }
    (retry_recommended, retry_strategy, retry_lane)
}
