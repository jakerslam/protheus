fn cron_runtime_jobs_path(control_runtime_root: &Path) -> PathBuf {
    control_runtime_root.join("cron/jobs.json")
}

fn cron_workspace_mirror_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("config/infring_assimilation/cron/jobs.json")
}

fn run_cron_drift(control_runtime_root: &Path, workspace_root: &Path) -> Value {
    let runtime = cron_runtime_jobs_path(control_runtime_root);
    let mirror = cron_workspace_mirror_path(workspace_root);
    let runtime_raw = fs::read_to_string(&runtime).unwrap_or_default();
    let mirror_raw = fs::read_to_string(&mirror).unwrap_or_default();
    let runtime_json = parse_json(runtime_raw.trim()).unwrap_or_else(|| json!({}));
    let mirror_json = parse_json(mirror_raw.trim()).unwrap_or_else(|| json!({}));
    let runtime_norm = serde_json::to_string(&runtime_json).unwrap_or_default();
    let mirror_norm = serde_json::to_string(&mirror_json).unwrap_or_default();
    let in_sync = !runtime_norm.is_empty() && runtime_norm == mirror_norm;
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_cron_drift",
        "in_sync": in_sync,
        "runtime_path": runtime.to_string_lossy().to_string(),
        "mirror_path": mirror.to_string_lossy().to_string(),
        "runtime_exists": runtime.exists(),
        "mirror_exists": mirror.exists(),
        "runtime_jobs_count": runtime_json.get("jobs").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "mirror_jobs_count": mirror_json.get("jobs").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
    }))
}

fn run_cron_sync(control_runtime_root: &Path, workspace_root: &Path) -> Result<Value, String> {
    let runtime = cron_runtime_jobs_path(control_runtime_root);
    let mirror = cron_workspace_mirror_path(workspace_root);
    if !runtime.exists() {
        return Err("cron_runtime_jobs_missing".to_string());
    }
    let raw =
        fs::read_to_string(&runtime).map_err(|err| format!("cron_runtime_read_failed:{err}"))?;
    if let Some(parent) = mirror.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("cron_mirror_mkdir_failed:{err}"))?;
    }
    fs::write(&mirror, raw).map_err(|err| format!("cron_mirror_write_failed:{err}"))?;
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_cron_sync",
        "runtime_path": runtime.to_string_lossy().to_string(),
        "mirror_path": mirror.to_string_lossy().to_string()
    })))
}

