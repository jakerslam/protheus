// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative)

use crate::deterministic_receipt_hash;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const DASHBOARD_CONNECT_TIMEOUT_MS: u64 = 1_500;
const DASHBOARD_IO_TIMEOUT_MS: u64 = 30_000;
const DASHBOARD_HEALTH_MAX_BYTES: usize = 4096;
const DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS: u64 = 2_000;
const DASHBOARD_WATCHDOG_INTERVAL_MIN_MS: u64 = 500;
const DASHBOARD_WATCHDOG_INTERVAL_MAX_MS: u64 = 60_000;
const DASHBOARD_WATCHDOG_STABLE_RETRIES: usize = 2;
const DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD: usize = 6;

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn parse_mode(argv: &[String]) -> Option<String> {
    for token in argv {
        if let Some(value) = token.strip_prefix("--mode=") {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let key_token = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&pref) {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
        if token == key_token {
            if let Some(next) = argv.get(idx + 1) {
                let out = next.trim().to_string();
                if !out.is_empty() {
                    return Some(out);
                }
            }
        }
        idx += 1;
    }
    None
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_u16(raw: Option<&str>, fallback: u16) -> u16 {
    raw.and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(fallback)
}

fn parse_u64(raw: Option<&str>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

#[derive(Debug, Clone)]
struct DashboardLaunchConfig {
    enabled: bool,
    open_browser: bool,
    host: String,
    port: u16,
    team: String,
    refresh_ms: u64,
    ready_timeout_ms: u64,
    watchdog_interval_ms: u64,
}

impl DashboardLaunchConfig {
    fn url(&self) -> String {
        format!("http://{}:{}/dashboard", self.host, self.port)
    }
}

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
    DashboardLaunchConfig {
        enabled,
        open_browser,
        host,
        port,
        team,
        refresh_ms,
        ready_timeout_ms,
        watchdog_interval_ms,
    }
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

fn dashboard_listener_pids(port: u16) -> Vec<u32> {
    #[cfg(unix)]
    {
        let query = format!("TCP:{port}");
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

fn dashboard_health_ok(host: &str, port: u16) -> bool {
    let addr = format!("{host}:{port}");
    let mut resolved = match addr.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => return false,
    };
    let Some(sock_addr) = resolved.next() else {
        return false;
    };
    let mut stream = match TcpStream::connect_timeout(
        &sock_addr,
        Duration::from_millis(DASHBOARD_CONNECT_TIMEOUT_MS),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(DASHBOARD_IO_TIMEOUT_MS)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(DASHBOARD_IO_TIMEOUT_MS)));
    if stream
        .write_all(
            format!("GET /healthz HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
    .is_err()
    {
        return false;
    }

    let mut collected = Vec::<u8>::new();
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                collected.extend_from_slice(&buf[..n]);
                if collected.len() > DASHBOARD_HEALTH_MAX_BYTES {
                    collected.truncate(DASHBOARD_HEALTH_MAX_BYTES);
                }
                if String::from_utf8_lossy(&collected).contains("200 OK") {
                    return true;
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => return false,
        }
    }
    String::from_utf8_lossy(&collected).contains("200 OK")
}

fn wait_for_dashboard(host: &str, port: u16, attempts: usize) -> bool {
    for _ in 0..attempts {
        if dashboard_health_ok(host, port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn wait_for_dashboard_stable(
    host: &str,
    port: u16,
    attempts: usize,
    required_successes: usize,
) -> bool {
    let needed = required_successes.max(1);
    let mut ok_streak = 0usize;
    for _ in 0..attempts {
        if dashboard_health_ok(host, port) {
            ok_streak += 1;
            if ok_streak >= needed {
                return true;
            }
        } else {
            ok_streak = 0;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn dashboard_wait_attempts(cfg: &DashboardLaunchConfig) -> usize {
    let ticks = (cfg.ready_timeout_ms / 150).max(1);
    usize::try_from(ticks).unwrap_or(240).clamp(1, 1200)
}

fn dashboard_log_tail(root: &Path, lines: usize) -> String {
    let log_path = dashboard_log_path(root);
    let raw = fs::read_to_string(log_path).unwrap_or_default();
    let mut tail = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .take(lines)
        .collect::<Vec<_>>();
    tail.reverse();
    tail.join("\n")
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return Command::new("open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(target_os = "linux")]
    {
        return Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[allow(unreachable_code)]
    false
}

fn clear_dashboard_stop_latch(root: &Path) {
    let _ = fs::remove_file(dashboard_stop_latch_path(root));
}

fn set_dashboard_stop_latch(root: &Path) {
    let latch = dashboard_stop_latch_path(root);
    if let Some(parent) = latch.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(latch, b"stop\n");
}

fn dashboard_stop_latch_active(root: &Path) -> bool {
    dashboard_stop_latch_path(root).exists()
}

fn append_watchdog_log(root: &Path, payload: &Value) {
    let path = dashboard_watchdog_log_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{}",
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn kill_dashboard_process(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let pid_path = dashboard_pid_path(root);
    let raw = fs::read_to_string(&pid_path).unwrap_or_default();
    let mut candidate_pids = Vec::<u32>::new();
    if let Some(pid) = raw.trim().parse::<u32>().ok() {
        candidate_pids.push(pid);
    }

    for pid in dashboard_listener_pids(cfg.port) {
        if !candidate_pids.contains(&pid) {
            candidate_pids.push(pid);
        }
    }

    let mut killed_pids = Vec::<u32>::new();
    for pid in candidate_pids {
        if kill_pid(pid) {
            killed_pids.push(pid);
        }
    }

    let _ = fs::remove_file(pid_path);
    let still_running = wait_for_dashboard(cfg.host.as_str(), cfg.port, 5);
    json!({
        "ok": true,
        "stopped": !killed_pids.is_empty() && !still_running,
        "killed_pids": killed_pids,
        "host": cfg.host.as_str(),
        "port": cfg.port,
        "still_running": still_running,
        "reason": if killed_pids.is_empty() { "no_pid_killed" } else { "pid_killed" }
    })
}

fn restart_dashboard_for_watchdog(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let previous_pid = read_pid_file(&dashboard_pid_path(root));
    let previous_running = previous_pid.map(pid_running).unwrap_or(false);
    let listeners_before = dashboard_listener_pids(cfg.port);
    let had_listener = !listeners_before.is_empty();

    let stop_attempt = if previous_running || had_listener {
        kill_dashboard_process(root, cfg)
    } else {
        json!({
            "ok": true,
            "stopped": false,
            "reason": "nothing_to_stop",
            "killed_pids": []
        })
    };

    let wait_attempts = dashboard_wait_attempts(cfg);
    let spawn = spawn_dashboard(root, cfg);
    let spawned_pid = spawn.as_ref().ok().copied();
    let spawn_error = spawn.as_ref().err().cloned();
    let launched = spawn.is_ok();
    let running = if launched {
        wait_for_dashboard_stable(
            cfg.host.as_str(),
            cfg.port,
            wait_attempts,
            DASHBOARD_WATCHDOG_STABLE_RETRIES,
        )
    } else {
        false
    };
    let pid = read_pid_file(&dashboard_pid_path(root)).or(spawned_pid);
    let mut out = json!({
        "ok": true,
        "running": running,
        "launched": launched,
        "pid": pid,
        "previous_pid": previous_pid,
        "previous_running": previous_running,
        "listeners_before": listeners_before,
        "stop_attempt": stop_attempt,
    });
    if let Some(err) = spawn_error {
        out["spawn_error"] = Value::String(err);
    }
    if !running {
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
    }
    out
}

fn spawn_dashboard(root: &Path, cfg: &DashboardLaunchConfig) -> Result<u32, String> {
    fs::create_dir_all(dashboard_state_dir(root))
        .map_err(|err| format!("dashboard_state_dir_create_failed:{err}"))?;
    let log_path = dashboard_log_path(root);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("dashboard_log_open_failed:{err}"))?;
    let log_err = log
        .try_clone()
        .map_err(|err| format!("dashboard_log_clone_failed:{err}"))?;
    // Canonical dashboard surface: TypeScript pipeline (OpenClaw-fork UI + chat-first controls).
    // This replaces the legacy Rust fallback dashboard shell to avoid split UI surfaces.
    let child = Command::new("node")
        .arg("client/runtime/lib/ts_entrypoint.ts")
        .arg("client/runtime/systems/ui/infring_dashboard.ts")
        .arg("serve")
        .arg(format!("--host={}", cfg.host))
        .arg(format!("--port={}", cfg.port))
        .arg(format!("--team={}", cfg.team))
        .arg(format!("--refresh-ms={}", cfg.refresh_ms))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|err| format!("dashboard_spawn_failed:{err}"))?;
    let _ = fs::write(dashboard_pid_path(root), format!("{}\n", child.id()));
    Ok(child.id())
}

fn dashboard_watchdog_status(root: &Path) -> Value {
    let pid_path = dashboard_watchdog_pid_path(root);
    let pid = read_pid_file(&pid_path);
    let running = pid.map(pid_running).unwrap_or(false);
    if !running {
        let _ = fs::remove_file(pid_path);
    }
    json!({
        "pid": pid,
        "running": running,
        "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
        "stop_latch_active": dashboard_stop_latch_active(root),
    })
}

fn stop_dashboard_watchdog(root: &Path) -> Value {
    let pid_path = dashboard_watchdog_pid_path(root);
    let pid = read_pid_file(&pid_path);
    let stopped = pid.map(kill_pid).unwrap_or(false);
    let _ = fs::remove_file(pid_path);
    json!({
        "ok": true,
        "stopped": stopped,
        "pid": pid,
        "stop_latch_active": dashboard_stop_latch_active(root),
    })
}

fn spawn_dashboard_watchdog(root: &Path, cfg: &DashboardLaunchConfig) -> Result<u32, String> {
    fs::create_dir_all(dashboard_state_dir(root))
        .map_err(|err| format!("dashboard_state_dir_create_failed:{err}"))?;
    let status = dashboard_watchdog_status(root);
    if status.get("running").and_then(Value::as_bool) == Some(true) {
        if let Some(pid) = status
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        {
            return Ok(pid);
        }
    }
    let log_path = dashboard_watchdog_log_path(root);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("dashboard_watchdog_log_open_failed:{err}"))?;
    let log_err = log
        .try_clone()
        .map_err(|err| format!("dashboard_watchdog_log_clone_failed:{err}"))?;
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("dashboard_watchdog_current_exe_failed:{err}"))?;
    let executable = resolve_dashboard_executable(&current_exe);
    let child = Command::new(executable)
        .arg("daemon-control")
        .arg("watchdog")
        .arg(format!("--dashboard-host={}", cfg.host))
        .arg(format!("--dashboard-port={}", cfg.port))
        .arg(format!("--dashboard-team={}", cfg.team))
        .arg(format!("--dashboard-refresh-ms={}", cfg.refresh_ms))
        .arg(format!(
            "--dashboard-ready-timeout-ms={}",
            cfg.ready_timeout_ms
        ))
        .arg(format!(
            "--dashboard-watchdog-interval-ms={}",
            cfg.watchdog_interval_ms
        ))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|err| format!("dashboard_watchdog_spawn_failed:{err}"))?;
    let _ = fs::write(dashboard_watchdog_pid_path(root), format!("{}\n", child.id()));
    Ok(child.id())
}

fn start_dashboard_with_config(root: &Path, cfg: &DashboardLaunchConfig, allow_browser: bool) -> Value {
    let url = cfg.url();
    if !cfg.enabled {
        return json!({
            "enabled": false,
            "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
            "opened_browser": false,
            "url": url
        });
    }
    if dashboard_health_ok(cfg.host.as_str(), cfg.port) {
        return json!({
            "enabled": true,
            "running": true,
            "launched": false,
            "opened_browser": false,
            "url": url,
            "ready_timeout_ms": cfg.ready_timeout_ms
        });
    }

    let wait_attempts = dashboard_wait_attempts(&cfg);
    let first_spawn = spawn_dashboard(root, &cfg);
    let mut running = wait_for_dashboard_stable(
        cfg.host.as_str(),
        cfg.port,
        wait_attempts,
        DASHBOARD_WATCHDOG_STABLE_RETRIES,
    );
    let mut launched = first_spawn.is_ok();
    let mut pid = first_spawn.as_ref().ok().copied();
    let mut recovery_attempted = false;
    let mut recovery = Value::Null;

    if !running {
        recovery_attempted = true;
        let stopped = kill_dashboard_process(root, &cfg);
        let second_spawn = spawn_dashboard(root, &cfg);
        running = wait_for_dashboard_stable(
            cfg.host.as_str(),
            cfg.port,
            wait_attempts,
            DASHBOARD_WATCHDOG_STABLE_RETRIES,
        );
        launched = second_spawn.is_ok();
        pid = second_spawn.as_ref().ok().copied();
        let mut recovery_payload = json!({
            "attempted": true,
            "stopped": stopped,
            "launched": second_spawn.is_ok()
        });
        if let Err(err) = second_spawn {
            recovery_payload["error"] = Value::String(err);
        }
        recovery = recovery_payload;
    }

    let mut out = json!({
        "enabled": true,
        "running": running,
        "launched": launched,
        "pid": pid,
        "opened_browser": false,
        "url": url,
        "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
        "ready_timeout_ms": cfg.ready_timeout_ms,
        "recovery_attempted": recovery_attempted,
        "recovery": recovery
    });
    if allow_browser && cfg.open_browser && running {
        out["opened_browser"] = Value::Bool(open_browser(cfg.url().as_str()));
    }
    if let Err(err) = first_spawn {
        out["spawn_error"] = Value::String(err);
    }
    if !running {
        out["error"] = Value::String("dashboard_healthz_not_ready".to_string());
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
    }
    if running {
        let watchdog = spawn_dashboard_watchdog(root, cfg);
        out["watchdog"] = match watchdog {
            Ok(pid) => json!({
                "running": true,
                "pid": pid,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
            Err(err) => json!({
                "running": false,
                "error": err,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
        };
    } else {
        out["watchdog"] = dashboard_watchdog_status(root);
    }
    out
}

fn start_dashboard_if_enabled(root: &Path, argv: &[String], command: &str) -> Value {
    let cfg = parse_dashboard_launch_config(argv, command);
    start_dashboard_with_config(root, &cfg, true)
}

fn run_dashboard_watchdog(root: &Path, argv: &[String]) -> i32 {
    let cfg = parse_dashboard_launch_config(argv, "start");
    if !cfg.enabled {
        print_json_line(&json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "running": false,
            "reason": "dashboard_disabled",
            "host": cfg.host,
            "port": cfg.port,
        }));
        return 0;
    }
    let _ = fs::write(dashboard_watchdog_pid_path(root), format!("{}\n", std::process::id()));
    append_watchdog_log(
        root,
        &json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "event": "started",
            "pid": std::process::id(),
            "host": cfg.host,
            "port": cfg.port,
            "interval_ms": cfg.watchdog_interval_ms,
            "fail_streak_threshold": DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD,
        }),
    );
    let mut fail_streak = 0usize;
    let mut last_health: Option<bool> = None;
    loop {
        if dashboard_stop_latch_active(root) {
            break;
        }
        let healthy = dashboard_health_ok(cfg.host.as_str(), cfg.port);
        if last_health != Some(healthy) {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "health_transition",
                    "healthy": healthy,
                    "fail_streak": fail_streak,
                    "dashboard_pid": read_pid_file(&dashboard_pid_path(root)),
                }),
            );
            last_health = Some(healthy);
        }
        if healthy {
            fail_streak = 0;
        } else {
            fail_streak = fail_streak.saturating_add(1);
        }
        if fail_streak >= DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_triggered",
                    "fail_streak": fail_streak,
                }),
            );
            let restarted = restart_dashboard_for_watchdog(root, &cfg);
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_result",
                    "payload": restarted,
                }),
            );
            if restarted.get("running").and_then(Value::as_bool) == Some(true) {
                fail_streak = 0;
            } else {
                std::thread::sleep(Duration::from_millis(1_500));
            }
        }
        std::thread::sleep(Duration::from_millis(cfg.watchdog_interval_ms));
    }
    let _ = fs::remove_file(dashboard_watchdog_pid_path(root));
    append_watchdog_log(
        root,
        &json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "event": "stopped",
            "reason": "stop_latch",
            "host": cfg.host,
            "port": cfg.port,
        }),
    );
    print_json_line(&json!({
        "ok": true,
        "type": "dashboard_watchdog",
        "running": false,
        "reason": "stop_latch",
        "host": cfg.host,
        "port": cfg.port,
    }));
    0
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops daemon-control <start|stop|restart|status|attach|subscribe|tick|diagnostics|watchdog> [--mode=<value>]");
    println!("  Optional start/restart flags:");
    println!("    --dashboard-autoboot=1|0   (default: 1)");
    println!("    --dashboard-open=1|0       (default: 1)");
    println!("    --dashboard-host=<ip>      (default: 127.0.0.1)");
    println!("    --dashboard-port=<n>       (default: 4173)");
    println!("    --dashboard-ready-timeout-ms=<n> (default: 36000)");
    println!("    --dashboard-watchdog-interval-ms=<n> (default: 2000)");
}

