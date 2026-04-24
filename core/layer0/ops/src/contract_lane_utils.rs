// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::contract_lane_utils (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::now_iso;

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|row| row.as_millis())
        .unwrap_or(0)
}

fn to_base36(mut value: u128) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + (digit - 10)) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

pub fn stable_id(prefix: &str, basis: &Value) -> String {
    let digest = crate::deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

pub fn parse_flag(argv: &[String], key: &str, allow_switch_true: bool) -> Option<String> {
    let with_eq = format!("--{key}=");
    let plain = format!("--{key}");
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some(v) = token.strip_prefix(&with_eq) {
            return Some(v.trim().to_string());
        }
        if token == plain {
            if let Some(next) = argv.get(i + 1) {
                if !next.trim_start().starts_with("--") {
                    return Some(next.trim().to_string());
                }
            }
            if allow_switch_true {
                return Some("true".to_string());
            }
            return None;
        }
        i += 1;
    }
    None
}
pub fn parse_cli_flags(argv: &[String]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if !token.starts_with("--") {
            i += 1;
            continue;
        }
        if let Some((k, v)) = token.split_once('=') {
            out.insert(k.trim_start_matches("--").to_string(), v.to_string());
            i += 1;
            continue;
        }
        let key = token.trim_start_matches("--").to_string();
        if let Some(next) = argv.get(i + 1) {
            if !next.starts_with("--") {
                out.insert(key, next.clone());
                i += 2;
                continue;
            }
        }
        out.insert(key, "true".to_string());
        i += 1;
    }
    out
}
pub fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    let Some(v) = raw else {
        return fallback;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}
