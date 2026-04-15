fn parse_dashboard_launch_config(argv: &[String], command: &str) -> DashboardLaunchConfig {
    let start_like = matches!(command, "start" | "restart");
    let enabled = parse_bool(
        parse_flag(argv, "dashboard-autoboot")
            .or_else(|| parse_flag(argv, "dashboard"))
            .as_deref(),
        start_like,
    );
    let open_browser = parse_bool(
        parse_flag(argv, "dashboard-open")
            .or_else(|| std::env::var("PROTHEUS_DASHBOARD_OPEN_ON_START").ok())
            .as_deref(),
        start_like,
    );
    let persistent_supervisor = parse_bool(
        parse_flag(argv, "gateway-persist")
            .or_else(|| parse_flag(argv, "gateway-supervisor"))
            .or_else(|| std::env::var("PROTHEUS_GATEWAY_PERSIST").ok())
            .as_deref(),
        start_like,
    );
    let host = parse_flag(argv, "dashboard-host")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = parse_u16(parse_flag(argv, "dashboard-port").as_deref(), 4173);
    let team = parse_flag(argv, "dashboard-team")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "ops".to_string());
    let refresh_ms = parse_u64(
        parse_flag(argv, "dashboard-refresh-ms").as_deref(),
        2000,
        800,
        60_000,
    );
    let ready_timeout_ms = parse_u64(
        parse_flag(argv, "dashboard-ready-timeout-ms")
            .or_else(|| std::env::var("PROTHEUS_DASHBOARD_READY_TIMEOUT_MS").ok())
            .as_deref(),
        36_000,
        1_500,
        180_000,
    );
    let watchdog_interval_ms = parse_u64(
        parse_flag(argv, "dashboard-watchdog-interval-ms")
            .or_else(|| std::env::var("PROTHEUS_DASHBOARD_WATCHDOG_INTERVAL_MS").ok())
            .as_deref(),
        DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS,
        DASHBOARD_WATCHDOG_INTERVAL_MIN_MS,
        DASHBOARD_WATCHDOG_INTERVAL_MAX_MS,
    );
    let node_binary = parse_flag(argv, "node-binary")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(resolve_node_binary);
    DashboardLaunchConfig {
        enabled,
        open_browser,
        persistent_supervisor,
        host,
        port,
        team,
        refresh_ms,
        ready_timeout_ms,
        watchdog_interval_ms,
        node_binary,
    }
}

fn gateway_supervisor_config(cfg: &DashboardLaunchConfig) -> GatewaySupervisorConfig {
    GatewaySupervisorConfig {
        host: cfg.host.clone(),
        port: cfg.port,
        team: cfg.team.clone(),
        refresh_ms: cfg.refresh_ms,
        ready_timeout_ms: cfg.ready_timeout_ms,
        watchdog_interval_ms: cfg.watchdog_interval_ms,
        node_binary: cfg.node_binary.clone(),
    }
}

fn supervisor_executable() -> Result<PathBuf, String> {
    let current = std::env::current_exe()
        .map_err(|err| format!("gateway_supervisor_current_exe_failed:{err}"))?;
    Ok(resolve_dashboard_executable(&current))
}

fn gateway_supervisor_enable(root: &Path, cfg: &DashboardLaunchConfig) -> GatewaySupervisorResult {
    let executable = match supervisor_executable() {
        Ok(path) => path,
        Err(err) => {
            return GatewaySupervisorResult {
                active: false,
                payload: json!({
                    "ok": false,
                    "action": "enable",
                    "error": err,
                }),
            };
        }
    };
    let supervisor_cfg = gateway_supervisor_config(cfg);
    gateway_supervisor::enable(
        root,
        &executable,
        &supervisor_cfg,
        &dashboard_watchdog_log_path(root),
    )
}

fn dashboard_state_dir(root: &Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("daemon_control")
}

fn dashboard_pid_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_ui.pid")
}

fn dashboard_log_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_ui.log")
}

fn dashboard_watchdog_pid_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_watchdog.pid")
}

fn dashboard_watchdog_log_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_watchdog.log")
}

fn dashboard_stop_latch_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard.stop")
}

fn dashboard_desired_state_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard.desired")
}

fn kill_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn read_pid_file(file_path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(file_path).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn pid_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        return Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(windows)]
    {
        return Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {pid}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
            .unwrap_or(false);
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn dashboard_listener_pids(_port: u16) -> Vec<u32> {
    #[cfg(unix)]
    {
        let query = format!("TCP:{_port}");
        let output = Command::new("lsof")
            .arg("-ti")
            .arg(query)
            .arg("-sTCP:LISTEN")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut pids = Vec::<u32>::new();
            for line in text.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
            return pids;
        }
    }
    Vec::new()
}

fn resolve_dashboard_executable(current_exe: &Path) -> PathBuf {
    let file_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !file_name.contains("protheusd") {
        return current_exe.to_path_buf();
    }
    let ext = current_exe
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let sibling_name = if ext.is_empty() {
        "protheus-ops".to_string()
    } else {
        format!("protheus-ops.{ext}")
    };
    let candidate = current_exe.with_file_name(sibling_name);
    if candidate.exists() {
        candidate
    } else {
        current_exe.to_path_buf()
    }
}

fn dashboard_backend_binary_hint() -> Option<String> {
    let current_exe = std::env::current_exe().ok()?;
    let resolved = resolve_dashboard_executable(&current_exe);

    let protheus_name = if cfg!(windows) {
        "protheus-ops.exe"
    } else {
        "protheus-ops"
    };

    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("target").join("debug").join(protheus_name));
        candidates.push(cwd.join("target").join("release").join(protheus_name));
    }
    candidates.push(resolved);

    let newest = candidates
        .into_iter()
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let mtime = fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
                .map(|dur| dur.as_millis())
                .unwrap_or(0);
            Some((mtime, path))
        })
        .max_by_key(|(mtime, _)| *mtime)
        .map(|(_, path)| path);

    if let Some(path) = newest {
        return Some(path.to_string_lossy().to_string());
    }
    None
}
