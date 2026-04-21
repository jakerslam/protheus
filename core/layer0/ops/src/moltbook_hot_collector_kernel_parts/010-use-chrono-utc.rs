// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

const DEFAULT_HOST: &str = "www.moltbook.com";

fn usage() {
    println!("moltbook-hot-collector-kernel commands:");
    println!("  protheus-ops moltbook-hot-collector-kernel run --payload-base64=<json>");
    println!("  protheus-ops moltbook-hot-collector-kernel preflight --payload-base64=<json>");
    println!("  protheus-ops moltbook-hot-collector-kernel classify-fetch-error --payload-base64=<json>");
    println!("  protheus-ops moltbook-hot-collector-kernel map-posts --payload-base64=<json>");
    println!("  protheus-ops moltbook-hot-collector-kernel collect --payload-base64=<json>");
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

fn sha16(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn parse_allowlist(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("allowed_domains")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 200).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn host_allowed(host: &str, allowlist: &[String]) -> bool {
    allowlist
        .iter()
        .any(|d| host == d || host.ends_with(&format!(".{d}")))
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
    headers: &[(String, String)],
    accept: &str,
) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let mut cmd = Command::new("curl");
    cmd.arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: infringing-moltbook-hot/1.0")
        .arg("-H")
        .arg(format!("Accept: {accept}"));
    for (k, v) in headers {
        cmd.arg("-H").arg(format!("{k}: {v}"));
    }
    cmd.arg("-w")
        .arg("\n__PROTHEUS_STATUS__:%{http_code}\n")
        .arg(url);

    let output = cmd
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

fn parse_json_or_null(raw: &str) -> Value { serde_json::from_str::<Value>(raw).unwrap_or(Value::Null) }

fn resolve_api_base(payload: &Map<String, Value>) -> String {
    let env_api_base = std::env::var("MOLTBOOK_API_BASE").ok();
    let candidate = clean_text(
        payload
            .get("api_base")
            .and_then(Value::as_str)
            .or(env_api_base.as_deref()),
        400,
    );
    let base = if candidate.is_empty() { "https://api.moltbook.com".to_string() } else { candidate };
    base.trim_end_matches('/').to_string()
}

fn requested_host(payload: &Map<String, Value>) -> String { clean_text(payload.get("host").and_then(Value::as_str), 200).to_ascii_lowercase() }

fn host_from_urlish(raw: &str) -> String {
    raw.trim().split_once("://").map(|(_, rest)| rest).unwrap_or(raw.trim()).trim_start_matches("//").split(['/', '?', '#']).next().unwrap_or("").rsplit('@').next().unwrap_or("").trim().split(':').next().unwrap_or("").trim_matches(|c| c == '[' || c == ']').to_ascii_lowercase()
}

fn resolved_host(payload: &Map<String, Value>) -> String {
    let from_api_base = host_from_urlish(&resolve_api_base(payload));
    if !from_api_base.is_empty() {
        return from_api_base;
    }
    match requested_host(payload) { requested if !requested.is_empty() => requested, _ => DEFAULT_HOST.to_string() }
}

fn auth_headers(payload: &Map<String, Value>) -> Vec<(String, String)> {
    let mut out = Vec::<(String, String)>::new();
    let env_api_key = std::env::var("MOLTBOOK_API_KEY").ok();
    let direct_api_key = clean_text(
        payload
            .get("api_key")
            .and_then(Value::as_str)
            .or(env_api_key.as_deref()),
        256,
    );
    if !direct_api_key.is_empty() {
        out.push((
            "Authorization".to_string(),
            format!("Bearer {direct_api_key}"),
        ));
    }
    let api_key_handle = clean_text(payload.get("api_key_handle").and_then(Value::as_str), 256);
    if !api_key_handle.is_empty() {
        out.push(("x-secret-handle".to_string(), api_key_handle));
    }
    out
}

fn normalize_topics(payload: &Map<String, Value>) -> Vec<Value> {
    let mut dedup = BTreeSet::<String>::new();
    if let Some(rows) = payload.get("topics").and_then(Value::as_array) {
        for row in rows {
            let topic = clean_text(row.as_str(), 64).to_ascii_lowercase();
            if !topic.is_empty() {
                dedup.insert(topic);
            }
        }
    }
    dedup
        .into_iter()
        .take(5)
        .map(Value::String)
        .collect::<Vec<_>>()
}

fn extract_posts(payload: &Map<String, Value>) -> Vec<Value> {
    let raw = payload.get("posts").cloned().unwrap_or(Value::Null);
    if let Some(rows) = raw.as_array() {
        return rows.clone();
    }
    if let Some(rows) = raw.get("posts").and_then(Value::as_array) {
        return rows.clone();
    }
    raw.get("data")
        .and_then(|d| d.get("posts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn post_id(post: &Map<String, Value>) -> String {
    clean_text(
        post.get("id")
            .and_then(Value::as_str)
            .or_else(|| post.get("post_id").and_then(Value::as_str)),
        120,
    )
}

fn post_url(post: &Map<String, Value>, pid: &str) -> String {
    let direct = clean_text(post.get("url").and_then(Value::as_str), 600);
    if !direct.is_empty() { direct } else if !pid.is_empty() { format!("https://www.moltbook.com/p/{pid}") } else { String::new() }
}

fn preflight_error(pre: &Value) -> String {
    let first = pre
        .get("failures")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let code = clean_text(first.get("code").and_then(Value::as_str), 64);
    let message = clean_text(first.get("message").and_then(Value::as_str), 180);
    if code.is_empty() { "moltbook_hot_preflight_failed".to_string() } else if message.is_empty() { format!("moltbook_hot_preflight_failed:{code}") } else { format!("moltbook_hot_preflight_failed:{code}:{message}") }
}
