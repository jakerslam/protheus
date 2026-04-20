
fn rollback_from_state(
    state: &mut Value,
    policy: &Policy,
    proposal_id: &str,
    trigger: &str,
    reason: &str,
    drift: Option<f64>,
    yield_drop: Option<f64>,
) -> Result<Value, String> {
    let object = state
        .as_object_mut()
        .ok_or_else(|| "invalid_state_object".to_string())?;
    let pending_rows = array_from(object, "pending_commit");
    let pending = remove_entry(pending_rows, proposal_id)
        .ok_or_else(|| format!("proposal_not_pending:{proposal_id}"))?;
    let rolled_back_rows = array_from(object, "rolled_back");
    let now = now_epoch_ms();
    let regret = json!({
        "proposal_id": proposal_id,
        "label": policy.regret_issue_label,
        "reason": reason,
        "trigger": trigger,
        "remediation_path": format!("review/{proposal_id}"),
        "ts_epoch_ms": now
    });
    let rolled = json!({
        "proposal": pending.get("proposal").cloned().unwrap_or_else(|| json!({"id": proposal_id})),
        "rolled_back_at_epoch_ms": now,
        "trigger": trigger,
        "reason": reason,
        "drift": drift,
        "yield_drop": yield_drop,
        "regret": regret
    });
    rolled_back_rows.push(rolled.clone());
    state["updated_at_epoch_ms"] = json!(now);
    append_jsonl(&policy.regrets_path, &regret)?;

    let mut receipt = base_receipt("autophagy_auto_approval_rollback", "rollback", policy);
    receipt["proposal_id"] = Value::String(proposal_id.to_string());
    receipt["trigger"] = Value::String(trigger.to_string());
    receipt["reason"] = Value::String(reason.to_string());
    receipt["drift"] = drift.map(Value::from).unwrap_or(Value::Null);
    receipt["yield_drop"] = yield_drop.map(Value::from).unwrap_or(Value::Null);
    receipt["regret_issue"] = regret;
    receipt["rolled_back_record"] = rolled;
    finalize_receipt(policy, &mut receipt)?;
    Ok(receipt)
}

fn monitor_command(argv: &[String], policy: &Policy) -> Result<Value, String> {
    let proposal_id =
        parse_cli_flag(argv, "proposal-id").ok_or_else(|| "missing_proposal_id".to_string())?;
    let apply = parse_bool(parse_cli_flag(argv, "apply").as_deref(), false);
    let drift = parse_f64(parse_cli_flag(argv, "drift").as_deref());
    let yield_drop = parse_f64(parse_cli_flag(argv, "yield-drop").as_deref());
    let now = now_epoch_ms();
    let mut state = load_state(&policy.state_path);

    let pending = state
        .get("pending_commit")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("proposal")
                    .and_then(Value::as_object)
                    .and_then(|proposal| proposal.get("id"))
                    .and_then(Value::as_str)
                    == Some(proposal_id.as_str())
            })
        })
        .cloned()
        .ok_or_else(|| format!("proposal_not_pending:{proposal_id}"))?;

    let deadline = pending
        .get("rollback_deadline_epoch_ms")
        .and_then(Value::as_i64)
        .unwrap_or(now);
    let drift_breach = drift
        .map(|value| value > policy.max_drift_delta)
        .unwrap_or(false);
    let yield_breach = yield_drop
        .map(|value| value > policy.max_yield_drop)
        .unwrap_or(false);
    let expired = now >= deadline;
    let trigger = if drift_breach || yield_breach {
        "degradation_threshold_breach"
    } else if expired {
        "rollback_window_expired"
    } else {
        "healthy_pending"
    };
    let should_rollback =
        policy.auto_rollback_on_degradation && (drift_breach || yield_breach || expired);

    if apply && should_rollback {
        let reason = if drift_breach || yield_breach {
            "degradation_detected"
        } else {
            "rollback_window_expired_without_commit"
        };
        let receipt = rollback_from_state(
            &mut state,
            policy,
            &proposal_id,
            trigger,
            reason,
            drift,
            yield_drop,
        )?;
        store_state(policy, &state)?;
        return Ok(receipt);
    }

    let mut receipt = base_receipt("autophagy_auto_approval_monitor", "monitor", policy);
    receipt["proposal_id"] = Value::String(proposal_id);
    receipt["drift"] = drift.map(Value::from).unwrap_or(Value::Null);
    receipt["yield_drop"] = yield_drop.map(Value::from).unwrap_or(Value::Null);
    receipt["rollback_deadline_epoch_ms"] = json!(deadline);
    receipt["expired"] = Value::Bool(expired);
    receipt["should_rollback"] = Value::Bool(should_rollback);
    receipt["trigger"] = Value::String(trigger.to_string());
    finalize_receipt(policy, &mut receipt)?;
    Ok(receipt)
}

fn commit_command(argv: &[String], policy: &Policy) -> Result<Value, String> {
    let proposal_id =
        parse_cli_flag(argv, "proposal-id").ok_or_else(|| "missing_proposal_id".to_string())?;
    let reason = parse_cli_flag(argv, "reason").unwrap_or_else(|| "human_confirmed".to_string());
    let mut state = load_state(&policy.state_path);
    let object = state
        .as_object_mut()
        .ok_or_else(|| "invalid_state_object".to_string())?;
    let pending_rows = array_from(object, "pending_commit");
    let pending = remove_entry(pending_rows, &proposal_id)
        .ok_or_else(|| format!("proposal_not_pending:{proposal_id}"))?;
    let committed_rows = array_from(object, "committed");
    let committed = json!({
        "proposal": pending.get("proposal").cloned().unwrap_or_else(|| json!({"id": proposal_id})),
        "committed_at_epoch_ms": now_epoch_ms(),
        "reason": reason
    });
    committed_rows.push(committed.clone());
    state["updated_at_epoch_ms"] = json!(now_epoch_ms());
    store_state(policy, &state)?;

    let mut receipt = base_receipt("autophagy_auto_approval_commit", "commit", policy);
    receipt["proposal_id"] = Value::String(proposal_id);
    receipt["reason"] = Value::String(reason);
    receipt["committed_record"] = committed;
    finalize_receipt(policy, &mut receipt)?;
    Ok(receipt)
}

fn rollback_command(argv: &[String], policy: &Policy) -> Result<Value, String> {
    let proposal_id =
        parse_cli_flag(argv, "proposal-id").ok_or_else(|| "missing_proposal_id".to_string())?;
    let reason = parse_cli_flag(argv, "reason").unwrap_or_else(|| "manual_rollback".to_string());
    let mut state = load_state(&policy.state_path);
    let receipt = rollback_from_state(
        &mut state,
        policy,
        &proposal_id,
        "manual_rollback",
        &reason,
        parse_f64(parse_cli_flag(argv, "drift").as_deref()),
        parse_f64(parse_cli_flag(argv, "yield-drop").as_deref()),
    )?;
    store_state(policy, &state)?;
    Ok(receipt)
}

fn cli_error(command: &str, error: &str) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "autophagy_auto_approval_cli_error",
        "authority": "core/layer2/ops",
        "command": command,
        "error": error,
        "ts_epoch_ms": now_epoch_ms()
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
