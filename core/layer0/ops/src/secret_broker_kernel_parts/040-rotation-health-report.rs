fn rotation_health_report(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> RotationHealthReport {
    let ids = payload
        .get("secret_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_string())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| policy.secrets.keys().cloned().collect());
    let mut ok_count = 0usize;
    let mut warn_count = 0usize;
    let mut critical_count = 0usize;
    let mut unknown_count = 0usize;
    let mut unavailable_count = 0usize;
    let mut checks = Vec::new();
    let now = now_ms(payload);
    for secret_id in ids {
        let loaded = load_secret_by_id(
            root,
            &json!({
                "secret_id": secret_id,
                "now_ms": now,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
            policy,
            audit_path,
            false,
        );
        if !loaded.ok {
            unavailable_count += 1;
            checks.push(RotationCheckRow {
                secret_id,
                status: "critical".to_string(),
                reason: loaded.error.clone(),
                available: false,
                provider_errors: loaded.provider_errors.clone(),
                ..RotationCheckRow::default()
            });
            continue;
        }
        let rotation = loaded.rotation.clone().unwrap_or_default();
        match rotation.status.as_str() {
            "ok" => ok_count += 1,
            "warn" => warn_count += 1,
            "critical" => critical_count += 1,
            _ => unknown_count += 1,
        }
        checks.push(RotationCheckRow {
            secret_id: loaded.secret_id,
            status: rotation.status.clone(),
            reason: Some(rotation.reason.clone()),
            available: true,
            provider_type: loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            provider_ref: if policy.include_backend_details {
                loaded
                    .backend
                    .as_ref()
                    .and_then(|row| row.provider_ref.clone())
            } else {
                None
            },
            external_backend: loaded.backend.as_ref().map(|row| row.external),
            rotated_at: rotation.rotated_at.clone(),
            age_days: rotation.age_days,
            warn_after_days: Some(rotation.warn_after_days),
            max_after_days: Some(rotation.max_after_days),
            enforce_on_issue: Some(rotation.enforce_on_issue),
            provider_errors: Vec::new(),
        });
    }
    let level = if critical_count > 0 || unavailable_count > 0 {
        "critical"
    } else if warn_count > 0 {
        "warn"
    } else {
        "ok"
    };
    let report = RotationHealthReport {
        ok: level != "critical",
        report_type: "secret_rotation_health".to_string(),
        ts: now_iso(),
        policy_path: policy.path.clone(),
        policy_version: policy.version.clone(),
        total: checks.len(),
        level: level.to_string(),
        counts: json!({
            "ok": ok_count,
            "warn": warn_count,
            "critical": critical_count,
            "unknown": unknown_count,
            "unavailable": unavailable_count
        }),
        checks,
    };
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_rotation_check",
                "level": report.level,
                "total": report.total,
                "counts": report.counts,
            }),
        );
    }
    report
}

fn secret_broker_status(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let state = read_state(state_path);
    let now = now_ms(payload);
    let issued_total = state.issued.len();
    let issued_active = state
        .issued
        .values()
        .filter(|row| parse_ts_ms(&Value::String(row.expires_at.clone())).unwrap_or(0) > now)
        .count();
    let rotation = serde_json::to_value(rotation_health_report(
        root, payload, policy, audit_path, false,
    ))
    .unwrap_or_else(|_| json!({"ok": false, "type": "secret_rotation_health"}));
    json!({
        "ok": true,
        "type": "secret_broker_status",
        "ts": now_iso(),
        "policy_path": policy.path,
        "policy_version": policy.version,
        "state_path": state_path.to_string_lossy(),
        "audit_path": audit_path.to_string_lossy(),
        "supported_secret_ids": policy.secrets.keys().cloned().collect::<Vec<_>>(),
        "issued_total": issued_total,
        "issued_active": issued_active,
        "rotation": rotation,
    })
}

