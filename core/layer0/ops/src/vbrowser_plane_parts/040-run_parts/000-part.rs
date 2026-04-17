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
