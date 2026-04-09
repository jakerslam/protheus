// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_LAUNCHD_LABEL: &str = "ai.infring.gateway";
#[cfg(target_os = "linux")]
const DEFAULT_SYSTEMD_UNIT: &str = "infring-gateway.service";

#[derive(Debug, Clone)]
pub struct GatewaySupervisorConfig {
    pub host: String,
    pub port: u16,
    pub team: String,
    pub refresh_ms: u64,
    pub ready_timeout_ms: u64,
    pub watchdog_interval_ms: u64,
    pub node_binary: String,
}

#[derive(Debug, Clone)]
pub struct GatewaySupervisorResult {
    pub active: bool,
    pub payload: Value,
}

fn trim_text(text: String, max_len: usize) -> String {
    let mut out = text.trim().to_string();
    if out.len() <= max_len {
        return out;
    }
    out.truncate(max_len);
    out
}

fn run_command(program: &str, args: &[String], cwd: Option<&Path>) -> Value {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    match cmd.output() {
        Ok(out) => json!({
            "ok": out.status.success(),
            "status": out.status.code().unwrap_or(-1),
            "program": program,
            "args": args,
            "stdout": trim_text(String::from_utf8_lossy(&out.stdout).to_string(), 12000),
            "stderr": trim_text(String::from_utf8_lossy(&out.stderr).to_string(), 12000),
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

fn command_ok(payload: &Value) -> bool {
    payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

fn home_dir() -> Option<PathBuf> {
    env::var("HOME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[allow(dead_code)]
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let mut out = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn launchd_label() -> String {
    env::var("INFRING_GATEWAY_LAUNCHD_LABEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_LAUNCHD_LABEL.to_string())
}

#[cfg(target_os = "linux")]
fn systemd_unit_name() -> String {
    env::var("INFRING_GATEWAY_SYSTEMD_UNIT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_SYSTEMD_UNIT.to_string())
}

fn watchdog_args(executable: &Path, cfg: &GatewaySupervisorConfig) -> Vec<String> {
    vec![
        executable.to_string_lossy().to_string(),
        "daemon-control".to_string(),
        "watchdog".to_string(),
        "--dashboard-autoboot=1".to_string(),
        "--gateway-persist=1".to_string(),
        format!("--dashboard-host={}", cfg.host),
        format!("--dashboard-port={}", cfg.port),
        format!("--dashboard-team={}", cfg.team),
        format!("--dashboard-refresh-ms={}", cfg.refresh_ms),
        format!("--dashboard-ready-timeout-ms={}", cfg.ready_timeout_ms),
        format!(
            "--dashboard-watchdog-interval-ms={}",
            cfg.watchdog_interval_ms
        ),
        format!("--node-binary={}", cfg.node_binary),
    ]
}

fn unsupported_payload(action: &str, reason: &str) -> GatewaySupervisorResult {
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": false,
            "platform": std::env::consts::OS,
            "action": action,
            "error": reason,
        }),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn current_uid() -> Option<String> {
    if let Ok(raw) = env::var("UID") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }
    let out = run_command("id", &[String::from("-u")], None);
    out.get("stdout")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
}

#[cfg(target_os = "macos")]
fn launchd_paths() -> Option<(String, String, PathBuf)> {
    let uid = current_uid()?;
    let label = launchd_label();
    let plist = home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"));
    Some((uid, label, plist))
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(target_os = "macos")]
fn launchd_env_path() -> String {
    let mut entries = vec![
        "/usr/local/bin".to_string(),
        "/opt/homebrew/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];
    if let Some(home) = home_dir() {
        let home_text = home.to_string_lossy().to_string();
        entries.insert(0, format!("{home_text}/.local/bin"));
        entries.insert(1, format!("{home_text}/.cargo/bin"));
    }
    let mut dedup = Vec::<String>::new();
    for entry in entries {
        let clean = entry.trim().to_string();
        if clean.is_empty() || dedup.iter().any(|row| row == &clean) {
            continue;
        }
        dedup.push(clean);
    }
    dedup.join(":")
}

#[cfg(target_os = "macos")]
fn refresh_codesign_signature(executable: &Path) -> Value {
    let executable_str = executable.to_string_lossy().to_string();
    let verify_args = vec![
        "--verify".to_string(),
        "--verbose=2".to_string(),
        executable_str.clone(),
    ];
    let verify_before = run_command("codesign", &verify_args, None);
    let resign = run_command(
        "codesign",
        &[
            "--force".to_string(),
            "--sign".to_string(),
            "-".to_string(),
            executable_str.clone(),
        ],
        None,
    );
    let verify_after = run_command("codesign", &verify_args, None);
    json!({
        "ok": command_ok(&verify_after),
        "executable": executable_str,
        "verify_before": verify_before,
        "resign": resign,
        "verify_after": verify_after,
    })
}

#[cfg(target_os = "macos")]
fn render_launchd_plist(
    root: &Path,
    log_path: &Path,
    label: &str,
    watchdog_args: &[String],
) -> String {
    let launchd_home = home_dir().unwrap_or_else(|| root.to_path_buf());
    let launchd_home_text = launchd_home.to_string_lossy().to_string();
    let launchd_path = launchd_env_path();
    let watchdog_bin = watchdog_args.first().cloned().unwrap_or_default();
    let mut args_xml = String::new();
    for arg in watchdog_args {
        args_xml.push_str(&format!(
            "    <string>{}</string>\n",
            xml_escape(arg.as_str())
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{label}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
{args_xml}  </array>\n\
  <key>WorkingDirectory</key>\n\
  <string>{working_dir}</string>\n\
  <key>EnvironmentVariables</key>\n\
  <dict>\n\
    <key>HOME</key>\n\
    <string>{env_home}</string>\n\
    <key>PATH</key>\n\
    <string>{env_path}</string>\n\
    <key>PROTHEUS_OPS_ALLOW_STALE</key>\n\
    <string>1</string>\n\
    <key>PROTHEUS_NPM_ALLOW_STALE</key>\n\
    <string>1</string>\n\
    <key>PROTHEUS_NPM_BINARY</key>\n\
    <string>{env_binary}</string>\n\
  </dict>\n\
  <key>KeepAlive</key>\n\
  <true/>\n\
  <key>RunAtLoad</key>\n\
  <true/>\n\
  <key>ThrottleInterval</key>\n\
  <integer>2</integer>\n\
  <key>StandardOutPath</key>\n\
  <string>{log_file}</string>\n\
  <key>StandardErrorPath</key>\n\
  <string>{log_file}</string>\n\
</dict>\n\
</plist>\n",
        label = xml_escape(label),
        args_xml = args_xml,
        working_dir = xml_escape(root.to_string_lossy().as_ref()),
        env_home = xml_escape(launchd_home_text.as_str()),
        env_path = xml_escape(launchd_path.as_str()),
        env_binary = xml_escape(watchdog_bin.as_str()),
        log_file = xml_escape(log_path.to_string_lossy().as_ref())
    )
}

#[cfg(target_os = "macos")]
fn launchctl_state(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let trimmed = line.trim();
        let (_, value) = trimmed.split_once("state =")?;
        let state = value.trim();
        if state.is_empty() {
            None
        } else {
            Some(state.to_string())
        }
    })
}

#[cfg(target_os = "macos")]
fn launchctl_status(uid: &str, label: &str, plist_path: &Path) -> GatewaySupervisorResult {
    let target = format!("gui/{uid}/{label}");
    let print = run_command("launchctl", &[String::from("print"), target.clone()], None);
    let stdout = print
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let active = command_ok(&print);
    let service_state = launchctl_state(stdout);
    let running = active && service_state.as_deref() == Some("running");
    GatewaySupervisorResult {
        active,
        payload: json!({
            "ok": true,
            "platform": "launchd",
            "label": label,
            "service_target": target,
            "service_file": plist_path.to_string_lossy().to_string(),
            "installed": plist_path.exists(),
            "active": active,
            "running": running,
            "service_state": service_state,
            "status_probe": print,
        }),
    }
}

#[cfg(target_os = "macos")]
fn launchd_enable(
    root: &Path,
    executable: &Path,
    cfg: &GatewaySupervisorConfig,
    log_path: &Path,
) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("enable", "launchd_identity_unavailable");
    };
    if let Some(parent) = plist_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let args = watchdog_args(executable, cfg);
    let codesign_refresh = refresh_codesign_signature(executable);
    let plist = render_launchd_plist(root, log_path, label.as_str(), &args);
    if let Err(err) = fs::write(&plist_path, plist) {
        return GatewaySupervisorResult {
            active: false,
            payload: json!({
                "ok": false,
                "platform": "launchd",
                "action": "enable",
                "error": format!("launchd_plist_write_failed:{err}"),
                "service_file": plist_path.to_string_lossy().to_string(),
                "codesign_refresh": codesign_refresh,
            }),
        };
    }

    let target = format!("gui/{uid}/{label}");
    let domain = format!("gui/{uid}");
    let plist_str = plist_path.to_string_lossy().to_string();
    let bootout_target = run_command(
        "launchctl",
        &[String::from("bootout"), target.clone()],
        None,
    );
    let bootout_path = run_command(
        "launchctl",
        &[String::from("bootout"), domain.clone(), plist_str.clone()],
        None,
    );
    let bootstrap = run_command(
        "launchctl",
        &[String::from("bootstrap"), domain.clone(), plist_str.clone()],
        None,
    );
    let fallback_load = if !command_ok(&bootstrap) {
        Some(run_command(
            "launchctl",
            &[String::from("load"), String::from("-w"), plist_str.clone()],
            None,
        ))
    } else {
        None
    };
    let enable = run_command("launchctl", &[String::from("enable"), target.clone()], None);
    let kickstart = run_command(
        "launchctl",
        &[
            String::from("kickstart"),
            String::from("-k"),
            target.clone(),
        ],
        None,
    );

    let mut status = launchctl_status(uid.as_str(), label.as_str(), &plist_path);
    let enabled = status.active;
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("enable".to_string()));
        obj.insert("bootout_target".to_string(), bootout_target);
        obj.insert("bootout_path".to_string(), bootout_path);
        obj.insert("bootstrap".to_string(), bootstrap);
        if let Some(load) = fallback_load {
            obj.insert("fallback_load".to_string(), load);
        }
        obj.insert("codesign_refresh".to_string(), codesign_refresh);
        obj.insert("enable_cmd".to_string(), enable);
        obj.insert("kickstart".to_string(), kickstart);
        obj.insert("watchdog_args".to_string(), json!(args));
    }
    status.active = enabled;
    status
}

#[cfg(target_os = "macos")]
fn launchd_disable(_root: &Path) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("disable", "launchd_identity_unavailable");
    };
    let target = format!("gui/{uid}/{label}");
    let domain = format!("gui/{uid}");
    let plist_str = plist_path.to_string_lossy().to_string();
    let bootout_target = run_command(
        "launchctl",
        &[String::from("bootout"), target.clone()],
        None,
    );
    let bootout_path = run_command(
        "launchctl",
        &[String::from("bootout"), domain.clone(), plist_str.clone()],
        None,
    );
    let unload = run_command(
        "launchctl",
        &[
            String::from("unload"),
            String::from("-w"),
            plist_str.clone(),
        ],
        None,
    );
    let removed = fs::remove_file(&plist_path).is_ok();
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": true,
            "platform": "launchd",
            "action": "disable",
            "label": label,
            "service_target": target,
            "service_file": plist_path.to_string_lossy().to_string(),
            "removed_service_file": removed,
            "bootout_target": bootout_target,
            "bootout_path": bootout_path,
            "unload": unload,
        }),
    }
}

