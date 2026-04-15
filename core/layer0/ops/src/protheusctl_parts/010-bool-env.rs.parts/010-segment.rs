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
#[path = "../../protheusctl_routes.rs"]
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

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse_workspace_root_candidate(raw: &str) -> Option<PathBuf> {
    let base = PathBuf::from(raw.trim());
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
}

fn workspace_root_from_env() -> Option<PathBuf> {
    [INFRING_WORKSPACE_ROOT_ENV, PROTHEUS_WORKSPACE_ROOT_ENV]
        .iter()
        .filter_map(|name| env_nonempty(name))
        .find_map(|raw| parse_workspace_root_candidate(&raw))
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
    if let Some(root) = workspace_root_from_env() {
        return Some(root);
    }

    let mut cursor = Some(start);
    while let Some(path) = cursor {
        if parse_workspace_root_candidate(path.to_string_lossy().as_ref()).is_some() {
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
    if let Some(raw) = env_nonempty("PROTHEUS_RUNTIME_MODE_STATE_PATH") {
        return PathBuf::from(raw);
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
