// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Datelike, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

pub(crate) fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
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
    let mut out = Vec::<Value>::new();

    let push_topic = |out: &mut Vec<Value>, raw: &str| {
        let value = clean_text(Some(raw), 48).to_lowercase();
        if value.is_empty() {
            return;
        }
        if out
            .iter()
            .any(|existing| existing.as_str() == Some(value.as_str()))
        {
            return;
        }
        out.push(Value::String(value));
    };

    for topic in defaults {
        push_topic(&mut out, topic);
    }

    if let Some(rows) = payload.get("topics").and_then(Value::as_array) {
        for row in rows {
            if let Some(raw) = row.as_str() {
                push_topic(&mut out, raw);
            }
            if out.len() >= 8 {
                break;
            }
        }
    }

    out.truncate(8);
    out
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
    let mut out = Vec::<Value>::new();
    if let Some(rows) = raw.and_then(Value::as_array) {
        for row in rows {
            let value = clean_text(row.as_str(), 120);
            if value.is_empty() {
                continue;
            }
            if out.iter().any(|existing| existing.as_str() == Some(value.as_str())) {
                continue;
            }
            out.push(Value::String(value));
            if out.len() >= 12 {
                break;
            }
        }
    }
    out
}

pub(crate) fn sha16(seed: &str) -> String {
    let digest = Sha256::digest(seed.as_bytes());
    hex::encode(digest)[..16].to_string()
}
