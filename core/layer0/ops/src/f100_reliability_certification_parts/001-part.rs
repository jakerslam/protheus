fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let policy_path = policy_override
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));

    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));

    let strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let active_tier = raw
        .get("active_tier")
        .and_then(Value::as_str)
        .unwrap_or("seed")
        .trim()
        .to_ascii_lowercase();
    let window_days = raw
        .get("window_days")
        .and_then(Value::as_i64)
        .unwrap_or(30)
        .clamp(7, 120);
    let missing_metric_fail_closed = raw
        .get("missing_metric_fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let default_seed = Tier {
        min_uptime: 0.90,
        max_receipt_p95_ms: 200.0,
        max_receipt_p99_ms: 300.0,
        max_incident_rate: 0.35,
        max_change_fail_rate: 0.50,
        max_error_budget_burn_ratio: 0.45,
    };
    let default_production = Tier {
        min_uptime: 0.999,
        max_receipt_p95_ms: 100.0,
        max_receipt_p99_ms: 150.0,
        max_incident_rate: 0.10,
        max_change_fail_rate: 0.20,
        max_error_budget_burn_ratio: 0.30,
    };

    let mut tiers = BTreeMap::<String, Tier>::new();
    tiers.insert("seed".to_string(), default_seed.clone());
    tiers.insert("production".to_string(), default_production.clone());

    if let Some(obj) = raw.get("tiers").and_then(Value::as_object) {
        for (name, v) in obj {
            let tier_obj = v.as_object();
            let tier = Tier {
                min_uptime: value_as_f64(tier_obj.and_then(|m| m.get("min_uptime")))
                    .unwrap_or(default_seed.min_uptime)
                    .clamp(0.0, 1.0),
                max_receipt_p95_ms: value_as_f64(
                    tier_obj.and_then(|m| m.get("max_receipt_p95_ms")),
                )
                .unwrap_or(default_seed.max_receipt_p95_ms)
                .max(1.0),
                max_receipt_p99_ms: value_as_f64(
                    tier_obj.and_then(|m| m.get("max_receipt_p99_ms")),
                )
                .unwrap_or(default_seed.max_receipt_p99_ms)
                .max(1.0),
                max_incident_rate: value_as_f64(tier_obj.and_then(|m| m.get("max_incident_rate")))
                    .unwrap_or(default_seed.max_incident_rate)
                    .clamp(0.0, 10.0),
                max_change_fail_rate: value_as_f64(
                    tier_obj.and_then(|m| m.get("max_change_fail_rate")),
                )
                .unwrap_or(default_seed.max_change_fail_rate)
                .clamp(0.0, 1.0),
                max_error_budget_burn_ratio: value_as_f64(
                    tier_obj.and_then(|m| m.get("max_error_budget_burn_ratio")),
                )
                .unwrap_or(default_seed.max_error_budget_burn_ratio)
                .clamp(0.0, 10.0),
            };
            tiers.insert(name.trim().to_ascii_lowercase(), tier);
        }
    }

    let sources = raw.get("sources").and_then(Value::as_object);
    let outputs = raw.get("outputs").and_then(Value::as_object);

    let drill_evidence_paths = sources
        .and_then(|s| s.get("drill_evidence_paths"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|p| resolve_path(root, Some(p), ""))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                root.join("local/state/ops/dr_gameday_gate_receipts.jsonl"),
                root.join("local/state/ops/continuous_chaos_resilience/latest.json"),
            ]
        });

    let rollback_evidence_paths = sources
        .and_then(|s| s.get("rollback_evidence_paths"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|p| resolve_path(root, Some(p), ""))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                root.join("local/state/ops/release_gate_canary_rollback_enforcer/latest.json"),
                root.join("local/state/ops/error_budget_release_gate/freeze_state.json"),
            ]
        });

    Policy {
        strict_default,
        active_tier,
        tiers,
        window_days,
        missing_metric_fail_closed,
        sources_execution_reliability_path: resolve_path(
            root,
            sources
                .and_then(|s| s.get("execution_reliability_path"))
                .and_then(Value::as_str),
            "local/state/ops/execution_reliability_slo.json",
        ),
        sources_error_budget_latest_path: resolve_path(
            root,
            sources
                .and_then(|s| s.get("error_budget_latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/error_budget_release_gate/latest.json",
        ),
        sources_error_budget_history_path: resolve_path(
            root,
            sources
                .and_then(|s| s.get("error_budget_history_path"))
                .and_then(Value::as_str),
            "local/state/ops/error_budget_release_gate/history.jsonl",
        ),
        sources_spine_runs_dir: resolve_path(
            root,
            sources
                .and_then(|s| s.get("spine_runs_dir"))
                .and_then(Value::as_str),
            "local/state/spine/runs",
        ),
        sources_incident_log_path: resolve_path(
            root,
            sources
                .and_then(|s| s.get("incident_log_path"))
                .and_then(Value::as_str),
            "local/state/security/autonomy_human_escalations.jsonl",
        ),
        drill_evidence_paths,
        rollback_evidence_paths,
        min_drill_evidence_count: sources
            .and_then(|s| s.get("min_drill_evidence_count"))
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize,
        min_rollback_evidence_count: sources
            .and_then(|s| s.get("min_rollback_evidence_count"))
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize,
        latest_path: resolve_path(
            root,
            outputs
                .and_then(|s| s.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/f100_reliability_certification/latest.json",
        ),
        history_path: resolve_path(
            root,
            outputs
                .and_then(|s| s.get("history_path"))
                .and_then(Value::as_str),
            "local/state/ops/f100_reliability_certification/history.jsonl",
        ),
        policy_path,
    }
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text).map_err(|e| format!("write_tmp_failed:{}:{e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, value)
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn evaluate(policy: &Policy) -> Result<Value, String> {
    let tier = policy
        .tiers
        .get(&policy.active_tier)
        .cloned()
        .or_else(|| policy.tiers.get("seed").cloned())
        .ok_or_else(|| "missing_slo_tier".to_string())?;

    let reliability =
        read_json(&policy.sources_execution_reliability_path).unwrap_or_else(|_| json!({}));
    let uptime = value_as_f64(
        reliability
            .get("measured")
            .and_then(|v| v.get("execution_success_rate")),
    );

    let error_budget =
        read_json(&policy.sources_error_budget_latest_path).unwrap_or_else(|_| json!({}));
    let burn_ratio =
        value_as_f64(error_budget.get("gate").and_then(|v| v.get("burn_ratio"))).unwrap_or(0.0);

    let (p95, p99, latency_samples, latency_files_scanned) =
        collect_spine_latency_metrics(&policy.sources_spine_runs_dir);

    let today = Utc::now().date_naive();
    let window_start = today - Duration::days(policy.window_days.saturating_sub(1));
    let (incident_rate, incident_count) =
        collect_incident_rate(&policy.sources_incident_log_path, window_start, today);
    let (change_fail_rate, change_window_total, change_window_failed) = collect_change_fail_rate(
        &policy.sources_error_budget_history_path,
        window_start,
        today,
    );

    let drill = evidence_status(
        &policy.drill_evidence_paths,
        policy.min_drill_evidence_count,
    );
    let rollback = evidence_status(
        &policy.rollback_evidence_paths,
        policy.min_rollback_evidence_count,
    );

    let mut checks = BTreeMap::<String, Value>::new();

    let uptime_ok = uptime
        .map(|v| v >= tier.min_uptime)
        .unwrap_or(!policy.missing_metric_fail_closed);
    checks.insert(
        "uptime".to_string(),
        json!({
            "ok": uptime_ok,
            "value": uptime,
            "target_min": tier.min_uptime,
            "source": policy.sources_execution_reliability_path
        }),
    );

    let p95_ok = p95
        .map(|v| v <= tier.max_receipt_p95_ms)
        .unwrap_or(!policy.missing_metric_fail_closed);
    checks.insert(
        "receipt_latency_p95_ms".to_string(),
        json!({
            "ok": p95_ok,
            "value": p95,
            "target_max": tier.max_receipt_p95_ms,
            "samples": latency_samples,
            "files_scanned": latency_files_scanned,
            "source": policy.sources_spine_runs_dir
        }),
    );

    let p99_ok = p99
        .map(|v| v <= tier.max_receipt_p99_ms)
        .unwrap_or(!policy.missing_metric_fail_closed);
    checks.insert(
        "receipt_latency_p99_ms".to_string(),
        json!({
            "ok": p99_ok,
            "value": p99,
            "target_max": tier.max_receipt_p99_ms,
            "samples": latency_samples,
            "files_scanned": latency_files_scanned,
            "source": policy.sources_spine_runs_dir
        }),
    );

    let incident_ok = incident_rate <= tier.max_incident_rate;
    checks.insert(
        "incident_rate".to_string(),
        json!({
            "ok": incident_ok,
            "value": incident_rate,
            "target_max": tier.max_incident_rate,
            "incidents": incident_count,
            "window_days": policy.window_days,
            "source": policy.sources_incident_log_path
        }),
    );

    let change_fail_ok = change_fail_rate <= tier.max_change_fail_rate;
    checks.insert(
        "change_fail_rate".to_string(),
        json!({
            "ok": change_fail_ok,
            "value": change_fail_rate,
            "target_max": tier.max_change_fail_rate,
            "window_total": change_window_total,
            "window_failed": change_window_failed,
            "window_days": policy.window_days,
            "source": policy.sources_error_budget_history_path
        }),
    );

    let burn_ok = burn_ratio <= tier.max_error_budget_burn_ratio;
    checks.insert(
        "error_budget_burn_ratio".to_string(),
        json!({
            "ok": burn_ok,
            "value": burn_ratio,
            "target_max": tier.max_error_budget_burn_ratio,
            "source": policy.sources_error_budget_latest_path
        }),
    );

    checks.insert("drill_evidence".to_string(), drill.clone());
    checks.insert("rollback_evidence".to_string(), rollback.clone());

    let blocking_checks = checks
        .iter()
        .filter_map(|(k, v)| {
            if v.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                Some(k.clone())
            }
        })
        .collect::<Vec<_>>();

    let ok = blocking_checks.is_empty();

    Ok(json!({
        "ok": ok,
        "schema_id": "f100_reliability_certification",
        "schema_version": "1.0",
        "ts": now_iso(),
        "tier": policy.active_tier,
        "window": {
            "start": window_start.to_string(),
            "end": today.to_string(),
            "window_days": policy.window_days
        },
        "checks": checks,
        "blocking_checks": blocking_checks,
        "monthly_scorecard": {
            "uptime": checks.get("uptime").cloned().unwrap_or(Value::Null),
            "receipt_latency_p95_ms": checks.get("receipt_latency_p95_ms").cloned().unwrap_or(Value::Null),
            "receipt_latency_p99_ms": checks.get("receipt_latency_p99_ms").cloned().unwrap_or(Value::Null),
            "incident_rate": checks.get("incident_rate").cloned().unwrap_or(Value::Null),
            "change_fail_rate": checks.get("change_fail_rate").cloned().unwrap_or(Value::Null)
        },
        "release_gate": {
            "burn_ratio": burn_ratio,
            "target_max": tier.max_error_budget_burn_ratio,
            "promotion_blocked": !burn_ok,
            "source": policy.sources_error_budget_latest_path
        },
        "drill_evidence": drill,
        "rollback_evidence": rollback,
        "sources": {
            "execution_reliability_path": policy.sources_execution_reliability_path,
            "error_budget_latest_path": policy.sources_error_budget_latest_path,
            "error_budget_history_path": policy.sources_error_budget_history_path,
            "spine_runs_dir": policy.sources_spine_runs_dir,
            "incident_log_path": policy.sources_incident_log_path
        },
        "claim_evidence": [
            {
                "id": "f100_reliability_error_budget_gate",
                "claim": "release_gate_blocks_when_error_budget_burn_exceeds_policy",
                "evidence": {
                    "burn_ratio": burn_ratio,
                    "max_error_budget_burn_ratio": tier.max_error_budget_burn_ratio,
                    "promotion_blocked": !burn_ok
                }
            },
            {
                "id": "f100_monthly_reliability_scorecard",
                "claim": "monthly_scorecard_emits_uptime_latency_incident_and_change_fail_metrics_with_drill_and_rollback_evidence",
                "evidence": {
                    "metrics": ["uptime", "receipt_latency_p95_ms", "receipt_latency_p99_ms", "incident_rate", "change_fail_rate"],
                    "drill_evidence_ok": drill.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    "rollback_evidence_ok": rollback.get("ok").and_then(Value::as_bool).unwrap_or(false)
                }
            }
        ]
    }))
}
