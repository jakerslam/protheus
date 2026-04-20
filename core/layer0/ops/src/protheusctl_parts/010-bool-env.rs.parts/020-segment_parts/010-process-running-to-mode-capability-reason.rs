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
