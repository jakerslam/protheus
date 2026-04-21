
fn parse_actions(
    root: &Path,
    argv: &[String],
    policy: &Policy,
) -> Result<(Vec<Value>, String), String> {
    let source_label =
        parse_flag(argv, "source").unwrap_or_else(|| "one_knowledge_sync".to_string());
    let now_ms = now_epoch_ms();
    if let Some(raw_json) = parse_flag(argv, "catalog-json") {
        let parsed = serde_json::from_str::<Value>(&raw_json)
            .map_err(|e| format!("catalog_json_parse_failed:{e}"))?;
        let rows = parsed
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| parsed.as_array().cloned())
            .unwrap_or_default();
        let actions = rows
            .iter()
            .filter_map(|row| normalize_action(row, &source_label, now_ms))
            .collect::<Vec<_>>();
        return Ok((actions, source_label));
    }

    let catalog_path = parse_flag(argv, "catalog-path")
        .map(PathBuf::from)
        .or_else(|| policy.source_catalog_path.clone());
    if let Some(path) = catalog_path {
        let resolved = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if let Some(parsed) = read_json(&resolved) {
            let rows = parsed
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| parsed.as_array().cloned())
                .unwrap_or_default();
            let actions = rows
                .iter()
                .filter_map(|row| normalize_action(row, &source_label, now_ms))
                .collect::<Vec<_>>();
            return Ok((actions, rel(root, &resolved)));
        }
    }

    Ok((builtin_actions(now_ms), "builtin_seed".to_string()))
}

fn lane_receipt(
    lane_type: &str,
    command: &str,
    argv: &[String],
    payload: Value,
    root: &Path,
    policy: &Policy,
) -> Value {
    with_hash(json!({
        "ok": payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
        "type": lane_type,
        "lane": "public_api_catalog",
        "ts_epoch_ms": now_epoch_ms(),
        "ts": now_iso(),
        "command": command,
        "argv": argv,
        "root": root.to_string_lossy(),
        "state_path": rel(root, &policy.state_path),
        "history_path": rel(root, &policy.history_path),
        "strict_fail_closed": policy.strict,
        "payload": payload,
        "claim_evidence": [{
            "id": "v6_tooling_047",
            "claim": "human_verified_action_schemas_are_routed_through_core_authority_with_receipts",
            "evidence": {"layer":"core/layer2/ops","route":"conduit","catalog":"public_api_catalog"}
        }]
    }))
}

fn err(
    root: &Path,
    policy: &Policy,
    command: &str,
    argv: &[String],
    code: &str,
    message: &str,
    exit_code: i32,
) -> CommandResult {
    CommandResult {
        exit_code,
        payload: lane_receipt(
            "public_api_catalog_error",
            command,
            argv,
            json!({"ok":false,"code":code,"error":clean_text(message,320),"routed_via":"conduit"}),
            root,
            policy,
        ),
    }
}

fn action_is_stale(action: &Value, now_ms: u64, max_age_days: f64) -> bool {
    let updated = action
        .get("updated_epoch_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if updated == 0 {
        return true;
    }
    let max_age_ms = (max_age_days * 24.0 * 60.0 * 60.0 * 1000.0).round() as u64;
    now_ms.saturating_sub(updated) > max_age_ms
}

fn action_template(action: &Value) -> Value {
    let platform = action
        .get("platform")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut headers = Map::new();
    headers.insert(
        "Content-Type".to_string(),
        Value::String("application/json".to_string()),
    );
    headers.insert(
        "Authorization".to_string(),
        Value::String(format!(
            "Bearer {{{{connection.{}.access_token}}}}",
            platform
        )),
    );
    json!({
        "action_id": action.get("id").cloned().unwrap_or(Value::Null),
        "platform": platform,
        "method": action.get("method").cloned().unwrap_or_else(|| Value::String("POST".to_string())),
        "url": action.get("url").cloned().unwrap_or(Value::Null),
        "headers": Value::Object(headers),
        "parameters": action.get("parameters").cloned().unwrap_or_else(|| json!({})),
        "response_schema": action.get("response_schema").cloned().unwrap_or_else(|| json!({})),
        "enforcement_rules": action.get("enforcement_rules").cloned().unwrap_or_else(|| json!({})),
        "examples": action.get("examples").cloned().unwrap_or_else(|| json!([]))
    })
}

fn lookup_json_path<'a>(root: &'a Value, expr: &str) -> Option<&'a Value> {
    let trimmed = expr.trim();
    if trimmed.is_empty() || trimmed == "$" {
        return Some(root);
    }
    let path = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))?;
    let mut cur = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        if let Some(idx_start) = segment.find('[') {
            let key = &segment[..idx_start];
            if !key.is_empty() {
                cur = cur.get(key)?;
            }
            let idx_end = segment[idx_start + 1..].find(']')?;
            let idx = segment[idx_start + 1..idx_start + 1 + idx_end]
                .parse::<usize>()
                .ok()?;
            cur = cur.get(idx)?;
        } else {
            cur = cur.get(segment)?;
        }
    }
    Some(cur)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().map(|n| n != 0.0).unwrap_or(false),
        Value::String(v) => !v.trim().is_empty(),
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
        Value::Null => false,
    }
}
