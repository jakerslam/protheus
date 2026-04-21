
fn with_success_criteria_verification(
    base_verification: Option<&Value>,
    success_criteria: Option<&Value>,
    options: &Map<String, Value>,
) -> Value {
    let mut base = base_verification
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fallback = options
        .get("fallback")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let criteria = to_success_criteria_record(success_criteria, &fallback);
    let criteria_obj = criteria.as_object().cloned().unwrap_or_default();
    let criteria_pass = if criteria_obj
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        criteria_obj
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || criteria_obj
                .get("deferred_pending")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    } else {
        true
    };
    let mut checks = base
        .get("checks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut replaced = false;
    for row in &mut checks {
        if row.get("name").and_then(Value::as_str) == Some("success_criteria_met") {
            *row = json!({ "name": "success_criteria_met", "pass": criteria_pass });
            replaced = true;
            break;
        }
    }
    if !replaced {
        checks.push(json!({ "name": "success_criteria_met", "pass": criteria_pass }));
    }
    let mut failed = base
        .get("failed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let token = short_text(Some(&row), 80);
            if token.is_empty() {
                None
            } else {
                Some(Value::String(token))
            }
        })
        .collect::<Vec<_>>();
    let already = failed
        .iter()
        .any(|row| row.as_str() == Some("success_criteria_met"));
    if criteria_pass {
        failed.retain(|row| row.as_str() != Some("success_criteria_met"));
    } else if !already {
        failed.push(Value::String("success_criteria_met".to_string()));
    }
    let passed = failed.is_empty();
    let mut outcome = short_text(base.get("outcome"), 80);
    if outcome.is_empty() {
        outcome = if passed {
            "shipped".to_string()
        } else {
            "no_change".to_string()
        };
    }
    if !criteria_pass
        && options
            .get("enforceNoChangeOnFailure")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && outcome == "shipped"
    {
        outcome = "no_change".to_string();
    }
    let primary_failure = if !criteria_pass {
        criteria_obj
            .get("primary_failure")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .filter(|v| !v.is_empty())
            .or_else(|| {
                let existing = short_text(base.get("primary_failure"), 180);
                if existing.is_empty() {
                    None
                } else {
                    Some(existing)
                }
            })
            .unwrap_or_else(|| "success_criteria_failed".to_string())
    } else {
        let existing = short_text(base.get("primary_failure"), 180);
        existing
    };
    base.insert("checks".to_string(), Value::Array(checks));
    base.insert("failed".to_string(), Value::Array(failed));
    base.insert("passed".to_string(), Value::Bool(passed));
    base.insert("outcome".to_string(), Value::String(outcome));
    base.insert(
        "primary_failure".to_string(),
        if primary_failure.is_empty() {
            Value::Null
        } else {
            Value::String(primary_failure)
        },
    );
    base.insert("success_criteria".to_string(), criteria);
    Value::Object(base)
}
