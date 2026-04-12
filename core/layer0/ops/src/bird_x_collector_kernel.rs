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

fn command_map_results(payload: &Map<String, Value>) -> Value {
    let max_items = support::clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let collector_id = support::clean_text(payload.get("collector_id").and_then(Value::as_str), 64);
    let collector_id = if collector_id.is_empty() {
        support::COLLECTOR_ID.to_string()
    } else {
        collector_id
    };

    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = support::normalize_seen_ids(payload);
    let mut items = Vec::<Value>::new();
    for row in results {
        if items.len() >= max_items {
            break;
        }
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let tweet_id = support::clean_text(
            obj.get("id")
                .and_then(Value::as_str)
                .or_else(|| obj.get("tweet_id").and_then(Value::as_str)),
            120,
        );
        if tweet_id.is_empty() || seen.contains(&tweet_id) {
            continue;
        }

        let content = support::clean_text(
            obj.get("text")
                .and_then(Value::as_str)
                .or_else(|| obj.get("content").and_then(Value::as_str)),
            1200,
        );
        let (author_handle, author_name) = support::extract_author_parts(obj);
        let title = support::first_line_title(&content, &author_handle);
        let likes = support::as_i64(obj, &["likes", "favorite_count"]);
        let retweets = support::as_i64(obj, &["retweets", "retweet_count"]);
        let url = if author_handle.is_empty() || author_handle == "unknown" {
            format!("https://x.com/i/web/status/{tweet_id}")
        } else {
            format!("https://x.com/{author_handle}/status/{tweet_id}")
        };
        let topics = support::infer_topics(&content)
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        let id = support::sha16(&format!(
            "{}|{}",
            tweet_id,
            support::clean_text(Some(&content), 200)
        ));
        let tags = vec![
            Value::String(support::clean_text(Some(&author_name), 120)),
            Value::String(format!("likes:{likes}")),
            Value::String(format!("rt:{retweets}")),
        ];
        let item = json!({
            "collected_at": support::now_iso(),
            "eye_id": collector_id,
            "id": id,
            "tweet_id": tweet_id,
            "title": title,
            "description": content,
            "url": url,
            "author": author_handle,
            "tags": tags,
            "topics": topics,
            "bytes": std::cmp::min(4096_usize, title.len() + support::clean_text(obj.get("text").and_then(Value::as_str), 1200).len() + 160)
        });
        items.push(item);
        seen.insert(support::clean_seen_id(&tweet_id));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 4000 {
        let drop = seen_ids.len() - 4000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn finalize_success(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
) -> Result<Value, String> {
    let mut meta = support::normalize_meta_value(payload.get("meta"));
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen_ids = payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(drop).collect::<Vec<_>>();
    }

    meta["seen_ids"] = Value::Array(seen_ids);
    meta["last_run"] = Value::String(support::now_iso());
    if !items.is_empty() {
        meta["last_success"] = Value::String(support::now_iso());
        support::write_json_atomic(
            &support::cache_path_for(root, payload),
            &json!({ "items": items }),
        )?;
    }
    support::write_json_atomic(&support::meta_path_for(root, payload), &meta)?;

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": support::COLLECTOR_ID,
        "items": items.into_iter().take(max_items).collect::<Vec<_>>(),
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests,
        "cadence_hours": min_hours
    }))
}

fn finalize_error(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
) -> Result<Value, String> {
    let mut meta = support::normalize_meta_value(payload.get("meta"));
    meta["last_run"] = Value::String(support::now_iso());
    support::write_json_atomic(&support::meta_path_for(root, payload), &meta)?;

    let failures = support::query_failures(payload);
    if !failures.is_empty() {
        let cached = support::load_cache_items(root, payload);
        if !cached.is_empty() {
            return Ok(json!({
                "ok": true,
                "success": true,
                "eye": support::COLLECTOR_ID,
                "cache_hit": true,
                "degraded": true,
                "items": cached.into_iter().take(max_items).collect::<Vec<_>>(),
                "bytes": bytes,
                "duration_ms": duration_ms,
                "requests": requests,
                "failure_count": failures.len(),
                "failures": failures.into_iter().take(3).collect::<Vec<_>>(),
                "cadence_hours": min_hours
            }));
        }

        let primary = failures
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let code = support::clean_text(primary.get("code").and_then(Value::as_str), 80);
        let message = support::clean_text(primary.get("message").and_then(Value::as_str), 220);
        let http_status = primary.get("http_status").and_then(Value::as_i64);
        return Ok(json!({
            "ok": false,
            "success": false,
            "eye": support::COLLECTOR_ID,
            "items": [],
            "bytes": bytes,
            "duration_ms": duration_ms,
            "requests": requests,
            "error": if message.is_empty() { Value::String("bird_x_all_queries_failed".to_string()) } else { Value::String(message) },
            "error_code": if code.is_empty() { Value::String("collector_error".to_string()) } else { Value::String(code) },
            "error_http_status": http_status,
            "failure_count": failures.len(),
            "failures": failures.into_iter().take(3).collect::<Vec<_>>(),
            "cadence_hours": min_hours
        }));
    }

    Ok(json!({
        "ok": false,
        "success": false,
        "eye": support::COLLECTOR_ID,
        "items": [],
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests,
        "error": "bird_x_no_results",
        "error_code": "collector_error",
        "error_http_status": Value::Null,
        "cadence_hours": min_hours
    }))
}

