fn analyze_result_consensus(results: &[AgentResult], field: &str, threshold: f64) -> Value {
    if results.is_empty() {
        return json!({
            "consensus_reached": false,
            "status": "no_results",
            "confidence": 0.0,
            "sample_size": 0,
            "outliers": [],
        });
    }
    let mut groups: BTreeMap<String, Vec<(String, String, Value)>> = BTreeMap::new();
    for result in results {
        let Some(value) = consensus_value_from_result(result, field) else {
            continue;
        };
        let fingerprint = deterministic_receipt_hash(&value);
        groups.entry(fingerprint).or_default().push((
            result.result_id.clone(),
            result.agent_label.clone(),
            value,
        ));
    }
    if groups.is_empty() {
        return json!({
            "consensus_reached": false,
            "status": "no_extractable_values",
            "confidence": 0.0,
            "sample_size": results.len(),
            "field": field,
            "outliers": [],
        });
    }

    let (leader_key, leader_group) = groups
        .iter()
        .max_by_key(|(_, rows)| rows.len())
        .expect("leader group");
    let extractable_count = groups.values().map(Vec::len).sum::<usize>();
    let confidence = leader_group.len() as f64 / extractable_count as f64;
    let status = if leader_group.len() == extractable_count {
        "full_agreement"
    } else if confidence >= threshold {
        "partial_agreement"
    } else {
        "disagreement"
    };

    let mut outliers = Vec::new();
    for (fingerprint, rows) in &groups {
        if fingerprint == leader_key {
            continue;
        }
        for (result_id, agent_label, value) in rows {
            outliers.push(json!({
                "result_id": result_id,
                "agent_label": agent_label,
                "value": value,
                "deviation": "majority_mismatch",
            }));
        }
    }
    let disagreement_count = extractable_count.saturating_sub(leader_group.len());
    let outlier_rate = if extractable_count == 0 {
        0.0
    } else {
        disagreement_count as f64 / extractable_count as f64
    };
    let confidence_band = if confidence >= 0.9 {
        "high"
    } else if confidence >= threshold {
        "medium"
    } else {
        "low"
    };
    let reason_code = if status == "full_agreement" {
        "majority_unanimous"
    } else if status == "partial_agreement" {
        "majority_with_outliers"
    } else {
        "insufficient_majority"
    };
    let recommended_action = if status == "full_agreement" {
        "accept_majority"
    } else if status == "partial_agreement" {
        "accept_with_outlier_review"
    } else {
        "request_additional_agents"
    };

    json!({
        "consensus_reached": confidence >= threshold,
        "status": status,
        "reason_code": reason_code,
        "confidence": confidence,
        "confidence_band": confidence_band,
        "threshold": threshold,
        "field": field,
        "sample_size": results.len(),
        "extractable_count": extractable_count,
        "group_count": groups.len(),
        "agreement_count": leader_group.len(),
        "disagreement_count": disagreement_count,
        "outlier_rate": outlier_rate,
        "dominant_fingerprint": clean_text(leader_key, 24),
        "agreed_value": leader_group.first().map(|(_, _, value)| value.clone()).unwrap_or(Value::Null),
        "recommended_action": recommended_action,
        "outliers": outliers,
    })
}

