
fn write_lane_receipt(policy: &Policy, row: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }
    write_json_atomic(&policy.paths.latest_path, row)?;
    append_jsonl(&policy.paths.receipts_path, row)?;
    append_jsonl(&policy.paths.history_path, row)
}

fn run_one(
    policy: &Policy,
    id: &str,
    apply: bool,
    strict: bool,
    root: &Path,
) -> Result<Value, String> {
    let mut state = load_state(policy);
    let out = lane_scale(id, policy, &mut state, apply, strict, root)?;
    let receipt_id = format!(
        "scale_{}",
        stable_hash(
            &serde_json::to_string(&json!({"id": id, "ts": now_iso(), "summary": out["summary"]}))
                .unwrap_or_else(|_| "{}".to_string()),
            16
        )
    );

    let mut receipt = out;
    receipt["receipt_id"] = Value::String(receipt_id.clone());

    state["last_run"] = Value::String(now_iso());
    if !state["lane_receipts"].is_object() {
        state["lane_receipts"] = json!({});
    }
    state["lane_receipts"][id] = json!({
        "ts": receipt["ts"].clone(),
        "ok": receipt["ok"].clone(),
        "receipt_id": receipt_id
    });

    if apply && receipt["ok"].as_bool().unwrap_or(false) {
        save_state(policy, &state, true)?;
        write_lane_receipt(policy, &receipt, true)?;
    }

    Ok(receipt)
}

fn list(policy: &Policy, root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "scale_readiness_program",
        "action": "list",
        "ts": now_iso(),
        "item_count": policy.items.len(),
        "items": policy.items,
        "policy_path": rel_path(root, &policy.policy_path)
    })
}

fn run_all(policy: &Policy, apply: bool, strict: bool, root: &Path) -> Result<Value, String> {
    let mut lanes = Vec::new();
    for id in SCALE_IDS {
        lanes.push(run_one(policy, id, apply, strict, root)?);
    }
    let ok = lanes
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let failed_lane_ids = lanes
        .iter()
        .filter_map(|row| {
            if row.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                row.get("lane_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            }
        })
        .collect::<Vec<_>>();

    let out = json!({
        "ok": ok,
        "type": "scale_readiness_program",
        "action": "run-all",
        "ts": now_iso(),
        "strict": strict,
        "apply": apply,
        "lane_count": lanes.len(),
        "lanes": lanes,
        "failed_lane_ids": failed_lane_ids
    });

    if apply {
        let row = json!({
            "schema_id": "scale_readiness_program_receipt",
            "schema_version": "1.0",
            "artifact_type": "receipt",
            "receipt_id": format!("scale_{}", stable_hash(&serde_json::to_string(&json!({"action":"run-all","ts":now_iso()})).unwrap_or_else(|_| "{}".to_string()), 16)),
            "ok": out["ok"],
            "type": out["type"],
            "action": out["action"],
            "ts": out["ts"],
            "strict": out["strict"],
            "apply": out["apply"],
            "lane_count": out["lane_count"],
            "lanes": out["lanes"],
            "failed_lane_ids": out["failed_lane_ids"]
        });
        write_lane_receipt(policy, &row, true)?;
    }

    Ok(out)
}

fn status(policy: &Policy, root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "scale_readiness_program",
        "action": "status",
        "ts": now_iso(),
        "policy_path": rel_path(root, &policy.policy_path),
        "state": load_state(policy),
        "latest": read_json(&policy.paths.latest_path)
    })
}

pub fn usage() {
    println!("Usage:");
    println!("  node client/runtime/systems/ops/scale_readiness_program.js list");
    println!("  node client/runtime/systems/ops/scale_readiness_program.js run --id=V4-SCALE-001 [--apply=1|0] [--strict=1|0]");
    println!("  node client/runtime/systems/ops/scale_readiness_program.js run-all [--apply=1|0] [--strict=1|0]");
    println!("  node client/runtime/systems/ops/scale_readiness_program.js status");
}