fn command_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = support::as_f64(payload.get("min_hours"), 0.0).clamp(0.0, 24.0 * 365.0);
    let max_items = support::clamp_u64(payload, "max_items", 15, 1, 200) as usize;
    let bytes = support::clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = support::clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = support::clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !items.is_empty() {
        return finalize_success(
            root,
            payload,
            min_hours,
            max_items,
            bytes,
            requests,
            duration_ms,
        );
    }
    finalize_error(
        root,
        payload,
        min_hours,
        max_items,
        bytes,
        requests,
        duration_ms,
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "preflight" => Ok(command_preflight(payload)),
        "prepare-run" => Ok(command_prepare_run(root, payload)),
        "map-results" => Ok(command_map_results(payload)),
        "finalize-run" => command_finalize_run(root, payload),
        "collect" => command_collect(root, payload),
        _ => Err("bird_x_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "bird_x_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "bird_x_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("bird_x_collector_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "bird_x_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        let nonce = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        root.push(format!("protheus_bird_x_kernel_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir temp root");
        root
    }

    #[test]
    fn preflight_reflects_missing_cli() {
        let payload = json!({"bird_cli_present": false});
        let out = command_preflight(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn map_results_dedupes_and_infers_topics() {
        let payload = json!({
            "max_items": 10,
            "seen_ids": [],
            "results": [
                { "id": "123", "text": "AI agent launch update", "author": { "handle": "ax", "name": "Axiom" }, "likes": 10, "retweets": 3 },
                { "id": "123", "text": "AI agent launch update", "author": { "handle": "ax", "name": "Axiom" }, "likes": 10, "retweets": 3 },
                { "id": "456", "text": "LLM news update", "author_name": "Anon" }
            ]
        });
        let out = command_map_results(lane_utils::payload_obj(&payload));
        let rows = out.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].get("url").and_then(Value::as_str), Some("https://x.com/i/web/status/456"));
        assert_eq!(rows[0].get("topics").and_then(Value::as_array).map(|topics| !topics.is_empty()), Some(true));
    }

    #[test]
    fn prepare_run_respects_cadence() {
        let root = temp_root("cadence");
        let payload = json!({"force":false,"min_hours":4.0});
        let meta_path = support::meta_path_for(&root, lane_utils::payload_obj(&payload));
        support::write_json_atomic(
            &meta_path,
            &json!({"last_run": support::now_iso(), "seen_ids": []}),
        )
        .expect("write meta");
        let out = command_prepare_run(&root, lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn finalize_run_returns_cache_on_failures() {
        let root = temp_root("cache_fallback");
        let payload = json!({});
        let cache_path = support::cache_path_for(&root, lane_utils::payload_obj(&payload));
        support::write_json_atomic(
            &cache_path,
            &json!({"items":[{"id":"cached","title":"cached title","bytes":11}]}),
        )
        .expect("write cache");
        let out = command_finalize_run(
            &root,
            lane_utils::payload_obj(&json!({
                "meta": {"seen_ids":[]},
                "items": [],
                "query_failures": [{"code":"timeout","message":"request timed out","http_status":null}],
                "bytes": 0,
                "requests": 1,
                "duration_ms": 10
            })),
        )
        .expect("finalize");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("cache_hit").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("items").and_then(Value::as_array).map(|rows| rows.len()), Some(1));
    }

    #[test]
    fn collect_returns_preflight_error_when_bird_missing() {
        let root = temp_root("collect_preflight_missing");
        let out = command_collect(
            &root,
            lane_utils::payload_obj(&json!({
                "bird_cli_present": false
            })),
        )
        .expect("collect");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("env_blocked"));
    }
}
