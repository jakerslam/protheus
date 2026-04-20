
fn compute_loop_trace(
    baseline_cost: f64,
    run_cost: f64,
    baseline_accuracy: f64,
    run_accuracy: f64,
    iterations: u64,
) -> Vec<Value> {
    let mut rows = Vec::<Value>::new();
    for idx in 0..iterations {
        let step = idx as f64;
        let sample_cost = (run_cost * (1.0 - 0.01 * step)).max(0.001);
        let sample_accuracy = (run_accuracy + (0.0005 * step)).min(1.0);
        let cost_gain_pct = ((baseline_cost - sample_cost) / baseline_cost.max(0.001)) * 100.0;
        let accuracy_drop_pp = (baseline_accuracy - sample_accuracy).max(0.0) * 100.0;
        let reward = cost_gain_pct - (accuracy_drop_pp * 2.0);
        rows.push(json!({
            "iteration": idx + 1,
            "cost_usd": sample_cost,
            "accuracy": sample_accuracy,
            "cost_gain_pct": cost_gain_pct,
            "accuracy_drop_pp": accuracy_drop_pp,
            "reward": reward
        }));
    }
    rows
}

fn run_experiment(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let config =
        read_json(&config_path(root)).unwrap_or_else(|| json!({"enabled_neuralavb": false}));
    let enabled = config
        .get("enabled_neuralavb")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if strict && !enabled {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "eval_plane_experiment_loop",
            "action": "experiment-loop",
            "errors": ["eval_neuralavb_not_enabled"]
        });
    }

    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version":"v1",
            "kind":"eval_loop_contract",
            "max_iterations": 32
        }),
    );
    let max_iterations = contract
        .get("max_iterations")
        .and_then(Value::as_u64)
        .unwrap_or(32)
        .max(1);
    let iterations = parse_u64(parsed.flags.get("iterations"), 4)
        .max(1)
        .min(max_iterations);

    let baseline_cost = parse_f64(parsed.flags.get("baseline-cost-usd"), 20.0).max(0.001);
    let run_cost = parse_f64(parsed.flags.get("run-cost-usd"), 8.0).max(0.001);
    let baseline_accuracy = parse_f64(parsed.flags.get("baseline-accuracy"), 0.92).clamp(0.0, 1.0);
    let run_accuracy = parse_f64(parsed.flags.get("run-accuracy"), 0.91).clamp(0.0, 1.0);

    let fixture = upsert_fixture(root, parsed);
    let trace = compute_loop_trace(
        baseline_cost,
        run_cost,
        baseline_accuracy,
        run_accuracy,
        iterations,
    );
    for row in &trace {
        let _ = append_jsonl(
            &trace_history_path(root),
            &json!({
                "ts": crate::now_iso(),
                "type": "eval_plane_trace",
                "row": row
            }),
        );
    }
    let reward_total = trace
        .iter()
        .map(|row| row.get("reward").and_then(Value::as_f64).unwrap_or(0.0))
        .sum::<f64>();
    let reward_avg = reward_total / (iterations as f64).max(1.0);
    let accepted = reward_avg >= 0.0;
    let loop_payload = json!({
        "version":"v1",
        "type":"eval_plane_loop_payload",
        "iterations": iterations,
        "baseline": {
            "cost_usd": baseline_cost,
            "accuracy": baseline_accuracy
        },
        "run": {
            "cost_usd": run_cost,
            "accuracy": run_accuracy
        },
        "trace": trace,
        "reward": {
            "total": reward_total,
            "average": reward_avg,
            "accepted": accepted
        },
        "fixture_path": fixture_path(root).display().to_string(),
        "updated_at": crate::now_iso()
    });
    let _ = write_json(&loop_latest_path(root), &loop_payload);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "eval_plane_experiment_loop",
        "lane": "core/layer0/ops",
        "action": "experiment-loop",
        "loop": loop_payload,
        "fixture": fixture,
        "artifact": {
            "loop_path": loop_latest_path(root).display().to_string(),
            "trace_path": trace_history_path(root).display().to_string(),
            "loop_sha256": sha256_hex_str(&read_json(&loop_latest_path(root)).unwrap_or(Value::Null).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-EVAL-001.1",
                "claim": "build_experiment_evaluate_loop_executes_as_core_runtime_sequence",
                "evidence": {"iterations": iterations}
            },
            {
                "id": "V6-EVAL-001.2",
                "claim": "ground_truth_fixture_and_rl_style_rewards_are_persisted_as_machine_usable_artifacts",
                "evidence": {
                    "fixture_path": fixture_path(root).display().to_string(),
                    "reward_avg": reward_avg
                }
            },
            {
                "id": "V6-EVAL-001.3",
                "claim": "cost_accuracy_tradeoff_metrics_are_emitted_for_observability_consumption",
                "evidence": {
                    "baseline_cost": baseline_cost,
                    "run_cost": run_cost,
                    "baseline_accuracy": baseline_accuracy,
                    "run_accuracy": run_accuracy
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
