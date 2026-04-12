use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stock_market_collector_kernel_test_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn extract_quotes_reads_yahoo_style_payload() {
    let html = r#"<script>root.App.main = {"marketSummaryAndSparkResponse":{"result":[{"symbol":"^GSPC","shortName":"S&P 500","regularMarketPrice":5300.12,"regularMarketChange":-10.5,"regularMarketChangePercent":-0.21,"regularMarketVolume":12345}]}};</script>"#;
    let quotes = extract_quotes_from_html(html);
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].symbol, "^GSPC");
}

#[test]
fn map_quotes_dedupes_seen_ids() {
    let payload = json!({
        "date": "2026-03-27",
        "max_items": 10,
        "seen_ids": [],
        "quotes": [
            {"symbol":"AAPL","shortName":"Apple","price":100.12,"change":1.2,"changePercent":1.1,"volume":12000000},
            {"symbol":"AAPL","shortName":"Apple","price":100.12,"change":1.2,"changePercent":1.1,"volume":12000000}
        ]
    });
    let out = map_quotes(lane_utils::payload_obj(&payload));
    assert_eq!(
        out.get("items")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
}

#[test]
fn build_fetch_plan_has_market_request() {
    let out = command_build_fetch_plan(&Map::new());
    let reqs = out
        .get("requests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(reqs.len(), 1);
    assert_eq!(
        reqs.first()
            .and_then(Value::as_object)
            .and_then(|o| o.get("key"))
            .and_then(Value::as_str),
        Some("market_html")
    );
}

#[test]
fn prepare_run_skips_when_recent() {
    let root = temp_root();
    let payload = json!({ "min_hours": 100.0, "force": false });
    let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
    let _ = write_json_atomic(
        &meta_path,
        &json!({
            "collector_id": COLLECTOR_ID,
            "last_run": now_iso(),
            "last_success": now_iso(),
            "seen_ids": []
        }),
    );
    let out = command_prepare_run(&root, lane_utils::payload_obj(&payload));
    assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
}

#[test]
fn collect_returns_skip_payload_when_cadence_not_met() {
    let root = temp_root();
    let payload = json!({ "min_hours": 100.0, "force": false });
    let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
    let _ = write_json_atomic(
        &meta_path,
        &json!({
            "collector_id": COLLECTOR_ID,
            "last_run": now_iso(),
            "last_success": now_iso(),
            "seen_ids": []
        }),
    );
    let out = command_collect(&root, lane_utils::payload_obj(&payload)).expect("collect");
    assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn map_quotes_encodes_index_symbol_urls() {
    let payload = json!({
        "date": "2026-03-27",
        "max_items": 10,
        "seen_ids": [],
        "quotes": [
            {"symbol":"^GSPC","shortName":"S&P 500","price":5300.12,"change":-10.5,"changePercent":-0.21,"volume":12345}
        ]
    });
    let out = map_quotes(lane_utils::payload_obj(&payload));
    assert_eq!(
        out.get("items")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("url"))
            .and_then(Value::as_str),
        Some("https://finance.yahoo.com/quote/%5EGSPC")
    );
}