fn issue_handle(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let key = match secret_broker_key(root) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "error": err
            })
        }
    };
    let secret_id = text(payload.get("secret_id"), 160);
    let scope = text(payload.get("scope"), 180);
    let caller = {
        let clean = text(payload.get("caller"), 180);
        if clean.is_empty() {
            "unknown".to_string()
        } else {
            clean
        }
    };
    let reason = {
        let clean = text(payload.get("reason"), 240);
        if clean.is_empty() {
            None
        } else {
            Some(clean)
        }
    };
    if secret_id.is_empty() {
        return json!({ "ok": false, "error": "secret_id_required" });
    }
    if scope.is_empty() {
        return json!({ "ok": false, "error": "scope_required" });
    }
    let ttl_sec = int_value(payload.get("ttl_sec"))
        .unwrap_or(300)
        .clamp(30, 3600);
    let loaded = load_secret_by_id(root, payload, policy, audit_path, true);
    if !loaded.ok {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_issue_denied",
                "secret_id": secret_id,
                "scope": scope,
                "caller": caller,
                "reason": loaded.error,
            }),
        );
        return serde_json::to_value(loaded)
            .unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" }));
    }
    let rotation = loaded.rotation.clone().unwrap_or_default();
    if rotation.enforce_on_issue && rotation.status == "critical" {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_issue_denied",
                "secret_id": secret_id,
                "scope": scope,
                "caller": caller,
                "reason": "rotation_policy_enforced",
                "rotation_status": rotation.status,
            }),
        );
        return json!({
            "ok": false,
            "error": "rotation_policy_enforced",
            "secret_id": secret_id,
            "rotation": rotation,
        });
    }
    let issued_ms = now_ms(payload);
    let expires_ms = issued_ms + ttl_sec * 1000;
    let handle_id = format!(
        "sh_{}",
        &crate::deterministic_receipt_hash(&json!({
            "secret_id": secret_id,
            "scope": scope,
            "caller": caller,
            "issued_ms": issued_ms,
        }))[..16]
    );
    let body_payload = json!({
        "v": "1.1",
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "reason": reason,
        "issued_at_ms": issued_ms,
        "issued_at": iso_from_ms(issued_ms),
        "expires_at_ms": expires_ms,
        "expires_at": iso_from_ms(expires_ms),
        "nonce": &crate::deterministic_receipt_hash(&json!({"handle_id": handle_id, "issued_ms": issued_ms}))[..16]
    });
    let body_text = serde_json::to_string(&body_payload).unwrap_or_else(|_| "{}".to_string());
    let body = URL_SAFE_NO_PAD.encode(body_text.as_bytes());
    let sig = match sign_handle(&body, &key) {
        Ok(value) => value,
        Err(err) => return json!({ "ok": false, "error": err }),
    };
    let handle = format!("{body}.{sig}");
    let mut state = read_state(state_path);
    state.issued.insert(
        handle_id.clone(),
        SecretHandleStateRow {
            handle_id: handle_id.clone(),
            secret_id: secret_id.clone(),
            scope: scope.clone(),
            caller: caller.clone(),
            reason: reason.clone(),
            issued_at: iso_from_ms(issued_ms),
            expires_at: iso_from_ms(expires_ms),
            value_hash: loaded.value_hash.clone(),
            backend_provider_type: loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            backend_provider_ref: loaded
                .backend
                .as_ref()
                .and_then(|row| row.provider_ref.clone()),
            rotation_status: loaded.rotation.as_ref().map(|row| row.status.clone()),
            ..SecretHandleStateRow::default()
        },
    );
    let _ = write_state(state_path, &state);
    let _ = append_audit(
        audit_path,
        json!({
            "type": "secret_handle_issued",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "scope": scope,
            "caller": caller,
            "ttl_sec": ttl_sec,
            "reason": reason,
            "backend_provider_type": loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            "backend_provider_ref": if policy.include_backend_details {
                loaded.backend.as_ref().and_then(|row| row.provider_ref.clone())
            } else { None },
            "rotation_status": loaded.rotation.as_ref().map(|row| row.status.clone()),
            "rotation_age_days": loaded.rotation.as_ref().and_then(|row| row.age_days),
        }),
    );
    json!({
        "ok": true,
        "handle": handle,
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "issued_at": iso_from_ms(issued_ms),
        "expires_at": iso_from_ms(expires_ms),
        "ttl_sec": ttl_sec,
        "backend": loaded.backend,
        "rotation": loaded.rotation,
    })
}

