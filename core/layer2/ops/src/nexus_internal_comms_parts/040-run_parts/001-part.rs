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
    let (task, role, context_text) = context_flags(argv);
    let extra = context_text.unwrap_or_else(|| text.clone());
    let seeded_modules = parse_flag(argv, "module")
        .map(|module| vec![normalize_module_name(module.as_str())])
        .unwrap_or_default();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        Some(extra.as_str()),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "compress", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "compress", &e),
                3,
            )
        }
    };
    let reverse = reverse_lexicon(&lexicon);
    let module = parse_flag(argv, "module")
        .map(|value| normalize_module_name(value.as_str()))
        .filter(|value| !value.is_empty());
    let (message, fallback_used) =
        compress_text_to_message(&from, &to, module, &cmd, &text, &reverse);
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
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "compress", &e),
                2,
            )
        }
    };
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_compress",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "fallback_used": fallback_used,
        "modules_loaded": modules,
        "event_receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "perf_proof": row.get("perf_proof").cloned().unwrap_or(Value::Null),
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
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "decompress", &e),
                2,
            )
        }
    };
    let (task, role, text) = context_flags(argv);
    let seeded_modules = message.module.clone().into_iter().collect::<Vec<String>>();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        text.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "decompress", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "decompress", &e),
                3,
            )
        }
    };
    let decompressed = decompress_message(&message, &lexicon);
    let mut out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_decompress",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "modules_loaded": modules
    }));
    if let Ok(row) = persist_message_event(root, "decompress", &message, &decompressed, None, false)
    {
        out["perf_proof"] = row.get("perf_proof").cloned().unwrap_or(Value::Null);
    }
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
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 2),
    };
    let (task, role, text) = context_flags(argv);
    let raw_text = parse_flag(argv, "raw-text");
    let seeded_modules = message.module.clone().into_iter().collect::<Vec<String>>();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        raw_text.as_deref().or(text.as_deref()),
    ) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "send", &e), 3),
    };
    if let Err(e) = validate_module_rules(&message, &modules) {
        return (error_payload("nexus_internal_comms_error", "send", &e), 3);
    }
    let decompressed = decompress_message(&message, &lexicon);
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
        "perf_proof": row.get("perf_proof").cloned().unwrap_or(Value::Null),
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
    let (task, role, text) = context_flags(argv);
    let seeded_modules = parse_flag(argv, "module")
        .map(|module| vec![normalize_module_name(module.as_str())])
        .unwrap_or_default();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        text.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "agent-prompt", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "agent-prompt", &e),
                3,
            )
        }
    };
    let ranked_modules = module_context_scores(task.as_deref(), role.as_deref(), text.as_deref())
        .into_iter()
        .take(8)
        .map(|(module, score)| json!({"module": module, "score": score}))
        .collect::<Vec<_>>();
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
        "task": task,
        "role": role,
        "modules_loaded": modules,
        "active_symbol_count": lexicon.len(),
        "module_context_scores": ranked_modules,
        "prompt": prompt
    }));
    (out, 0)
}

fn resolve_modules_command(_root: &Path, argv: &[String]) -> (Value, i32) {
    let explicit = match parse_modules(argv) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "resolve-modules", &e),
                2,
            )
        }
    };
    let seeded_modules = parse_flag(argv, "module")
        .map(|module| vec![normalize_module_name(module.as_str())])
        .unwrap_or_default();
    let (task, role, text) = context_flags(argv);
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        text.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "resolve-modules", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "resolve-modules", &e),
                3,
            )
        }
    };
    let ranked_modules = module_context_scores(task.as_deref(), role.as_deref(), text.as_deref())
        .into_iter()
        .take(8)
        .map(|(module, score)| json!({"module": module, "score": score}))
        .collect::<Vec<_>>();
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_resolve_modules",
        "task": task,
        "role": role,
        "text": text,
        "explicit_modules": explicit,
        "seeded_modules": seeded_modules,
        "modules_loaded": modules,
        "active_symbol_count": lexicon.len(),
        "core_symbol_count": core_lexicon_entries().len(),
        "module_catalog_count": module_catalog().len(),
        "ranked_candidates": ranked_modules
    }));
    (out, 0)
}

fn export_lexicon_command(_root: &Path, argv: &[String]) -> (Value, i32) {
    let (task, role, text) = context_flags(argv);
    let seeded_modules = parse_flag(argv, "module")
        .map(|module| vec![normalize_module_name(module.as_str())])
        .unwrap_or_default();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        text.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "export-lexicon", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "export-lexicon", &e),
                3,
            )
        }
    };
    let with_catalog = parse_bool(parse_flag(argv, "with-catalog"), true);
    let core_lexicon = core_lexicon_entries()
        .into_iter()
        .map(|(code, phrase)| (code.to_string(), phrase.to_string()))
        .collect::<BTreeMap<_, _>>();
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_lexicon_export",
        "core_symbol_count": core_lexicon_entries().len(),
        "module_catalog_count": module_catalog().len(),
        "task": task,
        "role": role,
        "seeded_modules": seeded_modules,
        "modules_loaded": modules,
        "core_lexicon": core_lexicon,
        "module_catalog": if with_catalog { module_catalog_manifest() } else { Value::Null },
        "lexicon": lexicon
    }));
    (out, 0)
}
