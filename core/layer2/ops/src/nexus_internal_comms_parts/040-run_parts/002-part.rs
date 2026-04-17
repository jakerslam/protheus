fn status_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let limit = parse_limit(parse_flag(argv, "limit"));
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
        Err(e) => return (error_payload("nexus_internal_comms_error", "status", &e), 2),
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => return (error_payload("nexus_internal_comms_error", "status", &e), 3),
    };
    let with_catalog = parse_bool(parse_flag(argv, "with-catalog"), false);
    let ranked_modules = module_context_scores(task.as_deref(), role.as_deref(), text.as_deref())
        .into_iter()
        .take(8)
        .map(|(module, score)| json!({"module": module, "score": score}))
        .collect::<Vec<_>>();
    let recent = read_recent_jsonl(&messages_path(root), limit);
    let out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_status",
        "format": "[FROM>TO|MOD] CMD k=v k=v ...",
        "core_symbol_count": core_lexicon_entries().len(),
        "module_catalog_count": module_catalog().len(),
        "active_symbol_count": lexicon.len(),
        "max_modules_per_agent": MAX_MODULES_PER_AGENT,
        "task": task,
        "role": role,
        "seeded_modules": seeded_modules,
        "modules_loaded": modules,
        "ranked_module_candidates": ranked_modules,
        "module_catalog": if with_catalog { module_catalog_manifest() } else { Value::Null },
        "recent_messages": recent,
        "burn": summarize_burn(root),
        "hot_path_allocators": hot_path_allocator_snapshot_json(),
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
        "resolve-modules" | "resolve" => resolve_modules_command(root, argv),
        "export-lexicon" => export_lexicon_command(root, argv),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => (
            error_payload(
                "nexus_internal_comms_error",
                cmd.as_str(),
                "unknown_command",
            ),
            1,
        ),
    };
    print_json(&payload);
    exit_code
}