fn analyze_result_outliers(results: &[AgentResult], field: &str) -> Value {
    let points = results
        .iter()
        .filter_map(|result| {
            consensus_value_from_result(result, field)
                .and_then(|value| value.as_f64())
                .map(|value| (result.result_id.clone(), result.agent_label.clone(), value))
        })
        .collect::<Vec<_>>();
    if points.len() < 3 {
        return json!({
            "status": "insufficient_data",
            "field": field,
            "sample_size": points.len(),
            "outliers": [],
        });
    }
    let mean = points.iter().map(|(_, _, value)| *value).sum::<f64>() / points.len() as f64;
    let variance = points
        .iter()
        .map(|(_, _, value)| (*value - mean).powi(2))
        .sum::<f64>()
        / points.len() as f64;
    let std_dev = variance.sqrt();
    let mut sorted = points
        .iter()
        .map(|(_, _, value)| *value)
        .collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    let mut abs_dev = sorted
        .iter()
        .map(|value| (value - median).abs())
        .collect::<Vec<_>>();
    abs_dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = if abs_dev.is_empty() {
        0.0
    } else if abs_dev.len() % 2 == 0 {
        (abs_dev[abs_dev.len() / 2 - 1] + abs_dev[abs_dev.len() / 2]) / 2.0
    } else {
        abs_dev[abs_dev.len() / 2]
    };
    if std_dev == 0.0 {
        return json!({
            "status": "stable",
            "field": field,
            "sample_size": points.len(),
            "mean": mean,
            "median": median,
            "std_dev": std_dev,
            "mad": mad,
            "outliers": [],
        });
    }
    let outliers = points
        .into_iter()
        .filter_map(|(result_id, agent_label, value)| {
            let z_score = (value - mean).abs() / std_dev;
            let robust_z_score = if mad > 0.0 {
                0.6745 * (value - median).abs() / mad
            } else {
                0.0
            };
            if z_score > 2.0 || robust_z_score > 3.5 {
                Some(json!({
                    "result_id": result_id,
                    "agent_label": agent_label,
                    "value": value,
                    "z_score": z_score,
                    "robust_z_score": robust_z_score,
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    json!({
        "status": if outliers.is_empty() { "stable" } else { "outliers_detected" },
        "field": field,
        "sample_size": results.len(),
        "mean": mean,
        "median": median,
        "std_dev": std_dev,
        "mad": mad,
        "outlier_count": outliers.len(),
        "outliers": outliers,
    })
}

fn create_channel(
    state: &mut SwarmState,
    channel_name: &str,
    participants: Vec<String>,
) -> Result<Value, String> {
    if participants.is_empty() {
        return Err("channel_participants_required".to_string());
    }
    let mut cleaned = participants
        .into_iter()
        .filter(|session_id| !session_id.trim().is_empty())
        .collect::<Vec<_>>();
    cleaned.sort();
    cleaned.dedup();
    for participant in &cleaned {
        if !session_exists(state, participant) {
            return Err(format!("unknown_channel_participant:{participant}"));
        }
    }
    let channel_id = format!(
        "chan-{}",
        &deterministic_receipt_hash(&json!({
            "name": channel_name,
            "participants": cleaned,
            "ts": now_epoch_ms(),
        }))[..12]
    );
    let channel = MessageChannel {
        channel_id: channel_id.clone(),
        name: channel_name.to_string(),
        participants: cleaned,
        created_at: now_iso(),
        messages: Vec::new(),
    };
    state.channels.insert(channel_id.clone(), channel.clone());
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_channel_create",
        "channel": channel,
    }))
}

fn publish_channel_message(
    state: &mut SwarmState,
    channel_id: &str,
    sender_session_id: &str,
    payload: &str,
    delivery: DeliveryGuarantee,
) -> Result<Value, String> {
    if sender_session_id != "coordinator" && !session_exists(state, sender_session_id) {
        return Err(format!("unknown_sender_session:{sender_session_id}"));
    }
    let participants = state
        .channels
        .get(channel_id)
        .map(|channel| channel.participants.clone())
        .ok_or_else(|| format!("unknown_channel:{channel_id}"))?;
    let message_id = format!(
        "chanmsg-{}",
        &deterministic_receipt_hash(&json!({
            "channel_id": channel_id,
            "sender": sender_session_id,
            "payload": payload,
            "ts": now_epoch_ms(),
        }))[..12]
    );
    if let Some(channel) = state.channels.get_mut(channel_id) {
        channel.messages.push(ChannelMessage {
            message_id: message_id.clone(),
            sender_session_id: sender_session_id.to_string(),
            payload: payload.to_string(),
            timestamp_ms: now_epoch_ms(),
        });
    }
    let mut delivered = Vec::new();
    for participant in participants {
        if participant == sender_session_id {
            continue;
        }
        let result = send_session_message(
            state,
            sender_session_id,
            &participant,
            payload,
            delivery.clone(),
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )?;
        delivered.push(result);
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_channel_publish",
        "channel_id": channel_id,
        "message_id": message_id,
        "delivery": delivery.as_label(),
        "delivered": delivered,
    }))
}

fn poll_channel_messages(
    state: &SwarmState,
    channel_id: &str,
    session_id: &str,
    since_ms: Option<u64>,
) -> Result<Value, String> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| format!("unknown_channel:{channel_id}"))?;
    if !channel
        .participants
        .iter()
        .any(|participant| participant == session_id)
    {
        return Err(format!(
            "channel_access_denied:channel={channel_id}:session={session_id}"
        ));
    }
    let messages = channel
        .messages
        .iter()
        .filter(|message| {
            since_ms
                .map(|value| message.timestamp_ms >= value)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_channel_poll",
        "channel_id": channel_id,
        "session_id": session_id,
        "message_count": messages.len(),
        "messages": messages,
    }))
}

