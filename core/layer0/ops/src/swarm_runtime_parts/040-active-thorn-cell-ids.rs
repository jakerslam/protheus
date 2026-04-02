fn active_thorn_cell_ids(state: &SwarmState) -> Vec<String> {
    state
        .sessions
        .iter()
        .filter(|(_, session)| session.thorn_cell && session.status == "thorn_active")
        .map(|(id, _)| id.clone())
        .collect()
}

fn set_service_health(state: &mut SwarmState, session_id: &str, healthy: bool) {
    for rows in state.service_registry.values_mut() {
        for row in rows.iter_mut() {
            if row.session_id == session_id {
                row.healthy = healthy;
            }
        }
    }
}

fn thorn_replacement_sessions(
    state: &SwarmState,
    target_id: &str,
    role: Option<&str>,
) -> Vec<String> {
    let Some(role) = role else {
        return Vec::new();
    };
    state
        .service_registry
        .get(role)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.session_id != target_id && row.healthy)
        .filter_map(|row| {
            state.sessions.get(&row.session_id).and_then(|session| {
                if session.reachable && !session.thorn_cell {
                    Some(row.session_id)
                } else {
                    None
                }
            })
        })
        .collect()
}

fn restore_quarantined_target(state: &mut SwarmState, target_id: &str, reason: &str, now_ms: u64) {
    if let Some(target) = state.sessions.get_mut(target_id) {
        target.reachable = true;
        target.status = target
            .quarantine_previous_status
            .clone()
            .unwrap_or_else(|| "running".to_string());
        target.quarantine_previous_status = None;
        target.quarantine_reason = Some(reason.to_string());
        target.context_vars.insert(
            "thorn_release".to_string(),
            json!({
                "released_at_ms": now_ms,
                "reason": reason,
            }),
        );
    }
    set_service_health(state, target_id, true);
}

fn release_thorn_cells_for_target(
    state: &mut SwarmState,
    target_id: &str,
    reason: &str,
    now_ms: u64,
) -> Result<Value, String> {
    let thorn_ids = state
        .sessions
        .iter()
        .filter(|(_, session)| {
            session.thorn_cell
                && session.status == "thorn_active"
                && session.thorn_target_session_id.as_deref() == Some(target_id)
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if thorn_ids.is_empty() {
        return Err(format!("no_active_thorn_cell_for_target:{target_id}"));
    }

    for thorn_id in &thorn_ids {
        if let Some(thorn) = state.sessions.get_mut(thorn_id) {
            thorn.status = "thorn_destroyed".to_string();
            thorn.reachable = false;
            thorn.quarantine_reason = Some(reason.to_string());
            thorn.thorn_expires_at_ms = Some(now_ms);
        }
        set_service_health(state, thorn_id, false);
        append_event(
            state,
            json!({
                "type": "swarm_thorn_self_destruct",
                "thorn_session_id": thorn_id,
                "target_session_id": target_id,
                "reason": reason,
                "timestamp": now_iso(),
                "timestamp_ms": now_ms,
            }),
        );
    }
    restore_quarantined_target(state, target_id, reason, now_ms);
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_thorn_release",
        "target_session_id": target_id,
        "thorn_session_ids": thorn_ids,
        "released_reason": reason,
        "released_at_ms": now_ms,
    }))
}

