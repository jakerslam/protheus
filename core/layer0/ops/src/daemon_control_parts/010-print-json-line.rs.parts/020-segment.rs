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

fn run_platform_command(program: &str, args: &[String]) -> Value {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => json!({
            "ok": out.status.success(),
            "status": out.status.code().unwrap_or(-1),
            "program": program,
            "args": args,
            "stdout": String::from_utf8_lossy(&out.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&out.stderr).trim().to_string(),
        }),
        Err(err) => json!({
            "ok": false,
            "status": -1,
            "program": program,
            "args": args,
            "error": format!("spawn_failed:{err}"),
        }),
    }
}

fn sanitize_dashboard_host_token_for_tmp(host: &str) -> String {
    let mut out = String::with_capacity(host.len());
    for ch in host.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "127_0_0_1".to_string()
    } else {
        out
    }
}

fn tmp_root_path_for_gateway_cleanup() -> PathBuf {
    let raw = std::env::var("TMPDIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/tmp".to_string());
    PathBuf::from(raw)
}

fn dashboard_tmp_pid_paths(cfg: &DashboardLaunchConfig) -> Vec<PathBuf> {
    let host_safe = sanitize_dashboard_host_token_for_tmp(&cfg.host);
    let port_safe = cfg.port.to_string();
    let tmp = tmp_root_path_for_gateway_cleanup();
    vec![
        tmp.join(format!("infring-dashboard-{host_safe}-{port_safe}.pid")),
        tmp.join(format!(
            "infring-dashboard-watchdog-{host_safe}-{port_safe}.pid"
        )),
    ]
}

fn cleanup_dashboard_pid_files(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let mut rows = Vec::<Value>::new();
    let mut paths = vec![dashboard_pid_path(root), dashboard_watchdog_pid_path(root)];
    paths.extend(dashboard_tmp_pid_paths(cfg));
    for path in paths {
        let existed = path.exists();
        let removed = if existed {
            fs::remove_file(&path).is_ok()
        } else {
            false
        };
        rows.push(json!({
            "path": path.to_string_lossy().to_string(),
            "existed": existed,
            "removed": removed
        }));
    }
    json!({
        "ok": rows.iter().all(|row| row.get("removed").and_then(Value::as_bool) == Some(true) || row.get("existed").and_then(Value::as_bool) == Some(false)),
        "rows": rows
    })
}

#[cfg(target_os = "macos")]
fn stale_launchd_labels_for_cleanup(current_label: &str) -> Vec<String> {
    let mut labels = vec![
        "ai.protheus.gateway".to_string(),
        "protheus.gateway".to_string(),
        "ai.infring.gateway.legacy".to_string(),
    ];
    labels.retain(|label| label != current_label);
    labels
}

#[cfg(not(target_os = "macos"))]
fn stale_launchd_labels_for_cleanup(_current_label: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn launchd_uid_for_cleanup() -> Option<String> {
    if let Ok(raw) = std::env::var("UID") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }
    let out = run_platform_command("id", &[String::from("-u")]);
    out.get("stdout")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
}

#[cfg(target_os = "macos")]
fn cleanup_stale_launchd_labels() -> Value {
    let current_label = std::env::var("INFRING_GATEWAY_LAUNCHD_LABEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "ai.infring.gateway".to_string());
    let Some(uid) = launchd_uid_for_cleanup() else {
        return json!({
            "ok": false,
            "active": true,
            "error": "launchd_uid_unavailable",
            "current_label": current_label,
        });
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let domain = format!("gui/{uid}");
    let mut rows = Vec::<Value>::new();
    for label in stale_launchd_labels_for_cleanup(&current_label) {
        let target = format!("{domain}/{label}");
        let plist_path = PathBuf::from(&home)
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{label}.plist"));
        let plist_text = plist_path.to_string_lossy().to_string();
        let bootout_target =
            run_platform_command("launchctl", &[String::from("bootout"), target.clone()]);
        let bootout_path = run_platform_command(
            "launchctl",
            &[
                String::from("bootout"),
                domain.clone(),
                plist_text.clone(),
            ],
        );
        let unload = run_platform_command(
            "launchctl",
            &[
                String::from("unload"),
                String::from("-w"),
                plist_text.clone(),
            ],
        );
        let removed_service_file = if plist_path.exists() {
            fs::remove_file(&plist_path).is_ok()
        } else {
            false
        };
        rows.push(json!({
            "label": label,
            "service_target": target,
            "service_file": plist_text,
            "bootout_target": bootout_target,
            "bootout_path": bootout_path,
            "unload": unload,
            "removed_service_file": removed_service_file
        }));
    }
    json!({
        "ok": true,
        "active": true,
        "current_label": current_label,
        "rows": rows
    })
}

#[cfg(not(target_os = "macos"))]
fn cleanup_stale_launchd_labels() -> Value {
    json!({
        "ok": true,
        "active": false,
        "reason": "platform_not_macos",
        "rows": []
    })
}

fn resolved_infring_home(root: &Path) -> String {
    std::env::var("INFRING_HOME")
        .ok()
        .or_else(|| std::env::var("PROTHEUS_HOME").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

fn verify_gateway_service_root(result_payload: &Value, expected_root: &str) -> Value {
    let service_file = result_payload
        .get("service_file")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if service_file.is_empty() {
        return json!({
            "checked": false,
            "ok": true,
            "reason": "service_file_unavailable"
        });
    }
    let content = fs::read_to_string(&service_file).unwrap_or_default();
    let expected_unix = expected_root.replace('\\', "/");
    let content_unix = content.replace('\\', "/");
    let matches = content.contains(expected_root) || content_unix.contains(&expected_unix);
    json!({
        "checked": true,
        "ok": matches,
        "service_file": service_file,
        "expected_infring_home": expected_root,
    })
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
    let pre_start_cleanup = json!({
        "stale_launchd_cleanup": cleanup_stale_launchd_labels(),
        "stale_pid_cleanup": cleanup_dashboard_pid_files(root, cfg)
    });
    let supervisor_cfg = gateway_supervisor_config(cfg);
    let mut result = gateway_supervisor::enable(
        root,
        &executable,
        &supervisor_cfg,
        &dashboard_watchdog_log_path(root),
    );
    let expected_home = resolved_infring_home(root);
    let root_contract = verify_gateway_service_root(&result.payload, expected_home.as_str());
    if let Some(obj) = result.payload.as_object_mut() {
        obj.insert("pre_start_cleanup".to_string(), pre_start_cleanup);
        obj.insert("service_root_contract".to_string(), root_contract.clone());
    }
    if root_contract.get("checked").and_then(Value::as_bool) == Some(true)
        && root_contract.get("ok").and_then(Value::as_bool) == Some(false)
    {
        result.active = false;
        if let Some(obj) = result.payload.as_object_mut() {
            obj.insert("ok".to_string(), Value::Bool(false));
            obj.insert(
                "error".to_string(),
                Value::String("service_root_mismatch".to_string()),
            );
            obj.insert(
                "expected_infring_home".to_string(),
                Value::String(expected_home),
            );
        }
    }
    result
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
