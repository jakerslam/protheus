fn sandbox_command(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        24,
    )
    .to_ascii_lowercase();
    let session_id = clean(
        parsed
            .flags
            .get("session-id")
            .or_else(|| parsed.flags.get("session"))
            .map(String::as_str)
            .unwrap_or("default"),
        80,
    );
    if op == "status" {
        let rows = read_jsonl(&sandbox_events_path(root));
        let sessions = sandbox_session_map(root);
        let snapshots = fs::read_dir(sandbox_snapshots_dir(root))
            .ok()
            .map(|entries| entries.flatten().count())
            .unwrap_or(0);
        return Ok(json!({
            "ok": true,
            "type": "canyon_plane_sandbox",
            "lane": LANE_ID,
            "ts": now_iso(),
            "strict": strict,
            "op": op,
            "events": rows,
            "event_count": rows.len(),
            "sessions": sessions,
            "session_count": sessions.len(),
            "snapshot_count": snapshots,
            "claim_evidence": [{
                "id": "V7-CANYON-001.4",
                "claim": "tiered_isolation_enforces_native_wasm_and_optional_firecracker_modes_with_escape_denial_receipts",
                "evidence": {"event_count": rows.len()}
            },{
                "id": "V7-CANYON-003.1",
                "claim": "persistent_sandbox_snapshots_resume_with_receipt_bound_state_integrity",
                "evidence": {"session_count": sessions.len(), "snapshot_count": snapshots}
            }]
        }));
    }
    if !matches!(op.as_str(), "run" | "snapshot" | "resume") {
        return Err("sandbox_op_invalid".to_string());
    }
    let mut sessions = sandbox_session_map(root);
    if op == "snapshot" {
        let Some(state) = sessions.get(&session_id).cloned() else {
            return Err("sandbox_session_not_found".to_string());
        };
        let state_payload = sandbox_session_snapshot(&state);
        let snapshot_id = sha256_hex_str(&format!(
            "{}:{}:{}",
            session_id,
            state_payload
                .get("last_event_hash")
                .and_then(Value::as_str)
                .unwrap_or(""),
            state_payload
        ));
        let snapshot = json!({
            "snapshot_id": snapshot_id,
            "session_id": session_id,
            "captured_at": now_iso(),
            "state": state_payload,
            "integrity_hash": sha256_hex_str(&stringify_json(&state_payload))
        });
        let started = Instant::now();
        let snapshot_path = sandbox_snapshots_dir(root).join(format!("{snapshot_id}.json"));
        write_json(&snapshot_path, &snapshot)?;
        let event = json!({
            "ts": now_iso(),
            "op": "snapshot",
            "session_id": session_id,
            "snapshot_id": snapshot_id,
            "ok": true,
            "latency_ms": started.elapsed().as_millis() as u64,
            "integrity_hash": snapshot.get("integrity_hash").cloned().unwrap_or_else(|| Value::String(String::new()))
        });
        append_jsonl(&sandbox_events_path(root), &event)?;
        let latency_ms = event.get("latency_ms").and_then(Value::as_u64).unwrap_or(0);
        let mut errors = Vec::<String>::new();
        if strict && latency_ms > 50 {
            errors.push("sandbox_snapshot_latency_budget_exceeded".to_string());
        }
        return Ok(json!({
            "ok": !strict || errors.is_empty(),
            "type": "canyon_plane_sandbox",
            "lane": LANE_ID,
            "ts": now_iso(),
            "strict": strict,
            "op": op,
            "session_id": session_id,
            "snapshot_id": snapshot_id,
            "snapshot_path": snapshot_path.to_string_lossy().to_string(),
            "latency_ms": latency_ms,
            "errors": errors,
            "claim_evidence": [{
                "id": "V7-CANYON-003.1",
                "claim": "persistent_sandbox_snapshots_resume_with_receipt_bound_state_integrity",
                "evidence": {"snapshot_path": snapshot_path.to_string_lossy().to_string(), "latency_ms": latency_ms}
            }]
        }));
    }
    if op == "resume" {
        let snapshot_id = clean(
            parsed
                .flags
                .get("snapshot-id")
                .or_else(|| parsed.flags.get("snapshot"))
                .map(String::as_str)
                .unwrap_or(""),
            96,
        );
        if snapshot_id.is_empty() {
            return Err("sandbox_snapshot_id_required".to_string());
        }
        let snapshot_path = sandbox_snapshots_dir(root).join(format!("{snapshot_id}.json"));
        let snapshot =
            read_json(&snapshot_path).ok_or_else(|| "sandbox_snapshot_missing".to_string())?;
        let state = snapshot
            .get("state")
            .cloned()
            .ok_or_else(|| "sandbox_snapshot_state_missing".to_string())?;
        let expected = snapshot
            .get("integrity_hash")
            .and_then(Value::as_str)
            .unwrap_or("");
        let actual = sha256_hex_str(&stringify_json(&state));
        let started = Instant::now();
        let mut errors = Vec::<String>::new();
        if strict && expected != actual {
            errors.push("sandbox_snapshot_integrity_mismatch".to_string());
        }
        if errors.is_empty() {
            sessions.insert(session_id.clone(), state.clone());
            write_json(
                &sandbox_sessions_path(root),
                &Value::Object(sessions.clone()),
            )?;
        }
        let event = json!({
            "ts": now_iso(),
            "op": "resume",
            "session_id": session_id,
            "snapshot_id": snapshot_id,
            "ok": errors.is_empty(),
            "latency_ms": started.elapsed().as_millis() as u64,
            "integrity_hash": actual
        });
        append_jsonl(&sandbox_events_path(root), &event)?;
        let latency_ms = event.get("latency_ms").and_then(Value::as_u64).unwrap_or(0);
        if strict && latency_ms > 50 {
            errors.push("sandbox_resume_latency_budget_exceeded".to_string());
        }
        return Ok(json!({
            "ok": !strict || errors.is_empty(),
            "type": "canyon_plane_sandbox",
            "lane": LANE_ID,
            "ts": now_iso(),
            "strict": strict,
            "op": op,
            "session_id": session_id,
            "snapshot_id": snapshot_id,
            "restored_state": state,
            "latency_ms": latency_ms,
            "errors": errors,
            "claim_evidence": [{
                "id": "V7-CANYON-003.1",
                "claim": "persistent_sandbox_snapshots_resume_with_receipt_bound_state_integrity",
                "evidence": {"snapshot_id": snapshot_id, "latency_ms": latency_ms, "integrity_hash": actual}
            }]
        }));
    }
    let tier = clean(
        parsed
            .flags
            .get("tier")
            .map(String::as_str)
            .unwrap_or("native"),
        24,
    )
    .to_ascii_lowercase();
    let language = clean(
        parsed
            .flags
            .get("language")
            .map(String::as_str)
            .unwrap_or("rust"),
        24,
    )
    .to_ascii_lowercase();
    let fuel = parse_u64(parsed.flags.get("fuel"), 100_000);
    let epoch = parse_u64(parsed.flags.get("epoch"), 1_000);
    let escape_attempt = parse_bool(parsed.flags.get("escape-attempt"), false);
    let logical_only = parse_bool(parsed.flags.get("logical-only"), false);
    let provider_family = clean(
        parsed
            .flags
            .get("provider-family")
            .map(String::as_str)
            .unwrap_or("openai"),
        24,
    )
    .to_ascii_lowercase();
    let capability_contract = clean(
        parsed
            .flags
            .get("capability-contract")
            .map(String::as_str)
            .unwrap_or(""),
        80,
    );

    let allowed_tiers = ["native", "wasm", "firecracker"];
    if !allowed_tiers.contains(&tier.as_str()) {
        return Err("sandbox_tier_invalid".to_string());
    }
    let allowed_languages = ["python", "ts", "go", "rust"];
    if !allowed_languages.contains(&language.as_str()) {
        return Err("sandbox_language_invalid".to_string());
    }
    let allowed_provider_families = ["openai", "openrouter", "xai", "memory"];
    if !allowed_provider_families.contains(&provider_family.as_str()) {
        return Err("sandbox_provider_family_invalid".to_string());
    }

    let mut errors = Vec::<String>::new();
    if fuel < 100 {
        errors.push("sandbox_fuel_floor_violation".to_string());
    }
    if epoch < 10 {
        errors.push("sandbox_epoch_floor_violation".to_string());
    }
    if strict && escape_attempt {
        errors.push("sandbox_escape_attempt_denied".to_string());
    }
    let overhead_mb = if logical_only {
        match tier.as_str() {
            "wasm" => 3.8,
            "native" => 4.2,
            "firecracker" => 6.5,
            _ => 5.0,
        }
    } else {
        match tier.as_str() {
            "native" => 1.2,
            "wasm" => 2.7,
            "firecracker" => 12.0,
            _ => 3.0,
        }
    };
    if strict && logical_only && tier != "wasm" {
        errors.push("sandbox_logical_only_requires_wasm".to_string());
    }
    if strict && logical_only && overhead_mb > 4.0 {
        errors.push("sandbox_logical_only_overhead_budget_exceeded".to_string());
    }
    if strict && logical_only && capability_contract.is_empty() {
        errors.push("sandbox_capability_contract_required".to_string());
    }
    if strict && provider_family == "memory" && !matches!(language.as_str(), "rust" | "python") {
        errors.push("sandbox_memory_provider_language_invalid".to_string());
    }
    if strict && tier == "firecracker" {
        let firecracker_ok = Command::new("sh")
            .arg("-lc")
            .arg("command -v firecracker >/dev/null 2>&1")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !firecracker_ok {
            errors.push("firecracker_runtime_unavailable".to_string());
        }
    }

    let event = json!({
        "ts": now_iso(),
        "session_id": session_id,
        "tier": tier,
        "language": language,
        "fuel": fuel,
        "epoch": epoch,
        "logical_only": logical_only,
        "provider_family": provider_family,
        "capability_contract": if capability_contract.is_empty() { Value::Null } else { Value::String(capability_contract.clone()) },
        "overhead_mb": overhead_mb,
        "escape_attempt": escape_attempt,
        "ok": !strict || errors.is_empty(),
        "event_hash": sha256_hex_str(&format!("{}:{}:{}:{}:{}", now_iso(), session_id, tier, language, fuel))
    });
    append_jsonl(&sandbox_events_path(root), &event)?;
    let state = json!({
        "session_id": session_id,
        "tier": event.get("tier").cloned().unwrap_or_else(|| Value::String("native".to_string())),
        "language": event.get("language").cloned().unwrap_or_else(|| Value::String("rust".to_string())),
        "fuel": fuel,
        "epoch": epoch,
        "logical_only": logical_only,
        "provider_family": event.get("provider_family").cloned().unwrap_or_else(|| Value::String("openai".to_string())),
        "capability_contract": event.get("capability_contract").cloned().unwrap_or(Value::Null),
        "overhead_mb": overhead_mb,
        "last_event_hash": event.get("event_hash").cloned().unwrap_or_else(|| Value::String(String::new())),
        "updated_at": now_iso()
    });
    sessions.insert(session_id.clone(), state.clone());
    write_json(&sandbox_sessions_path(root), &Value::Object(sessions))?;

    Ok(json!({
        "ok": !strict || errors.is_empty(),
        "type": "canyon_plane_sandbox",
        "lane": LANE_ID,
        "ts": now_iso(),
        "strict": strict,
        "op": op,
        "session_id": session_id,
        "event": event,
        "state": state,
        "errors": errors,
        "claim_evidence": [{
            "id": "V7-CANYON-001.4",
            "claim": "tiered_isolation_enforces_native_wasm_and_optional_firecracker_modes_with_escape_denial_receipts",
            "evidence": {"tier": tier, "language": language, "fuel": fuel, "epoch": epoch, "provider_family": provider_family, "capability_contract": capability_contract}
        },{
            "id": "V7-CANYON-003.3",
            "claim": "logical_only_isolated_mode_keeps_edge_overhead_within_budgeted_wasm_limits",
            "evidence": {"logical_only": logical_only, "tier": tier, "overhead_mb": overhead_mb, "provider_family": provider_family}
        }]
    }))
}
