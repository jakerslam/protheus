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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string())
}

fn has_node_runtime() -> bool {
    Command::new(node_bin())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
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
    for cmd in [
        "gateway [start|stop|restart|status] [--dashboard-open=1|0] [--gateway-persist=1|0]",
        "start [--dashboard-autoboot=1|0] [--dashboard-open=1|0] [--gateway-persist=1|0]",
        "stop",
        "restart",
        "task <submit|status|list|cancel|worker|slow-test> [flags]",
        "dashboard",
        "status",
        "session <status|register|resume|send|list>",
        "rag <status|search|chat|memory>",
        "memory <status|search>",
        "adaptive <status|propose|shadow-train|prioritize|graduate>",
        "enterprise-hardening <run|status|export-compliance|identity-surface|certify-scale|dashboard>",
        "benchmark <run|status>",
        "alpha-check [--strict=1|0] [--run-gates=1|0]",
        "research <status|diagnostics|fetch>",
        "help",
        "list",
        "version",
    ] {
        println!("  - {cmd}");
    }
    println!();
    println!("Install Node.js 22+ to unlock all CLI commands.");
    println!("Suggested install command: {}", node_install_command_hint());
    println!("Tip: rerun installer with --install-node to attempt automatic installation.");
}

fn emit_node_missing_error(cmd: &str, script_rel: &str) -> i32 {
    let install_hint = node_install_command_hint();
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
            "auto_install_hint": "Rerun installer with --install-node to attempt automatic Node installation."
        })
    );
    1
}

fn node_missing_fallback(root: &Path, route: &Route, json_mode: bool) -> Option<i32> {
    match route.script_rel.as_str() {
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
            if route.args.first().map(String::as_str) == Some("version") =>
        {
            let version =
                workspace_package_version(root).unwrap_or_else(|| "0.0.0-unknown".to_string());
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