#[cfg(target_os = "macos")]
fn launchd_status(root: &Path) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("status", "launchd_identity_unavailable");
    };
    let mut status = launchctl_status(uid.as_str(), label.as_str(), &plist_path);
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("status".to_string()));
        obj.insert(
            "root".to_string(),
            Value::String(root.to_string_lossy().to_string()),
        );
    }
    status
}

#[cfg(target_os = "linux")]
fn systemd_paths() -> Option<(String, PathBuf)> {
    let unit = systemd_unit_name();
    let service_path = home_dir()?
        .join(".config")
        .join("systemd")
        .join("user")
        .join(unit.as_str());
    Some((unit, service_path))
}

#[cfg(target_os = "linux")]
fn render_systemd_service(root: &Path, cfg: &GatewaySupervisorConfig, executable: &Path) -> String {
    let args = watchdog_args(executable, cfg)
        .into_iter()
        .map(|arg| shell_quote(arg.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let command = format!("{args}");
    format!(
        "[Unit]\n\
Description=Infring Gateway Watchdog\n\
After=network.target\n\
\n\
[Service]\n\
Type=simple\n\
WorkingDirectory={working_dir}\n\
ExecStart=/bin/sh -lc {exec_start}\n\
Restart=always\n\
RestartSec=1\n\
KillMode=process\n\
Environment=PROTHEUS_ROOT={root_env}\n\
\n\
[Install]\n\
WantedBy=default.target\n",
        working_dir = root.to_string_lossy(),
        exec_start = shell_quote(command.as_str()),
        root_env = root.to_string_lossy(),
    )
}

#[cfg(target_os = "linux")]
fn systemd_status(root: &Path) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("status", "systemd_identity_unavailable");
    };
    let active_cmd = run_command(
        "systemctl",
        &[
            String::from("--user"),
            String::from("is-active"),
            unit.clone(),
        ],
        None,
    );
    let enabled_cmd = run_command(
        "systemctl",
        &[
            String::from("--user"),
            String::from("is-enabled"),
            unit.clone(),
        ],
        None,
    );
    let active = command_ok(&active_cmd)
        && active_cmd
            .get("stdout")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("active");
    GatewaySupervisorResult {
        active,
        payload: json!({
            "ok": true,
            "platform": "systemd-user",
            "action": "status",
            "unit": unit,
            "service_file": service_path.to_string_lossy().to_string(),
            "installed": service_path.exists(),
            "active": active,
            "enabled": command_ok(&enabled_cmd),
            "active_cmd": active_cmd,
            "enabled_cmd": enabled_cmd,
            "root": root.to_string_lossy().to_string(),
        }),
    }
}

