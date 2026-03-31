fn run_audit_logs_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let max_events = parse_u64(parse_flag(argv, "max-events"), 500) as usize;
    let max_failures = parse_u64(parse_flag(argv, "max-failures"), 0);
    let security_events = read_jsonl(&security_history_path(root));
    let capability_events = read_jsonl(&capability_event_path(root));
    let blast_events = read_jsonl(&blast_radius_events_path(root));
    let secret_events = read_jsonl(&secrets_events_path(root));
    let remediation_events = read_jsonl(&remediation_gate_path(root));

    let mut failed = 0u64;
    let mut blocked = 0u64;
    let mut by_type = BTreeMap::<String, u64>::new();
    for row in security_events
        .iter()
        .rev()
        .take(max_events)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let ty = row
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *by_type.entry(ty).or_insert(0) += 1;
        if row.get("ok").and_then(Value::as_bool) == Some(false) {
            failed += 1;
        }
        if row.get("blocked").and_then(Value::as_bool) == Some(true) {
            blocked += 1;
        }
        if row
            .get("event")
            .and_then(|v| v.get("blocked"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            blocked += 1;
        }
    }

    let audit_blocked = failed > max_failures;
    let summary = json!({
        "ts": now_iso(),
        "max_events": max_events,
        "max_failures": max_failures,
        "security_events_considered": security_events.len().min(max_events),
        "failed_events": failed,
        "blocked_events": blocked,
        "capability_events": capability_events.len(),
        "blast_events": blast_events.len(),
        "secret_events": secret_events.len(),
        "remediation_events": remediation_events.len(),
        "events_by_type": by_type,
        "audit_blocked": audit_blocked
    });
    append_jsonl(&audit_history_path(root), &summary);
    write_json(&audit_latest_path(root), &summary);

    let out = json!({
        "ok": !audit_blocked,
        "type": "security_plane_audit_logs",
        "lane": "core/layer1/security",
        "mode": "audit-logs",
        "strict": strict,
        "summary": summary,
        "claim_evidence": [{
            "id": "V6-SEC-014",
            "claim": "security_audit_log_analysis_tracks_failed_and_blocked_events_with_fail_closed_thresholds",
            "evidence": {
                "failed_events": failed,
                "blocked_events": blocked,
                "max_failures": max_failures,
                "audit_blocked": audit_blocked
            }
        }]
    });
    (out, if strict && audit_blocked { 2 } else { 0 })
}
fn run_threat_model_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let scenario = clean(
        parse_flag(argv, "scenario").unwrap_or_else(|| "unspecified".to_string()),
        200,
    );
    let surface = clean(
        parse_flag(argv, "surface").unwrap_or_else(|| "control-plane".to_string()),
        120,
    );
    let vector = clean(parse_flag(argv, "vector").unwrap_or_default(), 200);
    let model = clean(
        parse_flag(argv, "model").unwrap_or_else(|| "security-default-v1".to_string()),
        80,
    );
    let threshold = parse_u64(parse_flag(argv, "block-threshold"), 70);
    let allow = parse_bool(parse_flag(argv, "allow"), false);

    let signal = format!(
        "{} {} {}",
        scenario.to_ascii_lowercase(),
        surface.to_ascii_lowercase(),
        vector.to_ascii_lowercase()
    );
    let mut score = 10u64;
    if signal.contains("exfil")
        || signal.contains("secret")
        || signal.contains("credential")
        || signal.contains("token")
    {
        score = score.saturating_add(55);
    }
    if signal.contains("rce")
        || signal.contains("shell")
        || signal.contains("exec")
        || signal.contains("privilege")
    {
        score = score.saturating_add(45);
    }
    if signal.contains("prompt")
        || signal.contains("injection")
        || signal.contains("poison")
        || signal.contains("jailbreak")
    {
        score = score.saturating_add(40);
    }
    if signal.contains("lateral")
        || signal.contains("persistence")
        || signal.contains("supply-chain")
        || signal.contains("supply chain")
    {
        score = score.saturating_add(35);
    }
    score = score.min(100);

    let severity = if score >= 80 {
        "critical"
    } else if score >= 60 {
        "high"
    } else if score >= 35 {
        "medium"
    } else {
        "low"
    };
    let recommendations = if score >= 80 {
        vec![
            "quarantine_execution_path",
            "require_human_approval",
            "enable_blast_radius_lockdown",
        ]
    } else if score >= 60 {
        vec![
            "tighten_allowlists",
            "enable_continuous_scan",
            "raise_audit_sampling",
        ]
    } else if score >= 35 {
        vec!["monitor_with_alerting", "add_regression_case"]
    } else {
        vec!["baseline_monitoring"]
    };
    let blocked = !allow && score >= threshold;

    let event = json!({
        "ts": now_iso(),
        "scenario": scenario,
        "surface": surface,
        "vector": vector,
        "model": model,
        "risk_score": score,
        "severity": severity,
        "block_threshold": threshold,
        "blocked": blocked,
        "recommendations": recommendations
    });
    append_jsonl(&threat_history_path(root), &event);
    write_json(&threat_latest_path(root), &event);

    let out = json!({
        "ok": !blocked,
        "type": "security_plane_threat_model",
        "lane": "core/layer1/security",
        "mode": "threat-model",
        "strict": strict,
        "event": event,
        "claim_evidence": [{
            "id": "V6-SEC-015",
            "claim": "threat_modeling_lane_classifies_attack_vectors_and_fail_closes_high_risk_scenarios",
            "evidence": {
                "risk_score": score,
                "severity": severity,
                "block_threshold": threshold,
                "blocked": blocked
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

#[derive(Debug, Clone)]
struct SecretHandleRow {
    provider: String,
    secret_path: String,
    scope: String,
    lease_expires_at: String,
    revoked: bool,
    revoked_at: Option<String>,
    rotated_at: Option<String>,
    secret_sha256: String,
}

fn read_secret_state(root: &Path) -> BTreeMap<String, SecretHandleRow> {
    let Some(value) = read_json(&secrets_state_path(root)) else {
        return BTreeMap::new();
    };
    value
        .get("handles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, row)| {
            let obj = row.as_object()?;
            Some((
                k,
                SecretHandleRow {
                    provider: obj
                        .get("provider")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    secret_path: obj
                        .get("secret_path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    scope: obj
                        .get("scope")
                        .and_then(Value::as_str)
                        .unwrap_or("default")
                        .to_string(),
                    lease_expires_at: obj
                        .get("lease_expires_at")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    revoked: obj.get("revoked").and_then(Value::as_bool).unwrap_or(false),
                    revoked_at: obj
                        .get("revoked_at")
                        .and_then(Value::as_str)
                        .map(|v| v.to_string()),
                    rotated_at: obj
                        .get("rotated_at")
                        .and_then(Value::as_str)
                        .map(|v| v.to_string()),
                    secret_sha256: obj
                        .get("secret_sha256")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                },
            ))
        })
        .collect::<BTreeMap<_, _>>()
}

fn write_secret_state(root: &Path, handles: &BTreeMap<String, SecretHandleRow>) {
    let payload = json!({
        "updated_at": now_iso(),
        "handles": handles.iter().map(|(id, row)| {
            (id.clone(), json!({
                "provider": row.provider,
                "secret_path": row.secret_path,
                "scope": row.scope,
                "lease_expires_at": row.lease_expires_at,
                "revoked": row.revoked,
                "revoked_at": row.revoked_at,
                "rotated_at": row.rotated_at,
                "secret_sha256": row.secret_sha256
            }))
        }).collect::<serde_json::Map<String, Value>>()
    });
    write_json(&secrets_state_path(root), &payload);
}

fn secret_env_var_name(provider: &str, secret_path: &str) -> String {
    let provider = provider
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .to_ascii_uppercase();
    let path = secret_path
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .to_ascii_uppercase();
    format!("PROTHEUS_SECRET_{}_{}", provider, path)
}