fn quarantine_into_thorn(
    state: &mut SwarmState,
    target_id: &str,
    anomaly_type: &str,
    reason: &str,
    now_ms: u64,
) -> Result<Value, String> {
    let cap = thorn_cell_limit(state);
    let active_before = active_thorn_cell_ids(state);
    if active_before.len() >= cap {
        return Err(format!(
            "thorn_capacity_exceeded:active={}:cap={cap}",
            active_before.len()
        ));
    }
    let (target_depth, target_role, target_status) = {
        let target = state
            .sessions
            .get(target_id)
            .ok_or_else(|| format!("unknown_session:{target_id}"))?;
        if target.thorn_cell {
            return Err(format!("target_is_thorn_cell:{target_id}"));
        }
        if target.status == "quarantined_thorn" {
            return Err(format!("session_already_quarantined:{target_id}"));
        }
        (target.depth, target.role.clone(), target.status.clone())
    };
    let thorn_id = next_session_id(
        state,
        &format!("thorn:{target_id}"),
        target_depth.saturating_add(1),
    );
    let replacement_sessions = thorn_replacement_sessions(state, target_id, target_role.as_deref());

    {
        let target = state
            .sessions
            .get_mut(target_id)
            .ok_or_else(|| format!("unknown_session:{target_id}"))?;
        target.reachable = false;
        target.quarantine_previous_status = Some(target_status.clone());
        target.status = "quarantined_thorn".to_string();
        target.quarantine_reason = Some(reason.to_string());
        target
            .anomalies
            .push(format!("thorn:{anomaly_type}:{reason}"));
        target.context_vars.insert(
            "thorn_quarantine".to_string(),
            json!({
                "thorn_session_id": thorn_id,
                "reason": reason,
                "anomaly_type": anomaly_type,
                "quarantined_at_ms": now_ms,
            }),
        );
    }
    set_service_health(state, target_id, false);

    let mut thorn = session_metadata_base(
        thorn_id.clone(),
        Some(target_id.to_string()),
        target_depth.saturating_add(1),
        format!("thorn quarantine for {target_id}"),
        "thorn_active".to_string(),
    );
    thorn.report = Some(json!({
        "restricted": true,
        "outbound_network": false,
        "tool_execution": false,
        "memory_access": "limited",
    }));
    thorn.role = Some("thorn_cell".to_string());
    thorn.agent_label = Some(format!("thorn-{target_id}"));
    thorn.tool_access = thorn_session_tool_access();
    thorn.context_vars = BTreeMap::from([
        (
            "restricted_capabilities".to_string(),
            json!({
                "outbound_network": false,
                "tool_execution": false,
                "memory_access": "limited",
            }),
        ),
        ("anomaly_type".to_string(), json!(anomaly_type)),
        ("reason".to_string(), json!(reason)),
        (
            "replacement_sessions".to_string(),
            json!(replacement_sessions.clone()),
        ),
    ]);
    thorn.context_mode = Some("thorn_quarantine".to_string());
    thorn.anomalies = vec![anomaly_type.to_string()];
    thorn.thorn_cell = true;
    thorn.thorn_target_session_id = Some(target_id.to_string());
    thorn.thorn_expires_at_ms = Some(now_ms.saturating_add(60_000));
    thorn.quarantine_reason = Some(reason.to_string());
    state.sessions.insert(thorn_id.clone(), thorn);
    state.mailboxes.insert(
        thorn_id.clone(),
        SessionMailbox {
            session_id: thorn_id.clone(),
            unread: Vec::new(),
            read: Vec::new(),
        },
    );
    register_service_instance(
        state,
        &thorn_id,
        Some("thorn_cell".to_string()),
        vec!["quarantine".to_string()],
    );
    append_event(
        state,
        json!({
            "type": "swarm_thorn_spawn",
            "thorn_session_id": thorn_id,
            "target_session_id": target_id,
            "anomaly_type": anomaly_type,
            "reason": reason,
            "timestamp": now_iso(),
            "timestamp_ms": now_ms,
        }),
    );
    append_event(
        state,
        json!({
            "type": "swarm_thorn_quarantine",
            "target_session_id": target_id,
            "thorn_session_id": thorn_id,
            "timestamp": now_iso(),
            "timestamp_ms": now_ms,
        }),
    );
    append_event(
        state,
        json!({
            "type": "swarm_thorn_reroute",
            "target_session_id": target_id,
            "replacement_sessions": replacement_sessions,
            "timestamp": now_iso(),
            "timestamp_ms": now_ms,
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_thorn_quarantine",
        "target_session_id": target_id,
        "thorn_session_id": thorn_id,
        "active_thorn_cells": active_before.len().saturating_add(1),
        "thorn_cell_cap": cap,
        "replacement_sessions": replacement_sessions,
        "spawn_latency_ms": 0,
        "ttl_sec": 60,
    }))
}

fn drain_expired_thorn_cells(state: &mut SwarmState, now_ms: u64) {
    let expired_targets = state
        .sessions
        .iter()
        .filter(|(_, session)| {
            session.thorn_cell
                && session.status == "thorn_active"
                && session
                    .thorn_expires_at_ms
                    .map(|deadline| deadline <= now_ms)
                    .unwrap_or(false)
        })
        .filter_map(|(_, session)| session.thorn_target_session_id.clone())
        .collect::<Vec<_>>();
    for target_id in expired_targets {
        let _ = release_thorn_cells_for_target(state, &target_id, "ttl_expired", now_ms);
    }
}

