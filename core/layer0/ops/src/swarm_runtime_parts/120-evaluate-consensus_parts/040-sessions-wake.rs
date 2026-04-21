
fn sessions_wake(state: &mut SwarmState, session_id: &str, now_ms: u64) -> Result<Value, String> {
    let Some(session) = state.sessions.get_mut(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    if session.persistent.is_none() {
        return Err(format!("session_not_persistent:{session_id}"));
    }
    if !matches!(
        session.status.as_str(),
        "persistent_running" | "background_running"
    ) {
        return Err(format!("session_not_active:{session_id}"));
    }
    let report = perform_persistent_check_in(session, "manual_wake", false)?;
    if let Some(runtime) = session.persistent.as_mut() {
        runtime.next_check_in_ms =
            now_ms.saturating_add(runtime.config.check_in_interval_sec.saturating_mul(1000));
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_wake",
        "session_id": session_id,
        "report": report,
    }))
}