pub fn parse_bool_extended(raw: Option<&str>, fallback: bool) -> bool {
    let Some(v) = raw else {
        return fallback;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "allow" | "enabled" => true,
        "0" | "false" | "no" | "off" | "deny" | "disabled" => false,
        _ => fallback,
    }
}
pub fn parse_u64(raw: Option<&str>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}
pub fn parse_u64_clamped(raw: Option<&str>, fallback: u64, lo: u64, hi: u64) -> u64 {
    parse_u64(raw, fallback).clamp(lo, hi)
}
pub fn parse_f64_clamped(raw: Option<&str>, fallback: f64, lo: f64, hi: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}
pub fn parse_opt_bool(raw: Option<&str>) -> Option<bool> {
    let v = raw?.trim().to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
pub fn parse_i64_clamped(raw: Option<&str>, fallback: i64, lo: i64, hi: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}
pub fn node_binary_usable(binary: &str) -> bool {
    let trimmed = binary.trim();
    if trimmed.is_empty() {
        return false;
    }
    Command::new(trimmed)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
pub fn resolve_binary_in_path(binary: &str) -> Option<String> {
    let locator = if cfg!(windows) { "where" } else { "which" };
    let out = Command::new(locator)
        .arg(binary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    raw.lines()
        .map(str::trim)
        .find(|row| !row.is_empty())
        .map(ToString::to_string)
}
pub fn resolve_node_from_runtime_root(runtime_root: &Path) -> Option<String> {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    let direct = runtime_root.join("bin").join(executable);
    if direct.is_file() {
        let candidate = direct.to_string_lossy().to_string();
        if node_binary_usable(candidate.as_str()) {
            return Some(candidate);
        }
    }
    let entries = fs::read_dir(runtime_root).ok()?;
    for entry in entries.flatten() {
        let candidate_path = entry.path().join("bin").join(executable);
        if !candidate_path.is_file() {
            continue;
        }
        let candidate = candidate_path.to_string_lossy().to_string();
        if node_binary_usable(candidate.as_str()) {
            return Some(candidate);
        }
    }
    None
}
pub fn infer_infring_home_from_exe() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let bin_dir = exe.parent()?;
    let home = bin_dir.parent()?;
    Some(home.to_path_buf())
}
pub fn resolve_preferred_node_binary() -> String {
    let mut candidates = Vec::<String>::new();

    for key in ["INFRING_NODE_BINARY", "NODE_BINARY"] {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                candidates.push(trimmed.to_string());
            }
        }
    }

    for key in ["INFRING_HOME"] {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                let runtime_root = Path::new(trimmed).join("node-runtime");
                if let Some(candidate) = resolve_node_from_runtime_root(runtime_root.as_path()) {
                    candidates.push(candidate);
                }
            }
        }
    }

    if let Some(home) = infer_infring_home_from_exe() {
        let runtime_root = home.join("node-runtime");
        if let Some(candidate) = resolve_node_from_runtime_root(runtime_root.as_path()) {
            candidates.push(candidate);
        }
    }

    if let Some(candidate) = resolve_binary_in_path(if cfg!(windows) { "node.exe" } else { "node" })
    {
        candidates.push(candidate);
    }

    for candidate in candidates {
        if node_binary_usable(candidate.as_str()) {
            return candidate;
        }
    }
    String::new()
}
pub fn resolve_infring_ops_command(root: &Path, domain: &str) -> (String, Vec<String>) {
    let explicit = env::var("INFRING_OPS_BIN").ok();
    if let Some(bin) = explicit {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), vec![domain.to_string()]);
        }
    }

    let release = root.join("target").join("release").join("infring-ops");
    if release.exists() {
        return (
            release.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    let debug = root.join("target").join("debug").join("infring-ops");
    if debug.exists() {
        return (
            debug.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }

    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            "core/layer0/ops/Cargo.toml".to_string(),
            "--bin".to_string(),
            "infring-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}
pub fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 160 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                out.push(ch);
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}
pub fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.split_whitespace().collect::<Vec<_>>().join(" ").chars() {
            if out.len() >= max_len {
                break;
            }
            out.push(ch);
        }
    }
    out.trim().to_string()
}
pub fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
pub fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
pub fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}
pub fn payload_json(argv: &[String], lane: &str) -> Result<Value, String> {
    if let Some(raw) = parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("{lane}_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("{lane}_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("{lane}_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("{lane}_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}
pub fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}
pub fn repo_path(root: &Path, rel: &str) -> PathBuf {
    let candidate = PathBuf::from(rel.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}
pub fn path_flag(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    flag: &str,
    payload_key: &str,
    default_rel: &str,
) -> PathBuf {
    parse_flag(argv, flag, false)
        .or_else(|| {
            payload
                .get(payload_key)
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(default_rel))
}
pub fn json_u64(raw: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(min, max)
}
pub fn json_bool(raw: Option<&Value>, fallback: bool) -> bool {
    raw.and_then(Value::as_bool).unwrap_or(fallback)
}
pub fn json_u64_coerce(raw: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(|value| match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    })
    .unwrap_or(fallback)
    .clamp(min, max)
}
pub fn json_f64_coerce(raw: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    raw.and_then(|value| match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    })
    .unwrap_or(fallback)
    .clamp(min, max)
}
pub fn json_bool_coerce(raw: Option<&Value>, fallback: bool) -> bool {
    raw.and_then(|value| match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    })
    .unwrap_or(fallback)
}
pub fn json_string_list(raw: Option<&Value>) -> Vec<String> {
    raw.and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|text| clean_token(Some(text), "")))
        .filter(|value| !value.is_empty())
        .collect()
}
pub fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(session_id, row)| {
                let row_task = row.get("task").and_then(Value::as_str);
                let report_task = row
                    .get("report")
                    .and_then(|value| value.get("task"))
                    .and_then(Value::as_str);
                (row_task == Some(task) || report_task == Some(task)).then(|| session_id.clone())
            })
        })
}
pub fn string_set(raw: Option<&Value>) -> Vec<String> {
    let mut out = BTreeSet::new();
    if let Some(items) = raw.and_then(Value::as_array) {
        for item in items {
            let value = clean_token(item.as_str(), "");
            if !value.is_empty() {
                out.insert(value);
            }
        }
    }
    out.into_iter().collect()
}
pub fn bridge_surface_prefix_allowed(path: &str) -> bool {
    ["adapters/", "client/runtime/", "client/lib/", "tests/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

pub fn prefix_allowed(path: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
}

pub fn normalize_prefixed_path(
    root: &Path,
    raw: &str,
    required_error: &str,
    parent_reference_error: &str,
    unsupported_error: &str,
    prefixes: &[&str],
) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err(required_error.to_string());
    }
    if candidate.contains("..") {
        return Err(parent_reference_error.to_string());
    }
    let rel = rel_path(root, &repo_path(root, candidate));
    if !prefix_allowed(&rel, prefixes) {
        return Err(unsupported_error.to_string());
    }
    Ok(rel)
}

