
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
