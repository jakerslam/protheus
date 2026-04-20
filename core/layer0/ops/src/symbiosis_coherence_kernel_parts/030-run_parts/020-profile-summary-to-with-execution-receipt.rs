
fn profile_summary(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let state = load_profile_state(&path);
    let settings = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    let checklist = profile_checklist(&settings);
    let deltas = state
        .get("deltas")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_summary",
        "path_rel": rel_path(root, &path),
        "settings": settings,
        "first_run_checklist": checklist,
        "edit_later_path": state
            .get("edit_later_path")
            .cloned()
            .unwrap_or_else(|| Value::String(profile_edit_later_path().to_string())),
        "delta_count": deltas.len(),
        "last_delta": deltas.last().cloned().unwrap_or(Value::Null),
        "updated_at": state.get("updated_at").cloned().unwrap_or(Value::Null)
    }))
}

fn profile_update(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let mut state = load_profile_state(&path);
    let existing = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    let existing_tone = normalize_tone(existing.get("tone"), "collaborative");
    let existing_depth = clamp_number(existing.get("depth"), 0.0, 1.0, 0.62);
    let existing_initiative = clamp_number(existing.get("initiative"), 0.0, 1.0, 0.54);
    let existing_tool = clamp_number(existing.get("tool_aggressiveness"), 0.0, 1.0, 0.34);
    let existing_response_style =
        normalize_response_style(existing.get("response_style"), "balanced");
    let existing_detail_level = normalize_detail_level(existing.get("detail_level"), "standard");
    let existing_proactivity = normalize_proactivity_tolerance(
        existing.get("proactivity_tolerance"),
        "medium",
    );
    let existing_risk = normalize_risk_appetite(existing.get("risk_appetite"), "balanced");

    let explicit = payload.get("explicit_feedback").and_then(Value::as_object);
    let implicit = payload.get("interaction_signals").and_then(Value::as_object);

    let target_tone = normalize_tone(
        payload
            .get("tone")
            .or_else(|| explicit.and_then(|row| row.get("tone"))),
        existing_tone.as_str(),
    );
    let response_style = normalize_response_style(
        payload
            .get("response_style")
            .or_else(|| explicit.and_then(|row| row.get("response_style"))),
        existing_response_style.as_str(),
    );
    let detail_level = normalize_detail_level(
        payload
            .get("detail_level")
            .or_else(|| explicit.and_then(|row| row.get("detail_level"))),
        existing_detail_level.as_str(),
    );
    let proactivity_tolerance = normalize_proactivity_tolerance(
        payload
            .get("proactivity_tolerance")
            .or_else(|| explicit.and_then(|row| row.get("proactivity_tolerance"))),
        existing_proactivity.as_str(),
    );
    let risk_appetite = normalize_risk_appetite(
        payload
            .get("risk_appetite")
            .or_else(|| explicit.and_then(|row| row.get("risk_appetite"))),
        existing_risk.as_str(),
    );
    let depth = (
        clamp_number(
            payload
                .get("depth")
                .or_else(|| explicit.and_then(|row| row.get("depth"))),
            0.0,
            1.0,
            existing_depth,
        ) + signed_delta(explicit.and_then(|row| row.get("depth_delta")), 0.0)
            + signed_delta(implicit.and_then(|row| row.get("depth_delta")), 0.0)
    )
    .clamp(0.0, 1.0);
    let initiative = (
        clamp_number(
            payload
                .get("initiative")
                .or_else(|| explicit.and_then(|row| row.get("initiative"))),
            0.0,
            1.0,
            existing_initiative,
        ) + signed_delta(explicit.and_then(|row| row.get("initiative_delta")), 0.0)
            + signed_delta(implicit.and_then(|row| row.get("initiative_delta")), 0.0)
    )
    .clamp(0.0, 1.0);
    let tool_aggressiveness = (
        clamp_number(
            payload
                .get("tool_aggressiveness")
                .or_else(|| explicit.and_then(|row| row.get("tool_aggressiveness"))),
            0.0,
            1.0,
            existing_tool,
        ) + signed_delta(
            explicit.and_then(|row| row.get("tool_aggressiveness_delta")),
            0.0,
        ) + signed_delta(
            implicit.and_then(|row| row.get("tool_aggressiveness_delta")),
            0.0,
        )
    )
    .clamp(0.0, 1.0);
    let now = now_iso();
    let next_settings = json!({
        "tone": target_tone,
        "depth": round_to(depth, 4),
        "initiative": round_to(initiative, 4),
        "tool_aggressiveness": round_to(tool_aggressiveness, 4),
        "response_style": response_style,
        "detail_level": detail_level,
        "proactivity_tolerance": proactivity_tolerance,
        "risk_appetite": risk_appetite
    });

    let delta = json!({
        "ts": now,
        "source": normalize_token(payload.get("source"), 64),
        "from": existing,
        "to": next_settings,
        "explicit_feedback": explicit.cloned().map(Value::Object).unwrap_or(Value::Null),
        "interaction_signals": implicit.cloned().map(Value::Object).unwrap_or(Value::Null)
    });
    let mut deltas = state
        .get("deltas")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    deltas.push(delta.clone());
    let start = deltas.len().saturating_sub(120);
    deltas = deltas[start..].to_vec();

    state["version"] = Value::String("1.0".to_string());
    state["updated_at"] = Value::String(now.clone());
    state["edit_later_path"] = Value::String(profile_edit_later_path().to_string());
    state["settings"] = next_settings.clone();
    state["deltas"] = Value::Array(deltas);
    write_json(&path, &state)?;

    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_update",
        "path_rel": rel_path(root, &path),
        "settings": next_settings,
        "delta": delta,
        "first_run_checklist": profile_checklist(&state["settings"]),
        "edit_later_path": profile_edit_later_path()
    }))
}

fn profile_reset(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let mut state = default_profile_state();
    state["updated_at"] = Value::String(now_iso());
    state["edit_later_path"] = Value::String(profile_edit_later_path().to_string());
    write_json(&path, &state)?;
    let settings = state.get("settings").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_reset",
        "path_rel": rel_path(root, &path),
        "settings": settings.clone(),
        "first_run_checklist": profile_checklist(&settings),
        "edit_later_path": profile_edit_later_path()
    }))
}

fn profile_checklist_cmd(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = profile_state_path(root, payload);
    let state = load_profile_state(&path);
    let settings = state.get("settings").cloned().unwrap_or_else(|| json!({}));
    Ok(json!({
        "ok": true,
        "type": "symbiosis_profile_checklist",
        "path_rel": rel_path(root, &path),
        "settings": settings.clone(),
        "first_run_checklist": profile_checklist(&settings),
        "edit_later_path": state
            .get("edit_later_path")
            .cloned()
            .unwrap_or_else(|| Value::String(profile_edit_later_path().to_string()))
    }))
}

fn with_execution_receipt(command: &str, status: &str, payload: Value) -> Value {
    json!({
        "execution_receipt": {
            "lane": "symbiosis_coherence_kernel",
            "command": command,
            "status": status,
            "source": "OPENCLAW-TOOLING-WEB-098",
            "tool_runtime_class": "receipt_wrapped"
        },
        "payload": payload
    })
}
