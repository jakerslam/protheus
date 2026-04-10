fn run_secrets_contract_probe(root: &Path, argv: &[String], strict: bool) {
    let _ =
        run_security_contract_command(root, argv, strict, "secrets-federation", "V6-SEC-016", &[]);
}

fn out_handle_id_or_null(out: &Value) -> Value {
    out.get("handle_id").cloned().unwrap_or(Value::Null)
}

fn run_secrets_federation_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let op = parse_subcommand(argv, "status");
    let provider = parse_flag(argv, "provider")
        .unwrap_or_else(|| "vault".to_string())
        .to_ascii_lowercase();
    let secret_path = parse_flag(argv, "path").unwrap_or_else(|| "default/secret".to_string());
    let scope = parse_flag(argv, "scope").unwrap_or_else(|| "default".to_string());
    let lease_seconds = parse_u64(parse_flag(argv, "lease-seconds"), 3600);
    let supported = ["vault", "aws", "1password", "onepassword"];
    if strict && !supported.contains(&provider.as_str()) {
        run_secrets_contract_probe(root, argv, strict);
        let out = json!({
            "ok": false,
            "type": "security_plane_secrets_federation",
            "lane": "core/layer1/security",
            "mode": op,
            "strict": strict,
            "error": format!("unsupported_provider:{}", provider),
            "claim_evidence": [{
                "id": "V6-SEC-016",
                "claim": "external_secrets_federation_rejects_unknown_provider_profiles_fail_closed",
                "evidence": {"provider": provider}
            }]
        });
        return (out, 2);
    }

    let mut handles = read_secret_state(root);
    let mut out = json!({
        "ok": true,
        "type": "security_plane_secrets_federation",
        "lane": "core/layer1/security",
        "mode": op,
        "strict": strict
    });

    match op.as_str() {
        "fetch" => {
            let env_name = secret_env_var_name(&provider, &secret_path);
            let secret_value = std::env::var(&env_name)
                .ok()
                .or_else(|| std::env::var("PROTHEUS_SECRET_VALUE").ok());
            let Some(secret_value) = secret_value else {
                out["ok"] = Value::Bool(false);
                out["error"] = Value::String("secret_not_found".to_string());
                out["env_name"] = Value::String(env_name);
                out["claim_evidence"] = json!([{
                    "id": "V6-SEC-016",
                    "claim": "external_secrets_federation_fails_closed_when_secret_material_is_missing",
                    "evidence": {"provider": provider, "secret_path": secret_path}
                }]);
                run_secrets_contract_probe(root, argv, strict);
                return (out, if strict { 2 } else { 0 });
            };
            let ts = now_iso();
            let handle_id = deterministic_receipt_hash(&json!({
                "provider": provider,
                "secret_path": secret_path,
                "scope": scope,
                "ts": ts
            }));
            let row = SecretHandleRow {
                provider: provider.clone(),
                secret_path: secret_path.clone(),
                scope: scope.clone(),
                lease_expires_at: now_iso(),
                revoked: false,
                revoked_at: None,
                rotated_at: None,
                secret_sha256: hash_text(&secret_value),
            };
            handles.insert(handle_id.clone(), row);
            out["handle_id"] = Value::String(handle_id);
            out["lease_seconds"] = Value::from(lease_seconds);
            out["scope"] = Value::String(scope);
            out["provider"] = Value::String(provider.clone());
            out["secret_path"] = Value::String(secret_path.clone());
            out["claim_evidence"] = json!([{
                "id": "V6-SEC-016",
                "claim": "external_secrets_federation_issues_scoped_handles_with_fail_closed_fetch_semantics",
                "evidence": {
                    "provider": out["provider"],
                    "secret_path": out["secret_path"],
                    "handle_id": out["handle_id"]
                }
            }]);
        }
        "rotate" => {
            let handle_id = parse_flag(argv, "handle-id").unwrap_or_default();
            if let Some(row) = handles.get_mut(&handle_id) {
                row.rotated_at = Some(now_iso());
                row.lease_expires_at = now_iso();
                out["handle_id"] = Value::String(handle_id);
                out["rotated"] = Value::Bool(true);
            } else {
                out["ok"] = Value::Bool(false);
                out["error"] = Value::String("handle_not_found".to_string());
            }
            out["claim_evidence"] = json!([{
                "id": "V6-SEC-016",
                "claim": "external_secrets_federation_supports_rotation_and_audit_receipts_for_issued_handles",
                "evidence": {"handle_id": out_handle_id_or_null(&out)}
            }]);
        }
        "revoke" => {
            let handle_id = parse_flag(argv, "handle-id").unwrap_or_default();
            if let Some(row) = handles.get_mut(&handle_id) {
                row.revoked = true;
                row.revoked_at = Some(now_iso());
                out["handle_id"] = Value::String(handle_id);
                out["revoked"] = Value::Bool(true);
            } else {
                out["ok"] = Value::Bool(false);
                out["error"] = Value::String("handle_not_found".to_string());
            }
            out["claim_evidence"] = json!([{
                "id": "V6-SEC-016",
                "claim": "external_secrets_federation_supports_revoke_semantics_for_issued_handles",
                "evidence": {"handle_id": out_handle_id_or_null(&out)}
            }]);
        }
        _ => {
            let active_handles = handles.values().filter(|row| !row.revoked).count();
            out["active_handles"] = Value::from(active_handles as u64);
            out["total_handles"] = Value::from(handles.len() as u64);
            out["providers"] = Value::Array(
                handles
                    .values()
                    .map(|row| Value::String(row.provider.clone()))
                    .collect::<Vec<_>>(),
            );
            out["claim_evidence"] = json!([{
                "id": "V6-SEC-016",
                "claim": "external_secrets_federation_status_exports_active_handle_and_provider_inventory",
                "evidence": {
                    "active_handles": out["active_handles"],
                    "total_handles": out["total_handles"]
                }
            }]);
        }
    }

    write_secret_state(root, &handles);
    run_secrets_contract_probe(root, argv, strict);
    append_jsonl(
        &secrets_events_path(root),
        &json!({
            "ts": now_iso(),
            "mode": op,
            "ok": out.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "provider": provider,
            "secret_path": secret_path,
            "handle_id": out_handle_id_or_null(&out)
        }),
    );
    let failed = !out.get("ok").and_then(Value::as_bool).unwrap_or(false);
    (out, if strict && failed { 2 } else { 0 })
}

