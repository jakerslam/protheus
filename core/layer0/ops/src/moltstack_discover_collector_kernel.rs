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
    lane_utils::clean_text(raw, max_len)
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

fn preflight(payload: &Map<String, Value>) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();
    let url = resolved_api_url(payload);
    let max_items = payload
        .get("max_items")
        .and_then(Value::as_u64)
        .unwrap_or(20);
    let max_seconds = payload
        .get("max_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(10);

    match split_scheme_host(&url) {
        Some((scheme, host)) => {
            if scheme != "https" {
                failures.push(json!({
                    "code": "invalid_config",
                    "message": format!("URL must use https: {url}")
                }));
            }
            let allowlist = parse_allowlist(payload);
            let allowed = if allowlist.is_empty() {
                host == "moltstack.net" || host.ends_with(".moltstack.net")
            } else {
                allowlist
                    .iter()
                    .any(|d| host == *d || host.ends_with(&format!(".{d}")))
            };
            if !allowed {
                failures.push(json!({
                    "code": "domain_not_allowlisted",
                    "message": format!("host not allowlisted: {host}")
                }));
            } else {
                checks.push(json!({
                    "name": "allowlisted_url",
                    "ok": true,
                    "host": host
                }));
            }
        }
        None => failures.push(json!({
            "code": "invalid_config",
            "message": format!("Invalid URL: {url}")
        })),
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

    if max_seconds == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_seconds must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_seconds_valid",
            "ok": true,
            "value": max_seconds
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "moltstack_discover",
        "checks": checks,
        "failures": failures
    })
}

fn extract_posts(raw: &Value) -> Vec<Value> {
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

fn map_posts(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let topics_cfg = configured_topics(payload);
    let posts = extract_posts(payload.get("posts").unwrap_or(&Value::Null));
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
        let slug = clean_text(obj.get("slug").and_then(Value::as_str), 120);
        let agent_slug = clean_text(
            obj.get("agent")
                .and_then(Value::as_object)
                .and_then(|a| a.get("slug"))
                .and_then(Value::as_str),
            120,
        );
        if title.is_empty() || slug.is_empty() {
            continue;
        }
        let explicit_url = clean_text(obj.get("url").and_then(Value::as_str), 600);
        let url = if !explicit_url.is_empty() {
            explicit_url
        } else if !agent_slug.is_empty() {
            format!("https://moltstack.net/{agent_slug}/{slug}")
        } else {
            format!("https://moltstack.net/discover/{slug}")
        };
        if host_from_url(&url).is_none() {
            continue;
        }
        let url_len = url.len();
        let topics = keyword_topics(&title, &topics_cfg)
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        items.push(json!({
            "collected_at": now_iso(),
            "id": sha16(&url),
            "url": url,
            "title": title,
            "topics": topics,
            "bytes": std::cmp::min(512_usize, title.len() + 64 + url_len)
        }));
    }
    json!({
        "ok": true,
        "items": items
    })
}

fn build_fetch_plan(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let max_seconds = clamp_u64(payload, "max_seconds", 10, 1, 30);
    let timeout_ms = (max_seconds.saturating_mul(1000)).min(15_000);
    let url = resolved_api_url(payload);
    json!({
        "ok": true,
        "max_items": max_items,
        "timeout_ms": timeout_ms,
        "requests": [
            {
                "key": "posts_json",
                "url": url,
                "required": true,
                "accept": "application/json"
            }
        ]
    })
}

fn classify_fetch_error(payload: &Map<String, Value>) -> Value {
    let code =
        clean_text(payload.get("error_code").and_then(Value::as_str), 80).to_ascii_lowercase();
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
    let fallback_allowed = fallback_codes.contains(&code.as_str());
    json!({
        "ok": true,
        "error_code": code,
        "fallback_allowed": fallback_allowed
    })
}

