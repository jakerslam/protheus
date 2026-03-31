fn check_rust_source_of_truth_contract(
    root: &Path,
    selected: &HashSet<String>,
) -> Result<Value, String> {
    let policy_path = root.join(RUST_SOURCE_OF_TRUTH_POLICY_REL);
    let raw = fs::read_to_string(&policy_path).map_err(|err| {
        format!(
            "read_rust_source_of_truth_policy_failed:{}:{err}",
            policy_path.display()
        )
    })?;
    let policy = serde_json::from_str::<Value>(&raw).map_err(|err| {
        format!(
            "parse_rust_source_of_truth_policy_failed:{}:{err}",
            policy_path.display()
        )
    })?;

    let run_entrypoint = should_run_rust_subcheck(selected, "rust_entrypoint_gate");
    let run_conduit = should_run_rust_subcheck(selected, "conduit_strict_gate");
    let run_conduit_budget = should_run_rust_subcheck(selected, "conduit_budget_gate");
    let run_status_dashboard = should_run_rust_subcheck(selected, "status_dashboard_gate");
    let run_js_wrapper = should_run_rust_subcheck(selected, "js_wrapper_contract");
    let run_rust_shim = should_run_rust_subcheck(selected, "rust_shim_contract");
    let run_primitive_wrapper = should_run_rust_subcheck(selected, "primitive_ts_wrapper_contract");
    let run_tcb_ts_exception = should_run_rust_subcheck(selected, "tcb_ts_exception_contract");

    let mut entrypoint_path: Option<String> = None;
    if run_entrypoint {
        let entrypoint_gate = require_object(&policy, "rust_entrypoint_gate")?;
        let path = require_rel_path(entrypoint_gate, "path")?;
        let tokens = require_string_array(entrypoint_gate, "required_tokens")?;
        if !path.ends_with(".rs") {
            return Err(format!(
                "rust_source_of_truth_path_extension_mismatch:rust_entrypoint_gate:{path}"
            ));
        }
        check_required_tokens_at_path(root, &path, &tokens, "rust_entrypoint_gate")?;
        entrypoint_path = Some(path);
    }

    let mut conduit_path: Option<String> = None;
    if run_conduit {
        let conduit_gate = require_object(&policy, "conduit_strict_gate")?;
        let path = require_rel_path(conduit_gate, "path")?;
        let tokens = require_string_array(conduit_gate, "required_tokens")?;
        if !path.ends_with(".ts") {
            return Err(format!(
                "rust_source_of_truth_path_extension_mismatch:conduit_strict_gate:{path}"
            ));
        }
        check_required_tokens_at_path(root, &path, &tokens, "conduit_strict_gate")?;
        conduit_path = Some(path);
    }

    let mut conduit_budget_path: Option<String> = None;
    if run_conduit_budget {
        let conduit_budget_gate = require_object(&policy, "conduit_budget_gate")?;
        let path = require_rel_path(conduit_budget_gate, "path")?;
        let tokens = require_string_array(conduit_budget_gate, "required_tokens")?;
        if !path.ends_with(".rs") {
            return Err(format!(
                "rust_source_of_truth_path_extension_mismatch:conduit_budget_gate:{path}"
            ));
        }
        check_required_tokens_at_path(root, &path, &tokens, "conduit_budget_gate")?;
        conduit_budget_path = Some(path);
    }

    let mut status_dashboard_path: Option<String> = None;
    if run_status_dashboard {
        let status_dashboard_gate = require_object(&policy, "status_dashboard_gate")?;
        let path = require_rel_path(status_dashboard_gate, "path")?;
        let tokens = require_string_array(status_dashboard_gate, "required_tokens")?;
        if !path.ends_with(".ts") {
            return Err(format!(
                "rust_source_of_truth_path_extension_mismatch:status_dashboard_gate:{path}"
            ));
        }
        check_required_tokens_at_path(root, &path, &tokens, "status_dashboard_gate")?;
        status_dashboard_path = Some(path);
    }

    let mut wrapper_paths_checked = 0usize;
    if run_js_wrapper {
        let wrapper_contract = require_object(&policy, "js_wrapper_contract")?;
        let wrapper_paths = require_string_array(wrapper_contract, "required_wrapper_paths")?;
        for rel in &wrapper_paths {
            if !rel.ends_with(".js") && !rel.ends_with(".ts") {
                return Err(format!("required_wrapper_must_be_ts_or_js:{rel}"));
            }
            let path = root.join(rel);
            let source = fs::read_to_string(&path)
                .map_err(|err| format!("read_wrapper_failed:{}:{err}", path.display()))?;
            if rel.ends_with(".js") && !is_ts_bootstrap_wrapper(&source) {
                return Err(format!("required_wrapper_not_bootstrap:{rel}"));
            }
        }
        wrapper_paths_checked = wrapper_paths.len();
    }

    let mut rust_shim_checked = 0usize;
    if run_rust_shim {
        let rust_shim_contract = require_object(&policy, "rust_shim_contract")?;
        let rust_shim_entries = rust_shim_contract
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| "rust_source_of_truth_policy_missing_array:entries".to_string())?;
        if rust_shim_entries.is_empty() {
            return Err("rust_source_of_truth_policy_empty_array:entries".to_string());
        }
        for entry in rust_shim_entries {
            let section = entry
                .as_object()
                .ok_or_else(|| "rust_source_of_truth_policy_invalid_entry:entries".to_string())?;
            let shim_path = require_rel_path(section, "path")?;
            if !shim_path.ends_with(".js") && !shim_path.ends_with(".ts") {
                return Err(format!("rust_shim_must_be_ts_or_js:{shim_path}"));
            }
            let shim_tokens = require_string_array(section, "required_tokens")?;
            check_required_tokens_at_path(root, &shim_path, &shim_tokens, "rust_shim_contract")?;
            rust_shim_checked += 1;
        }
    }

    let mut primitive_ts_wrappers_checked = 0usize;
    if run_primitive_wrapper {
        let primitive_wrapper_contract = require_object(&policy, "primitive_ts_wrapper_contract")?;
        let primitive_wrapper_entries = primitive_wrapper_contract
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "rust_source_of_truth_policy_missing_array:primitive_ts_wrapper_contract.entries"
                    .to_string()
            })?;
        if primitive_wrapper_entries.is_empty() {
            return Err(
                "rust_source_of_truth_policy_empty_array:primitive_ts_wrapper_contract.entries"
                    .to_string(),
            );
        }

        for entry in primitive_wrapper_entries {
            let section = entry.as_object().ok_or_else(|| {
                "rust_source_of_truth_policy_invalid_entry:primitive_ts_wrapper_contract.entries"
                    .to_string()
            })?;
            let wrapper_path = require_rel_path(section, "path")?;
            if !wrapper_path.ends_with(".ts") {
                return Err(format!("primitive_ts_wrapper_must_be_ts:{wrapper_path}"));
            }

            let required_tokens = require_string_array(section, "required_tokens")?;
            check_required_tokens_at_path(
                root,
                &wrapper_path,
                &required_tokens,
                "primitive_ts_wrapper_contract",
            )?;

            let forbidden_tokens = section
                .get("forbidden_tokens")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| row.trim().to_string())
                        .filter(|row| !row.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if !forbidden_tokens.is_empty() {
                let wrapper_source =
                    fs::read_to_string(root.join(&wrapper_path)).map_err(|err| {
                        format!(
                            "read_source_failed:{}:{err}",
                            root.join(&wrapper_path).display()
                        )
                    })?;
                let found_forbidden = forbidden_tokens
                    .iter()
                    .filter(|token| wrapper_source.contains(token.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                if !found_forbidden.is_empty() {
                    return Err(format!(
                        "forbidden_source_tokens:primitive_ts_wrapper_contract:{}:{}",
                        wrapper_path,
                        found_forbidden.join(",")
                    ));
                }
            }

            primitive_ts_wrappers_checked += 1;
        }
    }

    let mut ts_surface_allowlist_prefixes: Vec<String> = Vec::new();
    if run_conduit || run_status_dashboard {
        ts_surface_allowlist_prefixes = policy
            .get("ts_surface_allowlist_prefixes")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "rust_source_of_truth_policy_missing_array:ts_surface_allowlist_prefixes"
                    .to_string()
            })?
            .iter()
            .filter_map(Value::as_str)
            .map(|row| row.trim().to_string())
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>();
        if ts_surface_allowlist_prefixes.is_empty() {
            return Err(
                "rust_source_of_truth_policy_empty_array:ts_surface_allowlist_prefixes".to_string(),
            );
        }

        let mut ts_paths_to_validate: Vec<String> = Vec::new();
        if let Some(path) = conduit_path.clone() {
            ts_paths_to_validate.push(path);
        }
        if let Some(path) = status_dashboard_path.clone() {
            ts_paths_to_validate.push(path);
        }
        for ts_path in ts_paths_to_validate {
            let allowed = ts_surface_allowlist_prefixes
                .iter()
                .any(|prefix| ts_path.starts_with(prefix));
            if !allowed {
                return Err(format!(
                    "ts_path_outside_surface_allowlist:{ts_path}:{}",
                    ts_surface_allowlist_prefixes.join(",")
                ));
            }
        }
    }

    let mut tcb_ts_exception_manifest_path: Option<String> = None;
    let mut tcb_ts_exception_total_paths = 0usize;
    let mut tcb_ts_exception_wrapper_paths = 0usize;
    let mut tcb_ts_exception_explicit_paths = 0usize;
    let mut tcb_ts_exception_stale_entries = 0usize;
    if run_tcb_ts_exception {
        let exception_contract = require_object(&policy, "tcb_ts_exception_contract")?;
        let manifest_rel = require_rel_path(exception_contract, "path")?;
        let wrapper_tokens = require_string_array(exception_contract, "wrapper_tokens")?;
        let manifest_path = root.join(&manifest_rel);
        let manifest_raw = fs::read_to_string(&manifest_path).map_err(|err| {
            format!(
                "read_tcb_ts_exception_manifest_failed:{}:{err}",
                manifest_path.display()
            )
        })?;
        let manifest = serde_json::from_str::<Value>(&manifest_raw).map_err(|err| {
            format!(
                "parse_tcb_ts_exception_manifest_failed:{}:{err}",
                manifest_path.display()
            )
        })?;
        let exception_entries = manifest
            .get("exceptions")
            .and_then(Value::as_array)
            .ok_or_else(|| "tcb_ts_exception_manifest_missing_array:exceptions".to_string())?;

        let mut exception_reasons = HashMap::<String, String>::new();
        for entry in exception_entries {
            let section = entry
                .as_object()
                .ok_or_else(|| "tcb_ts_exception_manifest_invalid_entry".to_string())?;
            let path = require_rel_path(section, "path")?;
            let reason = section
                .get("reason")
                .and_then(Value::as_str)
                .map(|raw| raw.trim().to_string())
                .unwrap_or_default();
            if reason.is_empty() {
                return Err(format!("tcb_ts_exception_missing_reason:{path}"));
            }
            exception_reasons.insert(path, reason);
        }

        let tcb_prefixes = policy
            .get("tcb_rust_required_prefixes")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "rust_source_of_truth_policy_missing_array:tcb_rust_required_prefixes".to_string()
            })?
            .iter()
            .filter_map(Value::as_str)
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())
            .collect::<Vec<_>>();
        if tcb_prefixes.is_empty() {
            return Err(
                "rust_source_of_truth_policy_empty_array:tcb_rust_required_prefixes".to_string(),
            );
        }

        let mut missing_exceptions = Vec::<String>::new();
        let mut seen_exceptions = HashSet::<String>::new();
        for prefix in tcb_prefixes {
            let abs_prefix = root.join(&prefix);
            if !abs_prefix.exists() {
                continue;
            }
            for entry in WalkDir::new(&abs_prefix).into_iter().filter_map(Result::ok) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let file_path = entry.path();
                let ext = file_path.extension().and_then(|row| row.to_str()).unwrap_or("");
                if ext != "ts" && ext != "js" {
                    continue;
                }
                let rel = file_path
                    .strip_prefix(root)
                    .map_err(|_| "tcb_ts_exception_strip_prefix_failed".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel.contains("/tests/")
                    || rel.ends_with(".test.ts")
                    || rel.ends_with(".spec.ts")
                    || rel.ends_with(".d.ts")
                {
                    continue;
                }
                tcb_ts_exception_total_paths += 1;
                let source = fs::read_to_string(file_path)
                    .map_err(|err| format!("read_source_failed:{}:{err}", file_path.display()))?;
                let is_wrapper = wrapper_tokens
                    .iter()
                    .any(|token| !token.is_empty() && source.contains(token.as_str()));
                if is_wrapper {
                    tcb_ts_exception_wrapper_paths += 1;
                    continue;
                }
                if exception_reasons.contains_key(&rel) {
                    tcb_ts_exception_explicit_paths += 1;
                    seen_exceptions.insert(rel);
                } else {
                    missing_exceptions.push(rel);
                }
            }
        }

        if !missing_exceptions.is_empty() {
            missing_exceptions.sort();
            let preview = missing_exceptions
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join(",");
            return Err(format!(
                "tcb_ts_exception_missing:{}:{}",
                missing_exceptions.len(),
                preview
            ));
        }

        tcb_ts_exception_stale_entries = exception_reasons
            .keys()
            .filter(|path| !seen_exceptions.contains(path.as_str()))
            .count();
        tcb_ts_exception_manifest_path = Some(manifest_rel);
    }

    let mut scoped_check_ids = selected.iter().cloned().collect::<Vec<_>>();
    scoped_check_ids.sort();

    Ok(json!({
        "id": CHECK_ID_RUST_SOURCE_OF_TRUTH,
        "ok": true,
        "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
        "entrypoint_path": entrypoint_path,
        "conduit_path": conduit_path,
        "conduit_budget_path": conduit_budget_path,
        "status_dashboard_path": status_dashboard_path,
        "wrapper_paths_checked": wrapper_paths_checked,
        "rust_shims_checked": rust_shim_checked,
        "primitive_ts_wrappers_checked": primitive_ts_wrappers_checked,
        "ts_surface_allowlist_prefixes": ts_surface_allowlist_prefixes,
        "tcb_ts_exception_manifest_path": tcb_ts_exception_manifest_path,
        "tcb_ts_exception_total_paths": tcb_ts_exception_total_paths,
        "tcb_ts_exception_wrapper_paths": tcb_ts_exception_wrapper_paths,
        "tcb_ts_exception_explicit_paths": tcb_ts_exception_explicit_paths,
        "tcb_ts_exception_stale_entries": tcb_ts_exception_stale_entries,
        "scoped_check_ids": scoped_check_ids,
    }))
}

fn env_flag(name: &str, fallback: bool) -> bool {
    let Ok(raw) = std::env::var(name) else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn check_source_tokens(
    root: &Path,
    rel_path: &str,
    required_tokens: &[&str],
    check_id: &str,
) -> Result<Value, String> {
    let path = root.join(rel_path);
    let source = resolve_contract_source(&path)?;
    let tokens = required_tokens
        .iter()
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    let missing = missing_tokens(&source, &tokens);
    if !missing.is_empty() {
        return Err(format!(
            "missing_source_tokens:{}:{}",
            rel_path,
            missing.join(",")
        ));
    }

    Ok(json!({
        "id": check_id,
        "ok": true,
        "path": rel_path,
        "required_tokens": required_tokens.len(),
    }))
}

fn resolve_contract_source(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("read_source_failed:{}:{err}", path.display()))?;
    if !is_ts_bootstrap_wrapper(&source) {
        return Ok(source);
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("js") {
        return Ok(source);
    }
    let ts_path = path.with_extension("ts");
    if !ts_path.exists() {
        return Ok(source);
    }
    fs::read_to_string(&ts_path).map_err(|err| {
        format!(
            "read_bootstrap_ts_source_failed:{}:{err}",
            ts_path.display()
        )
    })
}

