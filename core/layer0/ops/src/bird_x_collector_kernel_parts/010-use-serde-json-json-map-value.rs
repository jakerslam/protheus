// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::path::Path;

use crate::bird_x_collector_kernel_support as support;
use crate::contract_lane_utils as lane_utils;

fn usage() {
    for line in [
        "bird-x-collector-kernel commands:",
        "  protheus-ops bird-x-collector-kernel preflight --payload-base64=<json>",
        "  protheus-ops bird-x-collector-kernel prepare-run --payload-base64=<json>",
        "  protheus-ops bird-x-collector-kernel map-results --payload-base64=<json>",
        "  protheus-ops bird-x-collector-kernel finalize-run --payload-base64=<json>",
        "  protheus-ops bird-x-collector-kernel collect --payload-base64=<json>",
    ] { println!("{line}"); }
}

fn command_preflight(payload: &Map<String, Value>) -> Value {
    let bird_present = payload
        .get("bird_cli_present")
        .and_then(Value::as_bool)
        .unwrap_or_else(support::bird_cli_present);
    if !bird_present {
        json!({
            "ok": false,
            "parser_type": "bird_x",
            "reachable": false,
            "authenticated": false,
            "items_sample": 0,
            "checks": [{ "name": "bird_cli_present", "ok": false }],
            "failures": [{ "code": "env_blocked", "message": "bird CLI not found in PATH" }],
            "error": "env_blocked"
        })
    } else {
        json!({
            "ok": true,
            "parser_type": "bird_x",
            "reachable": true,
            "authenticated": Value::Null,
            "items_sample": 0,
            "checks": [{ "name": "bird_cli_present", "ok": true }],
            "failures": [],
            "error": Value::Null
        })
    }
}

fn command_collect(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let preflight = command_preflight(payload);
    if preflight.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(preflight);
    }

    let force = support::as_bool(payload.get("force"), false);
    let min_hours = support::as_f64(payload.get("min_hours"), 0.0).clamp(0.0, 24.0 * 365.0);
    let max_items = support::normalize_max_items(payload) as usize;
    let max_items_per_query = support::normalize_max_items_per_query(payload);
    let timeout_ms = support::normalize_timeout_ms(payload);
    let retry_attempts = support::normalize_retry_attempts(payload);
    let queries = support::normalize_queries(payload);

    let prepared = command_prepare_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": support::COLLECTOR_ID,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": []
        }));
    }

    let meta = prepared
        .get("meta")
        .cloned()
        .unwrap_or_else(|| support::normalize_meta_value(None));
    let mut seen_ids = meta
        .as_object()
        .and_then(|row| row.get("seen_ids"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let started_ms = chrono::Utc::now().timestamp_millis();
    let mut items = Vec::<Value>::new();
    let mut bytes = 0u64;
    let mut requests = 0u64;
    let mut query_failures = Vec::<Value>::new();

    for query in queries {
        if items.len() >= max_items {
            break;
        }
        let mut attempt_error: Option<Value> = None;
        for attempt in 0..(retry_attempts as usize) {
            match support::run_bird_search_once(&query, max_items_per_query, timeout_ms) {
                Ok((results, response_bytes)) => {
                    requests = requests.saturating_add(1);
                    bytes = bytes.saturating_add(response_bytes);
                    let mapped = command_map_results(
                        &json!({
                            "results": results,
                            "seen_ids": seen_ids,
                            "collector_id": support::COLLECTOR_ID,
                            "max_items": (max_items.saturating_sub(items.len())).max(1),
                        })
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                    );
                    if let Some(rows) = mapped.get("items").and_then(Value::as_array) {
                        for item in rows {
                            if items.len() >= max_items {
                                break;
                            }
                            items.push(item.clone());
                        }
                    }
                    seen_ids = mapped
                        .get("seen_ids")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    if seen_ids.len() > 2000 {
                        let drop = seen_ids.len() - 2000;
                        seen_ids = seen_ids.into_iter().skip(drop).collect::<Vec<_>>();
                    }
                    attempt_error = None;
                    break;
                }
                Err(err_payload) => {
                    let code =
                        support::clean_text(err_payload.get("code").and_then(Value::as_str), 80);
                    let retryable =
                        code == "timeout" || code == "parse_failed" || code == "collector_error";
                    attempt_error = Some(err_payload);
                    if !retryable || attempt + 1 >= retry_attempts as usize {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(
                        support::sleep_backoff_ms(attempt),
                    ));
                }
            }
        }
        if let Some(err_payload) = attempt_error {
            query_failures.push(json!({
                "code": support::clean_text(err_payload.get("code").and_then(Value::as_str), 80),
                "message": support::clean_text(err_payload.get("message").and_then(Value::as_str), 220),
                "http_status": err_payload.get("http_status").cloned().unwrap_or(Value::Null),
            }));
        }
    }

    let duration_ms = (chrono::Utc::now().timestamp_millis() - started_ms).max(0) as u64;
    command_finalize_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "meta": meta,
            "items": items,
            "seen_ids": seen_ids,
            "max_items": max_items,
            "min_hours": min_hours,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "query_failures": query_failures
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn command_prepare_run(root: &Path, payload: &Map<String, Value>) -> Value {
    let force = support::as_bool(payload.get("force"), false);
    let min_hours = support::as_f64(payload.get("min_hours"), 0.0).clamp(0.0, 24.0 * 365.0);
    let meta_path = support::meta_path_for(root, payload);
    let meta = support::normalize_meta_value(Some(&support::read_json(
        &meta_path,
        support::normalize_meta_value(None),
    )));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(support::parse_iso_ms);
    let hours_since_last = last_run_ms
        .map(|ms| ((chrono::Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);
    json!({
        "ok": true,
        "collector_id": support::COLLECTOR_ID,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    })
}
