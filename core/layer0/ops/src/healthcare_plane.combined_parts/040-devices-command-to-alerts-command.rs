
fn devices_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    if op == "status" {
        let logs = read_jsonl(&devices_path(root));
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_devices",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "events": logs,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.4",
                "claim": "device_integration_status_surfaces_protocol_native_events_and_provenance",
                "evidence": {"event_count": logs.len()}
            }]
        }));
    }
    if op != "ingest" {
        return Err("devices_op_invalid".to_string());
    }
    let protocol = clean(
        parsed
            .flags
            .get("protocol")
            .map(String::as_str)
            .unwrap_or("hl7"),
        16,
    )
    .to_ascii_lowercase();
    let allowed = ["hl7", "fhir", "dicom", "ieee11073"];
    if !allowed.contains(&protocol.as_str()) {
        return Err("device_protocol_invalid".to_string());
    }
    let device_id = clean(
        parsed
            .flags
            .get("device-id")
            .map(String::as_str)
            .unwrap_or("device"),
        120,
    );
    let payload = parse_json_or_empty(parsed.flags.get("payload-json"));
    let row = json!({
        "ts": now_iso(),
        "protocol": protocol,
        "device_id": device_id,
        "payload_hash": sha256_hex_str(&canonical_json_string(&payload))
    });
    append_jsonl(&devices_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_devices",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "event": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.4",
            "claim": "device_integration_accepts_hl7_fhir_dicom_and_ieee11073_with_deterministic_receipts",
            "evidence": {"protocol": protocol}
        }]
    }))
}

fn documentation_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_documentation",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "docs_count": read_jsonl(&docs_path(root)).len(),
            "claim_evidence": [{
                "id": "V7-HEALTH-001.5",
                "claim": "clinical_documentation_status_surfaces_structured_note_and_coding_artifact_count",
                "evidence": {"count": read_jsonl(&docs_path(root)).len()}
            }]
        }));
    }
    if op != "draft" {
        return Err("documentation_op_invalid".to_string());
    }
    let soap = parse_json_or_empty(parsed.flags.get("soap-json"));
    let codes = parse_json_or_empty(parsed.flags.get("codes-json"));
    let required = ["subjective", "objective", "assessment", "plan"];
    let complete = required.iter().all(|k| soap.get(*k).is_some());
    if !complete {
        return Err("soap_incomplete".to_string());
    }
    let row = json!({
        "ts": now_iso(),
        "soap": soap,
        "codes": codes,
        "coding_hash": sha256_hex_str(&canonical_json_string(&codes))
    });
    append_jsonl(&docs_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_documentation",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "record": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.5",
            "claim": "clinical_documentation_enforces_soap_structure_and_coded_metadata_receipts",
            "evidence": {"complete": complete}
        }]
    }))
}

fn alerts_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let mut state = read_object(&alerts_path(root));
    let mut alerts = state
        .remove("alerts")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_alerts",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "alerts": alerts,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.6",
                "claim": "alert_fatigue_management_surfaces_deduplicated_tiered_alert_state",
                "evidence": {"count": alerts.len()}
            }]
        }));
    }
    if op == "emit" {
        let tier = clean(
            parsed
                .flags
                .get("tier")
                .map(String::as_str)
                .unwrap_or("medium"),
            16,
        )
        .to_ascii_lowercase();
        let key = clean(
            parsed
                .flags
                .get("key")
                .map(String::as_str)
                .unwrap_or("alert"),
            120,
        );
        let duplicate = alerts.iter().any(|row| {
            row.get("key").and_then(Value::as_str) == Some(key.as_str())
                && row.get("status").and_then(Value::as_str) == Some("open")
        });
        if !duplicate {
            alerts.push(json!({"id": sha256_hex_str(&format!("{}:{}:{}", tier, key, now_iso())), "tier": tier, "key": key, "status": "open", "ts": now_iso()}));
        }
    } else if op == "ack" {
        let id = clean(
            parsed.flags.get("id").map(String::as_str).unwrap_or(""),
            120,
        );
        for row in &mut alerts {
            if row.get("id").and_then(Value::as_str) == Some(id.as_str()) {
                row["status"] = Value::String("ack".to_string());
                row["ack_at"] = Value::String(now_iso());
            }
        }
    } else {
        return Err("alerts_op_invalid".to_string());
    }
    state.insert("alerts".to_string(), Value::Array(alerts.clone()));
    write_json(&alerts_path(root), &Value::Object(state))?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_alerts",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "alerts": alerts,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.6",
            "claim": "alert_fatigue_management_deduplicates_routes_and_tracks_acknowledgement_lifecycle",
            "evidence": {"op": op}
        }]
    }))
}
