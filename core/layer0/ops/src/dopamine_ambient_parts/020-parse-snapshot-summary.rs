
fn parse_snapshot_summary(snapshot: &Value) -> Value {
    snapshot
        .get("summary")
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn parse_string_array(value: Option<&Value>, max_items: usize, max_len: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), max_len))
                .filter(|row| !row.is_empty())
                .take(max_items)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_optional_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| {
        if let Some(n) = v.as_f64() {
            return Some(n);
        }
        if let Some(n) = v.as_i64() {
            return Some(n as f64);
        }
        if let Some(n) = v.as_u64() {
            return Some(n as f64);
        }
        None
    })
}

fn load_cached_status_from_latest(
    latest: &Value,
) -> Option<(Value, String, bool, Vec<String>, f64)> {
    if !latest.is_object() {
        return None;
    }

    let summary = latest
        .get("summary")
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let (inferred_severity, inferred_breached, inferred_reasons) = classify_threshold(&summary);
    let cached_severity =
        clean_text(latest.get("severity").and_then(Value::as_str), 20).to_ascii_lowercase();
    let severity = if matches!(cached_severity.as_str(), "critical" | "warn" | "info") {
        cached_severity
    } else {
        inferred_severity
    };
    let breached = latest
        .get("threshold_breached")
        .and_then(Value::as_bool)
        .unwrap_or(inferred_breached);
    let reasons = {
        let parsed = parse_string_array(latest.get("breach_reasons"), 12, 80);
        if parsed.is_empty() {
            inferred_reasons
        } else {
            parsed
        }
    };
    let sds =
        parse_optional_number(latest.get("sds")).unwrap_or_else(|| summary_number(&summary, "sds"));

    Some((summary, severity, breached, reasons, sds))
}

fn run_snapshot(
    root: &Path,
    policy: &DopamineAmbientPolicy,
    mode: &str,
    date: &str,
) -> Result<Value, String> {
    let node = std::env::var("PROTHEUS_NODE_BINARY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "node".to_string());
    let output = Command::new(node)
        .arg(policy.runtime_script.to_string_lossy().to_string())
        .arg(mode)
        .arg(format!("--date={date}"))
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("dopamine_runtime_spawn_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!(
            "dopamine_runtime_failed:{}:{}",
            output.status.code().unwrap_or(1),
            clean_text(Some(&stderr), 180)
        ));
    }
    parse_json_payload(&stdout).ok_or_else(|| {
        format!(
            "dopamine_runtime_invalid_json:{}",
            clean_text(Some(&stdout), 180)
        )
    })
}

fn resolve_protheus_ops_command(root: &PathBuf, domain: &str) -> (String, Vec<String>) {
    crate::contract_lane_utils::resolve_protheus_ops_command(root.as_path(), domain)
}

fn enqueue_attention(
    root: &Path,
    summary: &Value,
    severity: &str,
    breach_reasons: &[String],
    date: &str,
    run_context: &str,
) -> Result<Value, String> {
    let sds = summary_number(summary, "sds");
    let summary_line = format!(
        "dopamine threshold breach ({severity}) sds={:.2} reasons={}",
        sds,
        if breach_reasons.is_empty() {
            "none".to_string()
        } else {
            breach_reasons.join(",")
        }
    );
    let event = json!({
        "ts": now_iso(),
        "source": "dopamine_ambient",
        "source_type": "dopamine_threshold_breach",
        "severity": severity,
        "summary": summary_line,
        "attention_key": format!("dopamine:{date}:{severity}:{:.0}", sds * 100.0),
        "breach_reasons": breach_reasons,
        "sds": sds,
        "date": date
    });
    let payload = serde_json::to_string(&event)
        .map_err(|err| format!("attention_event_encode_failed:{err}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
    let root_buf = root.to_path_buf();
    let (command, mut args) = resolve_protheus_ops_command(&root_buf, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push(format!("--run-context={run_context}"));

    let output = Command::new(command)
        .args(args)
        .current_dir(root)
        .env(
            "PROTHEUS_NODE_BINARY",
            std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("attention_queue_spawn_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let receipt = parse_json_payload(&stdout).unwrap_or_else(|| json!({}));
    let mut out = json!({
        "ok": output.status.success(),
        "routed_via": "rust_attention_queue",
        "exit_code": output.status.code().unwrap_or(1),
        "decision": receipt.get("decision").and_then(Value::as_str).unwrap_or("unknown"),
        "queued": receipt.get("queued").and_then(Value::as_bool).unwrap_or(false),
        "receipt": receipt
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));

    if output.status.success() {
        Ok(out)
    } else {
        Err(format!(
            "attention_queue_failed:{}:{}",
            output.status.code().unwrap_or(1),
            clean_text(Some(&stderr), 180)
        ))
    }
}

fn update_mech_suit_status(policy: &DopamineAmbientPolicy, patch: Value) {
    let mut latest = read_json(&policy.status_path).unwrap_or_else(|| {
        json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        })
    });
    if !latest.is_object() {
        latest = json!({
            "ts": Value::Null,
            "active": policy.enabled,
            "components": {}
        });
    }
    latest["ts"] = Value::String(now_iso());
    latest["active"] = Value::Bool(policy.enabled);
    if !latest
        .get("components")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        latest["components"] = json!({});
    }
    latest["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    latest["components"]["dopamine"] = patch.clone();
    write_json(&policy.status_path, &latest);

    append_jsonl(
        &policy.history_path,
        &json!({
            "ts": now_iso(),
            "type": "mech_suit_status",
            "component": "dopamine",
            "active": policy.enabled,
            "patch": patch
        }),
    );
}

fn should_surface(
    policy: &DopamineAmbientPolicy,
    command: &str,
    severity: &str,
    breached: bool,
) -> bool {
    if command == "status" {
        return false;
    }
    if !policy
        .surface_levels
        .iter()
        .any(|level| level.as_str() == severity)
    {
        return false;
    }
    if policy.threshold_breach_only && !breached {
        return false;
    }
    true
}
