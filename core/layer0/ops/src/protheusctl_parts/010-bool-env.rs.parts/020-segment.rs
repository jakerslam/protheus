fn process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        return Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn dashboard_healthz_reachable(host: &str, port: u16, timeout_ms: u64) -> bool {
    let addr = format!("{host}:{port}");
    let timeout = Duration::from_millis(timeout_ms.max(100));
    let Ok(addrs) = addr.to_socket_addrs() else {
        return false;
    };
    for socket_addr in addrs {
        if TcpStream::connect_timeout(&socket_addr, timeout).is_ok() {
            return true;
        }
    }
    false
}

fn launchd_dashboard_loaded() -> bool {
    if env::consts::OS != "macos" {
        return false;
    }
    if !Command::new("launchctl")
        .arg("help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        return false;
    }
    let uid = match Command::new("id")
        .arg("-u")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => String::new(),
    };
    if uid.is_empty() {
        return false;
    }
    let label = "com.protheuslabs.infring.dashboard.shelltest2";
    for domain in [format!("gui/{uid}"), format!("user/{uid}")] {
        let target = format!("{domain}/{label}");
        if Command::new("launchctl")
            .arg("print")
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn node_install_command_hint() -> String {
    match env::consts::OS {
        "macos" => {
            if command_exists("brew") {
                "brew install node@22 && brew link --overwrite --force node@22".to_string()
            } else {
                "Install Homebrew from https://brew.sh then run: brew install node@22".to_string()
            }
        }
        "linux" => {
            if command_exists("apt-get") {
                "curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - && sudo apt-get install -y nodejs".to_string()
            } else if command_exists("dnf") {
                "sudo dnf install -y nodejs npm".to_string()
            } else if command_exists("yum") {
                "sudo yum install -y nodejs npm".to_string()
            } else if command_exists("pacman") {
                "sudo pacman -S --noconfirm nodejs npm".to_string()
            } else if command_exists("apk") {
                "sudo apk add --no-cache nodejs npm".to_string()
            } else {
                "Install Node.js 22+ from https://nodejs.org/en/download".to_string()
            }
        }
        "windows" => "winget install OpenJS.NodeJS.LTS".to_string(),
        _ => "Install Node.js 22+ from https://nodejs.org/en/download".to_string(),
    }
}

fn workspace_package_version(root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(root.join("package.json")).ok()?;
    let parsed: Value = serde_json::from_str(&raw).ok()?;
    parsed
        .get("version")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn workspace_install_release_version(root: &Path) -> Option<String> {
    let meta_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_meta.json");
    if let Ok(raw) = std::fs::read_to_string(&meta_path) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            if let Some(value) = parsed
                .get("release_version_normalized")
                .and_then(Value::as_str)
                .or_else(|| parsed.get("release_tag").and_then(Value::as_str))
            {
                let normalized = value.trim().trim_start_matches(['v', 'V']).to_string();
                if !normalized.is_empty() {
                    return Some(normalized);
                }
            }
        }
    }

    let tag_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_tag.txt");
    let raw = std::fs::read_to_string(tag_path).ok()?;
    let normalized = raw.trim().trim_start_matches(['v', 'V']).to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn command_list_mode(args: &[String]) -> String {
    args.iter()
        .find_map(|arg| arg.strip_prefix("--mode=").map(|value| value.to_string()))
        .unwrap_or_else(|| "list".to_string())
}

fn declared_install_mode() -> String {
    let from_env = env::var("INFRING_INSTALL_MODE")
        .unwrap_or_else(|_| env::var("INFRING_RUNTIME_MODE").unwrap_or_default())
        .trim()
        .to_ascii_lowercase();
    if matches!(
        from_env.as_str(),
        "full" | "minimal" | "pure" | "tiny-max"
    ) {
        return from_env;
    }
    if bool_env("INFRING_TINY_MAX_MODE", false) {
        return "tiny-max".to_string();
    }
    if bool_env("INFRING_PURE_MODE", false) {
        return "pure".to_string();
    }
    "full".to_string()
}

fn mode_capability_reason(mode: &str) -> (&'static str, &'static str) {
    match mode {
        "pure" => (
            "limited_optional",
            "rust_first_mode_optional_rich_surfaces_limited",
        ),
        "tiny-max" => (
            "limited_optional",
            "tiny_max_mode_minimal_footprint_optional_rich_surfaces_limited",
        ),
        "minimal" => (
            "optional_limited",
            "minimal_mode_install_light_optional_surfaces_may_require_explicit_setup",
        ),
        _ => ("available", "full_mode_complete_operator_surface"),
    }
}

fn mode_help_contract(
    mode: &str,
) -> (
    Vec<&'static str>,
    &'static str,
    Vec<(&'static str, &'static str)>,
) {
    match mode {
        "pure" | "tiny-max" => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway status",
                "infring doctor --json",
            ],
            "constrained_mode_optional_rich_surfaces_limited",
            vec![
                (
                    "infring dashboard",
                    "constrained_mode_optional_dashboard_surfaces_limited",
                ),
                (
                    "infring gateway start --dashboard-open=1",
                    "constrained_mode_optional_dashboard_surfaces_limited",
                ),
                (
                    "infring assimilate <target> ...",
                    "node_runtime_and_full_mode_required",
                ),
            ],
        ),
        "minimal" => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway",
                "infring gateway status",
                "infring doctor --json",
            ],
            "minimal_mode_operator_surface_requires_explicit_setup_on_some_hosts",
            vec![
                (
                    "infring dashboard",
                    "minimal_mode_optional_dashboard_requires_explicit_opt_in",
                ),
                (
                    "infring assimilate <target> ...",
                    "node_runtime_and_full_mode_required",
                ),
            ],
        ),
        _ => (
            vec![
                "infring help",
                "infring setup",
                "infring setup status --json",
                "infring gateway",
                "infring gateway status",
                "infring dashboard",
                "infring doctor --json",
            ],
            "full_mode_complete_operator_surface",
            vec![(
                "infring assimilate <target> ...",
                "node_runtime_required_for_full_surface",
            )],
        ),
    }
}

