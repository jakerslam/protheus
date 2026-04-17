// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use std::fs;
use std::path::Path;

fn assert_no_runtime_context_leak(raw: &str) {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    for marker in FORBIDDEN {
        assert!(
            !raw.contains(marker),
            "runtime payload leaked forbidden marker `{marker}`: {raw}"
        );
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
}

#[test]
fn fixed_microbenchmark_run_and_status_emit_receipts_and_persist_latest() {
    let root = tempfile::tempdir().expect("tempdir");
    let args = vec![
        "run".to_string(),
        "--rounds=3".to_string(),
        "--warmup-runs=0".to_string(),
        "--sample-ms=100".to_string(),
        "--work-factor=4".to_string(),
        "--workload-id=test-fixed-workload".to_string(),
    ];
    assert_eq!(
        protheus_ops_core::fixed_microbenchmark::run(root.path(), &args),
        0
    );

    let latest = root
        .path()
        .join("local/state/ops/fixed_microbenchmark/latest.json");
    let latest_json = read_json(&latest);
    assert_no_runtime_context_leak(&latest_json.to_string());
    assert_eq!(
        latest_json.get("type").and_then(Value::as_str),
        Some("fixed_microbenchmark")
    );
    assert_eq!(
        latest_json.get("workload_id").and_then(Value::as_str),
        Some("test-fixed-workload")
    );
    assert!(latest_json["metrics"]["ops_per_sec_p50"]
        .as_f64()
        .map(|value| value > 0.0)
        .unwrap_or(false));

    let status_args = vec!["status".to_string()];
    assert_eq!(
        protheus_ops_core::fixed_microbenchmark::run(root.path(), &status_args),
        0
    );
}
