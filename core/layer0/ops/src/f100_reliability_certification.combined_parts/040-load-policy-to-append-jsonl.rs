
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
