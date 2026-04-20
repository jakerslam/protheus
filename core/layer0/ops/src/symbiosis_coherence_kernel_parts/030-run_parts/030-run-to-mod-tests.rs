
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
        "profile-summary" => profile_summary(root, &payload),
        "profile-update" => profile_update(root, &payload),
        "profile-reset" => profile_reset(root, &payload),
        "profile-checklist" => profile_checklist_cmd(root, &payload),
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

    #[test]
    fn profile_update_and_reset_are_receipted_and_persisted() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let update_payload = json!({
            "source": "feedback",
            "explicit_feedback": {
                "tone": "direct",
                "depth_delta": 0.1,
                "initiative_delta": -0.05
            },
            "interaction_signals": {
                "tool_aggressiveness_delta": 0.08
            }
        });
        let updated = profile_update(root, update_payload.as_object().unwrap()).unwrap();
        assert_eq!(updated["ok"], Value::Bool(true));
        assert_eq!(
            updated["settings"]["tone"].as_str(),
            Some("direct")
        );

        let summary = profile_summary(root, &Map::new()).unwrap();
        assert_eq!(summary["ok"], Value::Bool(true));
        assert!(summary["delta_count"].as_u64().unwrap_or(0) >= 1);

        let reset = profile_reset(root, &Map::new()).unwrap();
        assert_eq!(reset["ok"], Value::Bool(true));
        assert_eq!(
            reset["settings"]["tone"].as_str(),
            Some("collaborative")
        );
    }
}
