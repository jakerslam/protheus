fn load_policy(policy_path: &Path) -> Policy {
    let mut out = default_policy();
    let raw = read_json(policy_path);
    if !raw.is_object() {
        return out;
    }

    if let Some(v) = raw.get("version").and_then(Value::as_str) {
        let clean = normalize_token(v, 24);
        if !clean.is_empty() {
            out.version = clean;
        }
    }
    out.enabled = raw
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(out.enabled);
    out.shadow_mode = raw
        .get("shadow_mode")
        .and_then(Value::as_bool)
        .unwrap_or(out.shadow_mode);

    if let Some(sleep) = raw.get("sleep_window_local") {
        out.sleep_window_local.enabled = sleep
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(out.sleep_window_local.enabled);
        out.sleep_window_local.start_hour = sleep
            .get("start_hour")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.sleep_window_local.start_hour)
            .clamp(0, 23);
        out.sleep_window_local.end_hour = sleep
            .get("end_hour")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.sleep_window_local.end_hour)
            .clamp(0, 23);
    }

    if let Some(gating) = raw.get("gating") {
        out.gating.min_consecutive_failures = gating
            .get("min_consecutive_failures")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.gating.min_consecutive_failures)
            .clamp(1, 20);
        out.gating.max_actions_per_run = gating
            .get("max_actions_per_run")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.gating.max_actions_per_run)
            .clamp(1, 100);
        out.gating.cooldown_sec_per_signature = gating
            .get("cooldown_sec_per_signature")
            .and_then(Value::as_i64)
            .unwrap_or(out.gating.cooldown_sec_per_signature)
            .clamp(0, 7 * 24 * 60 * 60);
        out.gating.max_repairs_per_signature_per_day = gating
            .get("max_repairs_per_signature_per_day")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.gating.max_repairs_per_signature_per_day)
            .clamp(1, 20);
    }

    if let Some(kill) = raw.get("kill_switch") {
        out.kill_switch.enabled = kill
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(out.kill_switch.enabled);
        out.kill_switch.window_hours = kill
            .get("window_hours")
            .and_then(Value::as_i64)
            .unwrap_or(out.kill_switch.window_hours)
            .clamp(1, 24 * 30);
        out.kill_switch.max_unknown_signatures_per_window = kill
            .get("max_unknown_signatures_per_window")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.kill_switch.max_unknown_signatures_per_window)
            .clamp(1, 1000);
        out.kill_switch.max_suspicious_signatures_per_window = kill
            .get("max_suspicious_signatures_per_window")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.kill_switch.max_suspicious_signatures_per_window)
            .clamp(1, 1000);
        out.kill_switch.max_repairs_per_window = kill
            .get("max_repairs_per_window")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.kill_switch.max_repairs_per_window)
            .clamp(1, 2000);
        out.kill_switch.max_rollbacks_per_window = kill
            .get("max_rollbacks_per_window")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.kill_switch.max_rollbacks_per_window)
            .clamp(1, 2000);
        out.kill_switch.max_same_signature_repairs_per_window = kill
            .get("max_same_signature_repairs_per_window")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(out.kill_switch.max_same_signature_repairs_per_window)
            .clamp(1, 2000);
        out.kill_switch.auto_reset_hours = kill
            .get("auto_reset_hours")
            .and_then(Value::as_i64)
            .unwrap_or(out.kill_switch.auto_reset_hours)
            .clamp(1, 24 * 30);
    }

    if let Some(recipes) = raw.get("recipes").and_then(Value::as_array) {
        let mut by_kind = HashMap::new();
        for recipe in recipes {
            let kind = recipe
                .get("applies_to")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(Value::as_str)
                .map(|s| normalize_token(s, 80))
                .unwrap_or_default();
            if kind.is_empty() {
                continue;
            }
            let steps = recipe
                .get("steps")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(|s| normalize_token(s, 120))
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if !steps.is_empty() {
                by_kind.insert(kind, steps);
            }
        }
        if !by_kind.is_empty() {
            out.recipes = by_kind;
        }
    }

    out
}

fn load_doctor_state(paths: &RuntimePaths) -> DoctorState {
    serde_json::from_value::<DoctorState>(read_json(&paths.state_path)).unwrap_or_else(|_| {
        DoctorState {
            updated_at: Some(now_iso()),
            signatures: HashMap::new(),
            history: Vec::new(),
            kill_switch: KillSwitchState::default(),
        }
    })
}

fn prune_history(state: &mut DoctorState, window_hours: i64, max_events: usize) {
    let cutoff = chrono::Utc::now().timestamp_millis() - (window_hours * 60 * 60 * 1000);
    state.history.retain(|row| {
        row.get("ts")
            .and_then(Value::as_str)
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|ts| ts.timestamp_millis() >= cutoff)
            .unwrap_or(false)
    });
    if state.history.len() > max_events {
        let trim = state.history.len() - max_events;
        state.history.drain(0..trim);
    }
}

