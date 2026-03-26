// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative)

use crate::deterministic_receipt_hash;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const DASHBOARD_CONNECT_TIMEOUT_MS: u64 = 800;
const DASHBOARD_IO_TIMEOUT_MS: u64 = 1_500;
const DASHBOARD_HEALTH_MAX_BYTES: usize = 4096;

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
    DashboardLaunchConfig {
        enabled,
        open_browser,
        host,
        port,
        team,
        refresh_ms,
        ready_timeout_ms,
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

#[cfg(test)]
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

fn kill_dashboard_process(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let pid_path = dashboard_pid_path(root);
    let raw = fs::read_to_string(&pid_path).unwrap_or_default();
    let mut candidate_pids = Vec::<u32>::new();
    if let Some(pid) = raw.trim().parse::<u32>().ok() {
        candidate_pids.push(pid);
    }

    if candidate_pids.is_empty() || dashboard_health_ok(cfg.host.as_str(), cfg.port) {
        for pid in dashboard_listener_pids(cfg.port) {
            if !candidate_pids.contains(&pid) {
                candidate_pids.push(pid);
            }
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

fn start_dashboard_if_enabled(root: &Path, argv: &[String], command: &str) -> Value {
    let cfg = parse_dashboard_launch_config(argv, command);
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
    let mut running = wait_for_dashboard_stable(cfg.host.as_str(), cfg.port, wait_attempts, 2);
    let mut launched = first_spawn.is_ok();
    let mut pid = first_spawn.as_ref().ok().copied();
    let mut recovery_attempted = false;
    let mut recovery = Value::Null;

    if !running {
        recovery_attempted = true;
        let stopped = kill_dashboard_process(root, &cfg);
        let second_spawn = spawn_dashboard(root, &cfg);
        running = wait_for_dashboard_stable(cfg.host.as_str(), cfg.port, wait_attempts, 2);
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
    if cfg.open_browser && running {
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
    out
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops daemon-control <start|stop|restart|status|attach|subscribe|tick|diagnostics> [--mode=<value>]");
    println!("  Optional start/restart flags:");
    println!("    --dashboard-autoboot=1|0   (default: 1)");
    println!("    --dashboard-open=1|0       (default: 1)");
    println!("    --dashboard-host=<ip>      (default: 127.0.0.1)");
    println!("    --dashboard-port=<n>       (default: 4173)");
    println!("    --dashboard-ready-timeout-ms=<n> (default: 36000)");
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

    if matches!(
        command.as_str(),
        "start" | "stop" | "restart" | "status" | "attach" | "subscribe" | "tick" | "diagnostics"
    ) {
        let mut receipt = success_receipt(command.as_str(), mode.as_deref(), argv, root);
        let dashboard = match command.as_str() {
            "start" => start_dashboard_if_enabled(root, argv, "start"),
            "restart" => {
                let cfg = parse_dashboard_launch_config(argv, "restart");
                let stopped = kill_dashboard_process(root, &cfg);
                let started = start_dashboard_if_enabled(root, argv, "restart");
                json!({
                    "stopped": stopped,
                    "started": started
                })
            }
            "stop" => {
                let cfg = parse_dashboard_launch_config(argv, "stop");
                kill_dashboard_process(root, &cfg)
            }
            "status" => {
                let cfg = parse_dashboard_launch_config(argv, "status");
                json!({
                    "enabled": cfg.enabled,
                    "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
                    "url": cfg.url(),
                    "log_path": dashboard_log_path(root).to_string_lossy().to_string()
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
            ],
            "start",
        );
        assert!(!cfg.enabled);
        assert!(!cfg.open_browser);
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 4321);
        assert_eq!(cfg.ready_timeout_ms, 1_500);
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
