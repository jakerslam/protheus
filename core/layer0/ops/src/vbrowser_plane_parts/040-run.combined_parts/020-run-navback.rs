
fn run_navback(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let wait_until = normalize_wait_until(
        parsed
            .flags
            .get("wait-until")
            .or_else(|| parsed.flags.get("wait_until")),
        "domcontentloaded",
    );
    let (path, mut session) = load_session_state(root, &sid);
    let prior_url = clean(
        session
            .get("target_url")
            .and_then(Value::as_str)
            .unwrap_or("about:blank"),
        400,
    );
    session["updated_at"] = Value::String(crate::now_iso());
    session["last_navigation"] = json!({
        "kind": "back",
        "from_url": prior_url.clone(),
        "wait_until": wait_until.clone(),
        "ts": crate::now_iso()
    });
    let mut history = session
        .get("navigation_history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    history.push(json!({
        "kind": "back",
        "from_url": prior_url.clone(),
        "wait_until": wait_until.clone(),
        "ts": crate::now_iso()
    }));
    session["navigation_history"] = Value::Array(history);
    let _ = write_json(&path, &session);

    let artifact = json!({
        "version": "v1",
        "session_id": sid,
        "from_url": prior_url,
        "wait_until": wait_until.clone(),
        "recorded_at": crate::now_iso(),
        "replay_step": {
            "type": "navback",
            "wait_until": wait_until.clone()
        }
    });
    let artifact_path = persist_automation_artifact(root, "navback_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_navback",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "navigation": artifact,
        "session": session,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-008",
                "claim": "navback_lane_records_back_navigation_intent_with_wait_mode_for_replay",
                "evidence": {
                    "wait_until": wait_until
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_wait(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let time_ms = parsed
        .flags
        .get("time-ms")
        .or_else(|| parsed.flags.get("time_ms"))
        .or_else(|| parsed.flags.get("time"))
        .map(|raw| parse_u64(Some(raw), 0))
        .unwrap_or(0)
        .min(120_000);
    if time_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(time_ms));
    }

    let (path, mut session) = load_session_state(root, &sid);
    session["updated_at"] = Value::String(crate::now_iso());
    session["last_wait"] = json!({
        "time_ms": time_ms,
        "ts": crate::now_iso()
    });
    let _ = write_json(&path, &session);

    let artifact = json!({
        "version": "v1",
        "session_id": sid.clone(),
        "time_ms": time_ms,
        "recorded_at": crate::now_iso(),
        "replay_step": {
            "type": "wait",
            "time_ms": time_ms
        }
    });
    let artifact_path = persist_automation_artifact(root, "wait_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_wait",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "wait": artifact,
        "session": session,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-009",
                "claim": "wait_lane_records_deterministic_pause_step_for_agent_replay_and_session_state",
                "evidence": {
                    "time_ms": time_ms
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

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
