// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_epoch_ms, parse_cli_flag};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_POLICY_REL: &str = "client/runtime/config/public_api_catalog_policy.json";
const DEFAULT_STATE_REL: &str = "local/state/ops/public_api_catalog/state.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/public_api_catalog/history.jsonl";
const DEFAULT_FRESHNESS_DAYS: f64 = 14.0;
const DEFAULT_MIN_SYNC_ACTIONS: usize = 1;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops public-api-catalog status [--state-path=<path>] [--policy=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog sync|run [--catalog-path=<path>|--catalog-json=<json>] [--source=<label>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog search --query=<text> [--limit=<n>] [--state-path=<path>]",
    "  protheus-ops public-api-catalog integrate --action-id=<id> [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog connect --platform=<name> [--connection-key=<key>] [--access-token=<token>] [--refresh-token=<token>] [--expires-epoch-ms=<u64>] [--oauth-passthrough=1|0] [--state-path=<path>]",
    "  protheus-ops public-api-catalog import-flow [--flow-path=<path>|--flow-json=<json>] [--workflow-id=<id>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog run-flow [--workflow-id=<id>|--flow-path=<path>] [--input-json=<json>] [--state-path=<path>] [--strict=1|0]",
    "  protheus-ops public-api-catalog verify [--state-path=<path>] [--max-age-days=<f64>] [--strict=1|0]",
];

#[derive(Debug, Clone)]
struct Policy {
    strict: bool,
    max_age_days: f64,
    min_sync_actions: usize,
    state_path: PathBuf,
    history_path: PathBuf,
    source_catalog_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CommandResult {
    exit_code: i32,
    payload: Value,
}

fn usage() {
    for row in USAGE {
        println!("{row}");
    }
}

fn print_json_line(value: &Value) {
    crate::print_json_line(value);
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    parse_cli_flag(argv, key)
}

fn first_positional(argv: &[String], skip: usize) -> Option<String> {
    argv.iter()
        .skip(skip)
        .find(|token| !token.trim_start().starts_with('-'))
        .cloned()
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if ["1", "true", "yes", "on"].contains(&v.as_str()) => true,
        Some(v) if ["0", "false", "no", "off"].contains(&v.as_str()) => false,
        _ => fallback,
    }
}

fn parse_u64(raw: Option<String>) -> Option<u64> {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
}

fn parse_f64(raw: Option<String>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
}

fn parse_usize(raw: Option<String>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .map(|v| v.clamp(min, max))
        .unwrap_or(fallback)
}

fn parse_json_flag(argv: &[String], key: &str) -> Option<Value> {
    parse_flag(argv, key).and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn clean_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn normalize_method(value: &str) -> String {
    match value.trim().to_ascii_uppercase().as_str() {
        "GET" => "GET".to_string(),
        "PUT" => "PUT".to_string(),
        "PATCH" => "PATCH".to_string(),
        "DELETE" => "DELETE".to_string(),
        _ => "POST".to_string(),
    }
}

fn hash_fingerprint(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("sha256:{}", &digest[..16.min(digest.len())])
}

fn resolve_root(cli_root: &Path) -> PathBuf {
    std::env::var("PUBLIC_API_CATALOG_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| cli_root.to_path_buf())
}

fn resolve_path(root: &Path, raw: Option<String>, fallback_rel: &str) -> PathBuf {
    match raw {
        Some(v) if !v.trim().is_empty() => {
            let p = PathBuf::from(v);
            if p.is_absolute() {
                p
            } else {
                root.join(p)
            }
        }
        _ => root.join(fallback_rel),
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(
        &tmp,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut line = serde_json::to_string(value).map_err(|e| e.to_string())?;
    line.push('\n');
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

fn with_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(&value));
    value
}
