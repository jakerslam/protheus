// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("moltstack-discover-collector-kernel commands:");
    println!("  protheus-ops moltstack-discover-collector-kernel run --payload-base64=<json>");
    println!(
        "  protheus-ops moltstack-discover-collector-kernel preflight --payload-base64=<json>"
    );
    println!("  protheus-ops moltstack-discover-collector-kernel build-fetch-plan --payload-base64=<json>");
    println!("  protheus-ops moltstack-discover-collector-kernel classify-fetch-error --payload-base64=<json>");
    println!(
        "  protheus-ops moltstack-discover-collector-kernel finalize-run --payload-base64=<json>"
    );
    println!(
        "  protheus-ops moltstack-discover-collector-kernel map-posts --payload-base64=<json>"
    );
    println!("  protheus-ops moltstack-discover-collector-kernel collect --payload-base64=<json>");
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
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

fn object_from_value(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn keyword_topics(title: &str, configured_topics: &[String]) -> Vec<String> {
    let t = title.to_lowercase();
    let mut out = BTreeSet::<String>::new();
    for topic in configured_topics {
        let cleaned = clean_text(Some(topic), 60).to_lowercase();
        if !cleaned.is_empty() {
            out.insert(cleaned);
        }
    }

    if t.contains("ai") || t.contains("agent") || t.contains("llm") {
        out.insert("ai_agents".to_string());
    }
    if t.contains("automation") || t.contains("workflow") {
        out.insert("automation".to_string());
    }
    if t.contains("startup") || t.contains("business") {
        out.insert("startups".to_string());
    }
    if t.contains("revenue") || t.contains("money") || t.contains("income") {
        out.insert("revenue".to_string());
    }
    if t.contains("privacy") || t.contains("security") {
        out.insert("security".to_string());
    }
    if t.contains("ethic") || t.contains("moral") {
        out.insert("ethics".to_string());
    }
    if t.contains("multi-agent") || t.contains("system") {
        out.insert("multi_agent".to_string());
    }
    if t.contains("consciousness") || t.contains("mind") {
        out.insert("consciousness".to_string());
    }
    if t.contains("surveillance") || t.contains("privacy") {
        out.insert("surveillance".to_string());
    }

    out.into_iter().take(5).collect::<Vec<_>>()
}

fn configured_topics(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("topics")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 60))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_allowlist(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("allowed_domains")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 200).to_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn split_scheme_host(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim();
    let idx = trimmed.find("://")?;
    let scheme = trimmed[..idx].to_ascii_lowercase();
    let rest = &trimmed[(idx + 3)..];
    if rest.is_empty() {
        return None;
    }
    let host_port = rest.split('/').next().unwrap_or("");
    let host = host_port.split('@').next_back().unwrap_or("");
    let host_only = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    if host_only.is_empty() {
        return None;
    }
    Some((scheme, host_only))
}

fn host_from_url(raw: &str) -> Option<String> {
    split_scheme_host(raw).map(|(_, host)| host)
}

fn resolved_api_url(payload: &Map<String, Value>) -> String {
    let url = clean_text(
        payload
            .get("api_url")
            .and_then(Value::as_str)
            .or_else(|| payload.get("url").and_then(Value::as_str)),
        600,
    );
    if url.is_empty() {
        "https://moltstack.net/api/posts".to_string()
    } else {
        url
    }
}

fn first_preflight_error(preflight_result: &Value) -> String {
    let first = preflight_result
        .get("failures")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let code = clean_text(first.get("code").and_then(Value::as_str), 64);
    let message = clean_text(first.get("message").and_then(Value::as_str), 180);
    if code.is_empty() {
        "moltstack_discover_preflight_failed".to_string()
    } else if message.is_empty() {
        format!("moltstack_discover_preflight_failed:{code}")
    } else {
        format!("moltstack_discover_preflight_failed:{code}:{message}")
    }
}

fn classify_curl_transport_error(stderr: &str) -> String {
    lane_utils::classify_curl_transport_error(stderr)
}

fn http_status_to_code(status: u64) -> &'static str {
    lane_utils::http_status_to_code(status)
}

fn curl_fetch_with_status(
    url: &str,
    timeout_ms: u64,
    accept: &str,
) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: Infring-Eyes/1.0")
        .arg("-H")
        .arg(format!("Accept: {accept}"))
        .arg("-w")
        .arg("\n__PROTHEUS_STATUS__:%{http_code}\n")
        .arg(url)
        .output()
        .map_err(|err| format!("collector_fetch_spawn_failed:{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = classify_curl_transport_error(&stderr);
        return Err(format!("{code}:{}", clean_text(Some(&stderr), 220)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = "\n__PROTHEUS_STATUS__:";
    let marker_pos = stdout
        .rfind(marker)
        .ok_or_else(|| "collector_fetch_missing_status_marker".to_string())?;
    let body = stdout[..marker_pos].to_string();
    let status_raw = stdout[(marker_pos + marker.len())..]
        .lines()
        .next()
        .unwrap_or("0")
        .trim()
        .to_string();
    let status = status_raw.parse::<u64>().unwrap_or(0);
    let bytes = body.as_bytes().len() as u64;
    Ok((status, body, bytes))
}

fn parse_json_or_null(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)
}
