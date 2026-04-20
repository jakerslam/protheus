
fn remove_entry(rows: &mut Vec<Value>, proposal_id: &str) -> Option<Value> {
    let idx = rows.iter().position(|row| {
        row.get("proposal")
            .and_then(Value::as_object)
            .and_then(|proposal| proposal.get("id"))
            .and_then(Value::as_str)
            == Some(proposal_id)
    })?;
    Some(rows.remove(idx))
}

fn insert_pending(state: &mut Value, pending: Value) {
    let object = state.as_object_mut().expect("state object");
    let rows = array_from(object, "pending_commit");
    let proposal_id = pending
        .get("proposal")
        .and_then(Value::as_object)
        .and_then(|proposal| proposal.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if !proposal_id.is_empty() {
        rows.retain(|row| {
            row.get("proposal")
                .and_then(Value::as_object)
                .and_then(|proposal| proposal.get("id"))
                .and_then(Value::as_str)
                != Some(proposal_id)
        });
    }
    rows.push(pending);
}

fn base_receipt(kind: &str, command: &str, policy: &Policy) -> Value {
    json!({
        "ok": true,
        "type": kind,
        "authority": "core/layer2/ops",
        "command": command,
        "state_path": policy.state_path.to_string_lossy(),
        "latest_path": policy.latest_path.to_string_lossy(),
        "receipts_path": policy.receipts_path.to_string_lossy(),
        "regrets_path": policy.regrets_path.to_string_lossy(),
        "ts_epoch_ms": now_epoch_ms()
    })
}

fn finalize_receipt(policy: &Policy, receipt: &mut Value) -> Result<(), String> {
    receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(receipt));
    write_json(&policy.latest_path, receipt)?;
    append_jsonl(&policy.receipts_path, receipt)?;
    Ok(())
}

fn status_receipt(policy: &Policy) -> Result<Value, String> {
    let state = load_state(&policy.state_path);
    let pending = state
        .get("pending_commit")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let committed = state
        .get("committed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rolled_back = state
        .get("rolled_back")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let now = now_epoch_ms();
    let overdue_pending = pending
        .iter()
        .filter(|row| {
            row.get("rollback_deadline_epoch_ms")
                .and_then(Value::as_i64)
                .map(|deadline| deadline <= now)
                .unwrap_or(false)
        })
        .count();
    let mut out = base_receipt("autophagy_auto_approval_status", "status", policy);
    out["policy"] = json!({
        "enabled": policy.enabled,
        "min_confidence": policy.min_confidence,
        "min_historical_success_rate": policy.min_historical_success_rate,
        "max_impact_score": policy.max_impact_score,
        "excluded_types": policy.excluded_types,
        "rollback_window_minutes": policy.rollback_window_minutes,
        "auto_rollback_on_degradation": policy.auto_rollback_on_degradation,
        "degradation_threshold": {
            "max_drift_delta": policy.max_drift_delta,
            "max_yield_drop": policy.max_yield_drop
        }
    });
    out["summary"] = json!({
        "pending_commit": pending.len(),
        "committed": committed.len(),
        "rolled_back": rolled_back.len(),
        "overdue_pending": overdue_pending
    });
    out["pending_commit"] = Value::Array(pending);
    out["committed"] = Value::Array(committed);
    out["rolled_back"] = Value::Array(rolled_back);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    Ok(out)
}

fn evaluate_command(root: &Path, argv: &[String], policy: &Policy) -> Result<Value, String> {
    let proposal = parse_proposal(argv)?;
    let summary = proposal_summary(&proposal);
    let apply = parse_bool(parse_cli_flag(argv, "apply").as_deref(), false);
    let (eligible, reasons) = evaluate_proposal(policy, &summary);
    let decision = if eligible {
        if apply {
            "auto_execute_pending_commit"
        } else {
            "auto_approve_eligible"
        }
    } else {
        "human_review_required"
    };

    let mut receipt = base_receipt("autophagy_auto_approval_evaluation", "evaluate", policy);
    receipt["root"] = Value::String(root.to_string_lossy().to_string());
    receipt["proposal"] = json!({
        "id": summary.id,
        "title": summary.title,
        "proposal_type": summary.proposal_type,
        "confidence": summary.confidence,
        "historical_success_rate": summary.historical_success_rate,
        "impact_score": summary.impact_score
    });
    receipt["decision"] = Value::String(decision.to_string());
    receipt["eligible"] = Value::Bool(eligible);
    receipt["apply"] = Value::Bool(apply);
    receipt["decision_reasons"] =
        Value::Array(reasons.iter().map(|v| Value::String(v.clone())).collect());
    receipt["claim_evidence"] = json!([
        {
            "id": "confidence_gated_auto_approval",
            "claim": "high_confidence_bounded_proposals_can_auto_execute_with_rollback_window",
            "evidence": {
                "eligible": eligible,
                "apply": apply,
                "rollback_window_minutes": policy.rollback_window_minutes
            }
        }
    ]);

    if eligible && apply {
        let mut state = load_state(&policy.state_path);
        let now = now_epoch_ms();
        let pending = json!({
            "proposal": {
                "id": summary.id,
                "title": summary.title,
                "proposal_type": summary.proposal_type,
                "confidence": summary.confidence,
                "historical_success_rate": summary.historical_success_rate,
                "impact_score": summary.impact_score,
                "raw": summary.raw
            },
            "approved_at_epoch_ms": now,
            "rollback_deadline_epoch_ms": now + (policy.rollback_window_minutes * 60 * 1000),
            "state": "pending_commit"
        });
        insert_pending(&mut state, pending.clone());
        state["last_decision"] = receipt.clone();
        state["updated_at_epoch_ms"] = json!(now);
        store_state(policy, &state)?;
        receipt["pending_record"] = pending;
    }

    finalize_receipt(policy, &mut receipt)?;
    Ok(receipt)
}
