// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use persona_dispatch_security_gate::{
    evaluate_persona_dispatch_gate, CHECK_ID as PERSONA_DISPATCH_SECURITY_GATE_CHECK_ID,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::io::IsTerminal;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::{clean, client_state_root};
#[path = "../protheusctl_routes.rs"]
mod protheusctl_routes;
#[derive(Debug, Clone)]
pub struct Route {
    pub script_rel: String,
    pub args: Vec<String>,
    pub forward_stdin: bool,
}

#[derive(Debug, Clone)]
pub struct DispatchSecurity {
    pub ok: bool,
    pub reason: String,
}

const PERSONA_VALID_LENSES_ENV: &str = "PROTHEUS_CTL_PERSONA_VALID_LENSES";
const PERSONA_VALID_LENSES_DEFAULT: &str = "operator,guardian,analyst";
const PERSONA_BLOCKED_PATHS_ENV: &str = "PROTHEUS_CTL_PERSONA_BLOCKED_PATHS";
const INFRING_WORKSPACE_ROOT_ENV: &str = "INFRING_WORKSPACE_ROOT";
const PROTHEUS_WORKSPACE_ROOT_ENV: &str = "PROTHEUS_WORKSPACE_ROOT";
const INSTALL_RUNTIME_MANIFEST_REL: &str = "client/runtime/config/install_runtime_manifest_v1.txt";
const INSTALL_RUNTIME_FALLBACK_ENTRYPOINTS: &[&str] = &[
    "client/runtime/systems/ops/protheusd.ts",
    "client/runtime/systems/ops/protheus_status_dashboard.ts",
    "client/runtime/systems/ops/protheus_unknown_guard.ts",
    "client/runtime/systems/ops/protheus_completion.ts",
    "client/runtime/systems/ops/protheus_repl.ts",
    "client/runtime/systems/ops/protheus_command_list.ts",
    "client/runtime/systems/ops/protheus_version_cli.ts",
];

fn bool_env(name: &str, fallback: bool) -> bool {
    match env::var(name) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        Err(_) => fallback,
    }
}

