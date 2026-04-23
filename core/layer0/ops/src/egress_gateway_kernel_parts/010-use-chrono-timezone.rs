// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::TimeZone;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

const DEFAULT_POLICY_REL: &str = "config/egress_gateway_policy.json";
const DEFAULT_STATE_REL: &str = "local/state/security/egress_gateway/state.json";
const DEFAULT_AUDIT_REL: &str = "local/state/security/egress_gateway/audit.jsonl";

#[derive(Clone, Debug)]
struct ScopeRule {
    id: String,
    methods: Vec<String>,
    domains: Vec<String>,
    require_runtime_allowlist: bool,
    rate_caps: RateCaps,
}

#[derive(Clone, Debug, Default)]
struct RateCaps {
    per_hour: Option<u64>,
    per_day: Option<u64>,
}

#[derive(Clone, Debug)]
struct Policy {
    version: String,
    default_decision: String,
    global_rate_caps: RateCaps,
    scopes: BTreeMap<String, ScopeRule>,
}

fn usage() {
    println!("egress-gateway-kernel commands:");
    println!("  infring-ops egress-gateway-kernel load-policy [--payload-base64=<json>]");
    println!("  infring-ops egress-gateway-kernel load-state [--payload-base64=<json>]");
    println!("  infring-ops egress-gateway-kernel authorize --payload-base64=<json>");
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        let keep = matches!(lower, 'a'..='z' | '0'..='9' | '_' | '.' | ':' | '/' | '-');
        if keep {
            out.push(lower);
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("INFRING_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.to_path_buf()
}

fn runtime_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("root"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
    if let Ok(raw) = std::env::var("INFRING_RUNTIME_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let workspace = workspace_root(root);
    let candidate = workspace.join("client").join("runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn resolve_path(runtime_root: &Path, explicit: &str, fallback_rel: &str) -> PathBuf {
    let trimmed = explicit.trim();
    if trimmed.is_empty() {
        return runtime_root.join(fallback_rel);
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        runtime_root.join(trimmed)
    }
}

fn read_json_or_default(file_path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(file_path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("egress_gateway_kernel_create_dir_failed:{err}"))?;
    }
    let tmp_path = file_path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("egress_gateway_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("egress_gateway_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("egress_gateway_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("egress_gateway_kernel_create_dir_failed:{err}"))?;
    }
    use std::io::Write;
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("egress_gateway_kernel_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("egress_gateway_kernel_append_failed:{err}"))?;
    Ok(())
}

fn parse_host(raw_url: &str) -> String {
    let normalized = raw_url.trim();
    if normalized.is_empty() {
        return String::new();
    }
    let lower = normalized.to_ascii_lowercase();
    let without_scheme = if let Some(idx) = lower.find("://") {
        &lower[(idx + 3)..]
    } else {
        lower.as_str()
    };
    let host_port = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();
    host_port
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn domain_matches(host: &str, domain: &str) -> bool {
    let needle = domain.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    host == needle || host.ends_with(&format!(".{needle}"))
}

fn clean_methods(value: Option<&Value>) -> Vec<String> {
    let mut methods = value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| normalize_token(&as_str(Some(row)), 20).to_ascii_uppercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if methods.is_empty() {
        methods.push("GET".to_string());
    }
    methods
}

fn clean_domains(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| clean_text(Some(row), 160).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn to_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(number)) => number
            .as_u64()
            .or_else(|| number.as_i64().map(|raw| raw.max(0) as u64)),
        Some(Value::String(raw)) => raw.trim().parse::<u64>().ok(),
        _ => None,
    }
}
