// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative)

use crate::deterministic_receipt_hash;
use crate::gateway_supervisor::{self, GatewaySupervisorConfig, GatewaySupervisorResult};
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

fn resolve_node_binary() -> String {
    if let Ok(explicit) = std::env::var("PROTHEUS_NODE_BINARY") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let locator = if cfg!(windows) { "where" } else { "which" };
    if let Ok(out) = Command::new(locator)
        .arg("node")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        if out.status.success() {
            let raw = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = raw.lines().map(str::trim).find(|row| !row.is_empty()) {
                return line.to_string();
            }
        }
    }
    "node".to_string()
}

#[derive(Debug, Clone)]
struct DashboardLaunchConfig {
    enabled: bool,
    open_browser: bool,
    persistent_supervisor: bool,
    host: String,
    port: u16,
    team: String,
    refresh_ms: u64,
    ready_timeout_ms: u64,
    watchdog_interval_ms: u64,
    node_binary: String,
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

fn dashboard_backend_binary_hint() -> Option<String> {
    let current_exe = std::env::current_exe().ok()?;
    let resolved = resolve_dashboard_executable(&current_exe);
    if resolved.is_file() {
        return Some(resolved.to_string_lossy().to_string());
    }
    None
}

