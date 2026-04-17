) {
    let ts = payload
        .get("ts")
        .and_then(Value::as_str)
        .map(|v| v.to_string())
        .unwrap_or_else(now_iso);
    let row = match build_receipt_row(
        payload,
        "rust_memory_transition_receipt",
        "1.0",
        "receipt",
        &ts,
        claims,
    ) {
        Ok(v) => v,
        Err(_) => return,
    };
    let _ = write_json_atomic(&policy.paths.latest_path, &row);
    let _ = append_jsonl(&policy.paths.receipts_path, &row);
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read_benchmark_rows(policy: &TransitionPolicy, scope_id: &str) -> Vec<Value> {
    let history = read_json(&policy.paths.benchmark_path, json!({ "rows": [] }));
    let Some(rows) = history.get("rows").and_then(Value::as_array) else {
        return vec![];
    };
    rows.iter()
        .filter(|row| {
            row.get("policy_scope")
                .and_then(Value::as_str)
                .map(|scope| clean_text(scope, 80) == scope_id)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<Value>>()
}

fn number_or_zero(row: &Value, key: &str) -> f64 {
    row.get(key)
        .and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn parity_or_zero(row: &Value) -> i64 {
    row.get("parity_error_count")
        .and_then(|v| match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.parse::<i64>().ok(),
            _ => None,
        })
        .unwrap_or(0)
        .max(0)
}

fn evaluate_auto_selector(policy: &TransitionPolicy) -> AutoDecision {
    let scope_id = policy_scope_id(policy);
    let rows = read_benchmark_rows(policy, &scope_id);
    let min_rows = policy.thresholds.min_stable_runs_for_retirement;
    let start = rows.len().saturating_sub(min_rows);
    let recent = rows[start..].to_vec();
    let avg_speedup = if recent.is_empty() {
        0.0
    } else {
        recent
            .iter()
            .map(|row| number_or_zero(row, "speedup"))
            .sum::<f64>()
            / recent.len() as f64
    };
    let avg_speedup = (avg_speedup * 1_000_000.0).round() / 1_000_000.0;
    let max_parity_errors = recent.iter().map(parity_or_zero).max().unwrap_or(0).max(0);
    let eligible = recent.len() >= min_rows
        && avg_speedup >= policy.thresholds.min_speedup_for_cutover
        && max_parity_errors <= policy.thresholds.max_parity_error_count;
    let backend = if eligible {
        "rust_shadow".to_string()
    } else {
        "js".to_string()
    };
    let active_engine = if backend == "js" {
        "js".to_string()
    } else {
        "rust".to_string()
    };
    AutoDecision {
        backend,
        active_engine,
        eligible,
        stable_runs: recent.len(),
        avg_speedup,
        max_parity_errors,
        auto_reason: if eligible {
            "benchmark_gate_pass".to_string()
        } else {
            "benchmark_gate_fail".to_string()
        },
    }
}

fn persist_selector(
    policy: &TransitionPolicy,
    backend: &str,
    active_engine: &str,
    auto_reason: Option<&str>,
) {
    let mut selector = json!({
        "schema_version": "1.0",
        "backend": backend,
        "active_engine": active_engine,
        "fallback_backend": "js",
        "updated_at": now_iso()
    });
    if let Some(reason) = auto_reason {
        selector["auto_selected"] = Value::Bool(true);
        selector["auto_reason"] = Value::String(reason.to_string());
    }
    let _ = write_json_atomic(&policy.paths.selector_path, &selector);
}

fn set_selector(policy: &TransitionPolicy, backend_raw: &str) -> Value {
    let backend = normalize_token(backend_raw, 20);
    if !["js", "rust", "rust_shadow", "rust_live"].contains(&backend.as_str()) {
        return json!({
            "ok": false,
            "error": "invalid_backend",
            "backend": backend
        });
    }
    let active_engine = if backend == "js" { "js" } else { "rust" };
    persist_selector(policy, &backend, active_engine, None);
    let out = json!({
        "ts": now_iso(),
        "type": "rust_memory_backend_selector",
        "ok": true,
        "backend": backend,
        "active_engine": active_engine,
        "fallback_backend": "js"
    });
    write_transition_receipt(
        policy,
        &out,
        &transition_claims(
            "selector decision is deterministic and fail-safe",
            vec![
                format!("path:{}", policy.paths.selector_path.to_string_lossy()),
                format!("backend:{}", out["backend"].as_str().unwrap_or("")),
            ],
            vec!["migration_guard", "operator_safety"],
        ),
    );
    out
}

fn auto_selector(policy: &TransitionPolicy) -> Value {
    let decision = evaluate_auto_selector(policy);
    persist_selector(
        policy,
        &decision.backend,
        &decision.active_engine,
        Some(&decision.auto_reason),
    );
    let out = json!({
        "ts": now_iso(),
        "type": "rust_memory_auto_selector",
        "ok": true,
        "backend": decision.backend,
        "active_engine": decision.active_engine,
        "eligible": decision.eligible,
        "stable_runs": decision.stable_runs,
        "avg_speedup": decision.avg_speedup,
        "max_parity_errors": decision.max_parity_errors
    });
    write_transition_receipt(
        policy,
        &out,
        &transition_claims(
            "auto selector is benchmark-threshold gated",
            vec![
                format!("path:{}", policy.paths.benchmark_path.to_string_lossy()),
                format!("stable_runs:{}", out["stable_runs"].as_u64().unwrap_or(0)),
                format!("avg_speedup:{}", out["avg_speedup"].as_f64().unwrap_or(0.0)),
            ],
            vec!["migration_guard", "performance_governor"],
        ),
    );
    out
}

fn retire_check(policy: &TransitionPolicy) -> Value {
    let decision = evaluate_auto_selector(policy);
    let scope_id = policy_scope_id(policy);
    let out = json!({
        "ts": now_iso(),
        "type": "rust_memory_retire_check",
        "ok": true,
        "policy_scope": scope_id,
        "eligible_for_js_artifact_retirement": decision.eligible,
        "stable_runs": decision.stable_runs,
        "avg_speedup": decision.avg_speedup,
        "max_parity_errors": decision.max_parity_errors
    });
    write_transition_receipt(
        policy,
        &out,
        &transition_claims(
            "retire check preserves parity and speedup gates",
            vec![
                format!("path:{}", policy.paths.benchmark_path.to_string_lossy()),
                format!("max_parity_errors:{}", decision.max_parity_errors),
                format!("avg_speedup:{}", decision.avg_speedup),
            ],
            vec!["migration_guard", "constitution_safety"],
        ),
    );
    out
}

fn status(policy: &TransitionPolicy, root: &Path) -> Value {
    let latest = read_json(&policy.paths.latest_path, json!({}));
    let benchmark_latest = read_json(&policy.paths.benchmark_latest_path, json!({}));
    let selector = read_json(
        &policy.paths.selector_path,
        json!({
            "backend": "js",
            "active_engine": "js",
            "fallback_backend": "js"
        }),
    );
    json!({
        "ok": true,
        "type": "rust_memory_transition_status",
        "shadow_only": policy.shadow_only,
        "soak": policy.raw_soak.clone(),
        "latest": latest,
        "benchmark_latest": benchmark_latest,
        "selector": selector,
        "paths": {
            "latest_path": rel_path(root, &policy.paths.latest_path),
            "receipts_path": rel_path(root, &policy.paths.receipts_path),
            "selector_path": rel_path(root, &policy.paths.selector_path)
        }
    })
}

fn usage() {
    println!("rust_memory_transition_lane.js");
    println!("Usage:");
    println!("  rust_memory_transition_lane selector --backend=js|rust|rust_shadow|rust_live [--policy=<path>]");
    println!("  rust_memory_transition_lane auto-selector [--policy=<path>]");
    println!("  rust_memory_transition_lane pilot [--policy=<path>]");
    println!("  rust_memory_transition_lane benchmark [--policy=<path>]");
    println!("  rust_memory_transition_lane consistency-check [--policy=<path>]");
    println!("  rust_memory_transition_lane index-probe [--policy=<path>]");
    println!("  rust_memory_transition_lane retire-check [--policy=<path>]");
    println!("  rust_memory_transition_lane soak-gate [--policy=<path>]");
    println!("  rust_memory_transition_lane status [--policy=<path>]");
    println!("  other commands fall back to legacy bridge");
}

pub fn maybe_run(root: &Path, argv: &[String]) -> Option<i32> {
    let cmd = argv
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let kv = parse_kv_args(if argv.is_empty() { &[] } else { &argv[1..] });
    let policy_path = {
        let explicit = kv.get("policy").cloned().or_else(|| {
            std::env::var("RUST_MEMORY_TRANSITION_POLICY_PATH")
                .ok()
                .map(|v| v.trim().to_string())
        });
        if let Some(path) = explicit {
            let candidate = PathBuf::from(path);
            if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            }
        } else {
            root.join("client/runtime/config/rust_memory_transition_policy.json")
        }
    };
    let policy = load_policy(root, &policy_path);
    if !policy.enabled {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"ok": false, "error": "rust_memory_transition_disabled"})
            )
            .unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return Some(1);
    }

    let out = match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            return Some(0);
        }
        "selector" => set_selector(&policy, kv.get("backend").map(|v| v.as_str()).unwrap_or("")),
        "auto-selector" => auto_selector(&policy),
        "retire-check" => retire_check(&policy),
        "status" => status(&policy, root),
        _ => return None,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
    Some(0)
}

#[cfg(test)]
