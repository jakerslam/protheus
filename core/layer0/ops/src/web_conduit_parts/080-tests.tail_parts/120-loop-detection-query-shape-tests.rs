
#[test]
fn search_query_shape_error_flags_repetitive_loop_prompt() {
    assert_eq!(
        search_query_shape_error_code("next round next round next round next round next round"),
        "query_shape_repetitive_loop"
    );
}

#[test]
fn fetch_early_validation_contract_includes_requested_url_cache_key() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_fetch(tmp.path(), &json!({"url": "  FTP://Example.com/Doc  "}));
    assert_eq!(
        out.pointer("/requested_url_cache_key").and_then(Value::as_str),
        Some("ftp://example.com/doc")
    );
}

#[test]
fn search_parse_nonnegative_i64_accepts_numeric_string() {
    assert_eq!(
        search_parse_nonnegative_i64(Some(&json!("42"))),
        42
    );
    assert_eq!(
        search_parse_nonnegative_i64(Some(&json!("-8"))),
        0
    );
}

#[test]
fn fetch_parse_nonnegative_i64_accepts_numeric_string() {
    assert_eq!(
        fetch_parse_nonnegative_i64(Some(&json!("15"))),
        15
    );
    assert_eq!(
        fetch_parse_nonnegative_i64(Some(&json!("-3"))),
        0
    );
}

#[test]
fn search_retry_after_seconds_from_value_normalizes_epoch_and_clamps() {
    let now_epoch_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0);
    let retry_epoch_seconds = now_epoch_seconds.saturating_add(45);
    let normalized = search_retry_after_seconds_from_value(Some(&json!(retry_epoch_seconds)));
    assert!(normalized <= 45);
    assert!(normalized >= 0);

    let large_epoch_seconds = now_epoch_seconds.saturating_add(999_999);
    let clamped = search_retry_after_seconds_from_value(Some(&json!(large_epoch_seconds)));
    assert_eq!(clamped, 86_400);
}

#[test]
fn fetch_retry_after_seconds_from_value_normalizes_epoch_and_clamps() {
    let now_epoch_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0);
    let retry_epoch_seconds = now_epoch_seconds.saturating_add(30);
    let normalized = fetch_retry_after_seconds_from_value(Some(&json!(retry_epoch_seconds)));
    assert!(normalized <= 30);
    assert!(normalized >= 0);

    let large_epoch_seconds = now_epoch_seconds.saturating_add(999_999);
    let clamped = fetch_retry_after_seconds_from_value(Some(&json!(large_epoch_seconds)));
    assert_eq!(clamped, 86_400);
}

#[test]
fn search_query_shape_stats_reports_repetition_fields() {
    let stats = search_query_shape_stats("next round next round next round now");
    assert!(
        stats
            .get("repetition_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            > 0.5
    );
    assert_eq!(
        stats.get("dominant_term").and_then(Value::as_str),
        Some("next")
    );
}