fn finalize_run(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let topics = payload
        .get("topics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let posts = payload.get("posts").cloned().unwrap_or(Value::Null);
    map_posts(&object_from_value(json!({
        "max_items": max_items,
        "topics": topics,
        "posts": posts
    })))
}

fn command_collect(payload: &Map<String, Value>) -> Result<Value, String> {
    let pre = preflight(payload);
    if pre.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(first_preflight_error(&pre));
    }

    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 80);
    if !fetch_error.is_empty() {
        let policy = classify_fetch_error(&object_from_value(json!({
            "error_code": fetch_error
        })));
        return Ok(json!({
            "ok": true,
            "success": false,
            "fallback_allowed": policy.get("fallback_allowed").cloned().unwrap_or(Value::Bool(false)),
            "error_code": clean_text(policy.get("error_code").and_then(Value::as_str), 80)
        }));
    }

    let mapped = finalize_run(&object_from_value(json!({
        "max_items": payload.get("max_items").cloned().unwrap_or(Value::from(20)),
        "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
        "posts": payload.get("posts_json").cloned().unwrap_or_else(|| payload.get("posts").cloned().unwrap_or(Value::Null))
    })));
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
    let max_seconds = clamp_u64(payload, "max_seconds", 10, 1, 30);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = build_fetch_plan(payload);
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = {
        let override_url = clean_text(payload.get("api_url").and_then(Value::as_str), 800);
        if override_url.is_empty() {
            clean_text(request.get("url").and_then(Value::as_str), 800)
        } else {
            override_url
        }
    };
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 160);
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            plan.get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or(10_000)
        })
        .clamp(1_000, 30_000);

    let (posts_json, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &accept) {
            Ok((status, body, _)) => {
                if status >= 400 {
                    (
                        Value::Null,
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    let b = body.as_bytes().len() as u64;
                    (parse_json_or_null(&body), b, 1_u64, None)
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

    let mut out = command_collect(&object_from_value(json!({
        "api_url": fetch_url,
        "allowed_domains": payload.get("allowed_domains").cloned().unwrap_or(Value::Array(Vec::new())),
        "max_seconds": max_seconds,
        "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
        "max_items": max_items,
        "posts_json": posts_json,
        "fetch_error": fetch_error,
        "duration_ms": duration_ms
    })))?;
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
        "build-fetch-plan" => Ok(build_fetch_plan(payload)),
        "classify-fetch-error" => Ok(classify_fetch_error(payload)),
        "finalize-run" => Ok(finalize_run(payload)),
        "map-posts" => Ok(map_posts(payload)),
        "collect" => command_collect(payload),
        _ => Err("moltstack_discover_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "moltstack_discover_collector_kernel")
    {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltstack_discover_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(&command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "moltstack_discover_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltstack_discover_collector_kernel_error",
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
    fn preflight_flags_non_https() {
        let payload = json!({
            "api_url": "http://moltstack.net/api/posts",
            "allowed_domains": ["moltstack.net"],
            "max_items": 10,
            "max_seconds": 5
        });
        let out = preflight(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn map_posts_emits_items() {
        let payload = json!({
            "max_items": 10,
            "topics": ["automation"],
            "posts": {
              "posts": [
                {"title":"AI workflow automation","slug":"ai-workflow","agent":{"slug":"agent-x"}}
              ]
            }
        });
        let out = map_posts(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn build_fetch_plan_defaults_url() {
        let out = build_fetch_plan(&Map::new());
        let url = out
            .get("requests")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_object)
            .and_then(|o| o.get("url"))
            .and_then(Value::as_str);
        assert_eq!(url, Some("https://moltstack.net/api/posts"));
    }

    #[test]
    fn classify_fetch_error_allows_fallback_for_rate_limited() {
        let out = classify_fetch_error(lane_utils::payload_obj(&json!({
            "error_code": "rate_limited"
        })));
        assert_eq!(
            out.get("fallback_allowed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn collect_returns_fallback_signal_on_fetch_error() {
        let out = command_collect(lane_utils::payload_obj(&json!({
            "api_url": "https://moltstack.net/api/posts",
            "allowed_domains": ["moltstack.net"],
            "max_items": 10,
            "max_seconds": 5,
            "fetch_error": "rate_limited"
        })))
        .expect("collect");
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("fallback_allowed").and_then(Value::as_bool),
            Some(true)
        );
    }
}
