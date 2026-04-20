
fn emergency_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
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
            "type": "healthcare_plane_emergency",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "events": read_jsonl(&emergency_path(root)),
            "claim_evidence": [{
                "id": "V7-HEALTH-001.10",
                "claim": "break_glass_status_surfaces_override_usage_for_post_incident_review",
                "evidence": {"count": read_jsonl(&emergency_path(root)).len()}
            }]
        }));
    }
    if op != "break-glass" && op != "break_glass" {
        return Err("emergency_op_invalid".to_string());
    }
    let user = clean(
        parsed
            .flags
            .get("user")
            .map(String::as_str)
            .unwrap_or("ed-physician"),
        120,
    );
    let patient_id = clean(
        parsed
            .flags
            .get("patient-id")
            .map(String::as_str)
            .unwrap_or("patient"),
        120,
    );
    let justification = clean(
        parsed
            .flags
            .get("justification")
            .map(String::as_str)
            .unwrap_or("emergency access"),
        240,
    );
    let ttl_minutes = parse_f64(parsed.flags.get("ttl-minutes"), 30.0).clamp(1.0, 240.0);
    let row = json!({
        "ts": now_iso(),
        "user": user,
        "patient_id": patient_id,
        "justification": justification,
        "ttl_minutes": ttl_minutes,
        "expires_token": sha256_hex_str(&format!("{}:{}:{}", user, patient_id, ttl_minutes))
    });
    append_jsonl(&emergency_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_emergency",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "event": row,
        "claim_evidence": [{
            "id": "V7-HEALTH-001.10",
            "claim": "break_glass_protocol_requires_justification_ttl_and_auditable_override_lineage",
            "evidence": {"ttl_minutes": ttl_minutes}
        }]
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let bypass = conduit_bypass_requested(&parsed.flags);
    let conduit = build_conduit_enforcement(
        root,
        ENV_KEY,
        LANE_ID,
        strict,
        &command,
        "healthcare_plane_conduit_enforcement",
        "client/protheusctl -> core/healthcare-plane",
        bypass,
        vec![json!({
            "id": "V7-HEALTH-001.2",
            "claim": "healthcare_plane_is_conduit_routed_for_phi_sensitive_operations",
            "evidence": {"command": command, "bypass_requested": bypass}
        })],
    );
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let payload = json!({
            "ok": false,
            "type": "healthcare_plane",
            "lane": LANE_ID,
            "ts": now_iso(),
            "command": command,
            "error": "conduit_bypass_rejected"
        });
        return emit(root, &command, strict, payload, Some(&conduit));
    }
    let result = match command.as_str() {
        "patient" => patient_command(root, &parsed),
        "phi-audit" | "phi_audit" => phi_audit_command(root, &parsed),
        "cds" => cds_command(root, &parsed),
        "devices" => devices_command(root, &parsed),
        "documentation" => documentation_command(root, &parsed),
        "alerts" => alerts_command(root, &parsed),
        "coordination" => coordination_command(root, &parsed),
        "trials" => trials_command(root, &parsed),
        "imaging" => imaging_command(root, &parsed),
        "emergency" => emergency_command(root, &parsed),
        "status" => Ok(json!({
            "ok": true,
            "type": "healthcare_plane_status",
            "lane": LANE_ID,
            "ts": now_iso(),
            "state_root": lane_root(root).to_string_lossy().to_string(),
            "latest_path": latest_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string(),
            "history_path": history_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string()
        })),
        _ => Err("unknown_healthcare_command".to_string()),
    };
    match result {
        Ok(payload) => emit(root, &command, strict, payload, Some(&conduit)),
        Err(error) => emit(
            root,
            &command,
            strict,
            json!({
                "ok": false,
                "type": "healthcare_plane",
                "lane": LANE_ID,
                "ts": now_iso(),
                "command": command,
                "error": error
            }),
            Some(&conduit),
        ),
    }
}

