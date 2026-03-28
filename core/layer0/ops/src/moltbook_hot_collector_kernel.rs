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
    lane_utils::clean_text(raw, max_len)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
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

fn http_status_to_code(status: u64) -> &'static str {
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

fn parse_json_or_null(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)
}

fn resolve_api_base(payload: &Map<String, Value>) -> String {
    let env_api_base = std::env::var("MOLTBOOK_API_BASE").ok();
    let candidate = clean_text(
        payload
            .get("api_base")
            .and_then(Value::as_str)
            .or(env_api_base.as_deref()),
        400,
    );
    let base = if candidate.is_empty() {
        "https://api.moltbook.com".to_string()
    } else {
        candidate
    };
    base.trim_end_matches('/').to_string()
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
        out.push(("Authorization".to_string(), format!("Bearer {direct_api_key}")));
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
    if !direct.is_empty() {
        return direct;
    }
    if !pid.is_empty() {
        return format!("https://www.moltbook.com/p/{pid}");
    }
    String::new()
}

fn preflight(payload: &Map<String, Value>) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();
    let max_items = clamp_u64(payload, "max_items", 20, 0, 200);
    let secret_present = payload
        .get("secret_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let host = clean_text(payload.get("host").and_then(Value::as_str), 200);
    let host = if host.is_empty() {
        DEFAULT_HOST.to_string()
    } else {
        host.to_ascii_lowercase()
    };

    if !secret_present {
        failures.push(json!({
            "code": "auth_missing",
            "message": "missing_moltbook_api_key"
        }));
    } else {
        checks.push(json!({
            "name": "api_key_present",
            "ok": true
        }));
    }

    if max_items == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_items must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_items_valid",
            "ok": true,
            "value": max_items
        }));
    }

    let allowlist = parse_allowlist(payload);
    if allowlist.is_empty() || !host_allowed(&host, &allowlist) {
        failures.push(json!({
            "code": "domain_not_allowlisted",
            "message": format!("collector host not allowlisted: {host}")
        }));
    } else {
        checks.push(json!({
            "name": "allowlisted_host",
            "ok": true,
            "host": host
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "moltbook_hot",
        "checks": checks,
        "failures": failures
    })
}

fn classify_fetch_error(payload: &Map<String, Value>) -> Value {
    let code = clean_text(payload.get("error_code").and_then(Value::as_str), 80).to_ascii_lowercase();
    let fallback_codes = [
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "tls_error",
        "network_error",
        "http_5xx",
        "rate_limited",
        "env_blocked",
    ];
    json!({
        "ok": true,
        "error_code": code,
        "fallback_allowed": fallback_codes.contains(&code.as_str())
    })
}

fn map_posts(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50) as usize;
    let topics = normalize_topics(payload);
    let posts = extract_posts(payload);
    let mut items = Vec::<Value>::new();
    for post in posts {
        if items.len() >= max_items {
            break;
        }
        let obj = match post.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 200);
        let pid = post_id(obj);
        let url = post_url(obj, &pid);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let id = if pid.is_empty() { sha16(&url) } else { pid };
        items.push(json!({
            "collected_at": now_iso(),
            "id": id,
            "url": url.clone(),
            "title": title,
            "topics": topics,
            "bytes": std::cmp::min(1024_usize, title.len() + url.len() + 64)
        }));
    }
    json!({
        "ok": true,
        "items": items
    })
}

