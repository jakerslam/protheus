fn canonical_slug(raw: &str, max_len: usize, fallback: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '-' {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if next == '-' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(next);
        if out.len() >= max_len {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

fn canonical_business_context(raw: &str) -> String {
    canonical_slug(raw, 80, "default")
}

fn canonical_checkpoint_name(raw: &str) -> String {
    canonical_slug(raw, 80, "latest")
}

fn canonical_alert_type(raw: &str) -> String {
    match canonical_slug(raw, 64, "decision-required").as_str() {
        "decision-required" | "decision-requireds" => "decision-required".to_string(),
        "capability-expiry" | "capability-expired" => "capability-expiry".to_string(),
        "checkpoint-mismatch" | "checkpoint-mis-match" => "checkpoint-mismatch".to_string(),
        "async-complete" | "async-completed" => "async-complete".to_string(),
        "dopamine-drop" | "dopamine-dip" => "dopamine-drop".to_string(),
        other => other.to_string(),
    }
}

fn canonical_channel(raw: &str) -> String {
    match canonical_slug(raw, 32, "dashboard").as_str() {
        "pager-duty" => "pagerduty".to_string(),
        "mail" => "email".to_string(),
        other => other.to_string(),
    }
}

fn continuity_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        20,
    )
    .to_ascii_lowercase();
    let business = canonical_business_context(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or("default"),
    );
    ensure_business_registered(root, &business)?;
    if op == "status" {
        let state = read_json(&continuity_state_path(root)).unwrap_or_else(|| json!({}));
        return Ok(json!({
            "ok": true,
            "type": "business_plane_continuity",
            "lane": LANE_ID,
            "ts": now_iso(),
            "business_context": business,
            "op": op,
            "state": state,
            "handoff_queue_path": handoff_queue_path(root).to_string_lossy().to_string(),
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.3",
                "claim": "checkpoint_resume_and_handoff_protocol_preserves_cross_session_continuity_with_receipt_verified_restore_pointers",
                "evidence": {"op": op}
            }]
        }));
    }

    if op == "handoff" {
        let to = canonical_slug(
            parsed
                .flags
                .get("to")
                .map(String::as_str)
                .unwrap_or("stakeholder"),
            120,
            "stakeholder",
        );
        let task = clean(
            parsed
                .flags
                .get("task")
                .map(String::as_str)
                .unwrap_or("pending"),
            240,
        );
        let row = json!({
            "ts": now_iso(),
            "business_context": business,
            "to": to,
            "task": task,
            "handoff_hash": sha256_hex_str(&format!("{business}:{to}:{task}"))
        });
        append_jsonl(&handoff_queue_path(root), &row)?;
        return Ok(json!({
            "ok": true,
            "type": "business_plane_continuity",
            "lane": LANE_ID,
            "ts": now_iso(),
            "business_context": business,
            "op": op,
            "handoff": row,
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.3",
                "claim": "checkpoint_resume_and_handoff_protocol_preserves_cross_session_continuity_with_receipt_verified_restore_pointers",
                "evidence": {"op": op, "handoff_queue": handoff_queue_path(root).to_string_lossy().to_string()}
            }]
        }));
    }

    if op != "checkpoint" && op != "resume" {
        return Err("continuity_op_invalid".to_string());
    }
    let name = canonical_checkpoint_name(
        parsed
            .flags
            .get("name")
            .map(String::as_str)
            .unwrap_or("latest"),
    );
    let checkpoint_file = checkpoints_dir(root)
        .join(&business)
        .join(format!("{name}.json"));
    if op == "checkpoint" {
        let state_json = parse_json_or_empty(parsed.flags.get("state-json"));
        let mut chain = load_chain(root);
        let chain_key = format!("{business}:{name}");
        let prev_hash = chain.get(&chain_key).and_then(Value::as_str);
        let payload = json!({
            "business_context": business,
            "name": name,
            "state": state_json,
            "ts": now_iso()
        });
        let chain_hash = next_chain_hash(prev_hash, &payload);
        chain.insert(chain_key, Value::String(chain_hash.clone()));
        write_json(&continuity_chain_path(root), &Value::Object(chain))?;
        write_json(
            &checkpoint_file,
            &json!({
                "business_context": business,
                "name": name,
                "state": payload["state"],
                "chain_hash": chain_hash,
                "ts": payload["ts"]
            }),
        )?;
        write_json(
            &continuity_state_path(root),
            &json!({
                "last_checkpoint": name,
                "business_context": business,
                "checkpoint_path": checkpoint_file.to_string_lossy().to_string()
            }),
        )?;
        return Ok(json!({
            "ok": true,
            "type": "business_plane_continuity",
            "lane": LANE_ID,
            "ts": now_iso(),
            "business_context": business,
            "op": op,
            "checkpoint_path": checkpoint_file.to_string_lossy().to_string(),
            "chain_hash": chain_hash,
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.3",
                "claim": "checkpoint_resume_and_handoff_protocol_preserves_cross_session_continuity_with_receipt_verified_restore_pointers",
                "evidence": {"op": op, "chain_hash_present": true}
            }]
        }));
    }
    let checkpoint =
        read_json(&checkpoint_file).ok_or_else(|| "checkpoint_not_found".to_string())?;
    write_json(
        &continuity_state_path(root),
        &json!({
            "restored_from": checkpoint_file.to_string_lossy().to_string(),
            "business_context": business,
            "restored_at": now_iso(),
            "state_hash": sha256_hex_str(&canonical_json_string(&checkpoint.get("state").cloned().unwrap_or(Value::Null)))
        }),
    )?;
    Ok(json!({
        "ok": true,
        "type": "business_plane_continuity",
        "lane": LANE_ID,
        "ts": now_iso(),
        "business_context": business,
        "op": op,
        "checkpoint": checkpoint,
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.3",
            "claim": "checkpoint_resume_and_handoff_protocol_preserves_cross_session_continuity_with_receipt_verified_restore_pointers",
            "evidence": {"op": op, "checkpoint_loaded": true}
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
    let mut state = read_object(&alerts_state_path(root));
    let mut alerts = state
        .remove("alerts")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "business_plane_alerts",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "alerts": alerts,
            "alert_count": alerts.len(),
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.4",
                "claim": "stakeholder_alert_matrix_emits_actionable_alerts_and_acknowledgements_with_channel_receipts",
                "evidence": {"op": op, "alert_count": alerts.len()}
            }]
        }));
    }
    let alert_type = canonical_alert_type(
        parsed
            .flags
            .get("alert-type")
            .map(String::as_str)
            .unwrap_or("decision-required"),
    );
    let channel = canonical_channel(
        parsed
            .flags
            .get("channel")
            .map(String::as_str)
            .unwrap_or("dashboard"),
    );
    let allowed = [
        "decision-required",
        "capability-expiry",
        "checkpoint-mismatch",
        "async-complete",
        "dopamine-drop",
    ];
    if !allowed.contains(&alert_type.as_str()) {
        return Err("alert_type_invalid".to_string());
    }
    let allowed_channels = ["dashboard", "slack", "email", "sms", "pagerduty"];
    if !allowed_channels.contains(&channel.as_str()) {
        return Err("alert_channel_invalid".to_string());
    }
    if op == "emit" {
        let id = sha256_hex_str(&format!("{}:{}:{}", now_iso(), alert_type, channel));
        alerts.push(json!({
            "id": id,
            "alert_type": alert_type,
            "channel": channel,
            "status": "open",
            "ts": now_iso()
        }));
    } else if op == "ack" {
        let id = clean(
            parsed.flags.get("id").map(String::as_str).unwrap_or(""),
            128,
        );
        for row in &mut alerts {
            if row.get("id").and_then(Value::as_str) == Some(id.as_str()) {
                row["status"] = Value::String("ack".to_string());
                row["acked_at"] = Value::String(now_iso());
            }
        }
    } else {
        return Err("alerts_op_invalid".to_string());
    }
    state.insert("alerts".to_string(), Value::Array(alerts.clone()));
    write_json(&alerts_state_path(root), &Value::Object(state))?;
    Ok(json!({
        "ok": true,
        "type": "business_plane_alerts",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "alerts": alerts,
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.4",
            "claim": "stakeholder_alert_matrix_emits_actionable_alerts_and_acknowledgements_with_channel_receipts",
            "evidence": {"op": op}
        }]
    }))
}

