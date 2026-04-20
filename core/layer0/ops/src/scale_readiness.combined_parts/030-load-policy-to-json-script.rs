
pub fn load_policy(root: &Path, policy_path: &Path) -> Policy {
    let base = default_policy(root);
    let raw = read_json(policy_path);

    let mut out = base.clone();
    if let Some(v) = raw.get("version").and_then(Value::as_str) {
        let c = clean(v, 24);
        if !c.is_empty() {
            out.version = c;
        }
    }
    out.enabled = raw
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(base.enabled);
    out.strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(base.strict_default);

    let items = raw
        .get("items")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut seen = std::collections::HashSet::new();
            rows.iter()
                .filter_map(|row| {
                    let id = normalize_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
                    if id.is_empty() || seen.contains(&id) {
                        return None;
                    }
                    seen.insert(id.clone());
                    let title = clean(row.get("title").and_then(Value::as_str).unwrap_or(&id), 260);
                    Some(ProgramItem {
                        id: id.clone(),
                        title: if title.is_empty() { id } else { title },
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| base.items.clone());
    out.items = items;

    out.stage_gates = raw
        .get("stage_gates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 20))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| base.stage_gates.clone());

    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    out.paths = Paths {
        state_path: resolve_path(
            root,
            paths.get("state_path"),
            "local/state/ops/scale_readiness_program/state.json",
        ),
        latest_path: resolve_path(
            root,
            paths.get("latest_path"),
            "local/state/ops/scale_readiness_program/latest.json",
        ),
        receipts_path: resolve_path(
            root,
            paths.get("receipts_path"),
            "local/state/ops/scale_readiness_program/receipts.jsonl",
        ),
        history_path: resolve_path(
            root,
            paths.get("history_path"),
            "local/state/ops/scale_readiness_program/history.jsonl",
        ),
        contract_dir: resolve_path(
            root,
            paths.get("contract_dir"),
            "client/runtime/config/scale_readiness",
        ),
        report_dir: resolve_path(
            root,
            paths.get("report_dir"),
            "local/state/ops/scale_readiness_program/reports",
        ),
    };

    let budgets = raw.get("budgets").cloned().unwrap_or(Value::Null);
    out.budgets = Budgets {
        max_cost_per_user_usd: budgets
            .get("max_cost_per_user_usd")
            .and_then(Value::as_f64)
            .unwrap_or(base.budgets.max_cost_per_user_usd),
        max_p95_latency_ms: clamp_int(
            budgets.get("max_p95_latency_ms").and_then(Value::as_i64),
            10,
            50_000,
            base.budgets.max_p95_latency_ms,
        ),
        max_p99_latency_ms: clamp_int(
            budgets.get("max_p99_latency_ms").and_then(Value::as_i64),
            10,
            50_000,
            base.budgets.max_p99_latency_ms,
        ),
        error_budget_pct: budgets
            .get("error_budget_pct")
            .and_then(Value::as_f64)
            .unwrap_or(base.budgets.error_budget_pct),
    };

    out.policy_path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };

    out
}

fn load_state(policy: &Policy) -> Value {
    let fallback = json!({
        "schema_id": "scale_readiness_program_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "last_run": Value::Null,
        "lane_receipts": {},
        "current_stage": "1k",
        "autoscaling_profile": Value::Null,
        "async_pipeline_profile": Value::Null,
        "partition_profile": Value::Null,
        "cache_profile": Value::Null,
        "region_profile": Value::Null,
        "release_profile": Value::Null,
        "sre_profile": Value::Null,
        "abuse_profile": Value::Null,
        "economics_profile": Value::Null
    });
    let raw = read_json(&policy.paths.state_path);
    if !raw.is_object() {
        return fallback;
    }
    let mut merged = fallback.as_object().cloned().unwrap_or_default();
    for (k, v) in raw.as_object().cloned().unwrap_or_default() {
        merged.insert(k, v);
    }
    Value::Object(merged)
}

fn save_state(policy: &Policy, state: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }
    let mut payload = state.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("updated_at".to_string(), Value::String(now_iso()));
    }
    write_json_atomic(&policy.paths.state_path, &payload)
}

fn write_contract(
    policy: &Policy,
    name: &str,
    payload: &Value,
    apply: bool,
    root: &Path,
) -> Result<String, String> {
    let abs = policy.paths.contract_dir.join(name);
    if apply {
        write_json_atomic(&abs, payload)?;
    }
    Ok(rel_path(root, &abs))
}

fn run_json_script(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let abs = root.join(script_rel);
    let out = Command::new("node")
        .arg(abs)
        .args(args)
        .current_dir(root)
        .output();

    let Ok(out) = out else {
        return json!({"ok": false, "status": 1, "payload": Value::Null, "stdout": "", "stderr": "spawn_failed"});
    };

    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = clean(String::from_utf8_lossy(&out.stderr), 600);

    let payload = serde_json::from_str::<Value>(&stdout)
        .ok()
        .or_else(|| {
            let idx = stdout.find('{')?;
            serde_json::from_str::<Value>(&stdout[idx..]).ok()
        })
        .unwrap_or(Value::Null);

    json!({
        "ok": status == 0,
        "status": status,
        "payload": payload,
        "stdout": stdout,
        "stderr": stderr
    })
}
