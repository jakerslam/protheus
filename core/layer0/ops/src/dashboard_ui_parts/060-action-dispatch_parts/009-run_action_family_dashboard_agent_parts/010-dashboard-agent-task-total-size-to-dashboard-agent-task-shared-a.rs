fn dashboard_agent_task_total_size(tasks: &[Value]) -> i64 {
    tasks
        .iter()
        .map(|row| serde_json::to_vec(row).map(|bytes| bytes.len() as i64).unwrap_or(0))
        .sum::<i64>()
}

fn dashboard_agent_task_status_counts(tasks: &[Value]) -> Value {
    let mut counts = serde_json::Map::<String, Value>::new();
    for row in tasks {
        let status = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("queued"),
            40,
        )
        .to_ascii_lowercase();
        let entry = counts.entry(status).or_insert_with(|| Value::from(0));
        let next = entry.as_i64().unwrap_or(0) + 1;
        *entry = Value::from(next);
    }
    Value::Object(counts)
}

fn dashboard_agent_task_shared_and_changed(before: &Value, after: &Value) -> (Value, Value) {
    let mut shared = serde_json::Map::<String, Value>::new();
    let mut changed = Vec::<Value>::new();
    let before_obj = before.as_object().cloned().unwrap_or_default();
    let after_obj = after.as_object().cloned().unwrap_or_default();

    let mut keys = std::collections::BTreeSet::<String>::new();
    for key in before_obj.keys() {
        keys.insert(clean_text(key, 120));
    }
    for key in after_obj.keys() {
        keys.insert(clean_text(key, 120));
    }

    for key in keys {
        if key.is_empty() {
            continue;
        }
        let before_value = before_obj.get(&key).cloned().unwrap_or(Value::Null);
        let after_value = after_obj.get(&key).cloned().unwrap_or(Value::Null);
        if before_value == after_value {
            shared.insert(key, after_value);
        } else {
            changed.push(json!({
                "field": key,
                "before": before_value,
                "after": after_value
            }));
        }
    }

    (Value::Object(shared), Value::Array(changed))
}
