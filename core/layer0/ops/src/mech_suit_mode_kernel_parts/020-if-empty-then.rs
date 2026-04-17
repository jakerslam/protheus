impl StringExt for String {
    fn if_empty_then(self, fallback: String) -> String {
        if self.is_empty() {
            fallback
        } else {
            self
        }
    }
}

fn resolve_policy_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("policy_path") {
        let s = as_str(Some(raw));
        if !s.is_empty() {
            return resolve_path(root, &s, DEFAULT_POLICY_REL);
        }
    }
    if let Ok(raw) = std::env::var("MECH_SUIT_MODE_POLICY_PATH") {
        if !raw.trim().is_empty() {
            return resolve_path(root, &raw, DEFAULT_POLICY_REL);
        }
    }
    root.join(DEFAULT_POLICY_REL)
}

fn load_policy(root: &Path, payload: &Map<String, Value>) -> Value {
    let policy_path = resolve_policy_path(root, payload);
    let raw = read_json(&policy_path)
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    normalize_policy(Some(&raw), root, &policy_path)
}

fn approx_token_count_value(value: &Value) -> i64 {
    let text = as_str(Some(value));
    if text.trim().is_empty() {
        0
    } else {
        ((text.len() + 3) / 4) as i64
    }
}

fn classify_severity_value(message: &str, patterns: &[String]) -> String {
    let line = message.to_ascii_lowercase();
    if line.trim().is_empty() {
        return "info".to_string();
    }
    let critical_terms = [
        "critical",
        "fail",
        "failed",
        "emergency",
        "blocked",
        "halt",
        "panic",
        "violation",
        "integrity",
        "outage",
        "fatal",
        "unauthorized",
        "forbidden",
        "rate limit",
        "timeout",
        "unreachable",
        "auth missing",
        "ssrf",
        "invalid response",
    ];
    let critical_status_markers = [
        "http 401",
        "http 403",
        "http 404",
        "http 422",
        "http 429",
        "http 500",
        "http 502",
        "http 503",
        "http 504",
        "status=401",
        "status=403",
        "status=404",
        "status=422",
        "status=429",
        "status=500",
        "status=502",
        "status=503",
        "status=504",
    ];
    if critical_terms.iter().any(|needle| line.contains(needle)) {
        return "critical".to_string();
    }
    if critical_status_markers
        .iter()
        .any(|needle| line.contains(needle))
    {
        return "critical".to_string();
    }
    if patterns
        .iter()
        .any(|needle| !needle.is_empty() && line.contains(&needle.to_ascii_lowercase()))
    {
        return "critical".to_string();
    }
    let warn_terms = [
        "warn",
        "warning",
        "degraded",
        "retry",
        "quarantine",
        "dormant",
        "slow",
        "parked",
        "backoff",
        "retry",
        "retrying",
        "aborted",
        "transient",
        "throttle",
    ];
    if warn_terms.iter().any(|needle| line.contains(needle)) {
        return "warn".to_string();
    }
    "info".to_string()
}

fn should_emit_console_value(message: &str, method: &str, policy: &Value) -> bool {
    if policy.get("enabled").and_then(Value::as_bool) != Some(true) {
        return true;
    }
    let patterns = as_array(policy.pointer("/spine/critical_patterns"))
        .iter()
        .map(|row| as_str(Some(row)).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let severity = classify_severity_value(message, &patterns);
    if severity == "critical" {
        return true;
    }
    if method == "error" && severity == "warn" {
        return false;
    }
    false
}

fn update_status_value(
    root: &Path,
    policy: &Value,
    component: &str,
    patch: &Value,
) -> Result<Value, String> {
    let latest_path = resolve_path(
        root,
        &as_str(policy.pointer("/state/status_path")),
        DEFAULT_STATUS_REL,
    );
    let history_path = resolve_path(
        root,
        &as_str(policy.pointer("/state/history_path")),
        DEFAULT_HISTORY_REL,
    );
    let mut latest = read_json(&latest_path).unwrap_or_else(|| {
        json!({
            "ts": Value::Null,
            "active": policy.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            "components": {}
        })
    });
    if latest
        .get("components")
        .and_then(Value::as_object)
        .is_none()
    {
        latest["components"] = json!({});
    }
    let ts = now_iso();
    latest["ts"] = Value::String(ts.clone());
    latest["active"] = Value::Bool(
        policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    );
    latest["policy_path"] = Value::String(as_str(policy.get("_policy_path")));
    let components = latest
        .get_mut("components")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "mech_suit_mode_kernel_components_invalid".to_string())?;
    let merged = if let Some(existing) = components.get(component).and_then(Value::as_object) {
        let mut map = existing.clone();
        if let Some(patch_obj) = patch.as_object() {
            for (k, v) in patch_obj {
                map.insert(k.clone(), v.clone());
            }
        }
        Value::Object(map)
    } else {
        patch.clone()
    };
    components.insert(component.to_string(), merged);
    write_json(&latest_path, &latest)?;
    append_jsonl(
        &history_path,
        &json!({
            "ts": ts,
            "type": "mech_suit_status",
            "component": component,
            "active": latest.get("active").and_then(Value::as_bool).unwrap_or(true),
            "patch": patch
        }),
    )?;
    Ok(latest)
}

