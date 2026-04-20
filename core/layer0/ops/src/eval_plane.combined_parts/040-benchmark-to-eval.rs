
fn run_benchmark(root: &Path, _parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let loop_payload = read_json(&loop_latest_path(root));
    if strict && loop_payload.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "eval_plane_benchmark",
            "action": "benchmark",
            "errors": ["eval_loop_missing"]
        });
    }
    let loop_payload = loop_payload.unwrap_or_else(|| {
        json!({
            "baseline":{"cost_usd":20.0,"accuracy":0.92},
            "run":{"cost_usd":20.0,"accuracy":0.92},
            "trace":[]
        })
    });
    let baseline_cost = loop_payload
        .pointer("/baseline/cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or(20.0);
    let run_cost = loop_payload
        .pointer("/run/cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or(20.0);
    let baseline_accuracy = loop_payload
        .pointer("/baseline/accuracy")
        .and_then(Value::as_f64)
        .unwrap_or(0.92);
    let run_accuracy = loop_payload
        .pointer("/run/accuracy")
        .and_then(Value::as_f64)
        .unwrap_or(0.92);
    let cost_delta_pct = ((run_cost - baseline_cost) / baseline_cost.max(0.001)) * 100.0;
    let accuracy_delta_pct =
        ((run_accuracy - baseline_accuracy) / baseline_accuracy.max(0.0001)) * 100.0;
    let tradeoff_score = (0.6 * (-cost_delta_pct)) + (0.4 * accuracy_delta_pct);
    let points = loop_payload
        .get("trace")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|row| {
            json!({
                "iteration": row.get("iteration").and_then(Value::as_u64).unwrap_or(0),
                "cost_usd": row.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
                "accuracy": row.get("accuracy").and_then(Value::as_f64).unwrap_or(0.0),
                "reward": row.get("reward").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();
    let benchmark = json!({
        "version":"v1",
        "type":"eval_plane_benchmark_payload",
        "cost_accuracy_deltas": {

            "baseline_cost_usd": baseline_cost,
            "run_cost_usd": run_cost,
            "cost_delta_pct": cost_delta_pct,
            "baseline_accuracy": baseline_accuracy,
            "run_accuracy": run_accuracy,
            "accuracy_delta_pct": accuracy_delta_pct
        },
        "tradeoff_score": tradeoff_score,
        "points": points,
        "updated_at": crate::now_iso()
    });
    let _ = write_json(&benchmark_latest_path(root), &benchmark);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "eval_plane_benchmark",
        "lane": "core/layer0/ops",
        "action": "benchmark",
        "benchmark": benchmark,
        "artifact": {
            "path": benchmark_latest_path(root).display().to_string(),
            "sha256": sha256_hex_str(&read_json(&benchmark_latest_path(root)).unwrap_or(Value::Null).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.3",
                "claim": "benchmark_receipts_expose_cost_accuracy_deltas_for_visual_tradeoff_surfaces",
                "evidence": {
                    "cost_delta_pct": cost_delta_pct,
                    "accuracy_delta_pct": accuracy_delta_pct,
                    "tradeoff_score": tradeoff_score
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_dashboard(root: &Path, strict: bool) -> Value {
    let benchmark = read_json(&benchmark_latest_path(root));
    if strict && benchmark.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "eval_plane_dashboard",
            "action": "dashboard",
            "errors": ["eval_benchmark_missing"]
        });
    }
    let benchmark = benchmark.unwrap_or_else(|| {
        json!({
            "cost_accuracy_deltas": {
                "baseline_cost_usd": 0.0,
                "run_cost_usd": 0.0,
                "cost_delta_pct": 0.0,
                "baseline_accuracy": 0.0,
                "run_accuracy": 0.0,
                "accuracy_delta_pct": 0.0
            },
            "tradeoff_score": 0.0
        })
    });
    let cost_delta_pct = benchmark
        .pointer("/cost_accuracy_deltas/cost_delta_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let accuracy_delta_pct = benchmark
        .pointer("/cost_accuracy_deltas/accuracy_delta_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let tradeoff_score = benchmark
        .get("tradeoff_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "eval_plane_dashboard",
        "lane": "core/layer0/ops",
        "action": "dashboard",
        "dashboard": {
            "cost_accuracy_deltas": benchmark.get("cost_accuracy_deltas").cloned().unwrap_or(Value::Null),
            "tradeoff_score": tradeoff_score,
            "latest_paths": {
                "benchmark": benchmark_latest_path(root).display().to_string(),
                "loop": loop_latest_path(root).display().to_string(),
                "fixture": fixture_path(root).display().to_string()
            }
        },
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.5",
                "claim": "public_eval_dashboard_surfaces_cost_accuracy_tradeoff_metrics_from_receipted_benchmarks",
                "evidence": {
                    "cost_delta_pct": cost_delta_pct,
                    "accuracy_delta_pct": accuracy_delta_pct,
                    "tradeoff_score": tradeoff_score
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_eval(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let enable = run_enable(root, parsed, strict);
    if strict && !enable.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return enable;
    }
    let experiment = run_experiment(root, parsed, strict);
    if strict
        && !experiment
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return experiment;
    }
    let benchmark = run_benchmark(root, parsed, strict);
    let ok = benchmark
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "eval_plane_run",
        "lane": "core/layer0/ops",
        "action": "run",
        "stages": {
            "enable": enable,
            "experiment": experiment,
            "benchmark": benchmark
        },
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.1",
                "claim": "run_command_executes_build_experiment_evaluate_sequence_in_single_receipted_flow",
                "evidence": {"ok": ok}
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
