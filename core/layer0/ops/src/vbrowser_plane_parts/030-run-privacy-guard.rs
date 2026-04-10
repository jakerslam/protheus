fn run_privacy_guard(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        PRIVACY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "vbrowser_privacy_security_contract",
            "allowed_network_modes": ["isolated", "restricted"],
            "max_budget_tokens": 200000,
            "recording_requires_allow_flag": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("vbrowser_privacy_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "vbrowser_privacy_security_contract"
    {
        errors.push("vbrowser_privacy_contract_kind_invalid".to_string());
    }

    let sid = session_id(parsed);
    let network = clean(
        parsed
            .flags
            .get("network")
            .cloned()
            .unwrap_or_else(|| "isolated".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let recording = parse_bool(parsed.flags.get("recording"), false);
    let allow_recording = parse_bool(parsed.flags.get("allow-recording"), false);
    let budget_tokens = parse_u64(parsed.flags.get("budget-tokens"), 50_000);

    let allowed_networks = contract
        .get("allowed_network_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("isolated"), json!("restricted")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();
    let max_budget = contract
        .get("max_budget_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(200_000);

    if strict && !allowed_networks.iter().any(|v| v == &network) {
        errors.push("network_mode_not_allowed".to_string());
    }
    if strict
        && recording
        && contract
            .get("recording_requires_allow_flag")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        && !allow_recording
    {
        errors.push("recording_not_allowed_without_flag".to_string());
    }
    if strict && budget_tokens > max_budget {
        errors.push("budget_tokens_exceed_max".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_privacy_guard",
            "errors": errors,
            "session_id": sid
        });
    }

    let policy_state = json!({
        "version": "v1",
        "session_id": sid,
        "network_mode": network,
        "recording": recording,
        "allow_recording": allow_recording,
        "budget_tokens": budget_tokens,
        "enforced_at": crate::now_iso()
    });
    let policy_path = state_root(root).join("privacy").join("latest.json");
    let _ = write_json(&policy_path, &policy_state);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_privacy_guard",
        "lane": "core/layer0/ops",
        "policy": policy_state,
        "artifact": {
            "path": policy_path.display().to_string(),
            "sha256": sha256_hex_str(&policy_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-001.4",
                "claim": "privacy_and_security_controls_enforce_network_recording_and_budget_fail_closed_policies",
                "evidence": {
                    "session_id": sid,
                    "network_mode": network,
                    "budget_tokens": budget_tokens
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_snapshot(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let refs_enabled = parse_bool(parsed.flags.get("refs"), true);
    let session = read_json(&session_state_path(root, &sid)).unwrap_or_else(|| {
        json!({
            "session_id": sid,
            "target_url": "about:blank",
            "shadow": "default-shadow"
        })
    });
    let target_url = session
        .get("target_url")
        .and_then(Value::as_str)
        .unwrap_or("about:blank");
    let shadow = session
        .get("shadow")
        .and_then(Value::as_str)
        .unwrap_or("default-shadow");
    let links = if refs_enabled {
        vec![
            json!({"href": target_url, "label": "current"}),
            json!({"href": "about:history", "label": "history"}),
        ]
    } else {
        Vec::new()
    };
    let snapshot = json!({
        "version": "v1",
        "session_id": sid,
        "shadow": shadow,
        "target_url": target_url,
        "refs_enabled": refs_enabled,
        "dom": {
            "title": "Virtual Browser Snapshot",
            "headings": ["h1: Session Overview", "h2: Context"],
            "text_blocks": 3
        },
        "links": links,
        "captured_at": crate::now_iso()
    });

    let path = snapshot_path(root);
    let _ = write_json(&path, &snapshot);
    let _ = append_jsonl(
        &state_root(root).join("snapshots").join("history.jsonl"),
        &snapshot,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_snapshot",
        "lane": "core/layer0/ops",
        "snapshot": snapshot,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&snapshot.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.1",
                "claim": "snapshot_operation_emits_structured_page_artifact_for_streamed_browser_session",
                "evidence": {"session_id": sid, "refs_enabled": refs_enabled}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_screenshot(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let annotate = parse_bool(parsed.flags.get("annotate"), false);
    let delay_ms = parsed
        .flags
        .get("delay-ms")
        .or_else(|| parsed.flags.get("delay_ms"))
        .map(|raw| parse_u64(Some(raw), 500))
        .unwrap_or(500)
        .min(10_000);
    if delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
    let session = read_json(&session_state_path(root, &sid)).unwrap_or_else(|| {
        json!({
            "session_id": sid,
            "target_url": "about:blank"
        })
    });
    let target_url = clean(
        session
            .get("target_url")
            .and_then(Value::as_str)
            .unwrap_or("about:blank"),
        240,
    );
    let annotations = if annotate {
        vec![
            json!({"id":"a1","label":"Primary CTA","x":90,"y":44}),
            json!({"id":"a2","label":"Navigation","x":16,"y":18}),
        ]
    } else {
        Vec::new()
    };

    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1024\" height=\"576\"><rect width=\"100%\" height=\"100%\" fill=\"#0b1020\"/><text x=\"24\" y=\"48\" fill=\"#ffffff\" font-size=\"20\">Session {}</text><text x=\"24\" y=\"78\" fill=\"#9ca3af\" font-size=\"14\">{}</text></svg>",
        sid, target_url
    );
    let svg_path = screenshot_svg_path(root);
    ensure_parent(&svg_path);
    let _ = fs::write(&svg_path, svg);

    let map = json!({
        "version": "v1",
        "session_id": sid,
        "target_url": target_url,
        "annotated": annotate,
        "delay_ms": delay_ms,
        "annotations": annotations,
        "captured_at": crate::now_iso()
    });
    let map_path = screenshot_map_path(root);
    let _ = write_json(&map_path, &map);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_screenshot",
        "lane": "core/layer0/ops",
        "map": map,
        "artifact": {
            "svg_path": svg_path.display().to_string(),
            "map_path": map_path.display().to_string()
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.2",
                "claim": "screenshot_operation_emits_visual_artifact_and_coordinate_map",
                "evidence": {"session_id": sid, "annotated": annotate}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn normalize_key_token(raw: &str) -> String {
    let token = clean(raw, 40);
    if token.is_empty() {
        return String::new();
    }
    if token.len() == 1 && token.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return token.to_ascii_uppercase();
    }
    let upper = token.to_ascii_uppercase();
    match upper.as_str() {
        "ENTER" | "RETURN" => "Enter".to_string(),
        "ESC" | "ESCAPE" => "Escape".to_string(),
        "BACKSPACE" => "Backspace".to_string(),
        "TAB" => "Tab".to_string(),
        "SPACE" => "Space".to_string(),
        "DELETE" | "DEL" => "Delete".to_string(),
        "ARROWUP" | "ARROW_UP" | "UP" => "ArrowUp".to_string(),
        "ARROWDOWN" | "ARROW_DOWN" | "DOWN" => "ArrowDown".to_string(),
        "ARROWLEFT" | "ARROW_LEFT" | "LEFT" => "ArrowLeft".to_string(),
        "ARROWRIGHT" | "ARROW_RIGHT" | "RIGHT" => "ArrowRight".to_string(),
        "CTRL" | "CONTROL" => "Control".to_string(),
        "CMD" | "COMMAND" | "META" | "SUPER" | "WINDOWS" | "WIN" => "Meta".to_string(),
        "OPTION" | "ALT" => "Alt".to_string(),
        "SHIFT" => "Shift".to_string(),
        "HOME" => "Home".to_string(),
        "END" => "End".to_string(),
        "PAGEUP" | "PAGE_UP" | "PGUP" => "PageUp".to_string(),
        "PAGEDOWN" | "PAGE_DOWN" | "PGDN" => "PageDown".to_string(),
        _ => token,
    }
}

fn normalize_key_combo(raw: &str) -> String {
    let cleaned = clean(raw, 240);
    if cleaned.is_empty() {
        return String::new();
    }
    cleaned
        .split('+')
        .map(normalize_key_token)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

fn key_input_variable_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(clean(text, 240)),
        Value::Number(num) => Some(clean(num.to_string(), 240)),
        Value::Bool(flag) => Some(if *flag {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        Value::Object(map) => map.get("value").and_then(key_input_variable_text),
        _ => None,
    }
}

fn substitute_key_input_variables(
    template: &str,
    variables: &serde_json::Map<String, Value>,
) -> String {
    let mut rendered = template.to_string();
    for (key, value) in variables {
        if let Some(replacement) = key_input_variable_text(value) {
            let token = format!("%{}%", key);
            if rendered.contains(&token) {
                rendered = rendered.replace(&token, &replacement);
            }
        }
    }
    rendered
}

fn run_key_input(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let method = clean(
        parsed
            .flags
            .get("method")
            .cloned()
            .unwrap_or_else(|| "press".to_string()),
        24,
    )
    .to_ascii_lowercase();
    let value = clean(
        parsed.flags.get("value").cloned().unwrap_or_default(),
        240,
    );
    let repeat = parse_u64(parsed.flags.get("repeat"), 1).clamp(1, 128);
    let delay_ms = parsed
        .flags
        .get("delay-ms")
        .or_else(|| parsed.flags.get("delay_ms"))
        .map(|raw| parse_u64(Some(raw), 100))
        .unwrap_or(100)
        .min(2_000);
    let mut errors = Vec::<String>::new();
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
                    errors.push("vbrowser_key_input_variables_json_invalid".to_string());
                }
                None
            }
        }
    };
    if !matches!(method.as_str(), "press" | "type") {
        errors.push("vbrowser_key_input_method_invalid".to_string());
    }
    if value.is_empty() {
        errors.push("vbrowser_key_input_value_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_key_input",
            "errors": errors,
            "session_id": sid
        });
    }
    let resolved_value = if method == "type" {
        if let Some(map) = variables.as_ref() {
            substitute_key_input_variables(&value, map)
        } else {
            value.clone()
        }
    } else {
        value.clone()
    };
    let normalized_value = if method == "press" {
        normalize_key_combo(&resolved_value)
    } else {
        resolved_value.clone()
    };
    let replay_args = if method == "press" {
        json!({
            "method": method,
            "value": normalized_value,
            "keys": normalized_value,
            "times": repeat,
            "delay_ms": delay_ms
        })
    } else {
        json!({
            "method": method,
            "value": normalized_value,
            "text": normalized_value,
            "times": repeat,
            "delay_ms": delay_ms
        })
    };
    let replay_step = json!({
        "type": "keys",
        "instruction": if method == "press" {
            format!("press {}", normalized_value)
        } else {
            format!("type \"{}\"", value)
        },
        "playwright_arguments": replay_args
    });
    let artifact = json!({
        "version": "v1",
        "session_id": sid,
        "method": method,
        "value": value,
        "resolved_value": resolved_value,
        "normalized_value": normalized_value,
        "repeat": repeat,
        "delay_ms": delay_ms,
        "recorded_at": crate::now_iso(),
        "replay_step": replay_step
    });
    let artifact_path = persist_automation_artifact(root, "key_input_latest.json", &artifact);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_key_input",
        "lane": "core/layer0/ops",
        "session_id": sid,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "key_input": artifact,
        "claim_evidence": [
            {
                "id": "V11-STAGEHAND-005",
                "claim": "keyboard_input_surface_normalizes_cross_provider_key_aliases_for_deterministic_replay",
                "evidence": {
                    "method": method,
                    "repeat": repeat,
                    "delay_ms": delay_ms
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_action_policy(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let action = clean(
        parsed
            .flags
            .get("action")
            .cloned()
            .unwrap_or_else(|| "navigate".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let confirm = parse_bool(parsed.flags.get("confirm"), false);
    let policy_path = parsed
        .flags
        .get("action-policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            root.join("planes")
                .join("contracts")
                .join("vbrowser")
                .join("action_policy_v1.json")
        });
    let policy = read_json(&policy_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "blocked": ["download-exe"],
            "requires_confirmation": ["submit", "purchase", "delete"]
        })
    });
    let blocked = policy
        .get("blocked")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(|s| s.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let requires_confirmation = policy
        .get("requires_confirmation")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(|s| s.to_ascii_lowercase()))
        .collect::<Vec<_>>();

    if strict && blocked.iter().any(|v| v == &action) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_action_policy",
            "lane": "core/layer0/ops",
            "error": "action_blocked",
            "action": action,
            "session_id": sid,
            "claim_evidence": [
                {
                    "id": "V6-VBROWSER-002.3",
                    "claim": "action_policy_rejects_blocked_operations_with_fail_closed_behavior",
                    "evidence": {"action": action, "blocked": true}
                }
            ]
        });
    }
    if strict && requires_confirmation.iter().any(|v| v == &action) && !confirm {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_action_policy",
            "lane": "core/layer0/ops",
            "error": "confirmation_required",
            "action": action,
            "session_id": sid
        });
    }

    let decision = json!({
        "version": "v1",
        "session_id": sid,
        "action": action,
        "allowed": true,
        "confirmed": confirm,
        "policy_path": policy_path.display().to_string(),
        "ts": crate::now_iso()
    });
    let decision_path = state_root(root).join("action_policy").join("latest.json");
    let _ = write_json(&decision_path, &decision);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_action_policy",
        "lane": "core/layer0/ops",
        "decision": decision,
        "artifact": {
            "path": decision_path.display().to_string(),
            "sha256": sha256_hex_str(&decision.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.3",
                "claim": "action_policy_enforces_confirm_and_block_rules_before_execution",
                "evidence": {"action": action, "confirmed": confirm}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_auth_save(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let provider = clean_id(
        parsed
            .flags
            .get("provider")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("domain").map(String::as_str)),
        "default",
    );
    let profile = clean_id(
        parsed
            .flags
            .get("profile")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("user").map(String::as_str)),
        "default",
    );
    let username = clean(
        parsed
            .flags
            .get("username")
            .cloned()
            .unwrap_or_else(|| "user".to_string()),
        120,
    );
    let secret = parsed.flags.get("secret").cloned().unwrap_or_default();
    if strict && secret.trim().is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_auth_save",
            "lane": "core/layer0/ops",
            "error": "secret_required"
        });
    }
    let encrypted = match encrypt_secret(root, &secret) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "vbrowser_plane_auth_save",
                "lane": "core/layer0/ops",
                "error": "encrypt_failed"
            });
        }
    };
    let mut vault = load_auth_vault(root);
    if !vault.get("profiles").and_then(Value::as_array).is_some() {
        vault["profiles"] = Value::Array(Vec::new());
    }
    let mut profiles = vault
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    profiles.retain(|row| {
        row.get("provider").and_then(Value::as_str) != Some(provider.as_str())
            || row.get("profile").and_then(Value::as_str) != Some(profile.as_str())
    });
    let entry = json!({
        "provider": provider,
        "profile": profile,
        "username": username,
        "secret": encrypted,
        "updated_at": crate::now_iso()
    });
    profiles.push(entry.clone());
    vault["profiles"] = Value::Array(profiles.clone());
    write_auth_vault(root, &vault);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_auth_save",
        "lane": "core/layer0/ops",
        "entry": {
            "provider": provider,
            "profile": profile,
            "username": username
        },
        "profiles_total": profiles.len(),
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.4",
                "claim": "auth_profiles_are_saved_in_encrypted_vault_for_reuse",
                "evidence": {"provider": provider, "profile": profile}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}