fn strip_status_dashboard_tokens(args: Vec<String>) -> Vec<String> {
    let mut filtered = Vec::<String>::new();
    for arg in args {
        let token = arg.trim().to_ascii_lowercase();
        if matches!(token.as_str(), "--dashboard" | "dashboard" | "--web") {
            continue;
        }
        filtered.push(arg);
    }
    filtered
}

fn print_node_free_command_list(mode: &str) {
    let install_mode = declared_install_mode();
    let (dashboard_surface, capability_reason) = mode_capability_reason(install_mode.as_str());
    let (mode_valid_commands, mode_help_reason, mode_unavailable_actions) =
        mode_help_contract(install_mode.as_str());
    if mode == "help" {
        usage();
        println!();
        println!("Node.js is not available, so full JS command help is unavailable.");
    } else {
        println!("Command list (Node-free fallback):");
    }
    println!(
        "Mode contract: mode={}, dashboard_surface={}, reason={}",
        install_mode, dashboard_surface, capability_reason
    );
    println!("Mode help reason: {}", mode_help_reason);
    println!("Mode-valid commands:");
    for cmd in mode_valid_commands {
        println!("  - {cmd}");
    }
    if !mode_unavailable_actions.is_empty() {
        println!();
        println!("Mode-unavailable actions:");
        for (command, reason) in mode_unavailable_actions {
            println!("  - {command} ({reason})");
        }
    }
    println!();
    println!("Unavailable until full mode + Node.js 22+:");
    println!("  - infring assimilate <target> ...");
    println!();
    println!("Install Node.js 22+ to unlock all CLI commands.");
    println!("Suggested install command: {}", node_install_command_hint());
    println!("Tip: rerun installer with --install-node to attempt automatic installation.");
    println!("Deterministic recovery path:");
    println!("  1) infring setup --yes --defaults");
    println!("  2) infring setup status --json");
    println!("  3) infring gateway status");
    println!("  4) infring doctor --json");
    let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let missing_runtime = runtime_missing_entrypoints(&effective_workspace_root(&root));
    if !missing_runtime.is_empty() {
        println!();
        println!(
            "Runtime assets also appear incomplete (manifest: {INSTALL_RUNTIME_MANIFEST_REL}):"
        );
        for rel in missing_runtime.iter().take(8) {
            println!("  - missing: {rel}");
        }
        if missing_runtime.len() > 8 {
            println!("  - ... {} more", missing_runtime.len() - 8);
        }
        println!("Run `infring doctor --json` for a full install integrity report.");
    }
}

fn emit_node_missing_error(root: &Path, cmd: &str, script_rel: &str) -> i32 {
    let install_hint = node_install_command_hint();
    let install_mode = declared_install_mode();
    let (dashboard_surface, capability_reason) = mode_capability_reason(install_mode.as_str());
    let missing_runtime = runtime_missing_entrypoints(root);
    let runtime_assets_missing = !missing_runtime.is_empty();
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": "protheusctl_dispatch",
            "error": "node_runtime_missing",
            "command": clean(cmd, 80),
            "script_rel": clean(script_rel, 220),
            "hint": clean(format!("Install Node.js 22+ (try: {install_hint}) or set PROTHEUS_NODE_BINARY to a valid node executable."), 220),
            "node_install_command": clean(install_hint, 220),
            "path_reload_command": ". \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true",
            "install_mode": install_mode,
            "mode_dashboard_surface": dashboard_surface,
            "mode_capability_reason": capability_reason,
            "auto_install_hint": "Rerun installer with --install-node to attempt automatic Node installation.",
            "setup_retry_command": "infring setup --yes --defaults",
            "setup_status_command": "infring setup status --json",
            "gateway_status_command": "infring gateway status",
            "doctor_command": "infring doctor --json",
            "runtime_assets_missing": runtime_assets_missing,
            "runtime_manifest_rel": INSTALL_RUNTIME_MANIFEST_REL,
            "missing_runtime_entrypoints": missing_runtime
        })
    );
    1
}

fn node_missing_fallback(root: &Path, route: &Route, json_mode: bool) -> Option<i32> {
    match route.script_rel.as_str() {
        "client/runtime/systems/ops/protheus_setup_wizard.ts"
        | "client/runtime/systems/ops/protheus_setup_wizard.js" => {
            let state_path = root
                .join("local")
                .join("state")
                .join("ops")
                .join("protheus_setup_wizard")
                .join("latest.json");
            let payload = json!({
                "type": "protheus_setup_wizard_state",
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
                        "type": "protheus_setup_wizard_fallback",
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
        "client/runtime/systems/ops/protheus_command_list.js"
        | "client/runtime/systems/ops/protheus_command_list.ts" => {
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
                        "type": "protheusctl_help_fallback",
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
        "client/runtime/systems/ops/protheus_version_cli.js"
        | "client/runtime/systems/ops/protheus_version_cli.ts" => {
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
                                "type": "protheusctl_update_fallback",
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
                                "type": "protheusctl_version_fallback",
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
        | "client/runtime/systems/ops/protheus_status_dashboard.ts" => {
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
