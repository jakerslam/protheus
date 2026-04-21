fn required_string_list(control: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    control
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn control_error(id: &str, title: &str, kind: &str, rel_path: &str, reason: &str) -> Value {
    json!({
        "id": id,
        "title": title,
        "type": kind,
        "ok": false,
        "path": rel_path,
        "reason": reason
    })
}

fn run_control(root: &Path, control: &serde_json::Map<String, Value>) -> Value {
    let id = control
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let title = control
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("untitled")
        .to_string();
    let kind = control
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("path_exists")
        .to_string();
    let rel_path = control
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if rel_path.trim().is_empty() {
        return json!({
            "id": id,
            "title": title,
            "ok": false,
            "reason": "missing_path"
        });
    }

    let path = root.join(&rel_path);
    match kind.as_str() {
        "path_exists" => {
            let ok = path.exists();
            json!({
                "id": id,
                "title": title,
                "type": kind,
                "ok": ok,
                "path": rel_path,
                "reason": if ok { Value::Null } else { Value::String("path_missing".to_string()) }
            })
        }
        "file_contains_all" => {
            let required_tokens = required_string_list(control, "required_tokens");
            if required_tokens.is_empty() {
                return control_error(&id, &title, &kind, &rel_path, "required_tokens_missing");
            }
            match file_contains_all(&path, &required_tokens) {
                Ok(missing) => json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": missing.is_empty(),
                    "path": rel_path,
                    "required_tokens": required_tokens.len(),
                    "missing_tokens": missing
                }),
                Err(err) => control_error(&id, &title, &kind, &rel_path, &err),
            }
        }
        "json_fields" => {
            let required_fields = required_string_list(control, "required_fields");
            if required_fields.is_empty() {
                return control_error(&id, &title, &kind, &rel_path, "required_fields_missing");
            }
            match read_json(&path) {
                Ok(payload) => {
                    let missing_fields = required_fields
                        .iter()
                        .filter(|field| resolve_json_path(&payload, field).is_none())
                        .cloned()
                        .collect::<Vec<_>>();
                    json!({
                        "id": id,
                        "title": title,
                        "type": kind,
                        "ok": missing_fields.is_empty(),
                        "path": rel_path,
                        "required_fields": required_fields,
                        "missing_fields": missing_fields
                    })
                }
                Err(err) => control_error(&id, &title, &kind, &rel_path, &err),
            }
        }
        "cron_delivery_integrity" => match check_cron_delivery_integrity(root, &rel_path) {
            Ok((ok, details)) => json!({
                "id": id,
                "title": title,
                "type": kind,
                "ok": ok,
                "path": rel_path,
                "details": details
            }),
            Err(err) => control_error(&id, &title, &kind, &rel_path, &err),
        },
        _ => control_error(
            &id,
            &title,
            &kind,
            &rel_path,
            &format!("unknown_control_type:{kind}"),
        ),
    }
}

fn run_with_policy(
    root: &Path,
    cmd: &str,
    strict: bool,
    policy_path_rel: &str,
) -> Result<Value, String> {
    let policy_path = root.join(policy_path_rel);
    let policy = read_json(&policy_path)?;
    let controls = policy
        .get("controls")
        .and_then(Value::as_array)
        .ok_or_else(|| "enterprise_policy_missing_controls".to_string())?;

    let mut results = Vec::<Value>::new();
    for control in controls {
        let Some(section) = control.as_object() else {
            results.push(json!({
                "id": "unknown",
                "ok": false,
                "reason": "invalid_control_entry"
            }));
            continue;
        };
        results.push(run_control(root, section));
    }

    let passed = results
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failed = results.len().saturating_sub(passed);
    let ok = if strict { failed == 0 } else { true };

    let mut out = json!({
        "ok": ok,
        "type": "enterprise_hardening",
        "lane": "enterprise_hardening",
        "mode": cmd,
        "strict": strict,
        "ts": now_iso(),
        "policy_path": policy_path_rel,
        "controls_total": results.len(),
        "controls_passed": passed,
        "controls_failed": failed,
        "controls": results,
        "claim_evidence": [
            {
                "id": "f100_controls_gate",
                "claim": "fortune_100_control_contract_is_enforced_before_release",
                "evidence": {
                    "controls_total": controls.len(),
                    "strict": strict,
                    "failed": failed
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    Ok(out)
}
