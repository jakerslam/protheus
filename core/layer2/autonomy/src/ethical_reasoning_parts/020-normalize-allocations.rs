
fn normalize_allocations(payload: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let rows = payload
        .get("value_context")
        .and_then(|m| m.get("allocations"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for row in rows {
        let obj = row.as_object();
        let metric_id = obj
            .and_then(|m| m.get("metric_id"))
            .and_then(Value::as_str)
            .map(|v| normalize_token(v, 80))
            .unwrap_or_default();
        if metric_id.is_empty() {
            continue;
        }
        let value_currency = obj
            .and_then(|m| m.get("value_currency"))
            .and_then(Value::as_str)
            .map(|v| normalize_token(v, 80))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "adaptive_value".to_string());
        let share = clamp_num(
            obj.and_then(|m| m.get("share"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            0.0,
            1.0,
            0.0,
        );
        let raw_score = clamp_num(
            obj.and_then(|m| m.get("raw_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            -10.0,
            10.0,
            0.0,
        );
        out.push(json!({
            "metric_id": metric_id,
            "value_currency": value_currency,
            "share": share,
            "raw_score": raw_score
        }));
    }

    out.sort_by(|a, b| {
        b.get("share")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .partial_cmp(&a.get("share").and_then(Value::as_f64).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn load_priors(path: &Path, fallback: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    let raw = read_json(path);
    let mut out = BTreeMap::new();
    if let Some(priors) = raw.get("priors").and_then(Value::as_object) {
        for (k, v) in priors {
            let key = normalize_token(k, 80);
            if key.is_empty() {
                continue;
            }
            out.insert(key, clamp_num(v.as_f64().unwrap_or(0.0), 0.0, 1.0, 0.0));
        }
    }
    if out.is_empty() {
        return fallback.clone();
    }
    out
}

fn normalize_priors(priors: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    if priors.is_empty() {
        return BTreeMap::new();
    }
    let sum: f64 = priors.values().copied().sum();
    if sum <= 0.0 {
        let even = round_to(1.0 / priors.len() as f64, 6);
        return priors.keys().map(|k| (k.clone(), even)).collect();
    }
    priors
        .iter()
        .map(|(k, v)| (k.clone(), round_to(clamp_num(*v / sum, 0.0, 1.0, 0.0), 6)))
        .collect()
}
