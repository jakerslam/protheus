
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
