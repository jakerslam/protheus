fn summarize_burn(root: &Path) -> Value {
    let latest = read_json(&latest_path(root)).unwrap_or_else(|| json!({}));
    let total_raw_tokens = latest
        .get("total_raw_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_nexus_tokens = latest
        .get("total_nexus_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let fallback_count = latest
        .get("fallback_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let burn_rate = if total_raw_tokens > 0 {
        ((total_nexus_tokens as f64 / total_raw_tokens as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };
    json!({
        "total_raw_tokens": total_raw_tokens,
        "total_nexus_tokens": total_nexus_tokens,
        "internal_token_burn_rate_pct": burn_rate,
        "fallback_count": fallback_count,
    })
}

fn persist_message_event(
    root: &Path,
    command: &str,
    message: &NexusMessage,
    decompressed: &Value,
    raw_text: Option<&str>,
    fallback_used: bool,
) -> Result<Value, String> {
    let nexus_line = format_nexus_message(message);
    let raw_tokens = raw_text.map(estimate_tokens).unwrap_or(estimate_tokens(&nexus_line));
    let nexus_tokens = estimate_tokens(&nexus_line);
    let savings_pct = estimate_savings(raw_tokens, nexus_tokens);
    let row = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_message",
        "ts_epoch_ms": now_epoch_ms(),
        "command": command,
        "message": nexus_line,
        "decompressed": decompressed,
        "raw_text": raw_text,
        "raw_tokens": raw_tokens,
        "nexus_tokens": nexus_tokens,
        "savings_pct": savings_pct,
        "fallback_used": fallback_used
    }));
    append_jsonl(&messages_path(root), &row)?;
    let current = read_json(&latest_path(root)).unwrap_or_else(|| json!({}));
    let updated = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_latest",
        "updated_ts_epoch_ms": now_epoch_ms(),
        "last_message": row,
        "total_raw_tokens": current.get("total_raw_tokens").and_then(Value::as_u64).unwrap_or(0) + raw_tokens as u64,
        "total_nexus_tokens": current.get("total_nexus_tokens").and_then(Value::as_u64).unwrap_or(0) + nexus_tokens as u64,
        "fallback_count": current.get("fallback_count").and_then(Value::as_u64).unwrap_or(0) + if fallback_used {1} else {0}
    }));
    write_json(&latest_path(root), &updated)?;
    Ok(row)
}

fn validate_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let message_raw = parse_flag(argv, "message").unwrap_or_default();
    if message_raw.trim().is_empty() {
        return (
            error_payload(
                "nexus_internal_comms_error",
                "validate",
                "missing_message_flag",
            ),
            2,
        );
    }
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "validate", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "validate", &e), 3),
    };
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "validate", &e), 2),
    };
    if let Err(e) = validate_module_rules(&message, &modules) {
        return (error_payload("nexus_internal_comms_error", "validate", &e), 3);
    }
    let decompressed = decompress_message(&message, &lexicon);
    let mut out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_validate",
        "format": "[FROM>TO|MOD] CMD k=v k=v ...",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "modules_loaded": modules,
        "claim_evidence": [
            {
                "id": "V6-INTERNAL-COMMS-001.1",
                "claim": "nexus_messages_use_strict_one_line_format_and_deterministic_parser",
                "evidence": {
                    "validated": true
                }
            }
        ]
    }));
    let _ = persist_message_event(root, "validate", &message, &decompressed, None, false);
    out["burn"] = summarize_burn(root);
    (out, 0)
}

fn compress_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let from = parse_flag(argv, "from").unwrap_or_else(|| "AG".to_string());
    let to = parse_flag(argv, "to").unwrap_or_else(|| "COORD".to_string());
    let cmd = parse_flag(argv, "cmd").unwrap_or_else(|| "LOG".to_string());
    let text = parse_flag(argv, "text").unwrap_or_default();
    if text.trim().is_empty() {
        return (
            error_payload(
                "nexus_internal_comms_error",
                "compress",
                "missing_text_flag",
            ),
            2,
        );
    }
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "compress", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "compress", &e), 3),
    };
    let reverse = reverse_lexicon(&lexicon);
    let module = parse_flag(argv, "module").map(|v| v.to_ascii_lowercase());
    let (message, fallback_used) = compress_text_to_message(&from, &to, module, &cmd, &text, &reverse);
    let decompressed = decompress_message(&message, &lexicon);
    let row = match persist_message_event(
        root,
        "compress",
        &message,
        &decompressed,
        Some(text.as_str()),
        fallback_used,
    ) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "compress", &e), 2),
    };
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_compress",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "fallback_used": fallback_used,
        "modules_loaded": modules,
        "event_receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "burn": summarize_burn(root),
        "claim_evidence": [
            {
                "id": "V6-INTERNAL-COMMS-001.4",
                "claim": "compressor_uses_lexicon_first_and_falls_back_to_nl_atom_when_needed",
                "evidence": {
                    "fallback_used": fallback_used
                }
            }
        ]
    }));
    (out, 0)
}

