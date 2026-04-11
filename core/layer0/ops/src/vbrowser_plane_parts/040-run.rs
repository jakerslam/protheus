fn stamp_receipt(out: &mut Value) {
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(out));
}

fn default_session_state(session_id: &str) -> Value {
    json!({
        "version": "v1",
        "session_id": session_id,
        "target_url": "about:blank",
        "started_at": crate::now_iso()
    })
}

fn load_session_state(root: &Path, session_id: &str) -> (PathBuf, Value) {
    let path = session_state_path(root, session_id);
    let session = read_json(&path).unwrap_or_else(|| default_session_state(session_id));
    (path, session)
}

fn persist_automation_artifact(root: &Path, file_name: &str, artifact: &Value) -> PathBuf {
    let path = state_root(root).join("automation").join(file_name);
    let _ = write_json(&path, artifact);
    path
}

fn run_auth_login(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let provider = clean_id(parsed.flags.get("provider").map(String::as_str), "default");
    let profile = clean_id(parsed.flags.get("profile").map(String::as_str), "default");
    let vault = load_auth_vault(root);
    let selected = vault
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|row| {
            row.get("provider").and_then(Value::as_str) == Some(provider.as_str())
                && row.get("profile").and_then(Value::as_str) == Some(profile.as_str())
        });
    let Some(entry) = selected else {
        return json!({
            "ok": !strict,
            "strict": strict,
            "type": "vbrowser_plane_auth_login",
            "lane": "core/layer0/ops",
            "error": "profile_not_found",
            "provider": provider,
            "profile": profile
        });
    };
    let secret = entry
        .get("secret")
        .and_then(|v| decrypt_secret(root, v))
        .unwrap_or_default();
    if strict && secret.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_auth_login",
            "lane": "core/layer0/ops",
            "error": "decrypt_failed",
            "provider": provider,
            "profile": profile
        });
    }
    let token = sha256_hex_str(&format!("{}:{}:{}", provider, profile, secret));
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_auth_login",
        "lane": "core/layer0/ops",
        "provider": provider,
        "profile": profile,
        "session_token_hint": &token[..16],
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.4",
                "claim": "auth_profiles_enable_deterministic_login_without_plaintext_secret_exposure",
                "evidence": {"provider": provider, "profile": profile}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_native(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let raw_url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .unwrap_or_else(|| "about:blank".to_string()),
        400,
    );
    let url = normalize_target_url(&raw_url);
    let session = json!({
        "version": "v1",
        "session_id": sid,
        "target_url": url,
        "origin": "protheusctl-browser-native",
        "native_mode": true,
        "host_state_access": false,
        "started_at": crate::now_iso()
    });
    let path = session_state_path(root, &sid);
    let _ = write_json(&path, &session);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_native",
        "lane": "core/layer0/ops",
        "session": session,
        "artifact": {"path": path.display().to_string()},
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.5",
                "claim": "native_cli_browser_surface_routes_to_core_vbrowser_runtime",
                "evidence": {"session_id": sid}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn normalize_wait_until(raw: Option<&String>, fallback: &str) -> String {
    let wait_until_raw =
        clean(raw.cloned().unwrap_or_else(|| fallback.to_string()), 32).to_ascii_lowercase();
    match wait_until_raw.as_str() {
        "load" | "domcontentloaded" | "networkidle" | "commit" => wait_until_raw,
        _ => fallback.to_string(),
    }
}

fn run_goto(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let raw_url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .unwrap_or_else(|| "about:blank".to_string()),
        400,
    );
    let url = normalize_target_url(&raw_url);
    let wait_until = normalize_wait_until(
        parsed
            .flags
            .get("wait-until")
            .or_else(|| parsed.flags.get("wait_until")),
        "load",
    );

    let (path, mut session) = load_session_state(root, &sid);
    session["target_url"] = Value::String(url.clone());
    session["updated_at"] = Value::String(crate::now_iso());
    session["last_navigation"] = json!({
        "url": url,
        "wait_until": wait_until,
        "ts": crate::now_iso()
    });
    let _ = write_json(&path, &session);

    let artifact = json!({
        "version": "v1",
        "session_id": sid,
        "url": url,
        "wait_until": wait_until,
        "recorded_at": crate::now_iso(),
        "replay_step": {
            "type": "goto",
            "url": url,
            "wait_until": wait_until
        }
    });
    let artifact_path = persist_automation_artifact(root, "goto_latest.json", &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_goto",
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
                "id": "V11-STAGEHAND-007",
                "claim": "goto_lane_normalizes_urls_and_updates_session_navigation_state_for_replay",
                "evidence": {
                    "wait_until": wait_until
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

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

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let raw_command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(raw_command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = canonical_vbrowser_command(&raw_command).to_string();

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": vbrowser_receipt_type(&command),
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "session-start" => run_session_start(root, &parsed, strict),
        "session-control" => run_session_control(root, &parsed, strict),
        "goto" => run_goto(root, &parsed, strict),
        "navback" => run_navback(root, &parsed, strict),
        "wait" => run_wait(root, &parsed, strict),
        "scroll" => run_scroll(root, &parsed, strict),
        "click" => run_click(root, &parsed, strict),
        "type" => run_type(root, &parsed, strict),
        "automate" => run_automate(root, &parsed, strict),
        "key-input" => run_key_input(root, &parsed, strict),
        "privacy-guard" => run_privacy_guard(root, &parsed, strict),
        "snapshot" => run_snapshot(root, &parsed, strict),
        "screenshot" => run_screenshot(root, &parsed, strict),
        "action-policy" => run_action_policy(root, &parsed, strict),
        "auth-save" => run_auth_save(root, &parsed, strict),
        "auth-login" => run_auth_login(root, &parsed, strict),
        "native" => run_native(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "vbrowser_plane_error",
            "error": "unknown_command",
            "command": command,
            "requested_command": raw_command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_defaults() {
        let parsed = crate::parse_args(&["status".to_string()]);
        assert_eq!(session_id(&parsed), "browser-session");
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["start".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "session-start");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn alias_commands_are_canonicalized_for_claims() {
        assert_eq!(canonical_vbrowser_command("open"), "session-start");
        assert_eq!(canonical_vbrowser_command("navigate"), "goto");
        assert_eq!(canonical_vbrowser_command("control"), "session-control");
        assert_eq!(canonical_vbrowser_command("keys"), "key-input");
        assert_eq!(canonical_vbrowser_command("privacy"), "privacy-guard");
    }

    #[test]
    fn alias_bypass_emits_action_specific_receipt_type() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "open".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ],
        );
        assert_eq!(exit, 1);
        let latest = crate::v8_kernel::read_json(&state_root(root.path()).join("latest.json"))
            .expect("latest vbrowser receipt");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("vbrowser_plane_session_start")
        );
        assert_eq!(
            latest
                .pointer("/conduit_enforcement/action")
                .and_then(Value::as_str),
            Some("session-start")
        );
    }
}