fn csv_list_env(name: &str, fallback_csv: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_else(|_| fallback_csv.to_string())
        .split(',')
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn requested_lens_arg(args: &[String]) -> Option<String> {
    let mut idx = 0usize;
    while idx < args.len() {
        let token = args[idx].trim();
        if let Some((_, value)) = token.split_once('=') {
            if token.starts_with("--lens=") || token.starts_with("--persona-lens=") {
                let lens = value.trim();
                if !lens.is_empty() {
                    return Some(lens.to_string());
                }
            }
        } else if token == "--lens" || token == "--persona-lens" {
            if let Some(value) = args.get(idx + 1) {
                let lens = value.trim();
                if !lens.is_empty() {
                    return Some(lens.to_string());
                }
            }
        }
        idx += 1;
    }
    None
}

fn should_offer_setup(root: &Path, skip_setup: bool) -> bool {
    if skip_setup
        || bool_env("PROTHEUS_SKIP_SETUP", false)
        || bool_env("PROTHEUS_SETUP_DISABLE", false)
    {
        return false;
    }
    if bool_env("PROTHEUS_SETUP_FORCE", false) {
        return true;
    }
    let latest_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("protheus_setup_wizard")
        .join("latest.json");
    let Ok(raw) = std::fs::read_to_string(latest_path) else {
        return true;
    };
    let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    !parsed
        .get("completed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn resolve_workspace_root(start: &Path) -> Option<PathBuf> {
    let parse_workspace_root = |raw: String| -> Option<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let base = PathBuf::from(trimmed);
        let candidate = if base.is_absolute() {
            base
        } else {
            let cwd = std::env::current_dir().ok()?;
            cwd.join(base)
        };
        if candidate
            .join("core")
            .join("layer0")
            .join("ops")
            .join("Cargo.toml")
            .exists()
            && candidate.join("client").join("runtime").exists()
        {
            Some(candidate)
        } else {
            None
        }
    };

    if let Ok(raw) = env::var(INFRING_WORKSPACE_ROOT_ENV) {
        if let Some(root) = parse_workspace_root(raw) {
            return Some(root);
        }
    }
    if let Ok(raw) = env::var(PROTHEUS_WORKSPACE_ROOT_ENV) {
        if let Some(root) = parse_workspace_root(raw) {
            return Some(root);
        }
    }

    let mut cursor = Some(start);
    while let Some(path) = cursor {
        if path
            .join("core")
            .join("layer0")
            .join("ops")
            .join("Cargo.toml")
            .exists()
            && path.join("client").join("runtime").exists()
        {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }
    None
}

fn effective_workspace_root(start: &Path) -> PathBuf {
    resolve_workspace_root(start).unwrap_or_else(|| start.to_path_buf())
}

fn node_bin() -> String {
    crate::contract_lane_utils::resolve_preferred_node_binary()
}

fn has_node_runtime() -> bool {
    crate::contract_lane_utils::node_binary_usable(node_bin().as_str())
}

fn script_exists_with_ts_js_fallback(root: &Path, rel: &str) -> bool {
    let primary = root.join(rel);
    if primary.exists() {
        return true;
    }
    if rel.ends_with(".js") {
        let ts_rel = format!("{}{}", rel.trim_end_matches(".js"), ".ts");
        return root.join(ts_rel).exists();
    }
    if rel.ends_with(".ts") {
        let js_rel = format!("{}{}", rel.trim_end_matches(".ts"), ".js");
        return root.join(js_rel).exists();
    }
    false
}

fn runtime_mode_state_path(root: &Path) -> PathBuf {
    if let Ok(raw) = env::var("PROTHEUS_RUNTIME_MODE_STATE_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join("local")
        .join("state")
        .join("ops")
        .join("runtime_mode.json")
}

fn runtime_mode_from_state(root: &Path) -> Option<String> {
    let path = runtime_mode_state_path(root);
    let raw = std::fs::read_to_string(path).ok()?;
    let payload: Value = serde_json::from_str(&raw).ok()?;
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if mode == "dist" || mode == "source" {
        Some(mode)
    } else {
        None
    }
}

fn resolved_runtime_mode(root: &Path) -> String {
    let env_mode = env::var("PROTHEUS_RUNTIME_MODE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if env_mode == "dist" || env_mode == "source" {
        return env_mode;
    }
    runtime_mode_from_state(root).unwrap_or_else(|| "source".to_string())
}

fn install_runtime_manifest_entries(root: &Path) -> Vec<String> {
    let manifest_path = root.join(INSTALL_RUNTIME_MANIFEST_REL);
    let mut entries = Vec::<String>::new();
    if let Ok(raw) = std::fs::read_to_string(manifest_path) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            entries.push(trimmed.to_string());
        }
    }
    if entries.is_empty() {
        entries = INSTALL_RUNTIME_FALLBACK_ENTRYPOINTS
            .iter()
            .map(|entry| (*entry).to_string())
            .collect();
    }
    entries
}

fn runtime_missing_entrypoints_for_mode(root: &Path, runtime_mode: &str) -> Vec<String> {
    let strict_dist = runtime_mode == "dist";
    install_runtime_manifest_entries(root)
        .into_iter()
        .filter(|rel| {
            if strict_dist && rel.ends_with(".js") {
                return !root.join(rel).exists();
            }
            !script_exists_with_ts_js_fallback(root, rel)
        })
        .collect()
}

fn runtime_missing_entrypoints(root: &Path) -> Vec<String> {
    let runtime_mode = resolved_runtime_mode(root);
    runtime_missing_entrypoints_for_mode(root, runtime_mode.as_str())
}

fn command_exists(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn parse_flag_value(args: &[String], key: &str) -> Option<String> {
    let prefix = format!("--{key}=");
    let exact = format!("--{key}");
    let mut idx = 0usize;
    while idx < args.len() {
        let token = args[idx].trim();
        if let Some(value) = token.strip_prefix(&prefix) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        } else if token == exact {
            if let Some(value) = args.get(idx + 1) {
                let trimmed = value.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('-') {
                    return Some(trimmed.to_string());
                }
            }
        }
        idx += 1;
    }
    None
}

fn sanitize_dashboard_host_token(host: &str) -> String {
    let mut out = String::with_capacity(host.len());
    for ch in host.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "127.0.0.1".to_string()
    } else {
        out
    }
}

