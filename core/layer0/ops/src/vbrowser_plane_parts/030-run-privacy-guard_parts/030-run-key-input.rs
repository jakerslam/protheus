
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
