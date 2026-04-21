// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("collector-content-kernel commands:");
    println!("  protheus-ops collector-content-kernel extract-entries --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel extract-json-rows --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel map-feed-items --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel map-json-items --payload-base64=<json>");
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
}

fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn value_text(value: Option<&Value>, max_len: usize) -> String {
    let raw = match value {
        Some(Value::String(v)) => v.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    clean_text(Some(&raw), max_len)
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn html_decode(raw: &str) -> String {
    let mut out = raw.to_string();
    if let Ok(cdata_re) = Regex::new(r#"(?is)<!\[CDATA\[(.*?)\]\]>"#) {
        out = cdata_re.replace_all(&out, "$1").to_string();
    }
    out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x2F;", "/")
        .replace("&#x2f;", "/");
    out
}

fn strip_tags(raw: &str) -> String {
    let without_tags = if let Ok(tags_re) = Regex::new(r#"(?is)<[^>]*>"#) {
        tags_re.replace_all(raw, " ").to_string()
    } else {
        raw.to_string()
    };
    clean_text(Some(&html_decode(&without_tags)), 8_192)
}

fn extract_tag_value(block: &str, tag: &str) -> String {
    let escaped = regex::escape(tag);
    let pattern = format!(r"(?is)<{}\b[^>]*>(.*?)</{}\s*>", escaped, escaped);
    match Regex::new(&pattern) {
        Ok(re) => re
            .captures(block)
            .and_then(|caps| caps.get(1).map(|m| strip_tags(m.as_str())))
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn extract_tag_attr(block: &str, tag: &str, attr: &str) -> String {
    let escaped_tag = regex::escape(tag);
    let escaped_attr = regex::escape(attr);
    let pattern = format!(
        r#"(?is)<{}\b[^>]*\b{}\s*=\s*"([^"]+)"[^>]*>"#,
        escaped_tag, escaped_attr
    );
    match Regex::new(&pattern) {
        Ok(re) => re
            .captures(block)
            .and_then(|caps| caps.get(1).map(|m| html_decode(m.as_str())))
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn extract_entries(xml: &str) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    if let Ok(item_re) = Regex::new(r#"(?is)<item\b.*?</item>"#) {
        for mat in item_re.find_iter(xml) {
            let block = mat.as_str();
            let title = extract_tag_value(block, "title");
            let link = {
                let direct = extract_tag_value(block, "link");
                if direct.is_empty() {
                    extract_tag_value(block, "guid")
                } else {
                    direct
                }
            };
            let description = {
                let desc = extract_tag_value(block, "description");
                if desc.is_empty() {
                    extract_tag_value(block, "content:encoded")
                } else {
                    desc
                }
            };
            let published = {
                let pd = extract_tag_value(block, "pubDate");
                if pd.is_empty() {
                    extract_tag_value(block, "dc:date")
                } else {
                    pd
                }
            };
            if title.is_empty() && link.is_empty() {
                continue;
            }
            items.push(json!({
                "title": clean_text(Some(&title), 220),
                "link": clean_text(Some(&link), 500),
                "description": clean_text(Some(&description), 420),
                "published": clean_text(Some(&published), 120),
            }));
        }
    }

    if let Ok(entry_re) = Regex::new(r#"(?is)<entry\b.*?</entry>"#) {
        for mat in entry_re.find_iter(xml) {
            let block = mat.as_str();
            let title = extract_tag_value(block, "title");
            let link = {
                let href = extract_tag_attr(block, "link", "href");
                if href.is_empty() {
                    extract_tag_value(block, "id")
                } else {
                    href
                }
            };
            let description = {
                let summary = extract_tag_value(block, "summary");
                if summary.is_empty() {
                    extract_tag_value(block, "content")
                } else {
                    summary
                }
            };
            let published = {
                let updated = extract_tag_value(block, "updated");
                if updated.is_empty() {
                    extract_tag_value(block, "published")
                } else {
                    updated
                }
            };
            if title.is_empty() && link.is_empty() {
                continue;
            }
            items.push(json!({
                "title": clean_text(Some(&title), 220),
                "link": clean_text(Some(&link), 500),
                "description": clean_text(Some(&description), 420),
                "published": clean_text(Some(&published), 120),
            }));
        }
    }

    items
}

fn topics_from_payload(payload: &Map<String, Value>) -> Vec<Value> {
    payload
        .get("topics")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| Value::String(clean_text(Some(v), 80)))
                .filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false))
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn seen_ids_from_payload(payload: &Map<String, Value>) -> HashSet<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean_text(Some(v), 120))
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn finalize_seen_ids(mut seen: HashSet<String>) -> Vec<String> {
    let mut seen_ids = seen.drain().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop_count = seen_ids.len() - 2000;
        seen_ids.drain(0..drop_count);
    }
    seen_ids
}

fn value_topics(raw: Option<&Value>, fallback: &[Value]) -> Vec<Value> {
    raw.and_then(Value::as_array)
        .map(|topics| {
            topics
                .iter()
                .filter_map(Value::as_str)
                .map(|topic| Value::String(clean_text(Some(topic), 80)))
                .filter(|topic| topic.as_str().map(|v| !v.is_empty()).unwrap_or(false))
                .take(8)
                .collect::<Vec<_>>()
        })
        .filter(|topics| !topics.is_empty())
        .unwrap_or_else(|| fallback.to_vec())
}
