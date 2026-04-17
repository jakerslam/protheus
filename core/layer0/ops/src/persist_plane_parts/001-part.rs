    }

    let path = mobile_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "connected": false,
            "session_id": null,
            "device": null,
            "last_action": null
        })
    });

    if op == "status" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_mobile_cockpit",
            "lane": "core/layer0/ops",
            "op": "status",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.2",
                    "claim": "mobile_cockpit_surfaces_live_daemon_state_and_intervention_controls",
                    "evidence": {
                        "connected": state
                            .get("connected")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "publish" {
        state["connected"] = Value::Bool(true);
        state["session_id"] = Value::String(clean_id(
            parsed.flags.get("session-id").map(String::as_str),
            "mobile-session",
        ));
        state["device"] = Value::String(clean(
            parsed
                .flags
                .get("device")
                .cloned()
                .unwrap_or_else(|| "mobile-client".to_string()),
            120,
        ));
        state["published_at"] = Value::String(crate::now_iso());
        state["last_action"] = Value::String("publish".to_string());
    } else {
        if strict
            && !state
                .get("connected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "persist_plane_mobile_cockpit",
                "errors": ["persist_mobile_cockpit_not_connected"]
            });
        }
        let action = clean(
            parsed
                .flags
                .get("action")
                .cloned()
                .unwrap_or_else(|| "pause".to_string()),
            20,
        )
        .to_ascii_lowercase();
        let action_allowed = contract
            .get("allowed_actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == action);
        if strict && !action_allowed {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "persist_plane_mobile_cockpit",
                "errors": ["persist_mobile_cockpit_action_invalid"]
            });
        }
        state["last_action"] = Value::String(action);
        state["intervened_at"] = Value::String(crate::now_iso());
    }

    state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("mobile").join("history.jsonl"),
        &json!({"op": op, "state": state, "ts": crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "persist_plane_mobile_cockpit",
        "lane": "core/layer0/ops",
        "op": op,
        "state": state,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-PERSIST-001.2",
                "claim": "mobile_cockpit_state_and_interventions_are_receipted_for_remote_control",
                "evidence": {
                    "op": op
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_mobile_daemon(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        MOBILE_DAEMON_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mobile_daemon_bitnet_contract",
            "allowed_ops": ["enable", "status", "handoff"],
            "allowed_platforms": ["android", "ios"],
            "allowed_edge_backends": ["bitnet"],
            "allowed_handoffs": ["edge", "cloud"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops_values = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let allowed_ops = allowed_ops_values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if strict && !allowed_ops.iter().any(|row| row == &op) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_mobile_daemon",
            "errors": ["persist_mobile_daemon_op_invalid"]
        });
    }
    let path = mobile_daemon_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "enabled": false,
            "platform": "android",
            "edge_backend": "bitnet",
            "sensor_lanes": ["camera", "mic", "gps"],
            "handoff_mode": "edge"
        })
    });

    if op == "status" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_mobile_daemon",
            "lane": "core/layer0/ops",
            "op": "status",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V7-MOBILE-001.1",
                    "claim": "mobile_daemon_profile_surfaces_bitnet_edge_state_and_sensor_lane_bindings",
                    "evidence": {
                        "enabled": state.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                        "edge_backend": state.get("edge_backend").cloned().unwrap_or(Value::Null)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "enable" {
        let platform = clean(
            parsed
                .flags
                .get("platform")
                .cloned()
                .unwrap_or_else(|| "android".to_string()),
            20,
        )
        .to_ascii_lowercase();
        let edge_backend = clean(
            parsed
                .flags
                .get("edge-backend")
                .cloned()
                .unwrap_or_else(|| "bitnet".to_string()),
            40,
        )
        .to_ascii_lowercase();
        let sensors = clean(
            parsed
                .flags
                .get("sensor-lanes")
                .cloned()
                .unwrap_or_else(|| "camera,mic,gps".to_string()),
            200,
        )
        .split(',')
        .map(|row| clean(row, 32).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
        let allowed_platforms_values = contract
            .get("allowed_platforms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let allowed_platforms = allowed_platforms_values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let allowed_backends_values = contract
            .get("allowed_edge_backends")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let allowed_backends = allowed_backends_values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if strict
            && (!allowed_platforms.iter().any(|row| row == &platform)
                || !allowed_backends.iter().any(|row| row == &edge_backend))
        {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "persist_plane_mobile_daemon",
                "errors": ["persist_mobile_daemon_profile_invalid"]
            });
        }
        state["enabled"] = Value::Bool(true);
        state["platform"] = Value::String(platform);
        state["edge_backend"] = Value::String(edge_backend);
        state["sensor_lanes"] = Value::Array(
            sensors
                .iter()
                .map(|row| Value::String(row.clone()))
                .collect::<Vec<_>>(),
        );
        state["handoff_mode"] = Value::String("edge".to_string());
        state["updated_at"] = Value::String(crate::now_iso());
    } else {
        let handoff = clean(
            parsed
                .flags
                .get("handoff")
                .cloned()
                .unwrap_or_else(|| "edge".to_string()),
            20,
        )
        .to_ascii_lowercase();
        let allowed_handoffs_values = contract
            .get("allowed_handoffs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let allowed_handoffs = allowed_handoffs_values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if strict && !allowed_handoffs.iter().any(|row| row == &handoff) {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "persist_plane_mobile_daemon",
                "errors": ["persist_mobile_daemon_handoff_invalid"]
            });
        }
        state["handoff_mode"] = Value::String(handoff);
        state["handoff_at"] = Value::String(crate::now_iso());
    }

    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("mobile").join("daemon_history.jsonl"),
        &json!({"op": op, "state": state, "ts": crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "persist_plane_mobile_daemon",
        "lane": "core/layer0/ops",
        "op": op,
        "state": state,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V7-MOBILE-001.1",
                "claim": "android_ios_mobile_daemon_uses_bitnet_edge_default_with_policy_bound_sensor_handoff_receipts",
                "evidence": {
                    "op": op
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

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
                "type": "persist_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "schedule" => run_schedule(root, &parsed, strict),
        "mobile-cockpit" => run_mobile_cockpit(root, &parsed, strict),
        "mobile-daemon" => run_mobile_daemon(root, &parsed, strict),
        "continuity" => run_continuity(root, &parsed, strict),
        "connector" => run_connector(root, &parsed, strict),
        "cowork" | "co-work" => run_cowork(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "persist_plane_error",
            "error": "unknown_command",
            "command": command
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
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["schedule".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "schedule");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn schedule_upsert_creates_registry() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_schedule(
            root.path(),
            &crate::parse_args(&[
                "schedule".to_string(),
                "--op=upsert".to_string(),
                "--job=daily-health".to_string(),
