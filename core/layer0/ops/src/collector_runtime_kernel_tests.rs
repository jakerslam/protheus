use super::*;
use tempfile::tempdir;

#[test]
fn prepare_attempt_and_circuit_flow() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let rate_path = root.join("collector_rate_state.json");

    let payload = json!({
        "collector_id": "feed_alpha",
        "rate_state_path": rate_path.display().to_string(),
        "min_interval_ms": 1,
        "base_backoff_ms": 2,
        "max_backoff_ms": 10,
        "circuit_after_failures": 1,
        "circuit_open_ms": 200
    });
    let obj = payload_obj(&payload);

    let pre = handle_prepare_attempt(root, obj).expect("prepare");
    assert_eq!(
        pre.get("circuit_open").and_then(Value::as_bool),
        Some(false)
    );

    let fail = handle_mark_failure(root, obj).expect("fail");
    let streak = fail
        .get("row")
        .and_then(|v| v.get("failure_streak"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert_eq!(streak, 1);

    let blocked = handle_prepare_attempt(root, obj).expect("blocked");
    assert_eq!(
        blocked.get("circuit_open").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        blocked
            .get("retry_after_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
}

#[test]
fn prepare_run_respects_cadence() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let payload = json!({
        "collector_id": "feed_beta",
        "min_hours": 24.0,
        "force": false
    });
    let meta_path = meta_path_for(root, payload_obj(&payload), "feed_beta");
    write_json(
        &meta_path,
        &json!({
            "collector_id": "feed_beta",
            "last_run": chrono::Utc::now().to_rfc3339(),
            "last_success": chrono::Utc::now().to_rfc3339(),
            "seen_ids": []
        }),
    )
    .expect("write meta");

    let out = handle_prepare_run(root, payload_obj(&payload)).expect("prepare-run");
    assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
}

#[test]
fn finalize_run_uses_cache_on_error() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let payload = json!({
        "collector_id": "json_beta",
        "max_items": 5,
        "min_hours": 4.0,
        "use_cache_when_empty": false,
        "fetch_error_code": "timeout",
        "items": [],
        "meta": {"collector_id":"json_beta","seen_ids":[]},
        "requests": 1,
        "bytes": 0,
        "duration_ms": 5
    });
    let cache_path = cache_path_for(root, payload_obj(&payload), "json_beta");
    write_json(
        &cache_path,
        &json!({
            "items": [
                {"id":"a","title":"cached row","bytes":77}
            ]
        }),
    )
    .expect("write cache");
    let out = handle_finalize_run(root, payload_obj(&payload)).expect("finalize");
    assert_eq!(out.get("cache_hit").and_then(Value::as_bool), Some(true));
    assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
}

#[test]
fn mark_failure_derives_retryable_from_code() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let payload = json!({
        "collector_id": "feed_gamma",
        "rate_state_path": root.join("collector_rate_state.json").display().to_string(),
        "code": "http_4xx"
    });
    let out = handle_mark_failure(root, payload_obj(&payload)).expect("mark-failure");
    assert_eq!(out.get("retryable").and_then(Value::as_bool), Some(false));

    let payload_retry = json!({
        "collector_id": "feed_gamma",
        "rate_state_path": root.join("collector_rate_state.json").display().to_string(),
        "code": "timeout"
    });
    let out_retry =
        handle_mark_failure(root, payload_obj(&payload_retry)).expect("mark-failure-timeout");
    assert_eq!(
        out_retry.get("retryable").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn classify_error_maps_http_404_and_dns() {
    let http_out = handle_classify_error(payload_obj(&json!({
        "message": "HTTP 404 for https://example.invalid"
    })));
    assert_eq!(
        http_out.get("code").and_then(Value::as_str),
        Some("http_404")
    );
    assert_eq!(
        http_out.get("retryable").and_then(Value::as_bool),
        Some(false)
    );

    let dns_out = handle_classify_error(payload_obj(&json!({
        "code": "ENOTFOUND",
        "message": "getaddrinfo ENOTFOUND example.invalid"
    })));
    assert_eq!(
        dns_out.get("code").and_then(Value::as_str),
        Some("dns_unreachable")
    );
    assert_eq!(
        dns_out.get("retryable").and_then(Value::as_bool),
        Some(true)
    );
}