fn decompress_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let message_raw = parse_flag(argv, "message").unwrap_or_default();
    if message_raw.trim().is_empty() {
        return (
            error_payload(
                "nexus_internal_comms_error",
                "decompress",
                "missing_message_flag",
            ),
            2,
        );
    }
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "decompress", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "decompress", &e), 3),
    };
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "decompress", &e), 2),
    };
    let decompressed = decompress_message(&message, &lexicon);
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_decompress",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "modules_loaded": modules
    }));
    let _ = persist_message_event(root, "decompress", &message, &decompressed, None, false);
    (out, 0)
}

fn send_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let message_raw = parse_flag(argv, "message").unwrap_or_default();
    if message_raw.trim().is_empty() {
        return (
            error_payload("nexus_internal_comms_error", "send", "missing_message_flag"),
            2,
        );
    }
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 3),
    };
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 2),
    };
    if let Err(e) = validate_module_rules(&message, &modules) {
        return (error_payload("nexus_internal_comms_error", "send", &e), 3);
    }
    let decompressed = decompress_message(&message, &lexicon);
    let raw_text = parse_flag(argv, "raw-text");
    let row = match persist_message_event(
        root,
        "send",
        &message,
        &decompressed,
        raw_text.as_deref(),
        false,
    ) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 2),
    };
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_send",
        "accepted": true,
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "modules_loaded": modules,
        "event_receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "burn": summarize_burn(root),
        "claim_evidence": [
            {
                "id": "V6-INTERNAL-COMMS-001.6",
                "claim": "nexus_send_events_emit_receipts_with_compressed_and_decompressed_views",
                "evidence": {
                    "logged": true
                }
            }
        ]
    }));
    (out, 0)
}

fn log_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let limit = parse_limit(parse_flag(argv, "limit"));
    let show_decompressed = parse_bool(parse_flag(argv, "decompressed"), true);
    let mut rows = read_recent_jsonl(&messages_path(root), limit);
    if !show_decompressed {
        for row in &mut rows {
            if let Some(obj) = row.as_object_mut() {
                obj.remove("decompressed");
            }
        }
    }
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_log",
        "limit": limit,
        "messages": rows,
        "burn": summarize_burn(root)
    }));
    (out, 0)
}

fn agent_prompt_command(_root: &Path, argv: &[String]) -> (Value, i32) {
    let agent = normalize_id(&parse_flag(argv, "agent").unwrap_or_else(|| "AG".to_string()));
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "agent-prompt", &e), 2),
    };
    let prompt = format!(
        "You communicate internally using the Nexus protocol for maximum efficiency.\nFormat: [FROM>TO|MOD] CMD k=v k=v ...\nLoaded Core + modules: {}\nUse lexicon keys whenever possible. Only use natural language fallback when lexicon cannot express the idea clearly.\nBe concise and deterministic.",
        if modules.is_empty() {
            "core".to_string()
        } else {
            format!("core,{}", modules.join(","))
        }
    );
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_agent_prompt",
        "agent": agent,
        "modules_loaded": modules,
        "prompt": prompt
    }));
    (out, 0)
}

fn export_lexicon_command(_root: &Path, argv: &[String]) -> (Value, i32) {
    let modules = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "export-lexicon", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "export-lexicon", &e), 3),
    };
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_lexicon_export",
        "core_symbol_count": core_lexicon_entries().len(),
        "module_catalog_count": module_catalog().len(),
        "modules_loaded": modules,
        "lexicon": lexicon
    }));
    (out, 0)
}

fn status_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let limit = parse_limit(parse_flag(argv, "limit"));
    let modules = parse_modules(argv).unwrap_or_default();
    let lexicon = active_lexicon(&modules).unwrap_or_default();
    let recent = read_recent_jsonl(&messages_path(root), limit);
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_status",
        "format": "[FROM>TO|MOD] CMD k=v k=v ...",
        "core_symbol_count": core_lexicon_entries().len(),
        "module_catalog_count": module_catalog().len(),
        "active_symbol_count": lexicon.len(),
        "max_modules_per_agent": MAX_MODULES_PER_AGENT,
        "modules_loaded": modules,
        "recent_messages": recent,
        "burn": summarize_burn(root),
        "paths": {
            "messages_path": messages_path(root).to_string_lossy().to_string(),
            "latest_path": latest_path(root).to_string_lossy().to_string()
        },
        "claim_evidence": [
            {
                "id": "V6-INTERNAL-COMMS-001.6",
                "claim": "nexus_status_reports_internal_token_burn_and_receipted_message_history",
                "evidence": {
                    "recent_count": limit
                }
            }
        ]
    }));
    (out, 0)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let (payload, exit_code) = match cmd.as_str() {
        "status" => status_command(root, argv),
        "validate" => validate_command(root, argv),
        "compress" => compress_command(root, argv),
        "decompress" => decompress_command(root, argv),
        "send" => send_command(root, argv),
        "log" => log_command(root, argv),
        "agent-prompt" | "prompt" => agent_prompt_command(root, argv),
        "export-lexicon" => export_lexicon_command(root, argv),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => (
            error_payload("nexus_internal_comms_error", cmd.as_str(), "unknown_command"),
            1,
        ),
    };
    print_json(&payload);
    exit_code
}