fn persistent_session_ids(state: &SwarmState) -> Vec<String> {
    state
        .sessions
        .iter()
        .filter(|(_, session)| {
            session.persistent.is_some()
                && matches!(
                    session.status.as_str(),
                    "persistent_running" | "background_running"
                )
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>()
}

fn apply_tool_plan_for_session(
    session: &mut SessionMetadata,
    tool_plan: &[(String, u32)],
) -> Result<(u32, Option<String>), String> {
    let mut budget_action_taken: Option<String> = None;
    if let Some(telemetry) = session.budget_telemetry.as_mut() {
        for (tool, requested_tokens) in tool_plan {
            match telemetry.record_tool_usage(tool, *requested_tokens) {
                BudgetUsageOutcome::Ok => {}
                BudgetUsageOutcome::Warning(event) => session.check_ins.push(event),
                BudgetUsageOutcome::ExhaustedAllowed { event, action } => {
                    session.check_ins.push(event);
                    budget_action_taken = Some(action);
                }
                BudgetUsageOutcome::ExceededDenied(reason) => {
                    session.status = "failed".to_string();
                    return Err(reason);
                }
            }
        }
        return Ok((telemetry.final_usage, budget_action_taken));
    }
    let usage = tool_plan.iter().map(|(_, tokens)| *tokens).sum::<u32>();
    Ok((usage, budget_action_taken))
}

fn perform_persistent_check_in(
    session: &mut SessionMetadata,
    reason: &str,
    final_report: bool,
) -> Result<Value, String> {
    let task = session
        .scaled_task
        .clone()
        .unwrap_or_else(|| session.task.clone());
    let token_budget = session
        .budget_telemetry
        .as_ref()
        .map(|telemetry| telemetry.budget_config.max_tokens);
    let plan = estimate_tool_plan(&task, token_budget);
    let active_tools = plan
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let response_latency_ms = if session
        .metrics
        .as_ref()
        .map(|metrics| metrics.execution_time_ms())
        .unwrap_or(0)
        == 0
    {
        1
    } else {
        session
            .metrics
            .as_ref()
            .map(|metrics| metrics.execution_time_ms())
            .unwrap_or(1)
    };
    let (token_usage, budget_action) = apply_tool_plan_for_session(session, &plan)?;
    if budget_action.is_some() {
        session.budget_action_taken = budget_action;
    }

    let (check_in_count, report_mode) = {
        let runtime = session
            .persistent
            .as_mut()
            .ok_or_else(|| "persistent_runtime_missing".to_string())?;
        runtime.check_in_count = runtime.check_in_count.saturating_add(1);
        runtime.last_check_in_ms = Some(now_epoch_ms());
        (runtime.check_in_count, runtime.config.report_mode.clone())
    };

    let snapshot = collect_metrics_snapshot(
        session.budget_telemetry.as_ref(),
        check_in_count,
        response_latency_ms,
        active_tools,
    );
    session.metrics_timeline.push(snapshot.clone());
    session.anomalies = detect_anomalies(&session.metrics_timeline);

    let report = json!({
        "type": "persistent_check_in",
        "session_id": session.session_id,
        "reason": reason,
        "timestamp_ms": snapshot.timestamp_ms,
        "check_in_count": check_in_count,
        "token_usage_estimate": token_usage,
        "metrics": snapshot,
        "anomalies": session.anomalies,
        "report_mode": report_mode.as_label(),
        "final_report": final_report,
    });
    session.check_ins.push(report.clone());

    let should_emit = report_mode_should_emit(&report_mode, &session.anomalies, final_report);
    if should_emit {
        session.report = Some(report.clone());
    }

    Ok(json!({
        "emitted": should_emit,
        "report": report,
    }))
}
