// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Datelike, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

pub(crate) fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

pub(crate) fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub(crate) fn clamp_u64(
    payload: &Map<String, Value>,
    key: &str,
    fallback: u64,
    lo: u64,
    hi: u64,
) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

pub(crate) fn resolve_path(
    root: &Path,
    payload: &Map<String, Value>,
    key: &str,
    default_rel: &str,
) -> PathBuf {
    if let Some(raw) = payload.get(key).and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join(default_rel)
}

pub(crate) fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

pub(crate) fn read_jsonl_tail(path: &Path, max_lines: usize) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let lines = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines.max(1));
    let mut rows = Vec::new();
    for line in &lines[start..] {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            rows.push(v);
        }
    }
    rows
}

pub(crate) fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("conversation_eye_collector_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("conversation_eye_collector_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body)
        .map_err(|err| format!("conversation_eye_collector_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path)
        .map_err(|err| format!("conversation_eye_collector_kernel_rename_failed:{err}"))
}

pub(crate) fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("conversation_eye_collector_kernel_create_dir_failed:{err}"))?;
    }
    let mut line = serde_json::to_string(row)
        .map_err(|err| format!("conversation_eye_collector_kernel_encode_failed:{err}"))?;
    line.push('\n');
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("conversation_eye_collector_kernel_append_open_failed:{err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("conversation_eye_collector_kernel_append_write_failed:{err}"))
}

fn clean_map_strings(raw: Option<&Value>, key_max: usize, val_max: usize) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(obj) = raw.and_then(Value::as_object) {
        for (k, v) in obj {
            let kk = clean_text(Some(k), key_max);
            if kk.is_empty() {
                continue;
            }
            let vv = clean_text(v.as_str(), val_max);
            if vv.is_empty() {
                continue;
            }
            out.insert(kk, Value::String(vv));
        }
    }
    out
}

fn clean_map_counts(raw: Option<&Value>, key_max: usize) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(obj) = raw.and_then(Value::as_object) {
        for (k, v) in obj {
            let kk = clean_text(Some(k), key_max);
            if kk.is_empty() {
                continue;
            }
            let count = v
                .as_u64()
                .or_else(|| v.as_i64().map(|n| n.max(0) as u64))
                .unwrap_or(0);
            out.insert(kk, Value::Number(count.into()));
        }
    }
    out
}

pub(crate) fn normalize_index(raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let emitted = clean_map_strings(obj.and_then(|o| o.get("emitted_node_ids")), 120, 80);
    let weekly_counts = clean_map_counts(obj.and_then(|o| o.get("weekly_counts")), 20);
    let weekly_promotions = clean_map_counts(obj.and_then(|o| o.get("weekly_promotions")), 20);

    json!({
        "version": "1.0",
        "updated_ts": clean_text(obj.and_then(|o| o.get("updated_ts")).and_then(Value::as_str), 80),
        "emitted_node_ids": emitted,
        "weekly_counts": weekly_counts,
        "weekly_promotions": weekly_promotions,
    })
}

pub(crate) fn to_iso_week(ts: &str) -> String {
    let parsed = DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let iso = parsed.iso_week();
    format!("{}-W{:02}", iso.year(), iso.week())
}

pub(crate) fn normalize_topics(payload: &Map<String, Value>) -> Vec<Value> {
    let defaults = ["conversation", "decision", "insight", "directive", "t1"];
    let mut dedup = BTreeMap::<String, ()>::new();

    for topic in defaults {
        dedup.insert(topic.to_string(), ());
    }

    if let Some(rows) = payload.get("topics").and_then(Value::as_array) {
        for row in rows {
            let value = clean_text(row.as_str(), 48).to_lowercase();
            if value.is_empty() {
                continue;
            }
            dedup.insert(value, ());
        }
    }

    dedup
        .keys()
        .take(8)
        .cloned()
        .map(Value::String)
        .collect::<Vec<_>>()
}

pub(crate) fn clean_tags(raw: Option<&Value>) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    if let Some(rows) = raw.and_then(Value::as_array) {
        for row in rows {
            let value = clean_text(row.as_str(), 64);
            if value.is_empty() {
                continue;
            }
            if !out
                .iter()
                .any(|existing| existing.as_str() == Some(value.as_str()))
            {
                out.push(Value::String(value));
            }
            if out.len() >= 12 {
                break;
            }
        }
    }
    if out.is_empty() {
        vec![
            Value::String("conversation".to_string()),
            Value::String("decision".to_string()),
            Value::String("insight".to_string()),
            Value::String("directive".to_string()),
            Value::String("t1".to_string()),
        ]
    } else {
        out
    }
}

pub(crate) fn clean_edges(raw: Option<&Value>) -> Vec<Value> {
    raw.and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 120))
                .filter(|row| !row.is_empty())
                .take(12)
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn sha16(seed: &str) -> String {
    let digest = Sha256::digest(seed.as_bytes());
    hex::encode(digest)[..16].to_string()
}
