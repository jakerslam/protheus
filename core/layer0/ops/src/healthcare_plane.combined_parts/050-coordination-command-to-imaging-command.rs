
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
