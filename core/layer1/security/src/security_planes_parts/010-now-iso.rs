// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

use crate::{parse_args, ParsedArgs};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

type HmacSha256 = Hmac<Sha256>;

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn clean(v: impl ToString, max_len: usize) -> String {
    v.to_string()
        .chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{2060}'
                    | '\u{FEFF}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
            ) && (!ch.is_control() || ch.is_ascii_whitespace())
        })
        .collect::<String>()
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

fn local_state_root(repo_root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("PROTHEUS_SECURITY_STATE_ROOT") {
        let t = v.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    repo_root.join("client").join("local").join("state")
}

fn runtime_config_path(repo_root: &Path, file_name: &str) -> PathBuf {
    runtime_root(repo_root).join("config").join(file_name)
}

fn resolve_runtime_or_state(repo_root: &Path, raw: &str) -> PathBuf {
    let trimmed = clean(raw, 600);
    if trimmed.is_empty() {
        return runtime_root(repo_root);
    }
    let candidate = PathBuf::from(&trimmed);
    if candidate.is_absolute() {
        return candidate;
    }
    let rel = normalize_rel(trimmed);
    if rel.starts_with("local/state/") {
        let stripped = rel.trim_start_matches("local/state/").to_string();
        return local_state_root(repo_root).join(stripped);
    }
    runtime_root(repo_root).join(rel)
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

fn split_csv(raw: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for token in raw.split(',') {
        if out.len() >= max {
            break;
        }
        let value = normalize_rel(clean(token, 240));
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
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

fn hmac_sha256_hex(secret: &str, payload: &Value) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| format!("hmac_key_invalid:{err}"))?;
    mac.update(stable_json_string(payload).as_bytes());
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
