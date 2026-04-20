
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
