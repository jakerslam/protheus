fn run_scroll(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let direction = clean(
        parsed
            .flags
            .get("direction")
            .cloned()
            .unwrap_or_else(|| "down".to_string()),
        16,
    )
    .to_ascii_lowercase();
    let percentage = parsed
        .flags
        .get("percentage")
        .map(|raw| parse_u64(Some(raw), 80))
        .unwrap_or(80)
        .clamp(1, 200);
    if strict && !matches!(direction.as_str(), "up" | "down") {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_scroll",
            "error": "direction_invalid",
            "direction": direction,
            "session_id": sid
        });
    }

    let (path, mut session) = load_session_state(root, &sid);

    let viewport_width = session
        .pointer("/viewport/width")
        .and_then(Value::as_u64)
        .unwrap_or(1280);
    let viewport_height = session
        .pointer("/viewport/height")
        .and_then(Value::as_u64)
        .unwrap_or(720);
    let default_x = viewport_width / 2;
    let default_y = viewport_height / 2;
    let anchor_x = parsed
        .flags
        .get("x")
        .map(|raw| parse_u64(Some(raw), default_x))
        .unwrap_or(default_x)
        .min(16_384);
    let anchor_y = parsed
        .flags
        .get("y")
        .map(|raw| parse_u64(Some(raw), default_y))
        .unwrap_or(default_y)
        .min(16_384);

    let scrolled_pixels_u64 = ((viewport_height * percentage) / 100).max(1);
    let scrolled_pixels = i64::try_from(scrolled_pixels_u64).unwrap_or(1);
    let delta_y = if direction == "up" {
        -scrolled_pixels
    } else {
        scrolled_pixels
    };
    let prior_offset = session
        .pointer("/scroll/offset_y")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let new_offset = prior_offset.saturating_add(delta_y);

    session["updated_at"] = Value::String(crate::now_iso());
    session["scroll"] = json!({
        "offset_y": new_offset,
        "last_delta_y": delta_y,
        "last_percentage": percentage,
        "last_direction": direction,
        "last_anchor": {"x": anchor_x, "y": anchor_y},
        "ts": crate::now_iso()
    });
    let _ = write_json(&path, &session);

    let replay_step = json!({
        "type": "scroll",
        "delta_x": 0,
        "delta_y": delta_y,
        "anchor": {"x": anchor_x, "y": anchor_y}
    });
    let artifact = json!({
        "version": "v1",
        "session_id": sid.clone(),
        "direction": direction,
        "percentage": percentage,
        "scrolled_pixels": scrolled_pixels_u64,
        "delta_y": delta_y,
        "anchor": {"x": anchor_x, "y": anchor_y},
        "recorded_at": crate::now_iso(),
        "replay_step": replay_step
    });
    let artifact_path = persist_automation_artifact(root, "scroll_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_scroll",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "scroll": artifact,
        "session": session,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-010",
                "claim": "scroll_lane_records_bounded_directional_scroll_with_anchor_for_replay",
                "evidence": {
                    "direction": direction,
                    "percentage": percentage
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn parse_coordinate_pair(raw: &str) -> Option<(u64, u64)> {
    let parts = raw
        .split(',')
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let x = parts.first()?.parse::<u64>().ok()?;
    let y = parts.get(1)?.parse::<u64>().ok()?;
    Some((x, y))
}

fn run_click(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let describe = clean(
        parsed
            .flags
            .get("describe")
            .cloned()
            .unwrap_or_else(|| "target element".to_string()),
        180,
    );
    let (path, mut session) = load_session_state(root, &sid);
    let viewport_width = session
        .pointer("/viewport/width")
        .and_then(Value::as_u64)
        .unwrap_or(1280);
    let viewport_height = session
        .pointer("/viewport/height")
        .and_then(Value::as_u64)
        .unwrap_or(720);
    let default_x = viewport_width / 2;
    let default_y = viewport_height / 2;
    let coordinates_pair = parsed
        .flags
        .get("coordinates")
        .and_then(|raw| parse_coordinate_pair(raw));
    let mut x = coordinates_pair.map(|(cx, _)| cx).unwrap_or(default_x);
    let mut y = coordinates_pair.map(|(_, cy)| cy).unwrap_or(default_y);
    x = parsed
        .flags
        .get("x")
        .map(|raw| parse_u64(Some(raw), x))
        .unwrap_or(x);
    y = parsed
        .flags
        .get("y")
        .map(|raw| parse_u64(Some(raw), y))
        .unwrap_or(y);
    x = x.min(16_384);
    y = y.min(16_384);
    if strict && describe.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_click",
            "error": "description_required",
            "session_id": sid
        });
    }

    session["updated_at"] = Value::String(crate::now_iso());
    session["last_click"] = json!({
        "describe": describe.clone(),
        "x": x,
        "y": y,
        "ts": crate::now_iso()
    });
    let _ = write_json(&path, &session);

    let replay_step = json!({
        "type": "click",
        "instruction": describe,
        "playwright_arguments": {"x": x, "y": y}
    });
    let artifact = json!({
        "version": "v1",
        "session_id": sid.clone(),
        "describe": describe,
        "coordinates": [x, y],
        "recorded_at": crate::now_iso(),
        "replay_step": replay_step
    });
    let artifact_path = persist_automation_artifact(root, "click_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_click",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "click": artifact,
        "session": session,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-011",
                "claim": "click_lane_records_coordinate_click_steps_for_deterministic_replay",
                "evidence": {
                    "x": x,
                    "y": y
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_type(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let describe = clean(
        parsed
            .flags
            .get("describe")
            .cloned()
            .unwrap_or_else(|| "target input".to_string()),
        180,
    );
    let text = clean(parsed.flags.get("text").cloned().unwrap_or_default(), 400);
    if strict && text.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_type",
            "error": "text_required",
            "session_id": sid
        });
    }
    let variables_json_raw = parsed
        .flags
        .get("variables-json")
        .or_else(|| parsed.flags.get("variables_json"))
        .map(|raw| raw.trim().to_string())
        .unwrap_or_default();
    let variables = if variables_json_raw.is_empty() {
        None
    } else {
        match serde_json::from_str::<Value>(&variables_json_raw) {
            Ok(Value::Object(map)) => Some(map),
            _ => {
                if strict {
                    return json!({
                        "ok": false,
                        "strict": strict,
                        "type": "vbrowser_plane_type",
                        "error": "variables_json_invalid",
                        "session_id": sid
                    });
                }
                None
            }
        }
    };
    let resolved_text = if let Some(map) = variables.as_ref() {
        substitute_key_input_variables(&text, map)
    } else {
        text.clone()
    };

    let (path, mut session) = load_session_state(root, &sid);
    let viewport_width = session
        .pointer("/viewport/width")
        .and_then(Value::as_u64)
        .unwrap_or(1280);
    let viewport_height = session
        .pointer("/viewport/height")
        .and_then(Value::as_u64)
        .unwrap_or(720);
    let default_x = viewport_width / 2;
    let default_y = viewport_height / 2;
    let coordinates_pair = parsed
        .flags
        .get("coordinates")
        .and_then(|raw| parse_coordinate_pair(raw));
    let mut x = coordinates_pair.map(|(cx, _)| cx).unwrap_or(default_x);
    let mut y = coordinates_pair.map(|(_, cy)| cy).unwrap_or(default_y);
    x = parsed
        .flags
        .get("x")
        .map(|raw| parse_u64(Some(raw), x))
        .unwrap_or(x)
        .min(16_384);
    y = parsed
        .flags
        .get("y")
        .map(|raw| parse_u64(Some(raw), y))
        .unwrap_or(y)
        .min(16_384);

    session["updated_at"] = Value::String(crate::now_iso());
    session["last_type"] = json!({
        "describe": describe.clone(),
        "x": x,
        "y": y,
        "text": text.clone(),
        "resolved_text": resolved_text.clone(),
        "ts": crate::now_iso()
    });
    let _ = write_json(&path, &session);

    let replay_step = json!({
        "type": "type",
        "instruction": describe.clone(),
        "playwright_arguments": {"x": x, "y": y, "text": resolved_text}
    });
    let artifact = json!({
        "version": "v1",
        "session_id": sid.clone(),
        "describe": describe,
        "text": text,
        "resolved_text": resolved_text,
        "coordinates": [x, y],
        "recorded_at": crate::now_iso(),
        "replay_step": replay_step
    });
    let artifact_path = persist_automation_artifact(root, "type_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_type",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "type_input": artifact,
        "session": session,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-012",
                "claim": "type_lane_records_coordinate_type_steps_with_variable_substitution_for_replay",
                "evidence": {
                    "x": x,
                    "y": y
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}
