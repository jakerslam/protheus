fn build_reasoning_receipt(contract: &Value, goal: &str, risk: &str, approved: bool) -> Value {
    let auto_allow = contract
        .get("reasoning_gate")
        .and_then(|v| v.get("auto_allow_risks"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![Value::String("low".to_string())]);
    let auto_allow_risks = auto_allow
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let requires_explicit_approval = !auto_allow_risks.iter().any(|v| v == risk);
    let continue_allowed = !requires_explicit_approval || approved;
    let mut out = json!({
        "type": "app_plane_reasoning_gate",
        "goal": clean(goal, 2000),
        "risk_class": risk,
        "approved": approved,
        "requires_explicit_approval": requires_explicit_approval,
        "continue_allowed": continue_allowed,
        "plan": [
            {"stage":"research","intent":"collect constraints and edge cases"},
            {"stage":"plan","intent":"derive execution graph and acceptance criteria"},
            {"stage":"code","intent":"materialize deterministic artifacts"},
            {"stage":"test","intent":"run bounded verification and critique loop"},
            {"stage":"package","intent":"emit delivery manifest with provenance"}
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
