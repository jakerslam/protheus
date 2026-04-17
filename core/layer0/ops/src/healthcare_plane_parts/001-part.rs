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

fn coordination_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
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
        let rows = read_jsonl(&coordination_path(root));
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_coordination",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "rows": rows,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.7",
                "claim": "care_coordination_status_surfaces_handoff_and_reconciliation_history",
                "evidence": {"count": read_jsonl(&coordination_path(root)).len()}
            }]
        }));
    }
    if op != "handoff" && op != "reconcile" {
        return Err("coordination_op_invalid".to_string());
    }
    let sbar = parse_json_or_empty(parsed.flags.get("sbar-json"));
    let meds = parse_json_or_empty(parsed.flags.get("meds-json"));
    let row = json!({
        "ts": now_iso(),
        "op": op,
        "sbar": sbar,
        "meds": meds,
        "reconciliation_hash": sha256_hex_str(&format!("{}:{}", canonical_json_string(&sbar), canonical_json_string(&meds)))
    });
    append_jsonl(&coordination_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_coordination",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "entry": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.7",
            "claim": "care_coordination_tracks_sbar_handoff_and_medication_reconciliation_with_transition_receipts",
            "evidence": {"op": op}
        }]
    }))
}

fn trials_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let mut state = read_object(&trials_path(root));
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_trials",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "trials": state,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.8",
                "claim": "trial_status_surfaces_screening_consent_and_sae_tracking_state",
                "evidence": {"trial_count": state.len()}
            }]
        }));
    }
    let patient_id = clean(
        parsed
            .flags
            .get("patient-id")
            .map(String::as_str)
            .unwrap_or("patient"),
        120,
    );
    let trial = clean(
        parsed
            .flags
            .get("trial")
            .map(String::as_str)
            .unwrap_or("trial"),
        120,
    );
    let key = format!("{trial}:{patient_id}");
    let mut row = state.get(&key).cloned().unwrap_or_else(|| {
        json!({"trial": trial, "patient_id": patient_id, "screened": false, "consented": false, "sae_count": 0_u64})
    });
    if op == "screen" {
        row["screened"] = Value::Bool(true);
        row["screened_at"] = Value::String(now_iso());
    } else if op == "consent" {
        row["consented"] = Value::Bool(true);
        row["consent_at"] = Value::String(now_iso());
    } else if op == "report-sae" || op == "report_sae" {
        let next = row.get("sae_count").and_then(Value::as_u64).unwrap_or(0) + 1;
        row["sae_count"] = Value::from(next);
        row["last_sae_at"] = Value::String(now_iso());
    } else {
        return Err("trials_op_invalid".to_string());
    }
    state.insert(key, row.clone());
    write_json(&trials_path(root), &Value::Object(state.clone()))?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_trials",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "record": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.8",
            "claim": "trial_engine_tracks_eligibility_consent_and_adverse_event_reporting_lifecycle",
            "evidence": {"op": op}
        }]
    }))
}

fn imaging_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        20,
    )
    .to_ascii_lowercase();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_imaging",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "rows": read_jsonl(&imaging_path(root)),
            "claim_evidence": [{
                "id": "V7-HEALTH-001.9",
                "claim": "imaging_status_surfaces_dicom_ingest_and_critical_routing_history",
                "evidence": {"count": read_jsonl(&imaging_path(root)).len()}
            }]
        }));
    }
    if op != "ingest" && op != "critical-route" && op != "critical_route" {
        return Err("imaging_op_invalid".to_string());
    }
    let study_id = clean(
        parsed
            .flags
            .get("study-id")
            .map(String::as_str)
            .unwrap_or("study"),
        120,
    );
    let finding = clean(
        parsed
            .flags
            .get("finding")
            .map(String::as_str)
            .unwrap_or("none"),
        240,
    );
    let row = json!({
        "ts": now_iso(),
        "op": op,
        "study_id": study_id,
        "finding": finding,
        "critical": op != "ingest"
    });
    append_jsonl(&imaging_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_imaging",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "entry": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.9",
            "claim": "imaging_lane_tracks_dicom_study_ingest_and_critical_finding_provider_routing",
            "evidence": {"critical": op != "ingest"}
        }]
    }))
}