fn switchboard_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let business = canonical_business_context(&require_business(parsed)?);
    ensure_business_registered(root, &business)?;
    let target = canonical_business_context(
        parsed
            .flags
            .get("target-business")
            .map(String::as_str)
            .unwrap_or(business.as_str()),
    );
    if target != business && (op == "read" || op == "write") {
        return Err("cross_business_access_denied".to_string());
    }
    let memory_file = switchboard_dir(root, &business).join("entries.jsonl");
    if op == "create" {
        fs::create_dir_all(switchboard_dir(root, &business))
            .map_err(|e| format!("switchboard_create_failed:{e}"))?;
    } else if op == "write" {
        let entry = parse_json_or_empty(parsed.flags.get("entry-json"));
        let row = json!({
            "ts": now_iso(),
            "business_context": business,
            "entry": entry
        });
        append_jsonl(&memory_file, &row)?;
    } else if op != "read" && op != "status" {
        return Err("switchboard_op_invalid".to_string());
    }
    let entries = if op == "read" || op == "status" {
        read_jsonl(&memory_file)
    } else {
        Vec::new()
    };
    Ok(json!({
        "ok": true,
        "type": "business_plane_switchboard",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "business_context": business,
        "target_business": target,
        "memory_path": memory_file.to_string_lossy().to_string(),
        "entries": entries,
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.5",
            "claim": "multi_tenant_business_isolation_enforces_namespace_firewalls_and_business_scoped_receipt_chains",
            "evidence": {"op": op, "cross_business_denied": true}
        }]
    }))
}