pub fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel_path(root, &abs);
    if !bridge_surface_prefix_allowed(&rel_path) {
        return Err("unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}
pub fn normalize_bridge_path_clean(
    root: &Path,
    raw: &str,
    unsupported_error: &str,
) -> Result<String, String> {
    let clean = clean_text(Some(raw), 260);
    if !bridge_surface_prefix_allowed(&clean) {
        return Err(unsupported_error.to_string());
    }
    Ok(rel_path(root, &repo_path(root, &clean)))
}
pub fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))
}
pub fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut encoded =
        serde_json::to_string_pretty(payload).map_err(|err| format!("encode_failed:{err}"))?;
    encoded.push('\n');
    fs::write(path, encoded).map_err(|err| format!("write_failed:{}:{err}", path.display()))
}
pub fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    use std::io::Write;
    let line = serde_json::to_string(row).map_err(|err| format!("encode_failed:{err}"))? + "\n";
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    let mut file = opts
        .open(path)
        .map_err(|err| format!("open_failed:{}:{err}", path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("append_failed:{}:{err}", path.display()))
}
pub fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}
pub fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}
fn is_invisible_unicode(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x206A..=0x206F
            | 0xFEFF
            | 0xE0000..=0xE007F
    )
}
pub fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| !is_invisible_unicode(*ch))
        .collect()
}

pub fn sanitize_web_tooling_query(raw: &str) -> String {
    let cleaned = strip_invisible_unicode(raw);
    let mut out = String::with_capacity(cleaned.len());
    for ch in cleaned.chars() {
        let control = ch.is_control() && ch != '\n' && ch != '\t';
        if control {
            continue;
        }
        out.push(ch);
    }
    clean_text(Some(out.as_str()), 1200)
}

pub fn normalize_web_tooling_domain_hint(raw: &str) -> String {
    let lowered = sanitize_web_tooling_query(raw).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let without_scheme = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
        .unwrap_or(&lowered)
        .to_string();
    clean_text(Some(without_scheme.split('/').next().unwrap_or("")), 200)
        .trim_matches('.')
        .to_string()
}

pub fn canonicalize_web_tooling_query(query: &str, domain_hint: Option<&str>) -> String {
    let sanitized = sanitize_web_tooling_query(query);
    if sanitized.is_empty() {
        return sanitized;
    }
    if sanitized.to_ascii_lowercase().contains("site:") {
        return sanitized;
    }
    if let Some(domain) = domain_hint
        .map(normalize_web_tooling_domain_hint)
        .filter(|value| !value.is_empty())
    {
        return format!("site:{domain} {sanitized}");
    }
    sanitized
}
pub fn classify_curl_transport_error(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("could not resolve host")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure in name resolution")
    {
        return "dns_unreachable".to_string();
    }
    if lower.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if lower.contains("operation timed out")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
    {
        return "timeout".to_string();
    }
    if lower.contains("ssl") || lower.contains("tls") || lower.contains("certificate") {
        return "tls_error".to_string();
    }
    "collector_error".to_string()
}
pub fn http_status_to_code(status: u64) -> &'static str {
    match status {
        401 => "auth_unauthorized",
        403 => "auth_forbidden",
        404 => "http_404",
        408 => "timeout",
        429 => "rate_limited",
        500..=u64::MAX => "http_5xx",
        400..=499 => "http_4xx",
        _ => "http_error",
    }
}

pub fn web_tooling_auth_sources_from_env(
    env: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for (label, key) in [
        ("openai", "OPENAI_API_KEY"),
        ("github", "GITHUB_TOKEN"),
        ("github_app", "GITHUB_APP_INSTALLATION_TOKEN"),
        ("brave", "BRAVE_API_KEY"),
        ("tavily", "TAVILY_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("exa", "EXA_API_KEY"),
    ] {
        let present = env
            .get(key)
            .map(|raw| !sanitize_web_tooling_query(raw).is_empty())
            .unwrap_or(false);
        if present {
            out.push(label.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flag_supports_switch_true_mode() {
        let argv = vec!["--strict".to_string()];
        assert_eq!(parse_flag(&argv, "strict", true).as_deref(), Some("true"));
        assert_eq!(parse_flag(&argv, "strict", false), None);
    }

    #[test]
    fn normalize_bridge_path_rejects_parent_references() {
        let root = Path::new("/tmp/workspace");
        assert_eq!(
            normalize_bridge_path(root, "../bad").unwrap_err(),
            "unsafe_bridge_path_parent_reference"
        );
    }

    #[test]
    fn string_set_dedupes_and_sanitizes() {
        let payload = json!(["Alpha", "Alpha", "beta!", ""]);
        assert_eq!(string_set(Some(&payload)), vec!["Alpha", "beta"]);
    }

    #[test]
    fn canonicalize_web_tooling_query_applies_site_prefix() {
        let query = canonicalize_web_tooling_query(
            "top ai agent frameworks\u{200b}",
            Some("https://langchain.com/docs"),
        );
        assert!(query.starts_with("site:langchain.com "));
        assert!(!query.contains('\u{200b}'));
    }
}
