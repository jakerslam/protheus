
fn node_missing_fallback(root: &Path, route: &Route, json_mode: bool) -> Option<i32> {
    match route.script_rel.as_str() {
        "client/runtime/systems/ops/infring_setup_wizard.ts"
        | "client/runtime/systems/ops/infring_setup_wizard.js" => {
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
                "completion_mode": "node_runtime_missing_fallback_deferred",
                "node_runtime_detected": false,
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
            if json_mode {
                println!(
                    "{}",
                    json!({
                        "ok": true,
                        "type": "infring_setup_wizard_fallback",
                        "mode": "node_runtime_missing_fallback",
                        "node_runtime_detected": false,
                        "deferred": true,
                        "dashboard_open_noninteractive_default": false,
                        "dashboard_opt_in_command": "infring gateway start --dashboard-open=1",
                        "dashboard_opt_in_reason": "noninteractive_sessions_require_explicit_dashboard_opt_in",
                        "next_action": "infring setup --yes --defaults",
                        "status_check_command": "infring setup status --json",
                        "gateway_check_command": "infring gateway status",
                        "recovery_hint": "install_node_then_run_setup_yes_defaults"
                    })
                );
            } else {
                println!("Setup wizard deferred because Node.js 22+ is unavailable.");
                println!("Install Node.js and run `infring setup --yes --defaults` to finish setup.");
                println!(
                    "Then verify: `infring setup status --json` and `infring gateway status`."
                );
                println!(
                    "Dashboard auto-open is disabled for non-interactive sessions; opt in with `infring gateway start --dashboard-open=1`."
                );
            }
            Some(0)
        }
        "client/runtime/systems/ops/infring_command_list.js"
        | "client/runtime/systems/ops/infring_command_list.ts" => {
            let mode = command_list_mode(&route.args);
            let install_mode = declared_install_mode();
            let (dashboard_surface, capability_reason) = mode_capability_reason(install_mode.as_str());
            let (mode_valid_commands, mode_help_reason, mode_unavailable_actions) =
                mode_help_contract(install_mode.as_str());
            let mode_unavailable_actions_json: Vec<Value> = mode_unavailable_actions
                .iter()
                .map(|(command, reason)| {
                    json!({
                        "command": command,
                        "reason": reason
                    })
                })
                .collect();
            if json_mode {
                println!(
                    "{}",
                    json!({
                        "ok": true,
                        "type": "infringctl_help_fallback",
                        "mode": mode,
                        "install_mode": install_mode,
                        "mode_dashboard_surface": dashboard_surface,
                        "mode_capability_reason": capability_reason,
                        "mode_help_reason": mode_help_reason,
                        "mode_valid_commands": mode_valid_commands,
                        "mode_unavailable_actions": mode_unavailable_actions_json,
                        "node_runtime_required_for_full_surface": true,
                        "node_runtime_detected": false
                    })
                );
            } else {
                print_node_free_command_list(mode.as_str());
            }
            Some(0)
        }
        "client/runtime/systems/ops/infring_version_cli.js"
        | "client/runtime/systems/ops/infring_version_cli.ts" => {
            let command = route
                .args
                .first()
                .map(|row| row.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "version".to_string());
            let version = workspace_install_release_version(root)
                .or_else(|| workspace_package_version(root))
                .unwrap_or_else(|| "0.0.0-unknown".to_string());
            match command.as_str() {
                "check-quiet" => Some(0),
                "update" => {
                    let install_hint = node_install_command_hint();
                    if json_mode {
                        println!(
                            "{}",
                            json!({
                                "ok": true,
                                "type": "infringctl_update_fallback",
                                "current_version": version,
                                "update_check_performed": false,
                                "node_runtime_detected": false,
                                "hint": clean(format!("Install Node.js 22+ (try: {install_hint}) to enable `infring update` release checks."), 220)
                            })
                        );
                    } else {
                        println!("[infring update] Node.js 22+ is required for release checks.");
                        println!("[infring update] current version: {version}");
                        println!("[infring update] install Node hint: {install_hint}");
                    }
                    Some(0)
                }
                _ => {
                    if json_mode {
                        println!(
                            "{}",
                            json!({
                                "ok": true,
                                "type": "infringctl_version_fallback",
                                "version": version,
                                "node_runtime_detected": false
                            })
                        );
                    } else {
                        println!("infring {version}");
                        println!("(Node.js not detected; using install metadata fallback)");
                    }
                    Some(0)
                }
            }
        }
        "client/runtime/systems/edge/mobile_ops_top.ts"
        | "client/runtime/systems/ops/infring_status_dashboard.ts" => {
            if !json_mode {
                eprintln!("Node.js is unavailable; falling back to core daemon status output.");
            }
            Some(run_core_domain(
                root,
                "daemon-control",
                &["status".to_string()],
                false,
            ))
        }
        _ => None,
    }
}

fn parse_json(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Some(v);
    }
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    for line in lines.iter().rev() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            return Some(v);
        }
    }
    None
}
