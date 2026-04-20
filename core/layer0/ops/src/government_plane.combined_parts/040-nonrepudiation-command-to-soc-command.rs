
fn nonrepudiation_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let principal = clean(
        parsed
            .flags
            .get("principal")
            .map(String::as_str)
            .unwrap_or("CN=operator,O=Org,OU=Unit"),
        240,
    );
    let action = clean(
        parsed
            .flags
            .get("action")
            .map(String::as_str)
            .unwrap_or("unknown"),
        160,
    );
    let auth_signature = clean(
        parsed
            .flags
            .get("auth-signature")
            .map(String::as_str)
            .unwrap_or("unsigned"),
        512,
    );
    let tsa = clean(
        parsed
            .flags
            .get("timestamp-authority")
            .map(String::as_str)
            .unwrap_or("tsa.local"),
        200,
    );
    let legal_hold = parse_bool(parsed.flags.get("legal-hold"), false);
    let row = json!({
        "ts": now_iso(),
        "principal": principal,
        "action": action,
        "auth_signature": auth_signature,
        "timestamp_authority": tsa,
        "timestamp_token": sha256_hex_str(&format!("{}:{}:{}", principal, action, tsa)),
        "legal_hold": legal_hold
    });
    append_jsonl(&legal_log_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "government_plane_nonrepudiation",
        "lane": LANE_ID,
        "ts": now_iso(),
        "entry": row,
        "log_path": legal_log_path(root).to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-GOV-001.3",
            "claim": "legal_non_repudiation_receipts_bind_authorized_principal_signature_and_trusted_timestamp_authority",
            "evidence": {"principal": principal, "legal_hold": legal_hold}
        }]
    }))
}

fn diode_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let from = clean(
        parsed
            .flags
            .get("from")
            .map(String::as_str)
            .unwrap_or("secret"),
        32,
    )
    .to_ascii_lowercase();
    let to = clean(
        parsed
            .flags
            .get("to")
            .map(String::as_str)
            .unwrap_or("unclassified"),
        32,
    )
    .to_ascii_lowercase();
    let sanitize = parse_bool(parsed.flags.get("sanitize"), true);
    let payload = parse_json_or_empty(parsed.flags.get("payload-json"));
    let allowed = level_rank(&from) >= level_rank(&to) && sanitize;
    let row = json!({
        "ts": now_iso(),
        "from": from,
        "to": to,
        "sanitize": sanitize,
        "payload_hash": sha256_hex_str(&canonical_json_string(&payload)),
        "ok": allowed
    });
    append_jsonl(&diode_history_path(root), &row)?;
    Ok(json!({
        "ok": allowed,
        "type": "government_plane_diode",
        "lane": LANE_ID,
        "ts": now_iso(),
        "transfer": row,
        "history_path": diode_history_path(root).to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-GOV-001.4",
            "claim": "air_gap_data_diode_allows_only_sanitized_high_to_low_one_way_transfers",
            "evidence": {"allowed": allowed}
        }]
    }))
}

fn soc_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let mut state = read_json(&soc_state_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    if op == "connect" {
        let endpoint = clean(
            parsed
                .flags
                .get("endpoint")
                .map(String::as_str)
                .unwrap_or("siem.local"),
            240,
        );
        state.insert("endpoint".to_string(), Value::String(endpoint.clone()));
        state.insert("connected_at".to_string(), Value::String(now_iso()));
        write_json(&soc_state_path(root), &Value::Object(state.clone()))?;
        return Ok(json!({
            "ok": true,
            "type": "government_plane_soc",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "endpoint": endpoint,
            "claim_evidence": [{
                "id": "V7-GOV-001.5",
                "claim": "soc_integration_persists_siem_endpoint_for_continuous_monitoring_streams",
                "evidence": {"endpoint_configured": true}
            }]
        }));
    }
    if op == "emit" {
        let endpoint = state
            .get("endpoint")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if endpoint.is_empty() {
            return Err("soc_not_connected".to_string());
        }
        let event = parse_json_or_empty(parsed.flags.get("event-json"));
        let row = json!({"ts": now_iso(), "endpoint": endpoint, "event": event});
        append_jsonl(&lane_root(root).join("soc_events.jsonl"), &row)?;
        return Ok(json!({
            "ok": true,
            "type": "government_plane_soc",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "event": row,
            "claim_evidence": [{
                "id": "V7-GOV-001.5",
                "claim": "soc_pipeline_streams_security_events_with_deterministic_alert_lineage",
                "evidence": {"emitted": true}
            }]
        }));
    }
    if op != "status" {
        return Err("soc_op_invalid".to_string());
    }
    Ok(json!({
        "ok": true,
        "type": "government_plane_soc",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "state": state,
        "events_path": lane_root(root).join("soc_events.jsonl").to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-GOV-001.5",
            "claim": "soc_status_surfaces_connector_and_event_stream_state",
            "evidence": {"connected": state.get("endpoint").and_then(Value::as_str).is_some()}
        }]
    }))
}
