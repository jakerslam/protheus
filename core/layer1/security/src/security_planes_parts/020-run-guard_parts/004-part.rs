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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct SoulTokenGuardPolicy {
    version: String,
    enabled: bool,
    enforcement_mode: String,
    bind_to_fingerprint: bool,
    default_attestation_valid_hours: i64,
    key_env: String,
    token_state_path: String,
    audit_path: String,
    attestation_path: String,
}

impl Default for SoulTokenGuardPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            enabled: true,
            enforcement_mode: "advisory".to_string(),
            bind_to_fingerprint: true,
            default_attestation_valid_hours: 24 * 7,
            key_env: "SOUL_TOKEN_GUARD_KEY".to_string(),
            token_state_path: "local/state/security/soul_token_guard.json".to_string(),
            audit_path: "local/state/security/soul_token_guard_audit.jsonl".to_string(),
            attestation_path: "local/state/security/release_attestations.jsonl".to_string(),
        }
    }
}

fn load_soul_token_policy(repo_root: &Path, parsed: &ParsedArgs) -> SoulTokenGuardPolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "soul_token_guard_policy.json"));
    if !policy_path.exists() {
        return SoulTokenGuardPolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<SoulTokenGuardPolicy>(&raw).unwrap_or_default(),
        Err(_) => SoulTokenGuardPolicy::default(),
    }
}

fn soul_fingerprint(repo_root: &Path) -> String {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let seed = format!(
        "{}|{}|{}|{}",
        hostname,
        std::env::consts::OS,
        std::env::consts::ARCH,
        repo_root.display()
    );
    format!("fp_{}", &sha256_hex_bytes(seed.as_bytes())[0..16])
}

