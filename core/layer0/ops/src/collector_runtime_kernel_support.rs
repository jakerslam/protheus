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