#[cfg(target_os = "linux")]
fn systemd_enable(
    root: &Path,
    executable: &Path,
    cfg: &GatewaySupervisorConfig,
) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("enable", "systemd_identity_unavailable");
    };
    if let Some(parent) = service_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let service = render_systemd_service(root, cfg, executable);
    if let Err(err) = fs::write(&service_path, service) {
        return GatewaySupervisorResult {
            active: false,
            payload: json!({
                "ok": false,
                "platform": "systemd-user",
                "action": "enable",
                "error": format!("systemd_service_write_failed:{err}"),
                "service_file": service_path.to_string_lossy().to_string(),
            }),
        };
    }

    let daemon_reload = run_command(
        "systemctl",
        &[String::from("--user"), String::from("daemon-reload")],
        None,
    );
    let enable_now = run_command(
        "systemctl",
        &[
            String::from("--user"),
            String::from("enable"),
            String::from("--now"),
            unit.clone(),
        ],
        None,
    );
    let restart = run_command(
        "systemctl",
        &[
            String::from("--user"),
            String::from("restart"),
            unit.clone(),
        ],
        None,
    );

    let mut status = systemd_status(root);
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("enable".to_string()));
        obj.insert("daemon_reload".to_string(), daemon_reload);
        obj.insert("enable_now".to_string(), enable_now);
        obj.insert("restart".to_string(), restart);
    }
    status
}

