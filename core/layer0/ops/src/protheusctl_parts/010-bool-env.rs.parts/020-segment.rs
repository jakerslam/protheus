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
    if mode == "help" {
        usage();
        println!();
        println!("Node.js is not available, so full JS command help is unavailable.");
    } else {
        println!("Command list (Node-free fallback):");
    }
    for cmd in crate::command_list_kernel::tier1_command_synopses() {
        println!("  - {cmd}");
    }
    println!();
    println!("Install Node.js 22+ to unlock all CLI commands.");
    println!("Suggested install command: {}", node_install_command_hint());
    println!("Tip: rerun installer with --install-node to attempt automatic installation.");
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
            "auto_install_hint": "Rerun installer with --install-node to attempt automatic Node installation.",
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
                "completed": true,
                "completed_at": crate::now_iso(),
                "completion_mode": "node_runtime_missing_fallback",
                "node_runtime_detected": false,
                "interaction_style": "silent",
                "notifications": "none",
                "covenant_acknowledged": false,
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
                        "node_runtime_detected": false
                    })
                );
            } else {
                println!("Setup wizard deferred because Node.js 22+ is unavailable.");
                println!("Install Node.js and run `infring setup --force` to finish setup later.");
            }
            Some(0)
        }
        "client/runtime/systems/ops/protheus_command_list.js"
        | "client/runtime/systems/ops/protheus_command_list.ts" => {
            let mode = command_list_mode(&route.args);
            if json_mode {
                println!(
                    "{}",
                    json!({
                        "ok": true,
                        "type": "protheusctl_help_fallback",
                        "mode": mode,
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

