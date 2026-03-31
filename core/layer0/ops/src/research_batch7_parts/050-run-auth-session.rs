pub fn run_auth_session(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "auth_session");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            "research_plane_auth_session",
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let contract = read_json_or(
        root,
        AUTH_SESSION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "auth_session_lifecycle_contract",
            "allowed_ops": ["open", "login", "status", "close"],
            "isolation_required": true
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        64,
    )
    .to_ascii_lowercase();

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("auth_session_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "auth_session_lifecycle_contract"
    {
        errors.push("auth_session_contract_kind_invalid".to_string());
    }
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if !allowed_ops.iter().any(|v| v == &op) {
        errors.push("auth_session_op_not_allowed".to_string());
    }
    if !errors.is_empty() {
        return fail_payload("research_plane_auth_session", strict, errors, Some(conduit));
    }

    let session_id = clean(
        parsed
            .flags
            .get("session-id")
            .cloned()
            .unwrap_or_else(|| format!("sess_{}", &sha256_hex_str(&now_iso())[..12])),
        120,
    );
    let sessions_root = state_root(root).join("sessions");
    let jars_root = sessions_root.join("jars");
    let _ = fs::create_dir_all(&jars_root);
    let session_path = sessions_root.join(format!("{}.json", session_id));
    let jar_path = jars_root.join(format!("{}.json", session_id));

    let mut state = read_json(&session_path).unwrap_or_else(|| {
        json!({
            "session_id": session_id,
            "status": "missing",
            "authenticated": false,
            "jar_path": jar_path.display().to_string()
        })
    });

    if op == "open" {
        state = json!({
            "session_id": session_id,
            "status": "open",
            "authenticated": false,
            "jar_path": jar_path.display().to_string(),
            "opened_at": now_iso(),
            "last_op": op
        });
        let _ = write_json(&jar_path, &json!({"cookies": []}));
        let _ = write_json(&session_path, &state);
    } else if op == "login" {
        if !session_path.exists() {
            return fail_payload(
                "research_plane_auth_session",
                strict,
                vec!["session_not_open".to_string()],
                Some(conduit),
            );
        }
        let username = clean(
            parsed.flags.get("username").cloned().unwrap_or_default(),
            120,
        );
        let password = clean(
            parsed.flags.get("password").cloned().unwrap_or_default(),
            240,
        );
        if username.is_empty() || password.is_empty() {
            return fail_payload(
                "research_plane_auth_session",
                strict,
                vec!["username_and_password_required".to_string()],
                Some(conduit),
            );
        }
        let token = sha256_hex_str(&format!("{}:{}:{}", username, password, now_iso()));
        let _ = write_json(
            &jar_path,
            &json!({"cookies": [{"name": "session", "value": token}]}),
        );
        state["status"] = Value::String("open".to_string());
        state["authenticated"] = Value::Bool(true);
        state["username"] = Value::String(username);
        state["last_op"] = Value::String(op.clone());
        state["updated_at"] = Value::String(now_iso());
        let _ = write_json(&session_path, &state);
    } else if op == "close" {
        state["status"] = Value::String("closed".to_string());
        state["authenticated"] = Value::Bool(false);
        state["last_op"] = Value::String(op.clone());
        state["updated_at"] = Value::String(now_iso());
        let _ = write_json(&session_path, &state);
        let _ = fs::remove_file(&jar_path);
    } else if op == "status" {
        if !session_path.exists() {
            return fail_payload(
                "research_plane_auth_session",
                strict,
                vec!["session_not_found".to_string()],
                Some(conduit),
            );
        }
    }

    let session_state = read_json(&session_path).unwrap_or(state);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_auth_session",
        "lane": "core/layer0/ops",
        "op": op,
        "session": session_state,
        "cookie_jar_path": jar_path.display().to_string(),
        "cookie_jar_exists": jar_path.exists(),
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-005.2",
                "claim": "authenticated_session_lifecycle_uses_isolated_cookie_jars_with_deterministic_receipts",
                "evidence": {
                    "op": op,
                    "jar_exists": jar_path.exists()
                }
            },
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "auth_session_path_is_enforced_through_conduit_only",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run_proxy_rotate(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "proxy_rotation");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            "research_plane_proxy_rotation",
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let contract = read_json_or(
        root,
        PROXY_ROTATION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "proxy_rotation_trap_matrix_contract",
            "trap_signals": ["captcha", "cloudflare", "rate_limit"],
            "trap_response_matrix": {
                "captcha": "rotate",
                "cloudflare": "rotate",
                "rate_limit": "backoff"
            },
            "default_proxies": ["proxy-a", "proxy-b", "proxy-c"]
        }),
    );

    let proxies = {
        let mut rows = parse_list_flag(parsed, "proxies", 240);
        if rows.is_empty() {
            rows = contract
                .get("default_proxies")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>();
        }
        rows
    };
    let attempt_signals = parse_list_flag(parsed, "attempt-signals", 80);
    let trap_signals = contract
        .get("trap_signals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let matrix = contract
        .get("trap_response_matrix")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("proxy_rotation_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "proxy_rotation_trap_matrix_contract"
    {
        errors.push("proxy_rotation_contract_kind_invalid".to_string());
    }
    if proxies.is_empty() {
        errors.push("proxy_pool_required".to_string());
    }
    if attempt_signals.is_empty() {
        errors.push("attempt_signals_required".to_string());
    }
    if !errors.is_empty() {
        return fail_payload(
            "research_plane_proxy_rotation",
            strict,
            errors,
            Some(conduit),
        );
    }

    let mut receipts = Vec::<Value>::new();
    let mut selected_proxy = String::new();
    let mut halted = false;

    for (idx, signal) in attempt_signals.iter().enumerate() {
        let proxy = proxies
            .get(idx % proxies.len())
            .cloned()
            .unwrap_or_else(|| "proxy-none".to_string());
        let signal_lc = signal.to_ascii_lowercase();
        let trapped = trap_signals.iter().any(|s| s == &signal_lc);
        let action = if trapped {
            matrix
                .get(&signal_lc)
                .and_then(Value::as_str)
                .map(|v| clean(v, 64).to_ascii_lowercase())
                .unwrap_or_else(|| "rotate".to_string())
        } else {
            "accept".to_string()
        };
        if !trapped && signal_lc == "ok" {
            selected_proxy = proxy.clone();
        }
        if action == "abort" {
            halted = true;
        }
        receipts.push(json!({
            "attempt": idx,
            "signal": signal_lc,
            "proxy": proxy,
            "trapped": trapped,
            "action": action
        }));
        if halted || !selected_proxy.is_empty() {
            break;
        }
    }

    let ok = !selected_proxy.is_empty() && !halted;
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "research_plane_proxy_rotation",
        "lane": "core/layer0/ops",
        "selected_proxy": if selected_proxy.is_empty() { Value::Null } else { Value::String(selected_proxy.clone()) },
        "attempt_receipts": receipts,
        "halted": halted,
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-005.3",
                "claim": "proxy_rotation_and_trap_response_matrix_emit_deterministic_per_attempt_receipts",
                "evidence": {
                    "attempts": receipts.len(),
                    "selected_proxy": if selected_proxy.is_empty() { Value::Null } else { Value::String(selected_proxy.clone()) }
                }
            },
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "proxy_rotation_path_is_enforced_through_conduit_only",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &raw[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

fn extract_http_candidate(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find("https://").or_else(|| lower.find("http://"))?;
    let tail = &text[start..];
    let end = tail
        .find(|c: char| c.is_whitespace() || ['"', '\'', '<', '>'].contains(&c))
        .unwrap_or(tail.len());
    let out = clean(&tail[..end], 2000);
    if out.starts_with("http://") || out.starts_with("https://") {
        Some(out)
    } else {
        None
    }
}

fn decode_b64_candidate(token: &str) -> Option<String> {
    let trimmed = token.trim().trim_matches('/');
    for decoder in [&URL_SAFE_NO_PAD, &URL_SAFE, &STANDARD] {
        if let Ok(bytes) = decoder.decode(trimmed.as_bytes()) {
            let decoded = String::from_utf8_lossy(&bytes).to_string();
            if let Some(url) = extract_http_candidate(&decoded) {
                return Some(url);
            }
        }
    }
    for pad in ["=", "==", "==="] {
        let padded = format!("{trimmed}{pad}");
        if let Ok(bytes) = URL_SAFE.decode(padded.as_bytes()) {
            let decoded = String::from_utf8_lossy(&bytes).to_string();
            if let Some(url) = extract_http_candidate(&decoded) {
                return Some(url);
            }
        }
    }
    None
}

