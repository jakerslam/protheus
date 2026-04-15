// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("local-state-digest-kernel commands:");
    println!("  protheus-ops local-state-digest-kernel preflight --payload-base64=<json>");
    println!("  protheus-ops local-state-digest-kernel collect --payload-base64=<json>");
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn sha16(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn today_str() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn read_json_safe(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn read_jsonl_safe(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn nested_obj<'a>(payload: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    payload.get(key).and_then(Value::as_object)
}

fn nested_u64(payload: &Map<String, Value>, key: &str) -> Option<u64> {
    nested_obj(payload, "budgets")
        .and_then(|b| b.get(key))
        .and_then(Value::as_u64)
}

fn resolve_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join("local").join("state")
}

fn resolve_date(payload: &Map<String, Value>) -> String {
    let override_date = payload
        .get("date")
        .and_then(Value::as_str)
        .map(|raw| clean_text(Some(raw), 32))
        .unwrap_or_default();
    if override_date.is_empty() {
        today_str()
    } else {
        override_date
    }
}

fn base_topics(payload: &Map<String, Value>) -> Vec<Value> {
    let defaults = ["automation", "system", "growth"];
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<Value>::new();

    if let Some(eye) = nested_obj(payload, "eye_config") {
        if let Some(rows) = eye.get("topics").and_then(Value::as_array) {
            for topic in rows {
                if let Some(raw) = topic.as_str() {
                    let topic_clean = clean_text(Some(&raw.to_lowercase()), 80);
                    if topic_clean.is_empty() || !seen.insert(topic_clean.clone()) {
                        continue;
                    }
                    out.push(Value::String(topic_clean));
                    if out.len() >= 5 {
                        return out;
                    }
                }
            }
        }
    }

    for d in defaults {
        let topic_clean = d.to_string();
        if seen.insert(topic_clean.clone()) {
            out.push(Value::String(topic_clean));
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

fn normalize_proposals_payload(raw: Option<Value>) -> Vec<Value> {
    match raw {
        Some(Value::Array(rows)) => rows,
        Some(Value::Object(obj)) => obj
            .get("proposals")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn proposal_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("sensory")
        .join("proposals")
        .join(format!("{date_str}.json"));
    let rows = normalize_proposals_payload(read_json_safe(&fp));
    let mut open = 0usize;
    let mut resolved = 0usize;
    for row in rows.iter() {
        let status = row
            .as_object()
            .and_then(|o| o.get("status"))
            .and_then(Value::as_str)
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "open".to_string());
        if status == "resolved" || status == "rejected" {
            resolved += 1;
        } else {
            open += 1;
        }
    }
    json!({
        "total": rows.len(),
        "open": open,
        "resolved": resolved,
        "path": fp.display().to_string()
    })
}

fn decision_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("queue")
        .join("decisions")
        .join(format!("{date_str}.jsonl"));
    let rows = read_jsonl_safe(&fp);
    let mut accepted = 0usize;
    let mut shipped = 0usize;
    let mut no_change = 0usize;
    let mut reverted = 0usize;

    for row in rows {
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let row_type = obj.get("type").and_then(Value::as_str).unwrap_or("");
        let decision = obj.get("decision").and_then(Value::as_str).unwrap_or("");
        let outcome = obj.get("outcome").and_then(Value::as_str).unwrap_or("");
        if row_type == "decision" && decision == "accept" {
            accepted += 1;
        }
        if row_type == "outcome" && outcome == "shipped" {
            shipped += 1;
        }
        if row_type == "outcome" && outcome == "no_change" {
            no_change += 1;
        }
        if row_type == "outcome" && outcome == "reverted" {
            reverted += 1;
        }
    }

    json!({
        "accepted": accepted,
        "shipped": shipped,
        "no_change": no_change,
        "reverted": reverted,
        "path": fp.display().to_string()
    })
}

fn git_outcome_stats(state_dir: &Path, date_str: &str) -> Value {
    let fp = state_dir
        .join("git")
        .join("outcomes")
        .join(format!("{date_str}.jsonl"));
    let rows = read_jsonl_safe(&fp);
    let latest = rows
        .iter()
        .rev()
        .find(|row| {
            row.as_object()
                .and_then(|o| o.get("type"))
                .and_then(Value::as_str)
                == Some("git_outcomes_ok")
        })
        .cloned();
    let obj = latest.as_ref().and_then(Value::as_object);
    json!({
        "tags_found": obj.and_then(|o| o.get("tags_found")).and_then(Value::as_u64).unwrap_or(0),
        "outcomes_recorded": obj.and_then(|o| o.get("outcomes_recorded")).and_then(Value::as_u64).unwrap_or(0),
        "outcomes_skipped": obj.and_then(|o| o.get("outcomes_skipped")).and_then(Value::as_u64).unwrap_or(0),
        "path": fp.display().to_string()
    })
}

fn outage_stats(state_dir: &Path) -> Value {
    let fp = state_dir.join("sensory").join("eyes").join("registry.json");
    let reg = read_json_safe(&fp).unwrap_or_else(|| json!({}));
    let outage = reg
        .as_object()
        .and_then(|o| o.get("outage_mode"))
        .and_then(Value::as_object);
    json!({
        "active": outage.and_then(|o| o.get("active")).and_then(Value::as_bool).unwrap_or(false),
        "failed_transport_eyes": outage.and_then(|o| o.get("last_failed_transport_eyes")).and_then(Value::as_u64).unwrap_or(0),
        "window_hours": outage.and_then(|o| o.get("last_window_hours")).and_then(Value::as_u64).unwrap_or(0),
        "since": outage.and_then(|o| o.get("since")).cloned().unwrap_or(Value::Null),
        "path": fp.display().to_string()
    })
}

fn preflight(payload: &Map<String, Value>, state_dir: &Path) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();

    let max_items = nested_u64(payload, "max_items");
    if max_items.unwrap_or(0) == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_items must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_items_valid",
            "ok": true,
            "value": max_items.unwrap_or(0)
        }));
    }

    if !state_dir.exists() {
        failures.push(json!({
            "code": "state_missing",
            "message": format!("state directory missing: {}", state_dir.display())
        }));
    } else {
        checks.push(json!({
            "name": "state_dir_present",
            "ok": true
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "local_state_digest",
        "checks": checks,
        "failures": failures
    })
}