fn capability_action(command: &str, argv: &[String], payload: &Value) -> Option<String> {
    if let Some(action) = payload.get("action").and_then(Value::as_str) {
        let clean = action.trim().to_ascii_lowercase();
        if clean == "grant" || clean == "revoke" {
            return Some(clean);
        }
    }
    if command == "capability-switchboard" || command == "capability_switchboard" {
        if payload.get("type").and_then(Value::as_str) == Some("capability_switchboard_set") {
            if let Some(enabled) = payload.get("enabled").and_then(Value::as_bool) {
                return Some(if enabled { "grant" } else { "revoke" }.to_string());
            }
            if let Some(state) = parse_flag(argv, "state") {
                let lowered = state.trim().to_ascii_lowercase();
                return Some(
                    if matches!(lowered.as_str(), "on" | "true" | "1") {
                        "grant"
                    } else {
                        "revoke"
                    }
                    .to_string(),
                );
            }
        }
    }
    None
}

fn append_capability_event(root: &Path, event: &Value) {
    let path = capability_event_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(event) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    }
}
fn wrap_capability_event(root: &Path, command: &str, argv: &[String], payload: Value) -> Value {
    let strict = parse_bool(parse_flag(argv, "strict"), true);
    let mut out = if payload.is_object() {
        payload
    } else {
        json!({
            "ok": false,
            "type": "security_plane_wrap_error",
            "payload": payload
        })
    };
    if out.get("lane").is_none() {
        out["lane"] = Value::String("core/layer1/security".to_string());
    }
    out["strict"] = Value::Bool(strict);
    out["policy_engine"] = Value::String("infring_layer1_security".to_string());
    out["authority"] = Value::String("rust_security_plane".to_string());
    out["ts"] = out
        .get("ts")
        .cloned()
        .unwrap_or_else(|| Value::String(now_iso()));

    let action = capability_action(command, argv, &out);
    let event = json!({
        "kind": "infring_capability_event",
        "command": clean(command, 120),
        "action": action.clone().unwrap_or_else(|| "observe".to_string()),
        "runtime_capability_change": action.is_some()
    });
    out["infring_capability_event"] = event.clone();
    let mut capability_hash_chain_ok = Value::Null;
    if let Some(action) = action {
        let capability = parse_flag(argv, "capability")
            .or_else(|| {
                out.get("capability")
                    .and_then(Value::as_str)
                    .map(|row| row.to_string())
            })
            .or_else(|| parse_flag(argv, "policy"))
            .unwrap_or_else(|| "global".to_string());
        let subject = parse_flag(argv, "subject")
            .or_else(|| {
                out.get("subject")
                    .and_then(Value::as_str)
                    .map(|row| row.to_string())
            })
            .unwrap_or_else(|| "global".to_string());
        let reason = parse_flag(argv, "reason").unwrap_or_else(|| {
            format!(
                "{}:{}",
                clean(command, 80),
                out.get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("runtime_change")
            )
        });
        out["grant_revoke_receipt"] = json!({
            "action": action,
            "ts": now_iso(),
            "source": "security_plane_runtime"
        });
        match crate::assimilation_controller::append_capability_hash_chain_event(
            root,
            out.get("grant_revoke_receipt")
                .and_then(|row| row.get("action"))
                .and_then(Value::as_str)
                .unwrap_or("observe"),
            &capability,
            &subject,
            &reason,
        ) {
            Ok(event_row) => {
                capability_hash_chain_ok = Value::Bool(true);
                out["capability_hash_chain_ledger"] = json!({
                    "ok": true,
                    "capability": capability,
                    "subject": subject,
                    "event": event_row
                });
            }
            Err(err) => {
                capability_hash_chain_ok = Value::Bool(false);
                out["capability_hash_chain_ledger"] = json!({
                    "ok": false,
                    "error": err,
                    "capability": capability,
                    "subject": subject
                });
                if strict {
                    out["ok"] = Value::Bool(false);
                    let mut errs = out
                        .get("errors")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    errs.push(Value::String(
                        "capability_hash_chain_append_failed".to_string(),
                    ));
                    out["errors"] = Value::Array(errs);
                }
            }
        }
    }

    let mut claim_rows = out
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    claim_rows.push(json!({
        "id": "V8-DIRECTIVES-001.3",
        "claim": "security_plane_operations_are_surfaced_as_infring_capability_events",
        "evidence": {
            "command": clean(command, 120),
            "policy_engine": "infring_layer1_security"
        }
    }));
    if out.get("grant_revoke_receipt").is_some() {
        claim_rows.push(json!({
            "id": "V7-ASSIMILATE-001.3",
            "claim": "runtime_capability_changes_emit_grant_revoke_receipts",
            "evidence": {
                "command": clean(command, 120)
            }
        }));
        claim_rows.push(json!({
            "id": "V7-ASM-003",
            "claim": "runtime_capability_changes_are_written_to_capability_hash_chain_ledger",
            "evidence": {
                "command": clean(command, 120),
                "capability_hash_chain_ok": capability_hash_chain_ok
            }
        }));
    }
    out["claim_evidence"] = Value::Array(claim_rows);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));

    let log_row = json!({
        "ts": now_iso(),
        "type": "security_plane_capability_event",
        "command": clean(command, 120),
        "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "event": event,
        "grant_revoke_receipt": out.get("grant_revoke_receipt").cloned().unwrap_or(Value::Null)
    });
    append_capability_event(root, &log_row);
    out
}
