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

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, argv);
    let result = match command.as_str() {
        "evaluate" => evaluate_command(root, argv, &policy),
        "monitor" => monitor_command(argv, &policy),
        "commit" => commit_command(argv, &policy),
        "rollback" => rollback_command(argv, &policy),
        "status" => status_receipt(&policy),
        _ => Err("unknown_command".to_string()),
    };

    match result {
        Ok(receipt) => {
            print_json_line(&receipt);
            if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(error) => {
            print_json_line(&cli_error(&command, &error));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_root(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "infring_autophagy_auto_approval_{name}_{}",
            now_epoch_ms()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join("client/runtime/config")).expect("config dir");
        path
    }

    fn write_policy(root: &Path) {
        let path = root.join("client/runtime/config/autophagy_auto_approval_policy.json");
        let policy = json!({
            "enabled": true,
            "auto_approval": {
                "enabled": true,
                "min_confidence": 0.85,
                "min_historical_success_rate": 0.90,
                "max_impact_score": 50,
                "excluded_types": ["safety_critical", "budget_hold"],
                "auto_rollback_on_degradation": true,
                "rollback_window_minutes": 1,
                "regret_issue_label": "auto_approval_regret",
                "degradation_threshold": {
                    "max_drift_delta": 0.01,
                    "max_yield_drop": 0.05
                }
            }
        });
        write_json(&path, &policy).expect("write policy");
    }

    #[test]
    fn evaluate_apply_creates_pending_commit_record() {
        let root = temp_root("evaluate");
        write_policy(&root);
        let args = vec![
            "evaluate".to_string(),
            "--apply=1".to_string(),
            "--proposal-json={\"id\":\"p1\",\"title\":\"Fix drift\",\"type\":\"ops_remediation\",\"confidence\":0.91,\"historical_success_rate\":0.94,\"impact_score\":18}".to_string(),
        ];
        assert_eq!(run(&root, &args), 0);
        let state = load_state(&root.join(DEFAULT_STATE_PATH));
        assert_eq!(
            state["pending_commit"].as_array().map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn excluded_type_requires_human_review() {
        let root = temp_root("excluded");
        write_policy(&root);
        let args = vec![
            "evaluate".to_string(),
            "--proposal-json={\"id\":\"p2\",\"title\":\"Touch safety\",\"type\":\"safety_critical\",\"confidence\":0.99,\"historical_success_rate\":0.99,\"impact_score\":1}".to_string(),
        ];
        let exit = run(&root, &args);