fn resolve_handle(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    state_path: &Path,
    audit_path: &Path,
) -> Value {
    let key = match secret_broker_key(root) {
        Ok(value) => value,
        Err(err) => return json!({ "ok": false, "error": err }),
    };
    let handle = text(payload.get("handle"), 8192);
    let parts = handle.split('.').collect::<Vec<_>>();
    if parts.len() != 2 {
        return json!({ "ok": false, "error": "handle_malformed" });
    }
    let body = parts[0];
    let sig = parts[1];
    if !verify_handle_sig(body, sig, &key) {
        return json!({ "ok": false, "error": "handle_signature_invalid" });
    }
    let decoded = match URL_SAFE_NO_PAD.decode(body.as_bytes()) {
        Ok(value) => value,
        Err(_) => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let payload_value = match serde_json::from_slice::<Value>(&decoded) {
        Ok(value) => value,
        Err(_) => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let handle_payload = match payload_value.as_object() {
        Some(value) => value,
        None => return json!({ "ok": false, "error": "handle_payload_invalid" }),
    };
    let handle_id = text(handle_payload.get("handle_id"), 160);
    let secret_id = text(handle_payload.get("secret_id"), 160);
    let scope = text(handle_payload.get("scope"), 180);
    let caller = text(handle_payload.get("caller"), 180);
    let expires_at_ms = int_value(handle_payload.get("expires_at_ms")).unwrap_or(0);
    let now = now_ms(payload);
    if handle_id.is_empty() || secret_id.is_empty() || scope.is_empty() || caller.is_empty() {
        return json!({ "ok": false, "error": "handle_payload_missing_fields" });
    }
    if expires_at_ms <= now {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_handle_resolve_denied",
                "reason": "handle_expired",
                "handle_id": handle_id,
                "secret_id": secret_id,
            }),
        );
        return json!({ "ok": false, "error": "handle_expired", "handle_id": handle_id, "secret_id": secret_id });
    }
    let required_scope = text(payload.get("scope"), 180);
    if !required_scope.is_empty() && required_scope != scope {
        return json!({
            "ok": false,
            "error": "scope_mismatch",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "required_scope": required_scope,
            "handle_scope": scope,
        });
    }
    let required_caller = text(payload.get("caller"), 180);
    if !required_caller.is_empty() && required_caller != caller {
        return json!({
            "ok": false,
            "error": "caller_mismatch",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "required_caller": required_caller,
            "handle_caller": caller,
        });
    }
    let mut state = read_state(state_path);
    if !state.issued.contains_key(&handle_id) {
        return json!({ "ok": false, "error": "handle_unknown", "handle_id": handle_id, "secret_id": secret_id });
    }
    let loaded = load_secret_by_id(
        root,
        &json!({
            "secret_id": secret_id,
            "policy_path": payload.get("policy_path").cloned().unwrap_or(Value::Null),
            "now_ms": now,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
        policy,
        audit_path,
        true,
    );
    if !loaded.ok {
        return serde_json::to_value(loaded)
            .unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" }));
    }
    if let Some(row) = state.issued.get_mut(&handle_id) {
        row.resolve_count += 1;
        row.last_resolved_at = Some(iso_from_ms(now));
        row.last_backend_provider_type = loaded
            .backend
            .as_ref()
            .map(|item| item.provider_type.clone());
        row.last_rotation_status = loaded.rotation.as_ref().map(|item| item.status.clone());
    }
    let _ = write_state(state_path, &state);
    let _ = append_audit(
        audit_path,
        json!({
            "type": "secret_handle_resolved",
            "handle_id": handle_id,
            "secret_id": secret_id,
            "scope": scope,
            "caller": caller,
            "resolve_count": state.issued.get(&handle_id).map(|row| row.resolve_count).unwrap_or(0),
            "backend_provider_type": loaded.backend.as_ref().map(|row| row.provider_type.clone()),
            "backend_provider_ref": if policy.include_backend_details {
                loaded.backend.as_ref().and_then(|row| row.provider_ref.clone())
            } else { None },
            "rotation_status": loaded.rotation.as_ref().map(|row| row.status.clone()),
            "rotation_age_days": loaded.rotation.as_ref().and_then(|row| row.age_days),
        }),
    );
    json!({
        "ok": true,
        "handle_id": handle_id,
        "secret_id": secret_id,
        "scope": scope,
        "caller": caller,
        "expires_at": handle_payload.get("expires_at").cloned().unwrap_or(Value::Null),
        "value": loaded.value,
        "value_hash": loaded.value_hash,
        "backend": loaded.backend,
        "rotation": loaded.rotation,
    })
}