fn run_doctor(control_runtime_root: &Path, workspace_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let routing = run_smoke_routing(control_runtime_root, parsed);
    let cron = run_cron_drift(control_runtime_root, workspace_root);
    let checks = vec![
        json!({
            "id": "routing_smoke",
            "ok": routing.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "detail": routing
        }),
        json!({
            "id": "cron_in_sync",
            "ok": cron.get("in_sync").and_then(Value::as_bool).unwrap_or(false),
            "detail": cron
        }),
        json!({
            "id": "agent_state_exists",
            "ok": state_path(control_runtime_root, parsed).exists(),
            "detail": state_path(control_runtime_root, parsed).to_string_lossy().to_string()
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    with_receipt(json!({
        "ok": ok,
        "type": "operator_tooling_doctor",
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "workspace_root": workspace_root.to_string_lossy().to_string(),
        "checks": checks
    }))
}

fn run_audit_plane(
    control_runtime_root: &Path,
    workspace_root: &Path,
    parsed: &crate::ParsedArgs,
) -> Value {
    let doctor = run_doctor(control_runtime_root, workspace_root, parsed);
    let memory_recent = run_memory_last_change(control_runtime_root, 10);
    with_receipt(json!({
        "ok": doctor.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "operator_tooling_audit_plane",
        "doctor": doctor,
        "memory_recent": memory_recent
    }))
}

fn run_daily_brief(
    control_runtime_root: &Path,
    workspace_root: &Path,
    parsed: &crate::ParsedArgs,
) -> Value {
    let state_file = state_path(control_runtime_root, parsed);
    let state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let last_task = state.get("last_task").cloned().unwrap_or_else(|| json!({}));
    let routing = state.get("routing").cloned().unwrap_or_else(|| json!({}));
    let prefs = state
        .get("preferences")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let spawn_events = read_jsonl_rows(&control_runtime_root.join("logs/spawn-safe.jsonl"), 40);
    let mut seen = HashSet::<String>::new();
    let mut recent_models = Vec::<String>::new();
    for row in spawn_events.iter().rev() {
        let model = row
            .get("model")
            .and_then(Value::as_str)
            .or_else(|| {
                row.pointer("/packet/handoff/selected_model")
                    .and_then(Value::as_str)
            })
            .map(|v| clean_text(v, 240))
            .unwrap_or_default();
        if model.is_empty() || !seen.insert(model.clone()) {
            continue;
        }
        recent_models.push(model);
        if recent_models.len() >= 5 {
            break;
        }
    }
    recent_models.reverse();

    let mut recommendations = Vec::<String>::new();
    if !state_file.exists() {
        recommendations.push(
            "Create state file by running `infring state-write` for the first task.".to_string(),
        );
    }
    if prefs
        .get("always_sync_allowlist")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !agent_root(control_runtime_root).join("models.json").exists()
    {
        recommendations
            .push("Allowlist appears missing. Run `infring sync-allowed-models`.".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push(
            "No blockers detected. Continue with tagged tasks via `infring smart-spawn`."
                .to_string(),
        );
    }

    let audit = run_audit_plane(control_runtime_root, workspace_root, parsed);

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_daily_brief",
        "generated_at": crate::now_iso(),
        "project": state.get("project").cloned().unwrap_or_else(|| json!({})),
        "last_task": last_task,
        "routing": {
            "required_tags_min": routing.get("required_tags_min").cloned().unwrap_or(json!(3)),
            "required_tags_max": routing.get("required_tags_max").cloned().unwrap_or(json!(6)),
            "high_risk_tags": routing.get("high_risk_tags").cloned().unwrap_or_else(|| json!([])),
            "high_risk_requires_plan": routing
                .get("high_risk_requires_plan")
                .cloned()
                .unwrap_or(json!(true))
        },
        "preferences": {
            "default_timeout_seconds": prefs.get("default_timeout_seconds").cloned().unwrap_or(json!(30)),
            "always_use_spawn_safe": prefs.get("always_use_spawn_safe").cloned().unwrap_or(json!(true)),
            "always_sync_allowlist": prefs.get("always_sync_allowlist").cloned().unwrap_or(json!(true))
        },
        "recent_models": recent_models,
        "recommendations": recommendations,
        "control_plane": audit
    }))
}

fn run_fail_playbook(
    control_runtime_root: &Path,
    workspace_root: &Path,
    parsed: &crate::ParsedArgs,
) -> Value {
    let doctor = run_doctor(control_runtime_root, workspace_root, parsed);
    let mut actions = Vec::<String>::new();
    if doctor.pointer("/checks/0/ok").and_then(Value::as_bool) == Some(false) {
        actions.push("Run: infring smoke-routing".to_string());
        actions.push("Then: infring sync-allowed-models".to_string());
    }
    if doctor.pointer("/checks/1/ok").and_then(Value::as_bool) == Some(false) {
        actions.push("Run: infring cron-sync".to_string());
    }
    if doctor.pointer("/checks/2/ok").and_then(Value::as_bool) == Some(false) {
        actions.push("Run: infring state-write --payload='{\"task\":\"bootstrap\"}'".to_string());
    }
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_fail_playbook",
        "actions": actions,
        "doctor": doctor
    }))
}

fn summary_status(control_runtime_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let policy = routing_policy_path(control_runtime_root, parsed);
    let state = state_path(control_runtime_root, parsed);
    let decisions = decision_log_path(control_runtime_root, parsed);
    let files = vec![
        ("routing_policy", policy),
        ("state", state),
        ("decisions", decisions),
        (
            "logs_spawn_safe",
            control_runtime_root.join("logs/spawn-safe.jsonl"),
        ),
        ("logs_spawn_run", control_runtime_root.join("logs/spawn-run.jsonl")),
    ];
    let file_rows = files
        .into_iter()
        .map(|(label, path)| {
            json!({
                "label": label,
                "path": path.to_string_lossy().to_string(),
                "exists": path.exists()
            })
        })
        .collect::<Vec<_>>();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_status",
        "control_runtime_root": control_runtime_root.to_string_lossy().to_string(),
        "commands": [
            "status",
            "route-model",
            "escalate-model",
            "plan-auto",
            "plan-validate",
            "postflight-validate",
            "output-validate",
            "state-read",
            "state-write",
            "decision-log-append",
            "safe-apply",
            "memory-search",
            "memory-summarize",
            "memory-last-change",
            "membrief",
            "trace-find",
            "sync-allowed-models",
            "smoke-routing",
            "spawn-safe",
            "smart-spawn",
            "auto-spawn",
            "execute-handoff",
            "safe-run",
            "control_runtime-health",
            "cron-drift",
            "cron-sync",
            "doctor",
            "audit-plane",
            "daily-brief",
            "fail-playbook"
        ],
        "paths": file_rows
    }))
}

fn error_receipt(command: &str, error: &str, code: i32) -> Value {
    with_receipt(json!({
        "ok": false,
        "type": "operator_tooling_error",
        "command": command,
        "error": clean_text(error, 220),
        "exit_code": code
    }))
}
