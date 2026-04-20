
pub fn run_remote_emergency_halt(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_remote_emergency_policy(repo_root, &parsed);
    let state_path = resolve_runtime_or_state(repo_root, &policy.paths.state);
    let nonce_store_path = resolve_runtime_or_state(repo_root, &policy.paths.nonce_store);
    let audit_path = resolve_runtime_or_state(repo_root, &policy.paths.audit);
    let key = std::env::var(&policy.key_env)
        .ok()
        .map(|v| clean(v, 4096))
        .unwrap_or_default();

    match cmd.as_str() {
        "status" => (
            json!({
                "ok": true,
                "type": "remote_emergency_halt_status",
                "ts": now_iso(),
                "enabled": policy.enabled,
                "key_env": policy.key_env,
                "state_path": normalize_rel(state_path.to_string_lossy()),
                "nonce_store_path": normalize_rel(nonce_store_path.to_string_lossy()),
                "state": read_json_or(&state_path, json!({"halted": false})),
                "nonces": read_json_or(&nonce_store_path, json!({"version":1, "entries": {}}))
                    .get("entries")
                    .and_then(Value::as_object)
                    .map(|m| m.len())
                    .unwrap_or(0)
            }),
            0,
        ),
        "sign-halt" | "sign-purge" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_sign", "error":"remote_emergency_halt_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let action = if cmd == "sign-purge" { "purge" } else { "halt" };
            let ttl = flag(&parsed, "ttl-sec")
                .or_else(|| flag(&parsed, "ttl_sec"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(120)
                .clamp(10, policy.max_ttl_seconds.max(10));
            let now_ms = Utc::now().timestamp_millis();
            let mut payload = json!({
                "type": "remote_emergency_command",
                "action": action,
                "command_id": format!("reh_{}", &sha256_hex_bytes(format!("{}|{}", now_ms, action).as_bytes())[0..12]),
                "nonce": format!("nonce_{}", &sha256_hex_bytes(format!("{}|{}", now_ms, std::process::id()).as_bytes())[0..10]),
                "issued_at": now_iso(),
                "issued_at_ms": now_ms,
                "expires_at_ms": now_ms + ttl * 1000,
                "scope": clean(flag(&parsed, "scope").unwrap_or("all"), 120),
                "approval_note": clean(flag(&parsed, "approval-note").or_else(|| flag(&parsed, "approval_note")).unwrap_or(""), 400),
                "pending_id": clean(flag(&parsed, "pending-id").or_else(|| flag(&parsed, "pending_id")).unwrap_or(""), 120)
            });
            let signature = match hmac_sha256_hex(&key, &payload) {
                Ok(v) => v,
                Err(err) => return (json!({"ok": false, "error": clean(err, 220)}), 1),
            };
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("signature".to_string(), Value::String(signature));
            }
            (
                json!({
                    "ok": true,
                    "type": "remote_emergency_halt_sign",
                    "action": action,
                    "command": payload.clone(),
                    "command_b64": BASE64_STANDARD.encode(stable_json_string(&payload))
                }),
                0,
            )
        }
        "receive-b64" => {
            let raw = clean(
                flag(&parsed, "command-b64")
                    .or_else(|| flag(&parsed, "command_b64"))
                    .unwrap_or(""),
                16_384,
            );
            if raw.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_b64_required"}),
                    1,
                );
            }
            let Some(cmd_payload) = decode_b64_json(&raw) else {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_b64_invalid"}),
                    1,
                );
            };
            let mut next_args = vec!["receive".to_string()];
            next_args.push(format!("--command={}", stable_json_string(&cmd_payload)));
            run_remote_emergency_halt(repo_root, &next_args)
        }
        "receive" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"remote_emergency_halt_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let raw_cmd = clean(flag(&parsed, "command").unwrap_or(""), 32_000);
            if raw_cmd.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_required"}),
                    1,
                );
            }
            let mut payload = match serde_json::from_str::<Value>(&raw_cmd) {
                Ok(v) => v,
                Err(_) => {
                    return (
                        json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_json_invalid"}),
                        1,
                    )
                }
            };
            let Some(signature) = payload
                .get("signature")
                .and_then(Value::as_str)
                .map(|v| clean(v, 240))
            else {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"signature_missing"}),
                    1,
                );
            };
            if let Some(obj) = payload.as_object_mut() {
                obj.remove("signature");
            }
            let expected = match hmac_sha256_hex(&key, &payload) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false, "error": clean(err, 220)}), 1),
            };
            if !secure_eq_hex(&signature, &expected) {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"signature_invalid"}),
                    1,
                );
            }
            let now_ms = Utc::now().timestamp_millis();
            let expires_at_ms = payload
                .get("expires_at_ms")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if expires_at_ms <= now_ms - (policy.max_clock_skew_seconds * 1000) {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_expired"}),
                    1,
                );
            }
            let nonce = clean(
                payload.get("nonce").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            if nonce.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"nonce_missing"}),
                    1,
                );
            }
            let mut nonce_doc =
                read_json_or(&nonce_store_path, json!({"version":1, "entries": {}}));
            let entries = nonce_doc
                .get_mut("entries")
                .and_then(Value::as_object_mut)
                .cloned()
                .unwrap_or_default();
            let mut entries_mut = entries;
            clean_expired_nonces(&mut entries_mut, now_ms);
            if entries_mut.contains_key(&nonce) {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"nonce_replay"}),
                    1,
                );
            }
            entries_mut.insert(
                nonce.clone(),
                Value::Number((now_ms + policy.replay_nonce_ttl_seconds * 1000).into()),
            );
            if let Some(obj) = nonce_doc.as_object_mut() {
                obj.insert("entries".to_string(), Value::Object(entries_mut));
                obj.insert("updated_at".to_string(), Value::String(now_iso()));
            }
            let _ = write_json_atomic(&nonce_store_path, &nonce_doc);

            let action = clean(
                payload.get("action").and_then(Value::as_str).unwrap_or(""),
                40,
            )
            .to_ascii_lowercase();
            let mut state = read_json_or(
                &state_path,
                json!({"halted": false, "updated_at": null, "last_command_id": null}),
            );
            let mut applied = false;
            let mut purge = json!({"executed": false, "deleted": []});
            if action == "halt" {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("halted".to_string(), Value::Bool(true));
                    obj.insert("updated_at".to_string(), Value::String(now_iso()));
                    obj.insert(
                        "last_command_id".to_string(),
                        payload.get("command_id").cloned().unwrap_or(Value::Null),
                    );
                }
                applied = true;
            } else if action == "purge" {
                if policy.secure_purge.enabled && policy.secure_purge.allow_live_purge {
                    let mut deleted = Vec::<Value>::new();
                    for rel in &policy.secure_purge.sensitive_paths {
                        let path = resolve_runtime_or_state(repo_root, rel);
                        if path.exists() && fs::remove_file(&path).is_ok() {
                            deleted.push(Value::String(normalize_rel(path.to_string_lossy())));
                        }
                    }
                    purge = json!({"executed": true, "deleted": deleted});
                    applied = true;
                } else if let Some(obj) = state.as_object_mut() {
                    obj.insert("purge_pending".to_string(), payload.clone());
                    obj.insert("updated_at".to_string(), Value::String(now_iso()));
                }
            } else {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"unknown_action"}),
                    1,
                );
            }
            let _ = write_json_atomic(&state_path, &state);
            let _ = append_jsonl(
                &audit_path,
                &json!({
                    "ts": now_iso(),
                    "type": "remote_emergency_halt_receive",
                    "action": action,
                    "applied": applied,
                    "command_id": payload.get("command_id").cloned().unwrap_or(Value::Null)
                }),
            );
            (
                json!({
                    "ok": true,
                    "type": "remote_emergency_halt_receive",
                    "action": action,
                    "applied": applied,
                    "state": state,
                    "purge": purge
                }),
                0,
            )
        }
        _ => (
            json!({
                "ok": false,
                "type": "remote_emergency_halt",
                "error": "unknown_command",
                "usage": [
                    "remote-emergency-halt status",
                    "remote-emergency-halt sign-halt --approval-note=<text> [--scope=<scope>] [--ttl-sec=<n>]",
                    "remote-emergency-halt sign-purge --pending-id=<id>",
                    "remote-emergency-halt receive --command=<json>",
                    "remote-emergency-halt receive-b64 --command-b64=<base64>"
                ]
            }),
            2,
        ),
    }
}
