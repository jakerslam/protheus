
fn run_node_script(root: &Path, script_rel: &str, args: &[String], forward_stdin: bool) -> i32 {
    let workspace_root = effective_workspace_root(root);
    let runtime_mode = resolved_runtime_mode(&workspace_root);
    if let Some((domain, mapped_args)) = maybe_redirect_ts_wrapper_to_core_domain(script_rel, args)
    {
        return run_core_domain(&workspace_root, &domain, &mapped_args, forward_stdin);
    }
    if let Some(domain) = script_rel.strip_prefix("core://") {
        return run_core_domain(&workspace_root, domain, args, forward_stdin);
    }

    let mut script_abs = workspace_root.join(script_rel);
    if !script_abs.exists() && script_rel.ends_with(".js") {
        let ts_rel = format!("{}{}", script_rel.trim_end_matches(".js"), ".ts");
        let ts_abs = workspace_root.join(&ts_rel);
        if ts_abs.exists() {
            if runtime_mode == "dist" {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "infringctl_dispatch",
                        "error": "dist_source_mismatch",
                        "detail": "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected",
                        "script_rel": clean(script_rel, 220),
                        "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                        "ts_candidate_rel": ts_rel,
                        "ts_candidate_exists": true,
                        "runtime_mode": runtime_mode,
                        "node_runtime_detected": has_node_runtime(),
                        "route_found": true
                    })
                );
                return 1;
            }
            script_abs = ts_abs;
        }
    }
    if !script_abs.exists() {
        let synthetic_route = Route {
            script_rel: script_rel.to_string(),
            args: args.to_vec(),
            forward_stdin,
        };
        if let Some(status) = node_missing_fallback(&workspace_root, &synthetic_route, false) {
            return status;
        }
        if matches!(
            script_rel,
            "client/runtime/systems/ops/infring_setup_wizard.ts"
                | "client/runtime/systems/ops/infring_setup_wizard.js"
        ) {
            return run_setup_wizard_missing_script_fallback(&workspace_root, args);
        }
        let ts_candidate_rel = if script_rel.ends_with(".js") {
            Some(format!("{}{}", script_rel.trim_end_matches(".js"), ".ts"))
        } else {
            None
        };
        let ts_candidate_exists = ts_candidate_rel
            .as_ref()
            .map(|rel| workspace_root.join(rel).exists())
            .unwrap_or(false);
        let script_missing_kind =
            if runtime_mode == "dist" && script_rel.ends_with(".js") && ts_candidate_exists {
                "dist_source_mismatch"
            } else {
                "script_missing"
            };
        let detail = if script_missing_kind == "dist_source_mismatch" {
            "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected"
        } else {
            "resolved route target script is missing from workspace runtime"
        };
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "infringctl_dispatch",
                "error": script_missing_kind,
                "detail": detail,
                "script_rel": clean(script_rel, 220),
                "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                "ts_candidate_rel": ts_candidate_rel,
                "ts_candidate_exists": ts_candidate_exists,
                "runtime_mode": runtime_mode,
                "node_runtime_detected": has_node_runtime(),
                "route_found": true
            })
        );
        return 1;
    }

    let ts_entrypoint = workspace_root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = script_abs
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut cmd = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        cmd.arg(ts_entrypoint).arg(&script_abs);
    } else {
        cmd.arg(&script_abs);
    }

    cmd.args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infringctl_dispatch",
                    "error": clean(format!("spawn_failed:{err}"), 220)
                })
            );
            1
        }
    }
}

fn run_setup_wizard_missing_script_fallback(root: &Path, args: &[String]) -> i32 {
    let state_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("infring_setup_wizard")
        .join("latest.json");
    let payload = json!({
        "type": "infring_setup_wizard_state",
        "completed": false,
        "completed_at": crate::now_iso(),
        "completion_mode": "missing_script_fallback_deferred",
        "node_runtime_detected": has_node_runtime(),
        "interaction_style": "silent",
        "notifications": "none",
        "covenant_acknowledged": false,
        "next_action": "infring setup --yes --defaults",
        "version": 1
    });
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(state_path, raw);
    }
    let json_mode = args.iter().any(|arg| arg == "--json" || arg == "--json=1");
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "infring_setup_wizard_fallback",
                "mode": "missing_script_fallback",
                "deferred": true,
                "next_action": "infring setup --yes --defaults",
                "message": "setup wizard script missing in this runtime; wrote deferred fallback state"
            })
        );
    } else {
        println!("Setup wizard script missing in this runtime; setup was deferred.");
        println!("Run `infring setup --yes --defaults` after repairing your runtime surface.");
    }
    0
}

fn has_json_flag(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json" || arg == "--json=1")
}

fn first_positional_command(args: &[String]) -> String {
    for token in args {
        let trimmed = token.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        return trimmed.to_string();
    }
    String::new()
}
