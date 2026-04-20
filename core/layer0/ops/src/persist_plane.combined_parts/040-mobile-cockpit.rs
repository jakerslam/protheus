
fn run_mobile_cockpit(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        MOBILE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "persist_mobile_cockpit_contract",
            "allowed_ops": ["publish", "status", "intervene"],
            "allowed_actions": ["pause", "resume", "abort"]
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
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_mobile_cockpit",
            "errors": ["persist_mobile_cockpit_op_invalid"]
        });

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