fn run_thorn_contract_in_state(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let sub = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let now_ms = now_epoch_ms();
    match sub.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "type": "swarm_runtime_thorn_status",
            "active_thorn_cells": active_thorn_cell_ids(state),
            "thorn_cell_cap": thorn_cell_limit(state),
            "quarantined_sessions": state.sessions.values().filter(|session| session.status == "quarantined_thorn").count(),
        })),
        "quarantine" => {
            let session_id = parse_flag(argv, "session-id")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "session_id_required".to_string())?;
            let anomaly_type = parse_flag(argv, "anomaly-type")
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "anomaly".to_string());
            let reason = parse_flag(argv, "reason")
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "thorn_quarantine".to_string());
            quarantine_into_thorn(state, &session_id, &anomaly_type, &reason, now_ms)
        }
        "release" => {
            let session_id = parse_flag(argv, "session-id")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "session_id_required".to_string())?;
            let reason = parse_flag(argv, "reason")
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "threat_removed".to_string());
            release_thorn_cells_for_target(state, &session_id, &reason, now_ms)
        }
        _ => Err(format!("unknown_thorn_subcommand:{sub}")),
    }
}

pub fn force_shutdown(root: &Path, argv: &[String]) -> Result<Value, String> {
    let state_file = state_path(root, argv);
    let mut state = load_state(&state_file)?;
    let reason = parse_flag(argv, "reason")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "t0_invariant_violation".to_string());
    let now_ms = now_epoch_ms();
    let affected = state.sessions.len();
    let session_ids = state.sessions.keys().cloned().collect::<Vec<_>>();
    for session in state.sessions.values_mut() {
        session.reachable = false;
        session.status = "shutdown_t0".to_string();
        session.quarantine_reason = Some(reason.clone());
    }
    for rows in state.service_registry.values_mut() {
        for row in rows.iter_mut() {
            row.healthy = false;
        }
    }
    append_event(
        &mut state,
        json!({
            "type": "swarm_force_shutdown",
            "reason": reason,
            "affected_sessions": session_ids,
            "timestamp": now_iso(),
            "timestamp_ms": now_ms,
        }),
    );
    state.updated_at = now_iso();
    save_state(&state_file, &state)?;
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_force_shutdown",
        "reason": parse_flag(argv, "reason").unwrap_or_else(|| "t0_invariant_violation".to_string()),
        "affected_sessions": affected,
        "state_path": state_file,
        "shutdown_at_ms": now_ms,
    }))
}

pub fn run_thorn_contract(root: &Path, argv: &[String]) -> (Value, i32) {
    let state_file = state_path(root, argv);
    let mut state = match load_state(&state_file) {
        Ok(state) => state,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "swarm_runtime_thorn_error",
                    "error": err,
                    "state_path": state_file,
                }),
                2,
            );
        }
    };
    let now_ms = now_epoch_ms();
    drain_expired_messages(&mut state, now_ms);
    drain_expired_thorn_cells(&mut state, now_ms);
    let result = run_thorn_contract_in_state(&mut state, argv);
    state.updated_at = now_iso();
    let save_result = save_state(&state_file, &state);
    match (result, save_result) {
        (Ok(mut payload), Ok(())) => {
            payload["claim_evidence"] = json!([{
                "id": "V6-SEC-THORN-001",
                "claim": "thorn_cells_quarantine_compromised_sessions_with_restricted_capabilities_and_receipted_reroute_self_destruct_flow",
                "evidence": {
                    "state_path": state_file,
                    "command": argv.first().cloned().unwrap_or_else(|| "status".to_string()),
                }
            }]);
            payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
            (payload, 0)
        }
        (Err(err), _) => (
            json!({
                "ok": false,
                "type": "swarm_runtime_thorn_error",
                "error": err,
                "state_path": state_file,
            }),
            2,
        ),
        (_, Err(err)) => (
            json!({
                "ok": false,
                "type": "swarm_runtime_thorn_error",
                "error": err,
                "state_path": state_file,
            }),
            2,
        ),
    }
}
