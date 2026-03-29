// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

type HmacSha256 = Hmac<Sha256>;

#[path = "../security_planes.rs"]
mod security_planes;
#[path = "../security_wave1.rs"]
mod security_wave1;

pub use security_planes::{
    run_anti_sabotage_shield, run_constitution_guardian, run_guard, run_remote_emergency_halt,
    run_soul_token_guard,
};
pub use security_wave1::{
    run_abac_policy_plane, run_black_box_ledger, run_capability_switchboard,
    run_directive_hierarchy_controller, run_dream_warden_guard, run_goal_preservation_kernel,
    run_truth_seeking_gate,
};

#[derive(Debug, Clone, Default)]
pub struct ParsedArgs {
    pub positional: Vec<String>,
    pub flags: HashMap<String, String>,
}

pub fn parse_args(raw: &[String]) -> ParsedArgs {
    let mut out = ParsedArgs::default();
    for token in raw {
        if !token.starts_with("--") {
            out.positional.push(token.clone());
            continue;
        }
        match token.split_once('=') {
            Some((k, v)) => {
                out.flags
                    .insert(k.trim_start_matches("--").to_string(), v.to_string());
            }
            None => {
                out.flags.insert(
                    token.trim_start_matches("--").to_string(),
                    "true".to_string(),
                );
            }
        }
    }
    out
}

fn flag<'a>(parsed: &'a ParsedArgs, key: &str) -> Option<&'a str> {
    parsed.flags.get(key).map(String::as_str)
}

fn bool_flag(parsed: &ParsedArgs, key: &str, fallback: bool) -> bool {
    match flag(parsed, key) {
        Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        None => fallback,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn clean(v: impl ToString, max_len: usize) -> String {
    v.to_string()
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_rel(raw: impl AsRef<str>) -> String {
    raw.as_ref()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn runtime_root(repo_root: &Path) -> PathBuf {
    repo_root.join("client").join("runtime")
}

fn state_root(repo_root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("PROTHEUS_SECURITY_STATE_ROOT") {
        let t = v.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    repo_root.join("client").join("local").join("state")
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    fs::write(&tmp, payload).map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let line = serde_json::to_string(value)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{line}").map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))
}

fn sha256_hex_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

fn sha256_hex_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read_file_failed:{}:{err}", path.display()))?;
    Ok(sha256_hex_bytes(&bytes))
}

fn stable_json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(rows) => format!(
            "[{}]",
            rows.iter()
                .map(stable_json_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                out.push_str(&stable_json_string(map.get(*key).unwrap_or(&Value::Null)));
            }
            out.push('}');
            out
        }
    }
}

fn hmac_sha256_hex(secret: &str, payload: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| format!("hmac_key_invalid:{err}"))?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn secure_eq_hex(a: &str, b: &str) -> bool {
    let a_norm = a.trim().to_ascii_lowercase();
    let b_norm = b.trim().to_ascii_lowercase();
    if a_norm.len() != b_norm.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_norm.as_bytes().iter().zip(b_norm.as_bytes().iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct IntegrityPolicy {
    version: String,
    target_roots: Vec<String>,
    target_extensions: Vec<String>,
    protected_files: Vec<String>,
    exclude_paths: Vec<String>,
    hashes: BTreeMap<String, String>,
}

impl Default for IntegrityPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            target_roots: vec![
                "systems/security".to_string(),
                "config/directives".to_string(),
            ],
            target_extensions: vec![".js".to_string(), ".yaml".to_string(), ".yml".to_string()],
            protected_files: vec!["lib/directive_resolver.js".to_string()],
            exclude_paths: Vec::new(),
            hashes: BTreeMap::new(),
        }
    }
}

fn normalize_integrity_policy(raw: IntegrityPolicy) -> IntegrityPolicy {
    let normalize_rows = |rows: Vec<String>| -> Vec<String> {
        let mut dedupe = BTreeSet::<String>::new();
        for row in rows {
            let clean_row = normalize_rel(row);
            if !clean_row.is_empty() {
                dedupe.insert(clean_row);
            }
        }
        dedupe.into_iter().collect()
    };

    let mut normalized_hashes = BTreeMap::new();
    for (path, hash) in raw.hashes {
        let rel = normalize_rel(path);
        let digest = clean(hash, 200).to_ascii_lowercase();
        if rel.is_empty() || rel.starts_with("..") || digest.is_empty() {
            continue;
        }
        normalized_hashes.insert(rel, digest);
    }

    IntegrityPolicy {
        version: clean(raw.version, 40),
        target_roots: normalize_rows(raw.target_roots),
        target_extensions: normalize_rows(raw.target_extensions)
            .into_iter()
            .map(|v| v.to_ascii_lowercase())
            .collect(),
        protected_files: normalize_rows(raw.protected_files),
        exclude_paths: normalize_rows(raw.exclude_paths),
        hashes: normalized_hashes,
    }
}

fn load_integrity_policy(policy_path: &Path) -> IntegrityPolicy {
    let fallback = IntegrityPolicy::default();
    if !policy_path.exists() {
        return fallback;
    }
    let raw = match fs::read_to_string(policy_path) {
        Ok(v) => v,
        Err(_) => return fallback,
    };
    match serde_json::from_str::<IntegrityPolicy>(&raw) {
        Ok(parsed) => normalize_integrity_policy(parsed),
        Err(_) => fallback,
    }
}

fn integrity_path_match(rel: &str, rule: &str) -> bool {
    let clean_rule = normalize_rel(rule);
    if clean_rule.is_empty() {
        return false;
    }
    if let Some(prefix) = clean_rule.strip_suffix("/**") {
        return rel.starts_with(prefix);
    }
    rel == clean_rule
}

fn integrity_is_excluded(rel: &str, policy: &IntegrityPolicy) -> bool {
    policy
        .exclude_paths
        .iter()
        .any(|rule| integrity_path_match(rel, rule))
}

fn integrity_has_allowed_extension(rel: &str, policy: &IntegrityPolicy) -> bool {
    if policy.target_extensions.is_empty() {
        return true;
    }
    let ext = Path::new(rel)
        .extension()
        .map(|v| format!(".{}", v.to_string_lossy().to_ascii_lowercase()));
    match ext {
        Some(v) => policy.target_extensions.iter().any(|want| want == &v),
        None => false,
    }
}

fn collect_integrity_present_files(runtime_root: &Path, policy: &IntegrityPolicy) -> Vec<String> {
    let mut files = BTreeSet::<String>::new();
    for root_rel in &policy.target_roots {
        let abs = runtime_root.join(root_rel);
        if !abs.exists() {
            continue;
        }
        for entry in WalkDir::new(abs).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = normalize_rel(
                entry
                    .path()
                    .strip_prefix(runtime_root)
                    .unwrap_or(entry.path())
                    .to_string_lossy(),
            );
            if rel.is_empty() || rel.starts_with("..") {
                continue;
            }
            if integrity_is_excluded(&rel, policy) {
                continue;
            }
            if !integrity_has_allowed_extension(&rel, policy) {
                continue;
            }
            files.insert(rel);
        }
    }

    for rel in &policy.protected_files {
        let abs = runtime_root.join(rel);
        if abs.is_file() && !integrity_is_excluded(rel, policy) {
            files.insert(normalize_rel(rel));
        }
    }

    files.into_iter().collect()
}

fn summarize_violation_counts(violations: &[Value]) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::<String, u64>::new();
    for row in violations {
        let k = row
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *out.entry(k).or_insert(0) += 1;
    }
    out
}

