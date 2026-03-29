// SPDX-License-Identifier: Apache-2.0
use crate::{now_iso, parse_args};
use chrono::{Datelike, Timelike, Utc};
use rand::RngCore;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const SCRATCHPAD_SCHEMA_VERSION: &str = "scratchpad/v1";
const TASKGROUP_SCHEMA_VERSION: &str = "taskgroup/v1";
const ITEM_INTERVAL: i64 = 10;
const TIME_INTERVAL_MS: i64 = 120_000;
const MAX_AUTO_RETRIES: i64 = 1;

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops orchestration invoke --op=<operation> [--payload-json=<json>]");
    println!("  protheus-ops orchestration help");
}

fn to_clean_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(v) => v.to_string().trim().to_string(),
        None => String::new(),
    }
}

fn get_string_any(payload: &Value, keys: &[&str]) -> String {
    keys.iter()
        .map(|key| to_clean_string(payload.get(*key)))
        .find(|v| !v.is_empty())
        .unwrap_or_default()
}

fn payload_root_dir(payload: &Value) -> Option<String> {
    let root_dir = get_string_any(payload, &["root_dir", "rootDir"]);
    if root_dir.is_empty() {
        None
    } else {
        Some(root_dir)
    }
}

fn get_i64_any(payload: &Value, keys: &[&str], default: i64) -> i64 {
    for key in keys {
        if let Some(value) = payload.get(*key) {
            let out = match value {
                Value::Number(num) => num.as_i64().or_else(|| num.as_u64().map(|v| v as i64)),
                Value::String(text) => text.trim().parse::<i64>().ok(),
                _ => None,
            };
            if let Some(parsed) = out {
                return parsed;
            }
        }
    }
    default
}

fn get_object(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[derive(Clone, Copy)]
enum FirstByteRule {
    AlphaNum,
    LowerOrDigit,
}

fn validate_identifier(
    value: &str,
    min_len: usize,
    max_len: usize,
    first_rule: FirstByteRule,
) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < min_len || bytes.len() > max_len {
        return false;
    }
    let first = bytes[0] as char;
    let first_ok = match first_rule {
        FirstByteRule::AlphaNum => first.is_ascii_alphanumeric(),
        FirstByteRule::LowerOrDigit => first.is_ascii_lowercase() || first.is_ascii_digit(),
    };
    if !first_ok {
        return false;
    }
    bytes.iter().all(|b| {
        let ch = *b as char;
        match first_rule {
            FirstByteRule::AlphaNum => {
                ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-')
            }
            FirstByteRule::LowerOrDigit => {
                ch.is_ascii_lowercase()
                    || ch.is_ascii_digit()
                    || matches!(ch, '.' | '_' | ':' | '-')
            }
        }
    })
}

fn is_valid_task_id(task_id: &str) -> bool {
    validate_identifier(task_id, 3, 128, FirstByteRule::AlphaNum)
}

fn validate_group_id(task_group_id: &str) -> bool {
    validate_identifier(task_group_id, 6, 128, FirstByteRule::LowerOrDigit)
}

fn validate_agent_id(agent_id: &str) -> bool {
    validate_identifier(agent_id, 2, 128, FirstByteRule::AlphaNum)
}

fn default_scratchpad_dir(root: &Path) -> PathBuf {
    root.join("local").join("workspace").join("scratchpad")
}

fn default_taskgroup_dir(root: &Path) -> PathBuf {
    default_scratchpad_dir(root).join("taskgroups")
}

fn scratchpad_path(root: &Path, task_id: &str, root_dir: Option<&str>) -> Result<PathBuf, String> {
    if !is_valid_task_id(task_id) {
        return Err(format!(
            "invalid_task_id:{}",
            if task_id.is_empty() {
                "<empty>"
            } else {
                task_id
            }
        ));
    }
    let base = root_dir
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_scratchpad_dir(root));
    Ok(base.join(format!("{task_id}.json")))
}

fn taskgroup_path(
    root: &Path,
    task_group_id: &str,
    root_dir: Option<&str>,
) -> Result<PathBuf, String> {
    let normalized = task_group_id.trim().to_ascii_lowercase();
    if !validate_group_id(&normalized) {
        return Err(format!(
            "invalid_task_group_id:{}",
            if task_group_id.trim().is_empty() {
                "<empty>"
            } else {
                task_group_id
            }
        ));
    }
    let base = root_dir
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_taskgroup_dir(root));
    Ok(base.join(format!("{normalized}.json")))
}

fn empty_scratchpad(task_id: &str) -> Value {
    let now = now_iso();
    json!({
        "schema_version": SCRATCHPAD_SCHEMA_VERSION,
        "task_id": task_id,
        "created_at": now,
        "updated_at": now,
        "progress": {
            "processed": 0,
            "total": 0
        },
        "findings": [],
        "checkpoints": []
    })
}

#[derive(Debug, Clone)]
struct LoadedScratchpad {
    scratchpad: Value,
    file_path: PathBuf,
    exists: bool,
}

