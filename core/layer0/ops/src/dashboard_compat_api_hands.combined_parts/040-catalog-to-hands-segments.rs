
fn catalog(root: &Path, snapshot: &Value, state: &Value) -> Vec<Value> {
    let platform = server_platform();
    let mut rows = base_catalog(root, snapshot);
    for row in &mut rows {
        let hand_id = clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        let cfg = hand_config(state, &hand_id);
        let requirements = row
            .get("requirements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let (evaluated, met) = evaluate_requirements(&requirements, &cfg);
        row["requirements"] = Value::Array(evaluated);
        row["requirements_met"] = Value::Bool(met);
        row["server_platform"] = Value::String(platform.clone());
        row["dashboard_metrics"] = Value::from(
            row.get("dashboard")
                .and_then(Value::as_array)
                .map(|v| v.len())
                .unwrap_or(0) as i64,
        );
    }
    rows
}

fn hand_from_catalog(catalog: &[Value], hand_id: &str) -> Option<Value> {
    let id = clean_id(hand_id, 80);
    catalog
        .iter()
        .find(|row| clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == id)
        .cloned()
}

fn uptime_seconds(activated_at: &str) -> i64 {
    let Some(ts) = parse_rfc3339(activated_at) else {
        return 0;
    };
    (Utc::now() - ts).num_seconds().max(0)
}

fn trader_default_metrics() -> Map<String, Value> {
    let mut out = Map::<String, Value>::new();
    out.insert(
        "Portfolio Value".to_string(),
        json!({"value": "100000", "format": "number"}),
    );
    out.insert(
        "Total P&L".to_string(),
        json!({"value": "0", "format": "number"}),
    );
    out.insert(
        "Win Rate".to_string(),
        json!({"value": "0%", "format": "text"}),
    );
    out.insert(
        "Sharpe Ratio".to_string(),
        json!({"value": "0.00", "format": "number"}),
    );
    out.insert(
        "Max Drawdown".to_string(),
        json!({"value": "0%", "format": "text"}),
    );
    out.insert(
        "Trades Executed".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    out
}

fn stats_for_instance(instance: &Value) -> Value {
    let mut metrics = Map::<String, Value>::new();
    let status = clean_text(
        instance
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("Active"),
        40,
    );
    let activated_at = clean_text(
        instance
            .get("activated_at")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    metrics.insert(
        "Status".to_string(),
        json!({"value": status, "format": "text"}),
    );
    metrics.insert(
        "Uptime".to_string(),
        json!({"value": uptime_seconds(&activated_at), "format": "duration"}),
    );
    metrics.insert(
        "Restarts".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    metrics.insert(
        "Errors".to_string(),
        json!({"value": 0, "format": "number"}),
    );
    let hand_id = clean_id(
        instance
            .get("hand_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    if hand_id == "trader" {
        for (key, value) in trader_default_metrics() {
            metrics.insert(key, value);
        }
    } else if hand_id == "browser" {
        metrics.insert(
            "Pages Visited".to_string(),
            json!({"value": 0, "format": "number"}),
        );
        metrics.insert(
            "Last URL".to_string(),
            json!({"value": clean_text(instance.pointer("/config/start_url").and_then(Value::as_str).unwrap_or(""), 300), "format": "text"}),
        );
    }
    json!({"ok": true, "metrics": metrics})
}

fn hands_segments(path_only: &str) -> Option<Vec<String>> {
    if path_only == "/api/hands" {
        return Some(Vec::new());
    }
    if let Some(rest) = path_only.strip_prefix("/api/hands/") {
        let segments = rest
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        return Some(segments);
    }
    None
}