fn priority_for(policy: &Value, severity: &str) -> i64 {
    policy
        .pointer(&format!("/eyes/attention_contract/priority_map/{severity}"))
        .and_then(Value::as_i64)
        .unwrap_or(match severity {
            "critical" => 100,
            "warn" => 60,
            _ => 20,
        })
}

fn build_attention_event_value(event: &Value, policy: &Value) -> Option<Value> {
    let row = event.as_object()?;
    let event_type = text_token(row.get("type"), 80);
    let allowed = as_array(policy.pointer("/eyes/push_event_types"))
        .iter()
        .map(|row| as_str(Some(row)))
        .collect::<std::collections::BTreeSet<_>>();
    let explicit_source = text_token(row.get("source"), 80);
    let explicit_source_type = text_token(row.get("source_type"), 80);
    let allow_generic = !explicit_source.is_empty() || !explicit_source_type.is_empty();
    if !allowed.contains(&event_type) && !allow_generic {
        return None;
    }

    let eye_id = text_token(row.get("eye_id"), 80).if_empty_then("unknown_eye".to_string());
    let parser_type = text_token(row.get("parser_type"), 60);
    let focus_score = as_f64(row.get("focus_score"));
    let fallback = row
        .get("fallback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut severity = text_token(row.get("severity"), 24).to_ascii_lowercase();
    if severity.is_empty() {
        severity = "info".to_string();
    }
    let mut summary =
        text_token(row.get("summary"), 140).if_empty_then(format!("{event_type}:{eye_id}"));

    match event_type.as_str() {
        "external_item" => {
            let focus_mode = text_token(row.get("focus_mode"), 24);
            let threshold = as_f64(policy.pointer("/eyes/focus_warn_score")).unwrap_or(0.7);
            severity = if fallback {
                "info".to_string()
            } else if focus_score.unwrap_or(0.0) >= threshold || focus_mode == "focus" {
                "warn".to_string()
            } else {
                "info".to_string()
            };
            summary =
                text_token(row.get("title"), 140).if_empty_then(format!("{eye_id} external item"));
        }
        "eye_run_failed" => {
            let code = text_token(row.get("error_code"), 80).to_ascii_lowercase();
            let critical_codes = as_array(policy.pointer("/eyes/critical_error_codes"))
                .iter()
                .map(|row| as_str(Some(row)).to_ascii_lowercase())
                .collect::<std::collections::BTreeSet<_>>();
            severity = if critical_codes.contains(&code) {
                "critical".to_string()
            } else {
                "warn".to_string()
            };
            summary = text_token(row.get("error"), 140)
                .if_empty_then(format!("{eye_id} collector failed"));
        }
        "infra_outage_state" => {
            let active = row.get("active").and_then(Value::as_bool).unwrap_or(false);
            severity = if active {
                "critical".to_string()
            } else {
                "warn".to_string()
            };
            summary = if active {
                format!(
                    "eyes outage active ({} failed)",
                    row.get("failed_transport_eyes")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                )
            } else {
                "eyes outage recovered".to_string()
            };
        }
        "eye_health_quarantine_set" => {
            severity = "warn".to_string();
            summary = format!(
                "{eye_id} quarantined: {}",
                text_token(row.get("reason"), 120).if_empty_then("health_quarantine".to_string())
            );
        }
        "eye_auto_dormant" => {
            severity = "warn".to_string();
            summary = format!(
                "{eye_id} dormant: {}",
                text_token(row.get("reason"), 120).if_empty_then("auto_dormant".to_string())
            );
        }
        "collector_proposal_added" => {
            severity = "warn".to_string();
            summary = format!("{eye_id} remediation proposal added");
        }
        _ => {}
    }

    let ts = text_token(row.get("ts"), 40).if_empty_then(now_iso());
    let attention_key = text_token(row.get("attention_key"), 160).if_empty_then(format!(
        "{}:{}:{}",
        event_type,
        eye_id,
        text_token(
            row.get("item_hash")
                .or_else(|| row.get("error_code"))
                .or_else(|| row.get("reason"))
                .or_else(|| row.get("title")),
            120
        )
    ));
    let parser_type_value = if parser_type.is_empty() {
        Value::Null
    } else {
        Value::String(parser_type)
    };
    let focus_mode = text_token(row.get("focus_mode"), 24);
    let focus_mode_value = if focus_mode.is_empty() {
        Value::Null
    } else {
        Value::String(focus_mode)
    };
    let error_code = text_token(row.get("error_code"), 80);
    let error_code_value = if error_code.is_empty() {
        Value::Null
    } else {
        Value::String(error_code)
    };
    let mut payload = json!({
        "ts": ts,
        "type": "attention_event",
        "source": explicit_source.if_empty_then("external_eyes".to_string()),
        "source_type": explicit_source_type.if_empty_then(event_type.clone()),
        "eye_id": eye_id,
        "parser_type": parser_type_value,
        "severity": severity,
        "priority": priority_for(policy, &severity),
        "summary": summary,
        "focus_mode": focus_mode_value,
        "focus_score": focus_score.map(Value::from).unwrap_or(Value::Null),
        "error_code": error_code_value,
        "attention_key": attention_key,
        "raw_event": event
    });
    payload["receipt_hash"] = Value::String(hex_sha256(&payload));
    Some(payload)
}

fn hex_sha256(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_string(value).unwrap_or_default().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn append_attention_event_value(
    root: &Path,
    policy: &Value,
    event: &Value,
    run_context: &str,
) -> Result<Value, String> {
    if policy.get("enabled").and_then(Value::as_bool) != Some(true)
        || policy
            .pointer("/eyes/push_attention_queue")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Ok(json!({"ok": true, "queued": false, "reason": "disabled"}));
    }
    let Some(attention) = build_attention_event_value(event, policy) else {
        return Ok(json!({"ok": true, "queued": false, "reason": "event_not_tracked"}));
    };

    let queue_path = resolve_path(
        root,
        &as_str(policy.pointer("/eyes/attention_queue_path")),
        DEFAULT_ATTENTION_QUEUE_REL,
    );
    let receipts_path = resolve_path(
        root,
        &as_str(policy.pointer("/eyes/receipts_path")),
        DEFAULT_ATTENTION_RECEIPTS_REL,
    );
    let latest_path = resolve_path(
        root,
        &as_str(policy.pointer("/eyes/latest_path")),
        DEFAULT_ATTENTION_LATEST_REL,
    );
    append_jsonl(&queue_path, &attention)?;
    append_jsonl(
        &receipts_path,
        &json!({
            "ts": attention.get("ts").cloned().unwrap_or(Value::String(now_iso())),
            "type": "attention_receipt",
            "queued": true,
            "severity": attention.get("severity").cloned().unwrap_or(Value::String("info".to_string())),
            "eye_id": attention.get("eye_id").cloned().unwrap_or(Value::String("unknown_eye".to_string())),
            "source_type": attention.get("source_type").cloned().unwrap_or(Value::String("unknown".to_string())),
            "receipt_hash": attention.get("receipt_hash").cloned().unwrap_or(Value::Null)
        }),
    )?;
    let mut latest = read_json(&latest_path).unwrap_or_else(|| json!({"queued_total": 0}));
    latest["ts"] = attention
        .get("ts")
        .cloned()
        .unwrap_or(Value::String(now_iso()));
    latest["active"] = Value::Bool(true);
    latest["queued_total"] = Value::from(
        latest
            .get("queued_total")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            + 1,
    );
    latest["last_event"] = json!({
        "eye_id": attention.get("eye_id").cloned().unwrap_or(Value::Null),
        "source_type": attention.get("source_type").cloned().unwrap_or(Value::Null),
        "severity": attention.get("severity").cloned().unwrap_or(Value::Null),
        "summary": attention.get("summary").cloned().unwrap_or(Value::Null),
    });
    write_json(&latest_path, &latest)?;
    let status = update_status_value(
        root,
        policy,
        "eyes",
        &json!({
            "ambient": true,
            "push_attention_queue": true,
            "quiet_non_critical": policy.pointer("/eyes/quiet_non_critical").and_then(Value::as_bool).unwrap_or(true),
            "last_attention_ts": attention.get("ts").cloned().unwrap_or(Value::Null),
            "last_attention_summary": attention.get("summary").cloned().unwrap_or(Value::Null),
            "attention_queue_path": as_str(policy.pointer("/eyes/attention_queue_path")),
            "attention_receipts_path": as_str(policy.pointer("/eyes/receipts_path")),
            "attention_last_decision": "admitted",
            "attention_routed_via": "rust_kernel",
            "run_context": run_context
        }),
    )?;
    Ok(json!({
        "ok": true,
        "queued": true,
        "event": attention,
        "decision": "admitted",
        "routed_via": "rust_kernel",
        "status": status
    }))
}