fn count_history(state: &DoctorState, event_type: &str, signature_id: Option<&str>) -> u32 {
    state
        .history
        .iter()
        .filter(|row| {
            if row.get("type").and_then(Value::as_str).unwrap_or_default() != event_type {
                return false;
            }
            if let Some(sig) = signature_id {
                return row
                    .get("signature_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    == sig;
            }
            true
        })
        .count() as u32
}

fn record_history_event(state: &mut DoctorState, event_type: &str, payload: Value) {
    let mut event = json!({
        "ts": now_iso(),
        "type": event_type,
    });
    if let (Some(dst), Some(src)) = (event.as_object_mut(), payload.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    state.history.push(event);
}

fn maybe_auto_release_kill_switch(state: &mut DoctorState, policy: &Policy) {
    if !state.kill_switch.engaged {
        return;
    }
    let auto_release_at = state
        .kill_switch
        .auto_release_at
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|ts| ts.timestamp_millis())
        .unwrap_or(i64::MAX);
    if chrono::Utc::now().timestamp_millis() >= auto_release_at {
        state.kill_switch.engaged = false;
        state.kill_switch.reason = Some("auto_release".to_string());
        state.kill_switch.engaged_at = None;
        state.kill_switch.auto_release_at = None;
        record_history_event(state, "kill_switch_auto_release", Value::Null);
    } else if state.kill_switch.auto_release_at.is_none() {
        let release =
            chrono::Utc::now() + chrono::Duration::hours(policy.kill_switch.auto_reset_hours);
        state.kill_switch.auto_release_at = Some(release.to_rfc3339());
    }
}

fn engage_kill_switch(state: &mut DoctorState, reason: &str, meta: Value, policy: &Policy) {
    state.kill_switch.engaged = true;
    state.kill_switch.reason = Some(clean_text(reason, 180));
    state.kill_switch.engaged_at = Some(now_iso());
    state.kill_switch.auto_release_at = Some(
        (chrono::Utc::now() + chrono::Duration::hours(policy.kill_switch.auto_reset_hours))
            .to_rfc3339(),
    );
    state.kill_switch.last_trip_meta = Some(meta.clone());
    record_history_event(
        state,
        "kill_switch_engaged",
        json!({"reason": reason, "meta": meta}),
    );
}

fn within_sleep_window(cfg: &SleepWindow) -> bool {
    if !cfg.enabled {
        return true;
    }
    let hour = chrono::Local::now().hour();
    if cfg.start_hour == cfg.end_hour {
        return true;
    }
    if cfg.start_hour < cfg.end_hour {
        hour >= cfg.start_hour && hour < cfg.end_hour
    } else {
        hour >= cfg.start_hour || hour < cfg.end_hour
    }
}

fn evaluate_kill_switch(state: &DoctorState, policy: &Policy) -> Option<(String, Value)> {
    if !policy.kill_switch.enabled {
        return None;
    }
    let unknown = count_history(state, "unknown_signature", None);
    if unknown >= policy.kill_switch.max_unknown_signatures_per_window {
        return Some((
            "kill_unknown_signature_spike".to_string(),
            json!({
                "count": unknown,
                "threshold": policy.kill_switch.max_unknown_signatures_per_window
            }),
        ));
    }

    let suspicious = count_history(state, "suspicious_signature", None);
    if suspicious >= policy.kill_switch.max_suspicious_signatures_per_window {
        return Some((
            "kill_suspicious_signature_spike".to_string(),
            json!({
                "count": suspicious,
                "threshold": policy.kill_switch.max_suspicious_signatures_per_window
            }),
        ));
    }

    let repairs = count_history(state, "repair_attempt", None);
    if repairs >= policy.kill_switch.max_repairs_per_window {
        return Some((
            "kill_repair_spike".to_string(),
            json!({
                "count": repairs,
                "threshold": policy.kill_switch.max_repairs_per_window
            }),
        ));
    }

    None
}

fn classify_failure_kind(result: &Value) -> String {
    if result
        .get("guard_ok")
        .and_then(Value::as_bool)
        .is_some_and(|ok| !ok)
    {
        return "guard_blocked".to_string();
    }
    if result
        .get("flaky")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "flaky".to_string();
    }
    let err_blob = format!(
        "{} {} {}",
        result
            .get("stderr_excerpt")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        result
            .get("stdout_excerpt")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        result
            .get("guard_reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
    )
    .to_ascii_lowercase();

    if err_blob.contains("etimedout")
        || err_blob.contains("timeout")
        || err_blob.contains("process_timeout")
        || err_blob.contains("timed out")
    {
        return "timeout".to_string();
    }
    let exit_code = result.get("exit_code").and_then(Value::as_i64).unwrap_or(0);
    if exit_code != 0 {
        return "exit_nonzero".to_string();
    }
    "assertion_failed".to_string()
}