#[cfg(target_os = "linux")]
fn systemd_disable(root: &Path) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("disable", "systemd_identity_unavailable");
    };
    let disable_now = run_command(
        "systemctl",
        &[
            String::from("--user"),
            String::from("disable"),
            String::from("--now"),
            unit.clone(),
        ],
        None,
    );
    let removed = fs::remove_file(&service_path).is_ok();
    let daemon_reload = run_command(
        "systemctl",
        &[String::from("--user"), String::from("daemon-reload")],
        None,
    );
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": true,
            "platform": "systemd-user",
            "action": "disable",
            "unit": unit,
            "service_file": service_path.to_string_lossy().to_string(),
            "removed_service_file": removed,
            "disable_now": disable_now,
            "daemon_reload": daemon_reload,
            "root": root.to_string_lossy().to_string(),
        }),
    }
}

pub fn enable(
    root: &Path,
    executable: &Path,
    cfg: &GatewaySupervisorConfig,
    log_path: &Path,
) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_enable(root, executable, cfg, log_path);
    }
    #[cfg(target_os = "linux")]
    {
        let _ = log_path;
        return systemd_enable(root, executable, cfg);
    }
    #[allow(unreachable_code)]
    unsupported_payload("enable", "platform_not_supported")
}

pub fn disable(root: &Path) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_disable(root);
    }
    #[cfg(target_os = "linux")]
    {
        return systemd_disable(root);
    }
    #[allow(unreachable_code)]
    unsupported_payload("disable", "platform_not_supported")
}

