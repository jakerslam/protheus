// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

fn json_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn json_f64(payload: &Map<String, Value>, key: &str, fallback: f64, lo: f64, hi: f64) -> f64 {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn json_bool(payload: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn env_u64(key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn env_bool(key: &str, fallback: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .map(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(fallback)
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
    .to_ascii_lowercase()
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn sanitize_feed_candidates(payload: &Map<String, Value>) -> Vec<Value> {
    payload
        .get("feed_candidates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(Some(raw), 600))
                .filter(|row| !row.is_empty())
                .take(20)
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn parse_headers(payload: &Map<String, Value>) -> Vec<(String, String)> {
    payload
        .get("headers")
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .map(|(k, v)| (clean_text(Some(k), 120), clean_text(v.as_str(), 400)))
                .filter(|(k, v)| !k.is_empty() && !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
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

pub fn split_error_code(err: &str) -> String {
    let trimmed = clean_text(Some(err), 240);
    let code = trimmed.split(':').next().unwrap_or("").trim();
    if code.is_empty() {
        "collector_error".to_string()
    } else {
        code.to_string()
    }
}

pub fn curl_fetch_with_status(
    url: &str,
    timeout_ms: u64,
    headers: &[(String, String)],
) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let mut cmd = Command::new("curl");
    cmd.arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: Infring-Eyes/1.0")
        .arg("-H")
        .arg("Accept: */*");
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

pub fn resolve_controls(payload: &Map<String, Value>) -> Value {
    let collector_id = clean_collector_id(payload);
    let default_scope = format!("sensory.collector.{collector_id}");
    let default_caller = format!("adaptive/sensory/eyes/collectors/{collector_id}");

    let scope = {
        let cleaned = clean_text(payload.get("scope").and_then(Value::as_str), 120);
        if cleaned.is_empty() {
            default_scope
        } else {
            cleaned
        }
    };
    let caller = {
        let cleaned = clean_text(payload.get("caller").and_then(Value::as_str), 220);
        if cleaned.is_empty() {
            default_caller
        } else {
            cleaned
        }
    };

    let timeout_ms = json_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let attempts = json_u64(payload, "attempts", 3, 1, 5);
    let min_interval_ms = json_u64(
        payload,
        "min_interval_ms",
        env_u64("EYES_COLLECTOR_MIN_INTERVAL_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let base_backoff_ms = json_u64(
        payload,
        "base_backoff_ms",
        env_u64("EYES_COLLECTOR_BACKOFF_BASE_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let max_backoff_ms = json_u64(
        payload,
        "max_backoff_ms",
        env_u64("EYES_COLLECTOR_BACKOFF_MAX_MS", 8_000, 200, 120_000),
        200,
        120_000,
    );
    let circuit_open_ms = json_u64(
        payload,
        "circuit_open_ms",
        env_u64("EYES_COLLECTOR_CIRCUIT_MS", 30_000, 500, 300_000),
        500,
        300_000,
    );
    let circuit_after_failures = json_u64(
        payload,
        "circuit_after_failures",
        env_u64("EYES_COLLECTOR_CIRCUIT_AFTER", 3, 1, 10),
        1,
        10,
    );
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);
    let max_items = json_u64(payload, "max_items", 20, 1, 200);
    let force = json_bool(payload, "force", false);
    let allow_direct_fetch_fallback = json_bool(
        payload,
        "allow_direct_fetch_fallback",
        env_bool("EYES_COLLECTOR_ALLOW_DIRECT_FETCH_FALLBACK", false),
    );
    let url = clean_text(payload.get("url").and_then(Value::as_str), 600);
    let feed_candidates = sanitize_feed_candidates(payload);

    json!({
        "ok": true,
        "collector_id": collector_id,
        "scope": scope,
        "caller": caller,
        "timeout_ms": timeout_ms,
        "attempts": attempts,
        "min_interval_ms": min_interval_ms,
        "base_backoff_ms": base_backoff_ms,
        "max_backoff_ms": max_backoff_ms,
        "circuit_open_ms": circuit_open_ms,
        "circuit_after_failures": circuit_after_failures,
        "min_hours": min_hours,
        "max_items": max_items,
        "force": force,
        "allow_direct_fetch_fallback": allow_direct_fetch_fallback,
        "url": if url.is_empty() { Value::Null } else { Value::String(url) },
        "feed_candidates": feed_candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_controls_normalizes_defaults() {
        let payload = json!({
            "collector_id": "Feed-Alpha",
            "scope": " sensory.collector.feed_alpha ",
            "feed_candidates": [" https://a.example/rss ", "", 12],
            "attempts": 9,
            "max_items": 999,
        });
        let out = resolve_controls(payload.as_object().expect("payload object"));
        assert_eq!(
            out.get("collector_id").and_then(Value::as_str),
            Some("feed-alpha")
        );
        assert_eq!(
            out.get("scope").and_then(Value::as_str),
            Some("sensory.collector.feed_alpha")
        );
        assert_eq!(out.get("attempts").and_then(Value::as_u64), Some(5));
        assert_eq!(out.get("max_items").and_then(Value::as_u64), Some(200));
        let feeds = out
            .get("feed_candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(feeds.len(), 1);
    }
}
