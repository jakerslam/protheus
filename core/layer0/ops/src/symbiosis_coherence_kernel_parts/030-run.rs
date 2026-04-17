fn recursion_request(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let signal = if let Some(signal) = payload.get("signal") {
        signal.clone()
    } else {
        load_signal(root, payload)?
    };
    let (requested_depth, parsed_unbounded) = parse_depth_request(
        payload
            .get("requested_depth")
            .or_else(|| payload.get("requestedDepth")),
    );
    let require_unbounded = bool_value(payload.get("require_unbounded"), false) || parsed_unbounded;
    let allowed_depth = signal
        .get("recursion_gate")
        .and_then(|v| v.get("allowed_depth"))
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64()
                    .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            }
        });
    let unbounded_allowed = signal
        .get("recursion_gate")
        .and_then(|v| v.get("unbounded_allowed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let mut blocked = false;
    if signal.get("available").and_then(Value::as_bool) != Some(true) {
        reasons.push(Value::String("symbiosis_signal_unavailable".to_string()));
    } else {
        if require_unbounded && !unbounded_allowed {
            blocked = true;
            reasons.push(Value::String("symbiosis_unbounded_not_allowed".to_string()));
        }
        if let (Some(requested), Some(allowed)) = (requested_depth, allowed_depth) {
            if requested > allowed {
                blocked = true;
                reasons.push(Value::String("symbiosis_depth_exceeds_allowed".to_string()));
            }
        }
    }

    let shadow_only = if payload.contains_key("shadow_only_override") {
        bool_value(payload.get("shadow_only_override"), true)
    } else {
        signal
            .get("shadow_only")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    let blocked_hard = blocked && !shadow_only;

    Ok(json!({
        "ok": !blocked_hard,
        "available": signal.get("available").and_then(Value::as_bool).unwrap_or(false),
        "blocked": blocked,
        "blocked_hard": blocked_hard,
        "shadow_violation": blocked && shadow_only,
        "shadow_only": shadow_only,
        "reason_codes": reasons,
        "requested_depth": requested_depth,
        "requested_unbounded": require_unbounded,
        "allowed_depth": allowed_depth,
        "unbounded_allowed": unbounded_allowed,
        "coherence_score": signal.get("coherence_score").and_then(Value::as_f64),
        "coherence_tier": signal.get("coherence_tier").cloned().unwrap_or(Value::Null),
        "sustained_high_samples": signal
            .get("recursion_gate")
            .and_then(|v| v.get("sustained_high_samples"))
            .and_then(Value::as_i64),
        "latest_path_rel": signal.get("latest_path_rel").cloned().unwrap_or_else(|| {
            signal
                .get("source_paths")
                .and_then(|v| v.get("latest_path"))
                .cloned()
                .unwrap_or(Value::Null)
        })
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

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };

    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("symbiosis_coherence_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();

    let result = match command {
        "load-policy" => Ok(json!({
            "ok": true,
            "policy": load_policy(root, &payload)
        })),
        "evaluate" => evaluate_signal(root, &payload),
        "load" => load_signal(root, &payload),
        "recursion-request" => recursion_request(root, &payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err("symbiosis_coherence_kernel_unknown_command".to_string()),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                "symbiosis_coherence_kernel",
                with_execution_receipt(command, "success", payload),
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_receipt(
                "symbiosis_coherence_kernel",
                with_execution_receipt(
                    command,
                    "error",
                    json!({
                        "ok": false,
                        "error": err,
                        "error_kind": "command_failed",
                        "retryable": false
                    }),
                ),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, value: &Value) {
        let path = root.join(rel);
        lane_utils::write_json(&path, value).unwrap();
    }

    #[test]
    fn evaluate_signal_persists_latest_and_recursion_gate() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let policy_path = root.join("client/runtime/config/symbiosis_coherence_policy.json");
        write(
            root,
            "client/runtime/config/symbiosis_coherence_policy.json",
            &json!({
                "version": "1.0",
                "shadow_only": true,
                "paths": {
                    "state_path": "local/state/symbiosis/coherence/state.json",
                    "latest_path": "local/state/symbiosis/coherence/latest.json",
                    "receipts_path": "local/state/symbiosis/coherence/receipts.jsonl",
                    "identity_latest_path": "local/state/autonomy/identity_anchor/latest.json",
                    "pre_neuralink_state_path": "local/state/symbiosis/pre_neuralink_interface/state.json",
                    "deep_symbiosis_state_path": "local/state/symbiosis/deep_understanding/state.json",
                    "observer_mirror_latest_path": "local/state/autonomy/observer_mirror/latest.json"
                }
            }),
        );
        write(
            root,
            "local/state/autonomy/identity_anchor/latest.json",
            &json!({"summary":{"identity_drift_score":0.12,"max_identity_drift_score":0.58,"blocked":0,"checked":10}}),
        );
        write(
            root,
            "local/state/symbiosis/pre_neuralink_interface/state.json",
            &json!({"consent_state":"granted","signals_total":20,"routed_total":18,"blocked_total":1}),
        );
        write(
            root,
            "local/state/symbiosis/deep_understanding/state.json",
            &json!({"samples":60,"style":{"directness":0.9,"brevity":0.8,"proactive_delta":0.85}}),
        );
        write(
            root,
            "local/state/autonomy/observer_mirror/latest.json",
            &json!({"observer":{"mood":"stable"},"summary":{"rates":{"ship_rate":0.8,"hold_rate":0.1}}}),
        );

        let payload = json!({
            "policy_path": policy_path,
            "persist": true
        });
        let out = evaluate_signal(root, payload.as_object().unwrap()).unwrap();
        assert_eq!(out["available"], Value::Bool(true));
        assert!(out["coherence_score"].as_f64().unwrap() > 0.7);
        assert!(out["recursion_gate"]["allowed_depth"].as_i64().unwrap() >= 3);
        assert!(root
            .join("local/state/symbiosis/coherence/latest.json")
            .exists());
    }

    #[test]
    fn recursion_request_flags_depth_violation() {
        let signal = json!({
            "available": true,
            "shadow_only": true,
            "coherence_score": 0.82,
            "coherence_tier": "high",
            "latest_path_rel": "local/state/symbiosis/coherence/latest.json",
            "recursion_gate": {
                "allowed_depth": 4,
                "unbounded_allowed": false,
                "sustained_high_samples": 3
            }
        });
        let dir = tempdir().unwrap();
        let payload = json!({
            "signal": signal,
            "requested_depth": 7
        });
        let out = recursion_request(dir.path(), payload.as_object().unwrap()).unwrap();
        assert_eq!(out["blocked"], Value::Bool(true));
        assert_eq!(out["blocked_hard"], Value::Bool(false));
        assert!(out["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "symbiosis_depth_exceeds_allowed"));
    }
}