fn read_jsonl_rows(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

pub fn run_soul_token_guard(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_soul_token_policy(repo_root, &parsed);
    let token_state_path = resolve_runtime_or_state(repo_root, &policy.token_state_path);
    let audit_path = resolve_runtime_or_state(repo_root, &policy.audit_path);
    let attestation_path = resolve_runtime_or_state(repo_root, &policy.attestation_path);
    let key = std::env::var(&policy.key_env)
        .ok()
        .map(|v| clean(v, 4096))
        .unwrap_or_default();

    match cmd.as_str() {
        "issue" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"soul_token_issue", "error":"soul_token_guard_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let instance_id = clean(
                flag(&parsed, "instance-id")
                    .or_else(|| flag(&parsed, "instance_id"))
                    .unwrap_or("default"),
                160,
            );
            let approval_note = clean(
                flag(&parsed, "approval-note")
                    .or_else(|| flag(&parsed, "approval_note"))
                    .unwrap_or(""),
                400,
            );
            let token_id = format!(
                "stg_{}",
                &sha256_hex_bytes(format!("{}|{}", now_iso(), instance_id).as_bytes())[0..12]
            );
            let fingerprint = soul_fingerprint(repo_root);
            let payload = json!({
                "token_id": token_id,
                "instance_id": instance_id,
                "issued_at": now_iso(),
                "fingerprint": fingerprint,
                "approval_note": approval_note
            });
            let signature = match hmac_sha256_hex(&key, &payload) {
                Ok(v) => v,
                Err(err) => return (json!({"ok": false, "error": clean(err, 220)}), 1),
            };
            let state = json!({
                "version": policy.version,
                "token": payload,
                "signature": signature,
                "issued_at": now_iso()
            });
            if let Err(err) = write_json_atomic(&token_state_path, &state) {
                return (
                    json!({"ok": false, "type":"soul_token_issue", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({"ts": now_iso(), "type": "soul_token_issue", "token_id": state.get("token").and_then(|v| v.get("token_id")).cloned().unwrap_or(Value::Null)}),
            );
            (
                json!({
                    "ok": true,
                    "type": "soul_token_issue",
                    "token_id": state.get("token").and_then(|v| v.get("token_id")).cloned().unwrap_or(Value::Null),
                    "token_state_path": normalize_rel(token_state_path.to_string_lossy())
                }),
                0,
            )
        }
        "stamp-build" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"soul_token_stamp_build", "error":"soul_token_guard_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let build_id = clean(
                flag(&parsed, "build-id")
                    .or_else(|| flag(&parsed, "build_id"))
                    .unwrap_or(""),
                180,
            );
            if build_id.is_empty() {
                return (
                    json!({"ok": false, "type":"soul_token_stamp_build", "error":"build_id_required"}),
                    1,
                );
            }
            let channel = clean(flag(&parsed, "channel").unwrap_or("default"), 80);
            let valid_hours = flag(&parsed, "valid-hours")
                .or_else(|| flag(&parsed, "valid_hours"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(policy.default_attestation_valid_hours)
                .clamp(1, 24 * 365);
            let now_ms = Utc::now().timestamp_millis();
            let attestation = json!({
                "type": "release_attestation",
                "build_id": build_id,
                "channel": channel,
                "issued_at": now_iso(),
                "expires_at_ms": now_ms + valid_hours * 3600 * 1000,
                "token_id": read_json_or(&token_state_path, Value::Null).get("token").and_then(|v| v.get("token_id")).cloned().unwrap_or(Value::Null)
            });
            let signature = match hmac_sha256_hex(&key, &attestation) {
                Ok(v) => v,
                Err(err) => return (json!({"ok": false, "error": clean(err, 220)}), 1),
            };
            let row = json!({"attestation": attestation, "signature": signature});
            if let Err(err) = append_jsonl(&attestation_path, &row) {
                return (
                    json!({"ok": false, "type":"soul_token_stamp_build", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({"ts": now_iso(), "type": "soul_token_stamp_build", "build_id": build_id}),
            );
            (
                json!({"ok": true, "type": "soul_token_stamp_build", "attestation": row}),
                0,
            )
        }
        "verify" => {
            let strict = bool_flag(&parsed, "strict", false);
            if key.is_empty() {
                let out = json!({"ok": false, "type":"soul_token_verify", "error":"soul_token_guard_key_missing", "key_env": policy.key_env});
                return (out, if strict { 1 } else { 0 });
            }
            let state = read_json_or(&token_state_path, Value::Null);
            let token = state.get("token").cloned().unwrap_or(Value::Null);
            let signature = clean(
                state.get("signature").and_then(Value::as_str).unwrap_or(""),
                240,
            );
            let mut ok = state.is_object() && token.is_object() && !signature.is_empty();
            let mut reason = "verified".to_string();
            if !ok {
                reason = "token_state_missing".to_string();
            } else {
                let expected = hmac_sha256_hex(&key, &token).unwrap_or_default();
                if !secure_eq_hex(&signature, &expected) {
                    ok = false;
                    reason = "signature_mismatch".to_string();
                } else if policy.bind_to_fingerprint {
                    let expected_fp = soul_fingerprint(repo_root);
                    let token_fp = clean(
                        token
                            .get("fingerprint")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        200,
                    );
                    if expected_fp != token_fp {
                        ok = false;
                        reason = "fingerprint_mismatch".to_string();
                    }
                }
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({"ts": now_iso(), "type": "soul_token_verify", "ok": ok, "reason": reason}),
            );
            let out = json!({
                "ok": ok,
                "type": "soul_token_verify",
                "reason": reason,
                "enforcement_mode": policy.enforcement_mode,
                "token_state_path": normalize_rel(token_state_path.to_string_lossy())
            });
            (out.clone(), if ok || !strict { 0 } else { 1 })
        }
        "status" => {
            let rows = read_jsonl_rows(&attestation_path);
            (
                json!({
                    "ok": true,
                    "type": "soul_token_status",
                    "ts": now_iso(),
                    "enabled": policy.enabled,
                    "enforcement_mode": policy.enforcement_mode,
                    "key_env": policy.key_env,
                    "token_state_path": normalize_rel(token_state_path.to_string_lossy()),
                    "token_state": read_json_or(&token_state_path, Value::Null),
                    "attestation_path": normalize_rel(attestation_path.to_string_lossy()),
                    "attestation_count": rows.len(),
                    "latest_attestation": rows.last().cloned().unwrap_or(Value::Null)
                }),
                0,
            )
        }
        _ => (
            json!({
                "ok": false,
                "type": "soul_token_guard",
                "error": "unknown_command",
                "usage": [
                    "soul-token-guard issue [--instance-id=<id>] [--approval-note=<text>]",
                    "soul-token-guard stamp-build --build-id=<id> [--channel=<name>] [--valid-hours=<n>]",
                    "soul-token-guard verify [--strict=1]",
                    "soul-token-guard status"
                ]
            }),
            2,
        ),
    }
}
