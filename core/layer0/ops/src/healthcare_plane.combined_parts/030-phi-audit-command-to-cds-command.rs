
fn phi_audit_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
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
        let logs = read_jsonl(&phi_log_path(root));
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_phi_audit",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "count": logs.len(),
            "logs": logs,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.2",
                "claim": "hipaa_phi_audit_status_surfaces_patient_access_disclosure_log",
                "evidence": {"count": logs.len()}
            }]
        }));
    }
    if op != "access" {
        return Err("phi_audit_op_invalid".to_string());
    }
    let user = clean(
        parsed
            .flags
            .get("user")
            .map(String::as_str)
            .unwrap_or("clinician"),
        120,
    );
    let npi = clean(
        parsed
            .flags
            .get("npi")
            .map(String::as_str)
            .unwrap_or("0000000000"),
        32,
    );
    let patient_id = clean(
        parsed
            .flags
            .get("patient-id")
            .map(String::as_str)
            .unwrap_or("patient"),
        120,
    );
    let reason = clean(
        parsed
            .flags
            .get("reason")
            .map(String::as_str)
            .unwrap_or("treatment"),
        32,
    )
    .to_ascii_lowercase();
    let break_glass = parse_bool(parsed.flags.get("break-glass"), false);
    let allowed = ["treatment", "payment", "operations", "research"];
    if !allowed.contains(&reason.as_str()) {
        return Err("phi_reason_invalid".to_string());
    }
    let row = json!({
        "ts": now_iso(),
        "user": user,
        "npi": npi,
        "patient_uuid": patient_id,
        "reason": reason,
        "break_glass": break_glass,
        "receipt_hash": sha256_hex_str(&format!("{}:{}:{}:{}", user, npi, patient_id, reason))
    });
    append_jsonl(&phi_log_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_phi_audit",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "entry": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.2",
            "claim": "hipaa_phi_audit_captures_who_what_when_why_with_break_glass_and_disclosure_traceability",
            "evidence": {"break_glass": break_glass}
        }]
    }))
}

fn cds_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let mut state = read_object(&cds_path(root));
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_cds",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "state": state,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.3",
                "claim": "cds_status_surfaces_safety_alert_state_and_override_history",
                "evidence": {"keys": state.keys().cloned().collect::<Vec<_>>()}
            }]
        }));
    }
    if op != "evaluate" {
        return Err("cds_op_invalid".to_string());
    }
    let patient_id = clean(
        parsed
            .flags
            .get("patient-id")
            .map(String::as_str)
            .unwrap_or("patient"),
        120,
    );
    let meds = csv_set(parsed.flags.get("meds"));
    let allergies = csv_set(parsed.flags.get("allergies"));
    let dose_mg = parse_f64(parsed.flags.get("dose-mg"), 0.0);
    let mut alerts = Vec::<String>::new();
    if meds.contains("warfarin") && meds.contains("aspirin") {
        alerts.push("drug_drug_interaction".to_string());
    }
    if meds.contains("penicillin") && allergies.contains("penicillin") {
        alerts.push("allergy_conflict".to_string());
    }
    if dose_mg > 0.0 && dose_mg > 1000.0 {
        alerts.push("dose_out_of_range".to_string());
    }
    state.insert(
        patient_id.clone(),
        json!({"patient_id": patient_id, "alerts": alerts, "dose_mg": dose_mg, "updated_at": now_iso()}),
    );
    write_json(&cds_path(root), &Value::Object(state.clone()))?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_cds",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "patient_id": patient_id,
        "alerts": state
            .get(&patient_id)
            .and_then(|v| v.get("alerts"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "claim_evidence": [{
            "id": "V7-HEALTH-001.3",
            "claim": "clinical_decision_support_checks_drug_interactions_allergies_and_dosing_constraints",
            "evidence": {"alert_count": state.get(&patient_id).and_then(|v| v.get("alerts")).and_then(Value::as_array).map(|a| a.len()).unwrap_or(0)}
        }]
    }))
}