fn command_collect(payload: &Map<String, Value>) -> Result<Value, String> {
    let pre = preflight(payload);
    if pre.get("ok").and_then(Value::as_bool) != Some(true) {
        let first = pre
            .get("failures")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let code = clean_text(first.get("code").and_then(Value::as_str), 64);
        let message = clean_text(first.get("message").and_then(Value::as_str), 180);
        return Err(if code.is_empty() {
            "moltbook_hot_preflight_failed".to_string()
        } else if message.is_empty() {
            format!("moltbook_hot_preflight_failed:{code}")
        } else {
            format!("moltbook_hot_preflight_failed:{code}:{message}")
        });
    }

    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 80);
    if !fetch_error.is_empty() {
        let policy = classify_fetch_error(
            &json!({
                "error_code": fetch_error
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        return Ok(json!({
            "ok": true,
            "success": false,
            "fallback_allowed": policy.get("fallback_allowed").cloned().unwrap_or(Value::Bool(false)),
            "error_code": clean_text(policy.get("error_code").and_then(Value::as_str), 80)
        }));
    }

    let mapped = map_posts(
        &json!({
            "max_items": payload.get("max_items").cloned().unwrap_or(Value::from(20)),
            "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
            "posts": payload.get("posts").cloned().unwrap_or(Value::Null)
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
    let items = mapped
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "success": true,
        "items": items
    }))
}

fn command_run(payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .or_else(|| {
            std::env::var("MOLTBOOK_HTTP_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
        })
        .unwrap_or(12_000)
        .clamp(2_000, 30_000);
    let api_base = resolve_api_base(payload);
    let fetch_url = format!("{api_base}/v1/posts/hot?limit={max_items}");
    let auth = auth_headers(payload);

    let (posts, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &auth, "application/json") {
            Ok((status, body, body_bytes)) => {
                if status >= 400 {
                    (
                        Value::Null,
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    (parse_json_or_null(&body), body_bytes, 1_u64, None)
                }
            }
            Err(err) => {
                let code = clean_text(Some(&err), 120)
                    .split(':')
                    .next()
                    .unwrap_or("collector_error")
                    .to_string();
                (Value::Null, 0_u64, 0_u64, Some(code))
            }
        };

    let duration_ms = Utc::now()
        .timestamp_millis()
        .max(0)
        .saturating_sub(started_at_ms as i64) as u64;

    let mut out = command_collect(
        &json!({
            "secret_present": payload.get("secret_present").cloned().unwrap_or(Value::Bool(false)),
            "host": payload.get("host").cloned().unwrap_or(Value::String(DEFAULT_HOST.to_string())),
            "allowed_domains": payload.get("allowed_domains").cloned().unwrap_or(Value::Array(Vec::new())),
            "max_items": max_items,
            "posts": posts,
            "fetch_error": fetch_error,
            "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new()))
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )?;
    if let Some(obj) = out.as_object_mut() {
        obj.insert("bytes".to_string(), Value::from(bytes));
        obj.insert("requests".to_string(), Value::from(requests));
        obj.insert("duration_ms".to_string(), Value::from(duration_ms));
    }
    Ok(out)
}

fn dispatch(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => command_run(payload),
        "preflight" => Ok(preflight(payload)),
        "classify-fetch-error" => Ok(classify_fetch_error(payload)),
        "map-posts" => Ok(map_posts(payload)),
        "collect" => command_collect(payload),
        _ => Err("moltbook_hot_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "moltbook_hot_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltbook_hot_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(&command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "moltbook_hot_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltbook_hot_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_requires_secret_and_allowlist() {
        let out = preflight(lane_utils::payload_obj(&json!({
            "secret_present": false,
            "allowed_domains": ["moltbook.com"],
            "max_items": 10
        })));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        let codes = out
            .get("failures")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.get("code").and_then(Value::as_str).map(str::to_string))
            .collect::<Vec<_>>();
        assert!(codes.contains(&"auth_missing".to_string()));
    }

    #[test]
    fn classify_fetch_error_allows_timeout() {
        let out = classify_fetch_error(lane_utils::payload_obj(&json!({
            "error_code": "timeout"
        })));
        assert_eq!(out.get("fallback_allowed").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn map_posts_outputs_items() {
        let out = map_posts(lane_utils::payload_obj(&json!({
            "max_items": 2,
            "topics": ["ai", "agents"],
            "posts": [
                {"id":"p1","title":"Ship agents","url":"https://www.moltbook.com/p/p1"}
            ]
        })));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items").and_then(Value::as_array).map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn collect_returns_fallback_signal_on_fetch_error() {
        let out = command_collect(lane_utils::payload_obj(&json!({
            "secret_present": true,
            "allowed_domains": ["moltbook.com", "www.moltbook.com"],
            "host": "www.moltbook.com",
            "max_items": 10,
            "fetch_error": "rate_limited"
        })))
        .expect("collect");
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("fallback_allowed").and_then(Value::as_bool), Some(true));
    }
}
