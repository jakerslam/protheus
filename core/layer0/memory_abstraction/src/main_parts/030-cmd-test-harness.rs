fn cmd_test_harness(root: &Path, subcmd: &str) -> Value {
    let p = harness_policy(root);
    if p.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
        return json!({"ok": false, "error": "memory_abstraction_test_harness_disabled"});
    }
    let latest_path = PathBuf::from(p.get("latest_path").and_then(|v| v.as_str()).unwrap_or(""));
    let receipts_path = PathBuf::from(
        p.get("receipts_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let baseline_path = PathBuf::from(
        p.get("baseline_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let drift_fail = p
        .get("drift_fail_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);

    match subcmd {
        "run" => {
            let now_ms = Utc::now().timestamp_millis();
            let token = format!("harness_token_{now_ms}");
            let id = format!("memory://{token}");
            let security_req = json!({
              "operation_id": format!("memory_harness_probe_{now_ms}"),
              "subsystem": "memory",
              "action": "harness",
              "actor": "client/runtime/systems/memory/abstraction/test_harness",
              "risk_class": "high",
              "tags": ["memory", "test_harness", "foundation_lock"],
              "audit_receipt_nonce": format!("nonce-{token}"),
              "zk_proof": format!("zk-{token}"),
              "ciphertext_digest": format!("sha256:{token}")
            });
            let security_probe = run_security_check(root, &security_req);

            let ingest_run = run_memory_core(
                root,
                &[
                    "ingest".to_string(),
                    format!("--id={id}"),
                    format!("--content=foundation lock harness sample memory row {token}"),
                    format!("--tags=foundation_lock,memory_harness,{token}"),
                    "--repetitions=2".to_string(),
                    "--lambda=0.02".to_string(),
                ],
            );
            let recall_run = run_memory_core(
                root,
                &[
                    "recall".to_string(),
                    format!("--query={token}"),
                    "--limit=5".to_string(),
                ],
            );
            let get_run = run_memory_core(root, &["get".to_string(), format!("--id={id}")]);
            let compress_run = run_memory_core(
                root,
                &["compress".to_string(), "--aggressive=0".to_string()],
            );
            let ebb_run = run_memory_core(
                root,
                &[
                    "ebbinghaus-score".to_string(),
                    "--age-days=1.5".to_string(),
                    "--repetitions=2".to_string(),
                    "--lambda=0.02".to_string(),
                ],
            );
            let crdt_run = run_memory_core(
                root,
                &[String::from("crdt-exchange"), String::from("--payload={\"left\":{\"topic\":{\"value\":\"alpha\",\"clock\":1,\"node\":\"left\"}},\"right\":{\"topic\":{\"value\":\"beta\",\"clock\":2,\"node\":\"right\"}}}")],
            );

            let recall_payload = &recall_run.payload;
            let get_payload = &get_run.payload;
            let compress_payload = &compress_run.payload;
            let ebb_payload = &ebb_run.payload;
            let crdt_payload = &crdt_run.payload;

            let metrics = json!({
              "recall_hit_count": recall_payload.get("hit_count").and_then(|v| v.as_u64()).unwrap_or(0),
              "get_ok": if get_payload.get("ok").and_then(|v| v.as_bool()) == Some(true) { 1 } else { 0 },
              "compacted_rows": compress_payload.get("compacted_rows").and_then(|v| v.as_u64()).unwrap_or(0),
              "retention_score": ebb_payload.get("retention_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
              "crdt_topic_clock": crdt_payload.get("merged").and_then(|v| v.get("topic")).and_then(|v| v.get("clock")).and_then(|v| v.as_u64()).unwrap_or(0),
              "security_gate_ok": if security_probe.ok { 1 } else { 0 }
            });

            let baseline_raw = read_json(&baseline_path);
            let baseline = if baseline_raw.is_object() {
                baseline_raw
            } else {
                json!({"ts": now_iso(), "metrics": metrics})
            };
            let bm = baseline.get("metrics").cloned().unwrap_or(json!({}));
            let d_recall = compute_drift_pct(
                metrics
                    .get("recall_hit_count")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("recall_hit_count")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(
                        metrics
                            .get("recall_hit_count")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                    ),
            );
            let d_get = compute_drift_pct(
                metrics
                    .get("get_ok")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("get_ok").and_then(|v| v.as_f64()).unwrap_or(
                    metrics
                        .get("get_ok")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                ),
            );
            let d_compact = compute_drift_pct(
                metrics
                    .get("compacted_rows")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("compacted_rows").and_then(|v| v.as_f64()).unwrap_or(
                    metrics
                        .get("compacted_rows")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                ),
            );
            let d_retention = compute_drift_pct(
                metrics
                    .get("retention_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("retention_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(
                        metrics
                            .get("retention_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                    ),
            );
            let d_crdt = compute_drift_pct(
                metrics
                    .get("crdt_topic_clock")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("crdt_topic_clock")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(
                        metrics
                            .get("crdt_topic_clock")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                    ),
            );
            let d_sec = compute_drift_pct(
                metrics
                    .get("security_gate_ok")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                bm.get("security_gate_ok")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(
                        metrics
                            .get("security_gate_ok")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                    ),
            );
            let max_drift = d_recall
                .max(d_get)
                .max(d_compact)
                .max(d_retention)
                .max(d_crdt)
                .max(d_sec);
            let ok = ingest_run.ok
                && recall_run.ok
                && get_run.ok
                && compress_run.ok
                && ebb_run.ok
                && crdt_run.ok
                && security_probe.ok
                && max_drift <= drift_fail;
            let receipt = json!({
              "ts": now_iso(),
              "type": "memory_abstraction_test_harness_run",
              "ok": ok,
              "backend": "rust_core_v6",
              "metrics": metrics,
              "max_drift_pct": max_drift,
              "drift_fail_pct": drift_fail,
              "drift_breakdown": {
                "recall_hit_count_pct": d_recall,
                "get_ok_pct": d_get,
                "compacted_rows_pct": d_compact,
                "retention_score_pct": d_retention,
                "crdt_topic_clock_pct": d_crdt,
                "security_gate_ok_pct": d_sec
              },
              "operations": {
                "ingest_ok": ingest_run.ok,
                "recall_ok": recall_run.ok,
                "get_ok": get_run.ok,
                "compress_ok": compress_run.ok,
                "ebbinghaus_ok": ebb_run.ok,
                "crdt_ok": crdt_run.ok,
                "security_gate_ok": security_probe.ok
              },
              "security_probe": security_probe.payload,
              "error": if ok { Value::Null } else { Value::String("memory_abstraction_harness_failed_or_drift_over_2pct".to_string()) }
            });
            write_json_atomic(&latest_path, &receipt);
            append_jsonl(&receipts_path, &receipt);
            receipt
        }
        "baseline-capture" => {
            let latest = read_json(&latest_path);
            if !latest.is_object() || latest.get("metrics").is_none() {
                return json!({"ok": false, "error": "test_harness_latest_missing"});
            }
            let baseline = json!({
              "ts": now_iso(),
              "metrics": latest.get("metrics").cloned().unwrap_or(Value::Null)
            });
            write_json_atomic(&baseline_path, &baseline);
            json!({"ok": true, "type": "memory_abstraction_test_harness_baseline_capture", "baseline": baseline})
        }
        "status" => json!({
          "ok": true,
          "type": "memory_abstraction_test_harness_status",
          "latest": read_json(&latest_path),
          "baseline": read_json(&baseline_path)
        }),
        _ => json!({"ok": false, "error": "unsupported_command", "cmd": subcmd}),
    }
}

fn usage() {
    println!("Usage:");
    println!("  memory_abstraction_core memory-view <query|get|snapshot|status> [--flags]");
    println!("  memory_abstraction_core analytics <run|baseline-capture|status>");
    println!("  memory_abstraction_core test-harness <run|baseline-capture|status>");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        std::process::exit(1);
    }
    let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let area = clean_text(&args[0], 80).to_lowercase();
    let subcmd = if args.len() > 1 {
        clean_text(&args[1], 80).to_lowercase()
    } else {
        "status".to_string()
    };
    let flags = parse_flags(&args);
    let payload = match area.as_str() {
        "memory-view" => cmd_memory_view(&root, &subcmd, &flags),
        "analytics" => cmd_analytics(&root, &subcmd),
        "test-harness" => cmd_test_harness(&root, &subcmd),
        "help" | "--help" | "-h" => {
            usage();
            json!({"ok": true})
        }
        _ => json!({"ok": false, "error": "unsupported_area", "area": area}),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
    let exit_ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    std::process::exit(if exit_ok { 0 } else { 1 });
}

