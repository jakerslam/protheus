
fn route_score(
    row: &Value,
    prefer_local: bool,
    complexity: &str,
    task_type: &str,
    budget_mode: &str,
) -> f64 {
    if !parse_bool(row.get("available"), true) {
        return -1000.0;
    }
    let power = parse_i64(row.get("power_scale"), 3) as f64;
    let cost = parse_i64(row.get("cost_scale"), 3) as f64;
    let context = parse_i64(row.get("context_scale"), 3) as f64;
    let is_local = parse_bool(row.get("is_local"), false);
    let needs_key = parse_bool(row.get("needs_key"), false);
    let auth_status = clean_text(
        row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let specialty = clean_text(
        row.get("specialty").and_then(Value::as_str).unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let route_bias = parse_f64_value(row.get("route_bias"));

    let mut score = 0.0;
    score += power
        * if complexity == "high" || complexity == "deep" {
            1.8
        } else {
            0.9
        };
    score += context * if task_type.contains("long") { 1.2 } else { 0.4 };
    score += if budget_mode.contains("cheap") || budget_mode.contains("low") {
        (6.0 - cost) * 1.2
    } else {
        power * 0.4
    };
    if task_type.contains("code") && (specialty.contains("code") || specialty.contains("dev")) {
        score += 2.0;
    }
    if prefer_local {
        score += if is_local { 4.0 } else { -4.0 };
    }
    if needs_key && !crate::dashboard_provider_runtime::auth_status_configured(&auth_status) {
        score -= 1.5;
    }
    score += route_bias;
    score
}