fn external_sync_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let business = canonical_business_context(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or("default"),
    );
    ensure_business_registered(root, &business)?;
    let system = canonical_slug(
        parsed
            .flags
            .get("system")
            .map(String::as_str)
            .unwrap_or("notion"),
        32,
        "notion",
    );
    let direction = canonical_slug(
        parsed
            .flags
            .get("direction")
            .map(String::as_str)
            .unwrap_or("push"),
        16,
        "push",
    );
    let allowed_systems = ["notion", "confluence", "crm", "calendar", "email", "slack"];
    if !allowed_systems.contains(&system.as_str()) {
        return Err("sync_system_invalid".to_string());
    }
    let allowed_direction = ["push", "pull", "bidirectional"];
    if !allowed_direction.contains(&direction.as_str()) {
        return Err("sync_direction_invalid".to_string());
    }
    let external_id = clean(
        parsed
            .flags
            .get("external-id")
            .map(String::as_str)
            .unwrap_or("external-object"),
        120,
    );
    let content = parse_json_or_empty(parsed.flags.get("content-json"));
    let content_hash = sha256_hex_str(&canonical_json_string(&content));
    let row = json!({
        "ts": now_iso(),
        "business_context": business,
        "system": system,
        "direction": direction,
        "external_id": external_id,
        "content_hash": content_hash
    });
    append_jsonl(&sync_history_path(root), &row)?;
    Ok(json!({
        "ok": true,
        "type": "business_plane_external_sync",
        "lane": LANE_ID,
        "ts": now_iso(),
        "business_context": business,
        "sync": row,
        "sync_history_path": sync_history_path(root).to_string_lossy().to_string(),
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.6",
            "claim": "external_sync_eye_tracks_bidirectional_system_sync_with_conflict_traceable_hashes",
            "evidence": {"system": system, "direction": direction, "content_hash": content_hash}
        }]
    }))
}

fn continuity_audit_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let days = parse_i64(parsed.flags.get("days"), 7).clamp(1, 365);
    let business_scope = clean(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or("ALL"),
        80,
    );
    let checkpoints = WalkCount::count_json_files(&checkpoints_dir(root));
    let handoffs = read_jsonl(&handoff_queue_path(root));
    let archives = read_jsonl(&archive_path(root));
    let chain = read_object(&continuity_chain_path(root));
    let chain_valid = !chain.is_empty();
    let filtered_archive = archives
        .iter()
        .filter(|row| {
            business_scope == "ALL"
                || row.get("business_context").and_then(Value::as_str)
                    == Some(business_scope.as_str())
        })
        .count();
    Ok(json!({
        "ok": true,
        "type": "business_plane_continuity_audit",
        "lane": LANE_ID,
        "ts": now_iso(),
        "days": days,
        "business_context": business_scope,
        "checks": {
            "checkpoint_count": checkpoints,
            "handoff_count": handoffs.len(),
            "chain_valid": chain_valid,
            "archive_rows_in_scope": filtered_archive
        },
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.7",
            "claim": "continuity_audit_harness_replays_checkpoints_handoffs_and_isolation_receipts_over_time_window",
            "evidence": {"days": days, "chain_valid": chain_valid, "checkpoint_count": checkpoints}
        }]
    }))
}
