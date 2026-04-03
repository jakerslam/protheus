fn run_skill_quarantine_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let mode = parse_subcommand(argv, "status");
    let path = skill_quarantine_state_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| json!({"quarantined": {}}));
    if !state.is_object() {
        state = json!({"quarantined": {}});
    }
    if state
        .get("quarantined")
        .and_then(Value::as_object)
        .is_none()
    {
        state["quarantined"] = json!({});
    }

    let skill_id = parse_flag(argv, "skill-id")
        .or_else(|| parse_flag(argv, "skill"))
        .unwrap_or_default();
    let skill_id = clean(&skill_id, 120);
    let reason = clean(
        parse_flag(argv, "reason").unwrap_or_else(|| "manual".to_string()),
        240,
    );
    let mut ok = true;
    let mut error = Value::Null;

    match mode.as_str() {
        "quarantine" => {
            if skill_id.is_empty() {
                ok = false;
                error = Value::String("skill_id_required".to_string());
            } else {
                state["quarantined"][skill_id.clone()] = json!({
                    "skill_id": skill_id,
                    "reason": reason,
                    "quarantined_at": now_iso(),
                });
                append_jsonl(
                    &skill_quarantine_events_path(root),
                    &json!({
                        "ts": now_iso(),
                        "action": "quarantine",
                        "skill_id": skill_id,
                        "reason": reason
                    }),
                );
            }
        }
        "release" | "unquarantine" => {
            if skill_id.is_empty() {
                ok = false;
                error = Value::String("skill_id_required".to_string());
            } else if let Some(map) = state.get_mut("quarantined").and_then(Value::as_object_mut) {
                map.remove(&skill_id);
                append_jsonl(
                    &skill_quarantine_events_path(root),
                    &json!({
                        "ts": now_iso(),
                        "action": "release",
                        "skill_id": skill_id
                    }),
                );
            }
        }
        "status" => {}
        other => {
            ok = false;
            error = Value::String(format!("unknown_mode:{other}"));
        }
    }

    if ok {
        write_json(&path, &state);
    }
    let count = state
        .get("quarantined")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let out = json!({
        "ok": ok,
        "type": "security_plane_skill_quarantine",
        "strict": strict,
        "mode": mode,
        "error": error,
        "state_path": path.display().to_string(),
        "quarantined_count": count,
        "quarantined": state.get("quarantined").cloned().unwrap_or_else(|| json!({})),
        "claim_evidence": [{
            "id": "V6-SEC-SKILL-QUARANTINE-001",
            "claim": "skills_can_be_quarantined_and_released_with_receipted_state_and_history",
            "evidence": {
                "mode": mode,
                "state_path": path.display().to_string(),
                "quarantined_count": count
            }
        }]
    });
    (out, if strict && !ok { 2 } else { 0 })
}