fn sanitize_dashboard_port_token(port: &str) -> String {
    let mut out = String::with_capacity(port.len());
    for ch in port.chars() {
        if ch.is_ascii_digit() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "4173".to_string()
    } else {
        out
    }
}

fn tmp_root_path() -> PathBuf {
    let raw = env::var("TMPDIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/tmp".to_string());
    PathBuf::from(raw)
}

fn dashboard_pid_file(host: &str, port: &str) -> PathBuf {
    let host_safe = sanitize_dashboard_host_token(host);
    let port_safe = sanitize_dashboard_port_token(port);
    tmp_root_path().join(format!("infring-dashboard-{host_safe}-{port_safe}.pid"))
}

fn dashboard_watchdog_pid_file(host: &str, port: &str) -> PathBuf {
    let host_safe = sanitize_dashboard_host_token(host);
    let port_safe = sanitize_dashboard_port_token(port);
    tmp_root_path().join(format!(
        "infring-dashboard-watchdog-{host_safe}-{port_safe}.pid"
    ))
}

fn read_pid_file(path: &Path) -> Option<u32> {
    let raw = std::fs::read_to_string(path).ok()?;
    let digits = raw
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

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
            let version =
                workspace_package_version(root).unwrap_or_else(|| "0.0.0-unknown".to_string());
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
                        println!("(Node.js not detected; using package.json fallback)");
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

fn security_request(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let digest_seed = serde_json::to_string(&json!({
        "script": script_rel,
        "args": args
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(digest_seed.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let now_ms = chrono::Utc::now().timestamp_millis();

    json!({
        "operation_id": clean(format!("protheusctl_dispatch_{}_{}", now_ms, &digest[..10]), 160),
        "subsystem": "ops",
        "action": "cli_dispatch",
        "actor": "client/runtime/systems/ops/protheusctl",
        "risk_class": if bool_env("PROTHEUS_CTL_SECURITY_HIGH_RISK", false) { "high" } else { "normal" },
        "payload_digest": format!("sha256:{digest}"),
        "tags": ["protheusctl", "dispatch", "foundation_lock"],
        "covenant_violation": bool_env("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION", false),
        "tamper_signal": bool_env("PROTHEUS_CTL_SECURITY_TAMPER_SIGNAL", false),
        "key_age_hours": env::var("PROTHEUS_CTL_SECURITY_KEY_AGE_HOURS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(1),
        "operator_quorum": env::var("PROTHEUS_CTL_SECURITY_OPERATOR_QUORUM").ok().and_then(|v| v.parse::<u8>().ok()).unwrap_or(2),
        "audit_receipt_nonce": clean(format!("nonce-{}-{}", &digest[..12], now_ms), 120),
        "zk_proof": clean(env::var("PROTHEUS_CTL_SECURITY_ZK_PROOF").unwrap_or_else(|_| "zk-protheusctl-dispatch".to_string()), 220),
        "ciphertext_digest": clean(format!("sha256:{}", &digest[..32]), 220),
        "state_root": clean(client_state_root(root).to_string_lossy().to_string(), 500)
    })
}

fn evaluate_persona_dispatch_security(
    script_rel: &str,
    args: &[String],
    req: &Value,
) -> DispatchSecurity {
    let requested_lens = requested_lens_arg(args);
    let valid_lenses = csv_list_env(PERSONA_VALID_LENSES_ENV, PERSONA_VALID_LENSES_DEFAULT);
    let blocked_paths = csv_list_env(PERSONA_BLOCKED_PATHS_ENV, "");
    let valid_lens_refs = valid_lenses.iter().map(String::as_str).collect::<Vec<_>>();
    let blocked_path_refs = blocked_paths.iter().map(String::as_str).collect::<Vec<_>>();
    let covenant_violation = req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tamper_signal = req
        .get("tamper_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let decision = evaluate_persona_dispatch_gate(
        script_rel,
        requested_lens.as_deref(),
        &valid_lens_refs,
        &blocked_path_refs,
        covenant_violation,
        tamper_signal,
    );
    if !decision.ok {
        return DispatchSecurity {
            ok: false,
            reason: format!(
                "security_gate_blocked:{PERSONA_DISPATCH_SECURITY_GATE_CHECK_ID}:{}",
                decision.code
            ),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}
