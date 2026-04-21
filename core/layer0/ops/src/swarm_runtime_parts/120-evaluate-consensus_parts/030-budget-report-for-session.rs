
fn budget_report_for_session(state: &SwarmState, session_id: &str) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    let Some(telemetry) = session.budget_telemetry.as_ref() else {
        return Err(format!("budget_not_configured_for_session:{session_id}"));
    };
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_budget_report",
        "session_id": session_id,
        "report": telemetry.generate_report(),
    }))
}

fn session_context_flag_true(session: &SessionMetadata, key: &str) -> bool {
    session
        .context_vars
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn session_report_goal_met(session: &SessionMetadata) -> bool {
    session
        .report
        .as_ref()
        .and_then(|report| report.get("result"))
        .and_then(Value::as_str)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "goal_met" | "completed" | "done" | "success"
            )
        })
        .unwrap_or(false)
}

fn evaluate_should_terminate_contract(session: &SessionMetadata, now_ms: u64) -> Value {
    let goal_met = session_context_flag_true(session, "goal_met") || session_report_goal_met(session);
    let budget_exceeded = session
        .budget_telemetry
        .as_ref()
        .map(|telemetry| {
            telemetry.budget_exhausted
                || (telemetry.budget_config.max_tokens > 0 && telemetry.remaining_tokens() == 0)
        })
        .unwrap_or(false);
    let policy_stop = session_context_flag_true(session, "policy_stop")
        || session_context_flag_true(session, "stop_requested")
        || session_context_flag_true(session, "terminate_now");

    let stalled = session
        .persistent
        .as_ref()
        .map(|runtime| {
            let stall_window_ms = runtime
                .config
                .check_in_interval_sec
                .saturating_mul(1000)
                .saturating_mul(3);
            let overdue = now_ms > runtime.next_check_in_ms.saturating_add(stall_window_ms);
            overdue && runtime.check_in_count >= 3
        })
        .unwrap_or(false);

    let mut reason = "continue".to_string();
    let mut detail = "none".to_string();
    let should_terminate = if goal_met {
        reason = "goal_met".to_string();
        detail = "goal_marker_detected".to_string();
        true
    } else if budget_exceeded {
        reason = "budget_exceeded".to_string();
        detail = "budget_guard_triggered".to_string();
        true
    } else if stalled {
        reason = "stalled".to_string();
        detail = "check_in_overdue".to_string();
        true
    } else if policy_stop {
        reason = "policy_stop".to_string();
        detail = "policy_stop_requested".to_string();
        true
    } else if session
        .persistent
        .as_ref()
        .map(|runtime| now_ms >= runtime.deadline_ms)
        .unwrap_or(false)
    {
        reason = "policy_stop".to_string();
        detail = "lifespan_deadline_reached".to_string();
        true
    } else {
        false
    };

    json!({
        "should_terminate": should_terminate,
        "reason": reason,
        "detail": detail,
        "contract": {
            "goal_met": goal_met,
            "budget_exceeded": budget_exceeded,
            "stalled": stalled,
            "policy_stop": policy_stop,
        },
        "deterministic": true,
    })
}

fn termination_status_for_reason(reason: &str) -> &'static str {
    match reason {
        "goal_met" => "completed",
        "budget_exceeded" => "terminated_budget_exceeded",
        "stalled" => "terminated_stalled",
        "policy_stop" => "terminated_policy_stop",
        _ => "terminated",
    }
}

fn tick_persistent_sessions(
    state: &mut SwarmState,
    now_ms: u64,
    max_check_ins: u64,
) -> Result<Value, String> {
    let mut processed_sessions = 0u64;
    let mut check_ins = 0u64;
    let mut finalized_sessions = Vec::new();
    let mut reports = Vec::new();

    for session_id in persistent_session_ids(state) {
        let mut local_processed = 0u64;
        loop {
            if local_processed >= max_check_ins {
                break;
            }
            let termination_contract = match state.sessions.get(&session_id) {
                Some(session) => evaluate_should_terminate_contract(session, now_ms),
                None => json!({
                    "should_terminate": false,
                    "reason": "missing_session",
                    "detail": "session_missing",
                    "contract": {
                        "goal_met": false,
                        "budget_exceeded": false,
                        "stalled": false,
                        "policy_stop": false
                    },
                    "deterministic": true,
                }),
            };
            let should_finalize = termination_contract
                .get("should_terminate")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let should_check_in = match state
                .sessions
                .get(&session_id)
                .and_then(|session| session.persistent.as_ref())
            {
                Some(runtime) => should_finalize || now_ms >= runtime.next_check_in_ms,
                None => false,
            };
            if !should_check_in {
                break;
            }

            let Some(session) = state.sessions.get_mut(&session_id) else {
                break;
            };
            let terminate_reason = termination_contract
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("policy_stop")
                .to_string();
            let parent_id = session.parent_id.clone();
            let role_card = session.role_card.clone();
            let report = if should_finalize {
                let result = perform_persistent_check_in(session, &terminate_reason, true)?;
                session.status = termination_status_for_reason(&terminate_reason).to_string();
                if let Some(runtime) = session.persistent.as_mut() {
                    runtime.terminated_at_ms = Some(now_ms);
                    runtime.terminated_reason = Some(terminate_reason.clone());
                }
                mark_service_instance_unhealthy(state, &session_id);
                settle_budget_reservation(state, &session_id);
                finalized_sessions.push(session_id.clone());
                append_event(
                    state,
                    json!({
                        "type": "swarm_session_terminated",
                        "session_id": session_id,
                        "parent_id": parent_id.clone(),
                        "lineage_parent_id": parent_id.clone(),
                        "reason": terminate_reason,
                        "should_terminate": termination_contract.clone(),
                        "role_card": role_card.clone(),
                        "timestamp": now_iso(),
                    }),
                );
                result
            } else {
                let result = perform_persistent_check_in(session, "interval", false)?;
                if let Some(runtime) = session.persistent.as_mut() {
                    runtime.next_check_in_ms = now_ms
                        .saturating_add(runtime.config.check_in_interval_sec.saturating_mul(1000));
                }
                result
            };

            reports.push(json!({
                "session_id": session_id,
                "result": report,
                "should_terminate": termination_contract,
            }));
            local_processed = local_processed.saturating_add(1);
            check_ins = check_ins.saturating_add(1);
            if should_finalize {
                break;
            }
        }
        if local_processed > 0 {
            processed_sessions = processed_sessions.saturating_add(1);
        }
    }

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_tick",
        "processed_sessions": processed_sessions,
        "check_ins": check_ins,
        "finalized_sessions": finalized_sessions,
        "reports": reports,
    }))
}
