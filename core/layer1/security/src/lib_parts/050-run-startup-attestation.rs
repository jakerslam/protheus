pub fn run_startup_attestation(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let strict = bool_flag(&parsed, "strict", false);
    let policy_path = startup_policy_path(repo_root);
    let state_path = startup_state_path(repo_root);
    let audit_path = startup_audit_path(repo_root);
    let policy = load_startup_policy(&policy_path);

    match cmd.as_str() {
        "issue" => {
            let Some(secret) = startup_resolve_secret(repo_root) else {
                let out = json!({"ok": false, "reason": "attestation_key_missing"});
                return (out, if strict { 1 } else { 0 });
            };
            let ttl_hours = flag(&parsed, "ttl-hours")
                .or_else(|| flag(&parsed, "ttl_hours"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(policy.ttl_hours)
                .clamp(1, 240);
            let ts = now_iso();
            let now_ms = Utc::now().timestamp_millis();
            let expires_at_ms = now_ms + ttl_hours * 3600 * 1000;
            let (critical_hashes, missing_paths) = startup_hash_critical_paths(repo_root, &policy);
            let mut payload = Map::<String, Value>::new();
            payload.insert(
                "type".to_string(),
                Value::String("startup_attestation".to_string()),
            );
            payload.insert("version".to_string(), Value::String(policy.version.clone()));
            payload.insert("ts".to_string(), Value::String(ts.clone()));
            payload.insert(
                "expires_at".to_string(),
                Value::String(
                    chrono::DateTime::<Utc>::from_timestamp_millis(expires_at_ms)
                        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
                        .unwrap_or_else(now_iso),
                ),
            );
            payload.insert("ttl_hours".to_string(), Value::Number(ttl_hours.into()));
            let policy_rel = policy_path
                .strip_prefix(runtime_root(repo_root))
                .unwrap_or(&policy_path)
                .to_string_lossy()
                .replace('\\', "/");
            payload.insert("policy_path".to_string(), Value::String(policy_rel));
            payload.insert(
                "critical_hashes".to_string(),
                Value::Array(critical_hashes.clone()),
            );
            payload.insert(
                "missing_paths".to_string(),
                Value::Array(
                    missing_paths
                        .iter()
                        .map(|v| Value::String(v.clone()))
                        .collect(),
                ),
            );
            let payload_value = Value::Object(payload.clone());
            let signature = match hmac_sha256_hex(&secret, &stable_json_string(&payload_value)) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false,"error":clean(err,220)}), 1),
            };
            payload.insert("signature".to_string(), Value::String(signature));
            let signed = Value::Object(payload);
            if let Err(err) = write_json_atomic(&state_path, &signed) {
                return (json!({"ok":false,"error":clean(err,220)}), 1);
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({
                    "ts": now_iso(),
                    "type": "startup_attestation_issue",
                    "ok": true,
                    "expires_at": signed.get("expires_at").cloned().unwrap_or(Value::Null),
                    "hashes": critical_hashes.len(),
                    "missing_paths": missing_paths.len()
                }),
            );
            (
                json!({
                    "ok": true,
                    "type": "startup_attestation_issue",
                    "ts": ts,
                    "expires_at": signed.get("expires_at").cloned().unwrap_or(Value::Null),
                    "hashes": critical_hashes.len(),
                    "missing_paths": missing_paths
                }),
                0,
            )
        }
        "verify" | "run" | "check" => {
            let secret = startup_resolve_secret(repo_root);
            let state = read_json_or(&state_path, Value::Null);
            let mut ok = true;
            let mut reason = "verified".to_string();
            let mut drift = Value::Null;
            let expires_at = state.get("expires_at").cloned().unwrap_or(Value::Null);

            if !state.is_object()
                || state.get("type").and_then(Value::as_str) != Some("startup_attestation")
            {
                ok = false;
                reason = "attestation_missing_or_invalid".to_string();
            } else if secret.is_none() {
                ok = false;
                reason = "attestation_key_missing".to_string();
            } else {
                let exp = state
                    .get("expires_at")
                    .and_then(Value::as_str)
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0);
                if exp <= Utc::now().timestamp_millis() {
                    ok = false;
                    reason = "attestation_stale".to_string();
                } else {
                    let signature = clean(
                        state.get("signature").and_then(Value::as_str).unwrap_or(""),
                        240,
                    )
                    .to_ascii_lowercase();
                    if signature.is_empty() {
                        ok = false;
                        reason = "signature_missing".to_string();
                    } else {
                        let mut payload = state.clone();
                        if let Some(obj) = payload.as_object_mut() {
                            obj.remove("signature");
                        }
                        let expected = hmac_sha256_hex(
                            &secret.unwrap_or_default(),
                            &stable_json_string(&payload),
                        )
                        .unwrap_or_default();
                        if !secure_eq_hex(&signature, &expected) {
                            ok = false;
                            reason = "signature_mismatch".to_string();
                        } else {
                            let (current_rows, _) = startup_hash_critical_paths(repo_root, &policy);
                            let expected_rows = state
                                .get("critical_hashes")
                                .and_then(Value::as_array)
                                .cloned()
                                .unwrap_or_default();
                            let mut expected_map = BTreeMap::<String, String>::new();
                            for row in expected_rows {
                                let p = row.get("path").and_then(Value::as_str).unwrap_or("");
                                let h = row.get("sha256").and_then(Value::as_str).unwrap_or("");
                                if !p.is_empty() && !h.is_empty() {
                                    expected_map.insert(p.to_string(), h.to_string());
                                }
                            }
                            let mut drift_rows = Vec::<Value>::new();
                            for row in &current_rows {
                                let p = row.get("path").and_then(Value::as_str).unwrap_or("");
                                let h = row.get("sha256").and_then(Value::as_str).unwrap_or("");
                                let prior = expected_map.get(p).cloned();
                                match prior {
                                    None => {
                                        drift_rows.push(json!({"path": p, "reason": "new_path"}))
                                    }
                                    Some(v) => {
                                        if v != h {
                                            drift_rows.push(
                                                json!({"path": p, "reason": "hash_mismatch"}),
                                            );
                                        }
                                    }
                                }
                            }
                            for p in expected_map.keys() {
                                if !current_rows.iter().any(|row| {
                                    row.get("path").and_then(Value::as_str) == Some(p.as_str())
                                }) {
                                    drift_rows.push(json!({"path": p, "reason": "missing_now"}));
                                }
                            }
                            if !drift_rows.is_empty() {
                                ok = false;
                                reason = "critical_hash_drift".to_string();
                                drift = Value::Array(drift_rows.into_iter().take(50).collect());
                            }
                        }
                    }
                }
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({
                    "ts": now_iso(),
                    "type": "startup_attestation_verify",
                    "ok": ok,
                    "reason": reason
                }),
            );
            let out = json!({
                "ok": ok,
                "type": "startup_attestation_verify",
                "reason": reason,
                "expires_at": expires_at,
                "drift": drift
            });
            let code = if strict && !ok {
                1
            } else if ok {
                0
            } else {
                1
            };
            (out, code)
        }
        "status" => (
            json!({
                "ok": true,
                "type": "startup_attestation_status",
                "policy": policy,
                "state": read_json_or(&state_path, Value::Null),
                "state_path": state_path.to_string_lossy()
            }),
            0,
        ),
        _ => (
            json!({
                "ok": false,
                "error": "unknown_command",
                "usage": [
                    "startup-attestation issue [--ttl-hours=<n>] [--strict=1|0]",
                    "startup-attestation verify [--strict=1|0]",
                    "startup-attestation status"
                ]
            }),
            2,
        ),
    }
}