pub(crate) fn success_receipt(
    command: &str,
    mode: Option<&str>,
    argv: &[String],
    root: &Path,
) -> Value {
    let mut out = protheus_ops_core_v1::daemon_control_receipt(command, mode);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("argv".to_string(), json!(argv));
        obj.insert(
            "root".to_string(),
            Value::String(root.to_string_lossy().to_string()),
        );
        obj.insert(
            "lazy_init".to_string(),
            json!({
                "enabled": true,
                "boot_scope": ["conduit", "safety_kernel"],
                "deferred": ["layer0_noncritical", "layer1_policy_extensions", "client_surfaces"]
            }),
        );
        obj.insert(
            "claim_evidence".to_string(),
            json!([
                {
                    "id": "daemon_control_core_lane",
                    "claim": "daemon_control_commands_are_core_authoritative",
                    "evidence": {
                        "command": command,
                        "mode": mode
                    }
                }
            ]),
        );
    }
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn inprocess_lazy_probe_receipt(root: &Path) -> Value {
    success_receipt(
        "start",
        Some("lazy-minimal"),
        &[
            "start".to_string(),
            "--mode=lazy-minimal".to_string(),
            "--lazy-init=1".to_string(),
        ],
        root,
    )
}

fn error_receipt(error: &str, argv: &[String]) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "daemon_control_error",
        "error": error,
        "argv": argv,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let mode = parse_mode(argv)
        .or_else(|| std::env::var("PROTHEUSD_DEFAULT_COMMAND").ok())
        .filter(|value| !value.trim().is_empty());

    if command == "watchdog" {
        return run_dashboard_watchdog(root, argv);
    }

    if matches!(
        command.as_str(),
        "start" | "stop" | "restart" | "status" | "attach" | "subscribe" | "tick" | "diagnostics"
    ) {
        let mut receipt = success_receipt(command.as_str(), mode.as_deref(), argv, root);
        let dashboard = match command.as_str() {
            "start" => {
                clear_dashboard_stop_latch(root);
                start_dashboard_if_enabled(root, argv, "start")
            }
            "restart" => {
                set_dashboard_stop_latch(root);
                let cfg = parse_dashboard_launch_config(argv, "restart");
                let watchdog_stopped = stop_dashboard_watchdog(root);
                let stopped = kill_dashboard_process(root, &cfg);
                clear_dashboard_stop_latch(root);
                let started = start_dashboard_if_enabled(root, argv, "restart");
                json!({
                    "watchdog_stopped": watchdog_stopped,
                    "stopped": stopped,
                    "started": started
                })
            }
            "stop" => {
                set_dashboard_stop_latch(root);
                let cfg = parse_dashboard_launch_config(argv, "stop");
                let watchdog_stopped = stop_dashboard_watchdog(root);
                let stopped = kill_dashboard_process(root, &cfg);
                json!({
                    "watchdog_stopped": watchdog_stopped,
                    "stopped": stopped
                })
            }
            "status" => {
                let cfg = parse_dashboard_launch_config(argv, "start");
                json!({
                    "enabled": cfg.enabled,
                    "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
                    "url": cfg.url(),
                    "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
                    "stop_latch_active": dashboard_stop_latch_active(root),
                    "watchdog": dashboard_watchdog_status(root),
                })
            }
            _ => json!({}),
        };
        receipt["dashboard"] = dashboard;
        receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));
        print_json_line(&receipt);
        return 0;
    }

    usage();
    print_json_line(&error_receipt("unknown_command", argv));
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn payload_for(command: &str) -> Value {
        success_receipt(
            command,
            Some("persistent"),
            &[command.to_string(), "--mode=persistent".to_string()],
            Path::new("."),
        )
    }

    #[test]
    fn daemon_control_supports_attach_subscribe_and_diagnostics() {
        for command in ["attach", "subscribe", "diagnostics"] {
            let payload = payload_for(command);
            assert_eq!(
                payload.get("command").and_then(Value::as_str),
                Some(command),
                "command should round-trip in receipt"
            );
            assert!(
                payload
                    .get("receipt_hash")
                    .and_then(Value::as_str)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false),
                "receipt hash should be present"
            );
            assert_eq!(
                payload.get("type").and_then(Value::as_str),
                Some("daemon_control_receipt"),
                "core lane type should remain authoritative"
            );
        }
    }

    #[test]
    fn unknown_command_returns_error_exit_code() {
        let root = Path::new(".");
        let exit = run(root, &[String::from("not-a-command")]);
        assert_eq!(exit, 2);
    }

    #[test]
    fn dashboard_launch_config_defaults_to_autoboot_for_start() {
        let cfg = parse_dashboard_launch_config(&[], "start");
        assert!(cfg.enabled);
        assert!(cfg.open_browser);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4173);
        assert_eq!(cfg.ready_timeout_ms, 36_000);
        assert_eq!(cfg.watchdog_interval_ms, DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS);
    }

    #[test]
    fn dashboard_launch_config_respects_disable_flags() {
        let cfg = parse_dashboard_launch_config(
            &[
                "--dashboard-autoboot=0".to_string(),
                "--dashboard-open=0".to_string(),
                "--dashboard-host=0.0.0.0".to_string(),
                "--dashboard-port=4321".to_string(),
                "--dashboard-ready-timeout-ms=1200".to_string(),
                "--dashboard-watchdog-interval-ms=150".to_string(),
            ],
            "start",
        );
        assert!(!cfg.enabled);
        assert!(!cfg.open_browser);
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 4321);
        assert_eq!(cfg.ready_timeout_ms, 1_500);
        assert_eq!(cfg.watchdog_interval_ms, DASHBOARD_WATCHDOG_INTERVAL_MIN_MS);
    }

    #[test]
    fn resolve_dashboard_executable_prefers_sibling_protheus_ops_for_protheusd() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path();
        let current = dir.join("protheusd");
        let sibling = dir.join("protheus-ops");
        std::fs::write(&current, b"#!/bin/sh\n").expect("write current");
        std::fs::write(&sibling, b"#!/bin/sh\n").expect("write sibling");
        let resolved = resolve_dashboard_executable(&current);
        assert_eq!(resolved, sibling);
    }

    #[test]
    fn resolve_dashboard_executable_keeps_current_when_sibling_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let current = temp.path().join("protheusd");
        std::fs::write(&current, b"#!/bin/sh\n").expect("write current");
        let resolved = resolve_dashboard_executable(&current);
        assert_eq!(resolved, current);
    }
}
