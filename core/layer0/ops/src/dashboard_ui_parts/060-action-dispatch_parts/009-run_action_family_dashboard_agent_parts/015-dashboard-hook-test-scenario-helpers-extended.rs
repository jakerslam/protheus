fn dashboard_hook_test_task_start(root: &Path, payload: &Value) -> Value {
    dashboard_hook_test_process_simulate(
        root,
        &json!({
            "hook_id": dashboard_hook_resolve_id(payload),
            "phase": "task_start",
            "status": "completed",
            "message": "task start simulated"
        }),
    )
}

fn dashboard_hook_test_user_prompt_submit(root: &Path, payload: &Value) -> Value {
    let prompt = clean_text(
        payload
            .get("prompt")
            .and_then(Value::as_str)
            .or_else(|| payload.get("user_input").and_then(Value::as_str))
            .unwrap_or(""),
        1000,
    );
    let result = dashboard_hook_test_process_simulate(
        root,
        &json!({
            "hook_id": dashboard_hook_resolve_id(payload),
            "phase": "user_prompt_submit",
            "status": "completed",
            "context": prompt,
            "message": "user prompt submit simulated"
        }),
    );
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_user_prompt_submit_simulate",
        "prompt_length": prompt.len() as i64,
        "result": result
    })
}

fn dashboard_hook_test_precompact_evaluate(payload: &Value) -> Value {
    let before = payload
        .get("before_bytes")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let after = payload
        .get("after_bytes")
        .and_then(Value::as_i64)
        .unwrap_or(before)
        .max(0);
    let saved = before.saturating_sub(after);
    let ratio = if before > 0 {
        ((saved as f64) / (before as f64)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_precompact_evaluate",
        "before_bytes": before,
        "after_bytes": after,
        "saved_bytes": saved,
        "saved_ratio": ratio
    })
}

fn dashboard_hook_test_templates_render(payload: &Value) -> Value {
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    let mut rendered = template.clone();
    if let Some(values) = payload.get("values").and_then(Value::as_object) {
        for (k, v) in values {
            let token = format!("{{{{{}}}}}", clean_text(k, 120));
            let value = clean_text(v.as_str().unwrap_or(""), 500);
            rendered = rendered.replace(&token, &value);
        }
    }
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_templates_render",
        "rendered": rendered
    })
}

fn dashboard_hook_test_templates_placeholders(payload: &Value) -> Value {
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    let mut placeholders = Vec::<String>::new();
    let mut i = 0usize;
    let bytes = template.as_bytes();
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let mut j = i + 2;
            while j + 1 < bytes.len() {
                if bytes[j] == b'}' && bytes[j + 1] == b'}' {
                    let raw = &template[i + 2..j];
                    let key = clean_text(raw.trim(), 120);
                    if !key.is_empty() {
                        placeholders.push(key);
                    }
                    i = j + 2;
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }
    placeholders.sort();
    placeholders.dedup();
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_templates_placeholders",
        "placeholders": placeholders
    })
}

fn dashboard_hook_test_utils_digest(payload: &Value) -> Value {
    let digest = crate::deterministic_receipt_hash(payload);
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_utils_digest",
        "digest": digest
    })
}

fn dashboard_hook_test_ignore_evaluate(payload: &Value) -> Value {
    let path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    )
    .to_ascii_lowercase();
    let patterns = payload
        .get("patterns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|s| clean_text(s, 200).to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let ignored = patterns.iter().any(|pattern| {
        if pattern.is_empty() {
            return false;
        }
        path.contains(pattern)
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_ignore_evaluate",
        "path": path,
        "ignored": ignored,
        "pattern_count": patterns.len() as i64
    })
}

fn dashboard_hook_test_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.hooks.test.taskStart.simulate" => {
            Some(dashboard_hook_test_task_start(root, payload))
        }
        "dashboard.hooks.test.userPromptSubmit.simulate" => {
            Some(dashboard_hook_test_user_prompt_submit(root, payload))
        }
        "dashboard.hooks.test.precompact.evaluate" => {
            Some(dashboard_hook_test_precompact_evaluate(payload))
        }
        "dashboard.hooks.test.templates.render" => {
            Some(dashboard_hook_test_templates_render(payload))
        }
        "dashboard.hooks.test.templates.placeholders" => {
            Some(dashboard_hook_test_templates_placeholders(payload))
        }
        "dashboard.hooks.test.utils.digest" => Some(dashboard_hook_test_utils_digest(payload)),
        "dashboard.hooks.test.ignore.evaluate" => {
            Some(dashboard_hook_test_ignore_evaluate(payload))
        }
        _ => None,
    }
}

include!("016-dashboard-lock-permission-prompt-helpers.rs");