fn run_autonomous_skill_necessity_audit(
    root: &Path,
    argv: &[String],
    strict: bool,
) -> (Value, i32) {
    let registry = read_json(&skills_registry_path(root)).unwrap_or_else(|| json!({}));
    let installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let required = split_csv(parse_flag(argv, "required-skills"));
    let required_set = required.iter().cloned().collect::<BTreeSet<_>>();
    let mut installed_ids = installed
        .keys()
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>();
    installed_ids.sort();
    let unnecessary = installed_ids
        .iter()
        .filter(|id| !required_set.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    let max_installed = parse_u64(parse_flag(argv, "max-installed"), 24);
    let overloaded = (installed_ids.len() as u64) > max_installed;
    let out = json!({
        "ok": !overloaded,
        "type": "security_plane_autonomous_skill_necessity_audit",
        "strict": strict,
        "registry_path": skills_registry_path(root).display().to_string(),
        "installed_count": installed_ids.len(),
        "max_installed": max_installed,
        "required_skills": required,
        "unnecessary_skills": unnecessary,
        "overloaded": overloaded,
        "claim_evidence": [{
            "id": "V6-SEC-SKILL-AUDIT-001",
            "claim": "autonomous_skill_necessity_audit_flags_skill_sprawl_from_installed_registry_state",
            "evidence": {
                "installed_count": installed_ids.len(),
                "overloaded": overloaded
            }
        }]
    });
    (out, if strict && overloaded { 2 } else { 0 })
}

fn run_repo_hygiene_guard(root: &Path, argv: &[String], strict: bool, mode: &str) -> (Value, i32) {
    let scan_root = parse_flag(argv, "scan-root")
        .map(|value| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    let max_files = parse_u64(parse_flag(argv, "max-files"), 4000) as usize;
    let mut hits = Vec::<Value>::new();
    let mut scanned = 0usize;

    for entry in WalkDir::new(&scan_root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "node_modules" && name != "target"
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        scanned += 1;
        if scanned > max_files {
            break;
        }
        let path = entry.path();
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        let has_conflict =
            raw.contains("<<<<<<<") || raw.contains("=======") && raw.contains(">>>>>>>");
        let has_runtime_stub = raw.contains("compatibility_only\": true");
        let flagged = if mode == "conflict-marker-guard" {
            has_conflict
        } else {
            has_conflict || has_runtime_stub
        };
        if !flagged {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        hits.push(json!({
            "path": rel,
            "conflict_marker": has_conflict,
            "compatibility_stub_marker": has_runtime_stub
        }));
        if hits.len() >= 25 {
            break;
        }
    }

    let blocked = !hits.is_empty();
    let out = json!({
        "ok": !blocked,
        "type": "security_plane_repo_hygiene_guard",
        "strict": strict,
        "mode": mode,
        "scan_root": scan_root.display().to_string(),
        "scanned_files": scanned,
        "hit_count": hits.len(),
        "hits": hits,
        "claim_evidence": [{
            "id": if mode == "conflict-marker-guard" { "V6-SEC-CONFLICT-GUARD-001" } else { "V6-SEC-REPO-HYGIENE-001" },
            "claim": "repository_hygiene_and_conflict_markers_are_enforced_with_fail_closed_scan_receipts",
            "evidence": {
                "mode": mode,
                "hit_count": hits.len()
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

fn run_log_redaction_guard(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let mut source = "text".to_string();
    let mut content = parse_flag(argv, "text").unwrap_or_default();
    if content.is_empty() {
        if let Some(path) = parse_flag(argv, "log-path") {
            let candidate = if Path::new(&path).is_absolute() {
                PathBuf::from(&path)
            } else {
                root.join(&path)
            };
            source = candidate.display().to_string();
            content = fs::read_to_string(&candidate).unwrap_or_default();
        }
    }
    let lower = content.to_ascii_lowercase();
    let patterns = [
        ("openai_api_key", "sk-"),
        ("frontier_provider_api_key", "sk-ant-"),
        ("aws_access_key", "akia"),
        ("private_key", "-----begin private key-----"),
        ("github_pat", "ghp_"),
    ];
    let mut hits = Vec::<Value>::new();
    for (name, pattern) in patterns {
        if lower.contains(pattern) {
            hits.push(json!({"pattern": name}));
        }
    }
    let mut redacted = content.clone();
    for needle in ["sk-", "sk-ant-", "ghp_", "AKIA"] {
        if redacted.contains(needle) {
            redacted = redacted.replace(needle, "[REDACTED]");
        }
    }
    if redacted.len() > 400 {
        redacted.truncate(400);
    }

    let blocked = !hits.is_empty();
    let out = json!({
        "ok": !blocked,
        "type": "security_plane_log_redaction_guard",
        "strict": strict,
        "source": source,
        "hit_count": hits.len(),
        "hits": hits,
        "redacted_preview": redacted,
        "claim_evidence": [{
            "id": "V6-SEC-LOG-REDACTION-001",
            "claim": "log_redaction_guard_detects_secret_egress_patterns_before_output_release",
            "evidence": {
                "hit_count": hits.len()
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

fn path_size_bytes(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }
    let mut total = 0u64;
    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            total = total.saturating_add(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
        }
    }
    total
}

fn run_workspace_dump_guard(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let target = parse_flag(argv, "path").unwrap_or_default();
    let target_path = if Path::new(&target).is_absolute() {
        PathBuf::from(&target)
    } else {
        root.join(&target)
    };
    let exists = !target.is_empty() && target_path.exists();
    let bytes = if exists {
        path_size_bytes(&target_path)
    } else {
        parse_u64(parse_flag(argv, "bytes"), 0)
    };
    let max_bytes = parse_u64(parse_flag(argv, "max-bytes"), 5_000_000);
    let lower_target = target.to_ascii_lowercase();
    let sensitive_path = lower_target.contains(".env")
        || lower_target.contains("secret")
        || lower_target.contains("key");
    let blocked = bytes > max_bytes || sensitive_path || !exists;
    let out = json!({
        "ok": !blocked,
        "type": "security_plane_workspace_dump_guard",
        "strict": strict,
        "path": target_path.display().to_string(),
        "exists": exists,
        "bytes": bytes,
        "max_bytes": max_bytes,
        "sensitive_path": sensitive_path,
        "blocked": blocked,
        "claim_evidence": [{
            "id": "V6-SEC-WORKSPACE-DUMP-001",
            "claim": "workspace_dump_guard_blocks_sensitive_or_oversized_exports_before_egress",
            "evidence": {
                "blocked": blocked,
                "bytes": bytes,
                "max_bytes": max_bytes
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

fn run_llm_gateway_guard(_root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let provider = clean(parse_flag(argv, "provider").unwrap_or_default(), 80).to_ascii_lowercase();
    let model = clean(parse_flag(argv, "model").unwrap_or_default(), 120).to_ascii_lowercase();
    let providers = {
        let rows = split_csv(parse_flag(argv, "allow-providers"));
        if rows.is_empty() {
            vec![
                "openai".to_string(),
                "frontier_provider".to_string(),
                "local".to_string(),
            ]
        } else {
            rows
        }
    };
    let prefixes = {
        let rows = split_csv(parse_flag(argv, "allow-model-prefixes"));
        if rows.is_empty() {
            vec![
                "gpt-".to_string(),
                "o3".to_string(),
                "o4".to_string(),
                "claude-".to_string(),
                "llama-".to_string(),
            ]
        } else {
            rows
        }
    };

    let provider_allowed = providers.iter().any(|allowed| allowed == &provider);
    let model_allowed = prefixes
        .iter()
        .any(|prefix| !model.is_empty() && model.starts_with(prefix));
    let blocked = provider.is_empty() || model.is_empty() || !provider_allowed || !model_allowed;
    let out = json!({
        "ok": !blocked,
        "type": "security_plane_llm_gateway_guard",
        "strict": strict,
        "provider": provider,
        "model": model,
        "provider_allowed": provider_allowed,
        "model_allowed": model_allowed,
        "allow_providers": providers,
        "allow_model_prefixes": prefixes,
        "claim_evidence": [{
            "id": "V6-SEC-LLM-GATEWAY-001",
            "claim": "llm_gateway_guard_fail_closes_provider_and_model_routing_outside_declared_allowlists",
            "evidence": {
                "provider_allowed": provider_allowed,
                "model_allowed": model_allowed
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

fn run_startup_attestation_boot_gate(root: &Path, argv: &[String]) -> (Value, i32) {
    if argv.is_empty() {
        return infring_layer1_security::run_startup_attestation(root, &["status".to_string()]);
    }
    infring_layer1_security::run_startup_attestation(root, argv)
}
