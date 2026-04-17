fn bounded_policy_pct(raw: Option<&Value>, default: f64) -> f64 {
    let value = raw.and_then(|v| v.as_f64()).unwrap_or(default);
    if !value.is_finite() {
        return default.clamp(0.0, 100.0);
    }
    value.clamp(0.0, 100.0)
}

fn normalize_drift_thresholds(warn: f64, fail: f64) -> (f64, f64) {
    let normalized_fail = if fail.is_finite() {
        fail.clamp(0.0, 100.0)
    } else {
        2.0
    };
    let normalized_warn = if warn.is_finite() {
        warn.clamp(0.0, normalized_fail)
    } else {
        normalized_fail.min(1.0)
    };
    (normalized_warn, normalized_fail)
}

fn analytics_policy(root: &Path) -> Value {
    let policy_path = env::var("MEMORY_ABSTRACTION_ANALYTICS_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            root.join("client/runtime/config/memory_abstraction_analytics_policy.json")
        });
    let raw = read_json(&policy_path);
    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    let (drift_warn_pct, drift_fail_pct) = normalize_drift_thresholds(
        bounded_policy_pct(raw.get("drift_warn_pct"), 1.0),
        bounded_policy_pct(raw.get("drift_fail_pct"), 2.0),
    );
    json!({
      "enabled": raw.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
      "drift_warn_pct": drift_warn_pct,
      "drift_fail_pct": drift_fail_pct,
      "latest_path": resolve_path(root, paths.get("latest_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/analytics_latest.json"),
      "history_path": resolve_path(root, paths.get("history_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/analytics_history.jsonl"),
      "baseline_path": resolve_path(root, paths.get("baseline_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/analytics_baseline.json"),
      "view_receipts_path": resolve_path(root, paths.get("view_receipts_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/memory_view_receipts.jsonl"),
      "harness_receipts_path": resolve_path(root, paths.get("harness_receipts_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/test_harness_receipts.jsonl"),
      "security_alerts_path": resolve_path(root, paths.get("security_alerts_path").and_then(|v| v.as_str()), "local/state/security/human_alerts.jsonl")
    })
}

fn compute_drift_pct(curr: f64, baseline: f64) -> f64 {
    if !curr.is_finite() || !baseline.is_finite() {
        return 0.0;
    }
    if baseline == 0.0 {
        return if curr == 0.0 { 0.0 } else { 100.0 };
    }
    ((curr - baseline).abs() / baseline * 100.0 * 1_000_000.0).round() / 1_000_000.0
}

fn cmd_analytics(root: &Path, subcmd: &str) -> Value {
    let p = analytics_policy(root);
    if p.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
        return json!({"ok": false, "error": "memory_abstraction_analytics_disabled"});
    }
    let latest_path = PathBuf::from(p.get("latest_path").and_then(|v| v.as_str()).unwrap_or(""));
    let history_path = PathBuf::from(p.get("history_path").and_then(|v| v.as_str()).unwrap_or(""));
    let baseline_path = PathBuf::from(
        p.get("baseline_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let view_receipts_path = PathBuf::from(
        p.get("view_receipts_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let harness_receipts_path = PathBuf::from(
        p.get("harness_receipts_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let security_alerts_path = PathBuf::from(
        p.get("security_alerts_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let (drift_warn, drift_fail) = normalize_drift_thresholds(
        p.get("drift_warn_pct").and_then(|v| v.as_f64()).unwrap_or(1.0),
        p.get("drift_fail_pct").and_then(|v| v.as_f64()).unwrap_or(2.0),
    );

    match subcmd {
        "run" => {
            let view_receipts = read_jsonl(&view_receipts_path);
            let harness_receipts = read_jsonl(&harness_receipts_path);
            let security_alerts = read_jsonl(&security_alerts_path);
            let mut hits: Vec<Value> = Vec::new();
            for row in &view_receipts {
                if row.get("type").and_then(|v| v.as_str()) == Some("memory_view_query") {
                    if let Some(arr) = row.get("hits").and_then(|v| v.as_array()) {
                        hits.extend(arr.iter().cloned());
                    }
                }
            }
            let hit_count = hits.len() as f64;
            let mut matching_hits = 0.0;
            let mut ratios = Vec::new();
            for hit in &hits {
                let content = clean_text(
                    hit.get("content").and_then(|v| v.as_str()).unwrap_or(""),
                    2000,
                )
                .to_lowercase();
                let id = clean_text(hit.get("id").and_then(|v| v.as_str()).unwrap_or(""), 200)
                    .to_lowercase();
                let query =
                    clean_text(hit.get("query").and_then(|v| v.as_str()).unwrap_or(""), 200)
                        .to_lowercase();
                if query.is_empty() || content.contains(&query) || id.contains(&query) {
                    matching_hits += 1.0;
                }
                if let Some(r) = hit.get("compression_ratio").and_then(|v| v.as_f64()) {
                    if r.is_finite() && r >= 0.0 {
                        ratios.push(r);
                    }
                }
            }
            let recall_accuracy = if hit_count > 0.0 {
                (matching_hits / hit_count * 1_000_000.0).round() / 1_000_000.0
            } else {
                1.0
            };
            let compression_ratio = if ratios.is_empty() {
                0.0
            } else {
                (ratios.iter().sum::<f64>() / ratios.len() as f64 * 1_000_000.0).round()
                    / 1_000_000.0
            };
            let drift_pct = harness_receipts
                .last()
                .and_then(|v| v.get("max_drift_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let obs_run =
                run_memory_core(root, &[String::from("load-embedded-observability-profile")]);
            let scorer = obs_run
                .payload
                .get("embedded_observability_profile")
                .and_then(|v| v.get("sovereignty_scorer"))
                .cloned()
                .unwrap_or(Value::Null);
            let iw = scorer
                .get("integrity_weight_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(45.0);
            let cw = scorer
                .get("continuity_weight_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(25.0);
            let rw = scorer
                .get("reliability_weight_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(20.0);
            let cp = scorer
                .get("chaos_penalty_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(10.0);
            let alert_penalty = (security_alerts.len() as f64 * cp).min(100.0);
            let integrity = recall_accuracy * 100.0;
            let continuity = (100.0 - drift_pct).max(0.0);
            let reliability = (100.0
                - if compression_ratio > 1.0 {
                    (compression_ratio - 1.0) * 100.0
                } else {
                    0.0
                })
            .max(0.0);
            let weighted = (integrity * iw + continuity * cw + reliability * rw) / 100.0;
            let sovereignty_index = (weighted - alert_penalty).max(0.0);
            let sovereignty_index = (sovereignty_index * 1_000_000.0).round() / 1_000_000.0;

            let baseline_raw = read_json(&baseline_path);
            let baseline = if baseline_raw.is_object() {
                baseline_raw
            } else {
                json!({
                  "recall_accuracy": recall_accuracy,
                  "compression_ratio": compression_ratio,
                  "sovereignty_index": sovereignty_index,
                  "drift_pct": drift_pct
                })
            };
            let d_recall = compute_drift_pct(
                recall_accuracy,
                baseline
                    .get("recall_accuracy")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(recall_accuracy),
            );
            let d_compression = compute_drift_pct(
                compression_ratio,
                baseline
                    .get("compression_ratio")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(compression_ratio),
            );
            let d_sovereignty = compute_drift_pct(
                sovereignty_index,
                baseline
                    .get("sovereignty_index")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(sovereignty_index),
            );
            let d_drift = compute_drift_pct(
                drift_pct,
                baseline
                    .get("drift_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(drift_pct),
            );
            let max_drift = d_recall.max(d_compression).max(d_sovereignty).max(d_drift);
            let drift_status = if max_drift > drift_fail {
                "fail"
            } else if max_drift > drift_warn {
                "warn"
            } else {
                "ok"
            };
            let receipt = json!({
              "ts": now_iso(),
              "type": "memory_analytics_run",
              "ok": drift_status != "fail",
              "backend": "rust_core_v6",
              "metrics": {
                "drift_pct": drift_pct,
                "recall_accuracy": recall_accuracy,
                "compression_ratio": compression_ratio,
                "sovereignty_index": sovereignty_index,
                "security_alert_count": security_alerts.len()
              },
              "blob_powered": {
                "observability_profile_loaded": obs_run.ok,
                "sovereignty_scorer": scorer
              },
              "drift": {
                "status": drift_status,
                "max_drift_pct": max_drift,
                "threshold_fail_pct": drift_fail,
                "threshold_warn_pct": drift_warn,
                "breakdown": {
                  "recall_accuracy_drift_pct": d_recall,
                  "compression_ratio_drift_pct": d_compression,
                  "sovereignty_index_drift_pct": d_sovereignty,
                  "drift_pct_drift_pct": d_drift
                }
              },
              "baseline": baseline
            });
            write_json_atomic(&latest_path, &receipt);
            append_jsonl(&history_path, &receipt);
            receipt
        }
        "baseline-capture" => {
            let latest = read_json(&latest_path);
            if !latest.is_object() || latest.get("metrics").is_none() {
                return json!({"ok": false, "error": "analytics_latest_missing"});
            }
            let metrics = latest.get("metrics").cloned().unwrap_or(Value::Null);
            let baseline = json!({
              "ts": now_iso(),
              "recall_accuracy": metrics.get("recall_accuracy").and_then(|v| v.as_f64()).unwrap_or(1.0),
              "compression_ratio": metrics.get("compression_ratio").and_then(|v| v.as_f64()).unwrap_or(0.0),
              "sovereignty_index": metrics.get("sovereignty_index").and_then(|v| v.as_f64()).unwrap_or(0.0),
              "drift_pct": metrics.get("drift_pct").and_then(|v| v.as_f64()).unwrap_or(0.0)
            });
            write_json_atomic(&baseline_path, &baseline);
            json!({"ok": true, "type": "memory_analytics_baseline_capture", "baseline": baseline})
        }
        "status" => json!({
          "ok": true,
          "type": "memory_analytics_status",
          "latest": read_json(&latest_path),
          "baseline": read_json(&baseline_path)
        }),
        _ => json!({"ok": false, "error": "unsupported_command", "cmd": subcmd}),
    }
}

fn harness_policy(root: &Path) -> Value {
    let policy_path = env::var("MEMORY_ABSTRACTION_TEST_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            root.join("client/runtime/config/memory_abstraction_test_harness_policy.json")
        });
    let raw = read_json(&policy_path);
    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    json!({
      "enabled": raw.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
      "drift_fail_pct": raw.get("drift_fail_pct").and_then(|v| v.as_f64()).unwrap_or(2.0).max(0.0),
      "latest_path": resolve_path(root, paths.get("latest_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/test_harness_latest.json"),
      "receipts_path": resolve_path(root, paths.get("receipts_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/test_harness_receipts.jsonl"),
      "baseline_path": resolve_path(root, paths.get("baseline_path").and_then(|v| v.as_str()), "local/state/client/memory/abstraction/test_harness_baseline.json")
    })
}