pub fn status(root: &Path) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_status(root);
    }
    #[cfg(target_os = "linux")]
    {
        return systemd_status(root);
    }
    #[allow(unreachable_code)]
    unsupported_payload("status", "platform_not_supported")
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::render_systemd_service;
    #[cfg(target_os = "macos")]
    use super::{launchctl_state, render_launchd_plist};
    use super::{shell_quote, trim_text, watchdog_args, GatewaySupervisorConfig};
    use std::path::Path;

    #[test]
    fn shell_quote_escapes_single_quotes() {
        let input = "a'b";
        let quoted = shell_quote(input);
        assert_eq!(quoted, "'a'\\''b'");
    }

    #[test]
    fn trim_text_caps_output() {
        let raw = "x".repeat(20);
        let trimmed = trim_text(raw, 8);
        assert_eq!(trimmed.len(), 8);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchctl_state_extracts_running_state() {
        let stdout = "service = {\n    state = running\n}";
        assert_eq!(launchctl_state(stdout).as_deref(), Some("running"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchctl_state_handles_missing_state() {
        let stdout = "service = {\n    pid = 42\n}";
        assert!(launchctl_state(stdout).is_none());
    }

    #[test]
    fn watchdog_args_include_watchdog_command() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let args = watchdog_args(Path::new("/tmp/protheus-ops"), &cfg);
        assert_eq!(args.get(1).map(String::as_str), Some("daemon-control"));
        assert_eq!(args.get(2).map(String::as_str), Some("watchdog"));
        assert!(args
            .iter()
            .any(|row| row == "--dashboard-watchdog-interval-ms=2000"));
        assert!(args.iter().any(|row| row == "--node-binary=/usr/bin/node"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_includes_bootstrap_environment() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let args = watchdog_args(Path::new("/tmp/protheus-ops"), &cfg);
        let plist = render_launchd_plist(
            Path::new("/tmp/workspace"),
            Path::new("/tmp/watchdog.log"),
            "ai.infring.gateway",
            &args,
        );
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("<key>PROTHEUS_OPS_ALLOW_STALE</key>"));
        assert!(plist.contains("<key>PROTHEUS_NPM_ALLOW_STALE</key>"));
        assert!(plist.contains("<key>PROTHEUS_NPM_BINARY</key>"));
        assert!(plist.contains("/tmp/protheus-ops"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_service_contains_restart_and_watchdog() {
        let cfg = GatewaySupervisorConfig {
            host: "127.0.0.1".to_string(),
            port: 4173,
            team: "ops".to_string(),
            refresh_ms: 2000,
            ready_timeout_ms: 36000,
            watchdog_interval_ms: 2000,
            node_binary: "/usr/bin/node".to_string(),
        };
        let service = render_systemd_service(
            Path::new("/tmp/workspace"),
            &cfg,
            Path::new("/tmp/protheus-ops"),
        );
        assert!(service.contains("Restart=always"));
        assert!(service.contains("daemon-control watchdog"));
    }
}
