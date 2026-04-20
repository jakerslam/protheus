
fn run_schedule(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SCHEDULE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "persist_schedule_contract",
            "allowed_ops": ["upsert", "list", "kickoff"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_schedule",
            "errors": ["persist_schedule_op_invalid"]
        });
    }

    let path = schedules_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "jobs": {},
            "runs": []
        })
    });
    if !state.get("jobs").map(Value::is_object).unwrap_or(false) {
        state["jobs"] = Value::Object(serde_json::Map::new());
    }
    if !state.get("runs").map(Value::is_array).unwrap_or(false) {
        state["runs"] = Value::Array(Vec::new());
    }

    if op == "list" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_schedule",
            "lane": "core/layer0/ops",
            "op": "list",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.1",
                    "claim": "scheduled_background_task_lane_surfaces_registered_jobs",
                    "evidence": {
                        "job_count": state
                            .get("jobs")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let job_id = clean_id(
        parsed
            .flags
            .get("job")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("job-id").map(String::as_str)),
        "default-job",
    );
    if op == "upsert" {
        let cron = clean(
            parsed
                .flags
                .get("cron")
                .cloned()
                .unwrap_or_else(|| "*/5 * * * *".to_string()),
            160,
        );
        let workflow = clean(
            parsed
                .flags
                .get("workflow")
                .cloned()
                .unwrap_or_else(|| "default-workflow".to_string()),
            120,
        );
        let owner = clean(
            parsed
                .flags
                .get("owner")
                .cloned()
                .unwrap_or_else(|| "system".to_string()),
            120,
        );
        let job = json!({
            "job_id": job_id,
            "cron": cron,
            "workflow": workflow,
            "owner": owner,
            "updated_at": crate::now_iso()
        });
        state["jobs"][&job_id] = job.clone();
        state["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &state);
        let _ = append_jsonl(
            &state_root(root).join("schedules").join("history.jsonl"),
            &json!({"op":"upsert","job_id":job_id,"ts":crate::now_iso()}),
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_schedule",
            "lane": "core/layer0/ops",
            "op": "upsert",
            "job": job,
            "artifact": {
                "path": path.display().to_string(),
                "sha256": sha256_hex_str(&state.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.1",
                    "claim": "schedule_contract_supports_receipted_recurring_background_workflows",
                    "evidence": {
                        "job_id": job_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if strict && state["jobs"].get(&job_id).is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_schedule",
            "errors": ["persist_schedule_job_not_found"]
        });
    }
    let run_id = format!(
        "kickoff_{}",
        &sha256_hex_str(&format!("{job_id}:{}", crate::now_iso()))[..10]
    );
    let run = json!({
        "run_id": run_id,
        "job_id": job_id,
        "status": "started",
        "ts": crate::now_iso()
    });
    let mut runs = state
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    runs.push(run.clone());
    state["runs"] = Value::Array(runs);
    state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("schedules").join("history.jsonl"),
        &json!({"op":"kickoff","run":run,"ts":crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "persist_plane_schedule",
        "lane": "core/layer0/ops",
        "op": "kickoff",
        "run": run,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-PERSIST-001.1",
                "claim": "scheduled_background_runtime_kickoff_is_receipted",
                "evidence": {
                    "run_id": run_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