fn load_scratchpad(
    root: &Path,
    task_id: &str,
    root_dir: Option<&str>,
) -> Result<LoadedScratchpad, String> {
    let file_path = scratchpad_path(root, task_id, root_dir)?;
    match fs::read_to_string(&file_path) {
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(parsed @ Value::Object(_)) => Ok(LoadedScratchpad {
                scratchpad: parsed,
                file_path,
                exists: true,
            }),
            _ => Ok(LoadedScratchpad {
                scratchpad: empty_scratchpad(task_id),
                file_path,
                exists: false,
            }),
        },
        Err(_) => Ok(LoadedScratchpad {
            scratchpad: empty_scratchpad(task_id),
            file_path,
            exists: false,
        }),
    }
}

fn merge_objects(base: &Value, patch: &Value) -> Value {
    let mut out = get_object(base);
    if let Value::Object(map) = patch {
        for (k, v) in map {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

fn normalize_progress(progress: Option<&Value>) -> Value {
    let processed = progress
        .and_then(|p| p.get("processed"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let total = progress
        .and_then(|p| p.get("total"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    json!({
        "processed": if processed.is_finite() { processed } else { 0.0 },
        "total": if total.is_finite() { total } else { 0.0 }
    })
}

fn write_scratchpad(
    root: &Path,
    task_id: &str,
    patch: &Value,
    root_dir: Option<&str>,
) -> Result<Value, String> {
    let loaded = load_scratchpad(root, task_id, root_dir)?;
    let mut next = merge_objects(&loaded.scratchpad, patch);
    let now = now_iso();
    if let Value::Object(map) = &mut next {
        map.insert(
            "schema_version".to_string(),
            Value::String(SCRATCHPAD_SCHEMA_VERSION.to_string()),
        );
        map.insert("task_id".to_string(), Value::String(task_id.to_string()));
        map.insert("updated_at".to_string(), Value::String(now.clone()));
        if map
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            map.insert("created_at".to_string(), Value::String(now.clone()));
        }

        let progress = normalize_progress(map.get("progress"));
        map.insert("progress".to_string(), progress);

        if !map.get("findings").map(Value::is_array).unwrap_or(false) {
            map.insert("findings".to_string(), Value::Array(Vec::new()));
        }
        if !map.get("checkpoints").map(Value::is_array).unwrap_or(false) {
            map.insert("checkpoints".to_string(), Value::Array(Vec::new()));
        }
    }

    if let Some(parent) = loaded.file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("scratchpad_create_parent_failed:{}:{err}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(&next)
        .map_err(|err| format!("scratchpad_encode_failed:{err}"))?
        + "\n";
    fs::write(&loaded.file_path, payload).map_err(|err| {
        format!(
            "scratchpad_write_failed:{}:{err}",
            loaded.file_path.display()
        )
    })?;

    Ok(json!({
        "ok": true,
        "type": "orchestration_scratchpad_write",
        "task_id": task_id,
        "file_path": loaded.file_path,
        "scratchpad": next
    }))
}

fn is_datetime(value: &str) -> bool {
    if value.trim().is_empty() {
        return false;
    }
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn severity_order(severity: &str) -> i64 {
    match severity {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn status_order(status: &str) -> i64 {
    match status {
        "confirmed" => 5,
        "open" => 4,
        "needs-review" => 3,
        "resolved" => 2,
        "dismissed" => 1,
        _ => 0,
    }
}

fn normalize_finding(input: &Value) -> Value {
    let mut out = get_object(input);

    out.insert(
        "audit_id".to_string(),
        Value::String(to_clean_string(input.get("audit_id"))),
    );
    out.insert(
        "item_id".to_string(),
        Value::String(to_clean_string(input.get("item_id"))),
    );
    out.insert(
        "severity".to_string(),
        Value::String(to_clean_string(input.get("severity")).to_ascii_lowercase()),
    );
    out.insert(
        "status".to_string(),
        Value::String(to_clean_string(input.get("status")).to_ascii_lowercase()),
    );
    out.insert(
        "location".to_string(),
        Value::String(to_clean_string(input.get("location"))),
    );

    let timestamp = to_clean_string(input.get("timestamp"));
    out.insert(
        "timestamp".to_string(),
        Value::String(if timestamp.is_empty() {
            now_iso()
        } else {
            timestamp
        }),
    );

    let mut evidence = Vec::new();
    if let Some(rows) = input.get("evidence").and_then(Value::as_array) {
        for row in rows {
            let mut ev = Map::new();
            ev.insert(
                "type".to_string(),
                Value::String(to_clean_string(row.get("type"))),
            );
            ev.insert(
                "value".to_string(),
                Value::String(to_clean_string(row.get("value"))),
            );
            let source = to_clean_string(row.get("source"));
            if !source.is_empty() {
                ev.insert("source".to_string(), Value::String(source));
            }
            evidence.push(Value::Object(ev));
        }
    }
    out.insert("evidence".to_string(), Value::Array(evidence));

    Value::Object(out)
}

