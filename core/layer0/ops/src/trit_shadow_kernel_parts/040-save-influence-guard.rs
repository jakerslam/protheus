fn save_influence_guard(guard: &Value, path: &Path) -> Result<Value, String> {
    let mut next = if guard.is_object() {
        guard.clone()
    } else {
        default_influence_guard()
    };
    next["updated_at"] = Value::String(now_iso());
    write_json_atomic(path, &next)?;
    Ok(next)
}

fn is_influence_blocked(guard: &Value, now_ts: Option<&str>) -> Value {
    if guard.get("disabled").and_then(Value::as_bool) != Some(true) {
        return json!({"blocked": false, "reason": "enabled"});
    }
    let now_ms = now_ts
        .and_then(|v| {
            chrono::DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.timestamp_millis())
        })
        .unwrap_or_else(|| Utc::now().timestamp_millis());
    let until_ms = guard
        .get("disabled_until")
        .and_then(Value::as_str)
        .and_then(|v| {
            chrono::DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.timestamp_millis())
        });
    if let Some(until) = until_ms {
        if now_ms > until {
            return json!({"blocked": false, "reason": "expired"});
        }
    }
    json!({
        "blocked": true,
        "reason": as_str(guard.get("reason")).if_empty_then("disabled"),
        "disabled_until": guard.get("disabled_until").cloned().unwrap_or(Value::Null),
    })
}

fn apply_influence_guard(
    report_payload: &Value,
    policy: &Value,
    path: &Path,
) -> Result<Value, String> {
    let summary = report_payload.get("summary").and_then(Value::as_object);
    let gate = summary
        .and_then(|v| v.get("gate"))
        .and_then(Value::as_object);
    let status = as_str(summary.and_then(|v| v.get("status"))).to_ascii_lowercase();
    let should_disable =
        if gate.and_then(|v| v.get("enabled")).and_then(Value::as_bool) == Some(true) {
            gate.and_then(|v| v.get("pass")).and_then(Value::as_bool) == Some(false)
        } else {
            status == "critical"
        };
    let mut next = load_influence_guard(path);
    let disable_hours = clamp_number(
        policy.pointer("/influence/auto_disable_hours_on_regression"),
        1.0,
        (24 * 30) as f64,
        24.0,
    );
    if should_disable {
        next["disabled"] = Value::Bool(true);
        let reason = if gate.and_then(|v| v.get("enabled")).and_then(Value::as_bool) == Some(true)
            && gate.and_then(|v| v.get("pass")).and_then(Value::as_bool) == Some(false)
        {
            format!(
                "shadow_gate_failed:{}",
                as_str(gate.and_then(|v| v.get("reason")))
                    .if_empty_then("divergence_rate_exceeds_limit")
            )
        } else {
            "shadow_status_critical".to_string()
        };
        next["reason"] = Value::String(reason);
        next["disabled_until"] = Value::String(
            (Utc::now() + Duration::hours(disable_hours.round() as i64)).to_rfc3339(),
        );
    } else {
        next["disabled"] = Value::Bool(false);
        next["reason"] = Value::Null;
        next["disabled_until"] = Value::Null;
    }
    next["last_report_ts"] = report_payload
        .get("ts")
        .cloned()
        .unwrap_or_else(|| Value::String(now_iso()));
    save_influence_guard(&next, path)
}

fn command_payload_map<'a>(payload: &'a Map<String, Value>, key: &str) -> Map<String, Value> {
    payload
        .get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| payload.clone())
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    let paths = resolve_paths(root, payload);
    match command {
        "paths" => Ok(paths.as_json()),
        "default-policy" => Ok(default_policy()),
        "normalize-policy" => {
            let policy = command_payload_map(payload, "policy");
            Ok(normalize_policy(&policy))
        }
        "load-policy" => Ok(load_policy_from_path(&paths.policy)),
        "load-success-criteria" => Ok(load_success_criteria_from_path(&paths.success_criteria)),
        "load-trust-state" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(load_trust_state_from_path(&policy, &paths.trust_state))
        }
        "save-trust-state" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let state = payload
                .get("state")
                .cloned()
                .unwrap_or_else(|| Value::Object(payload.clone()));
            save_trust_state_to_path(&state, &policy, &paths.trust_state)
        }
        "build-trust-map" => {
            let trust_state = payload
                .get("trust_state")
                .cloned()
                .unwrap_or_else(|| Value::Object(payload.clone()));
            Ok(build_trust_map(&trust_state))
        }
        "evaluate-productivity" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(evaluate_productivity(&policy, &paths))
        }
        "evaluate-auto-stage" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(evaluate_auto_stage(&policy, &paths))
        }
        "resolve-stage-decision" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(resolve_stage_decision(&policy, &paths))
        }
        "resolve-stage" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let stage = resolve_stage_decision(&policy, &paths)
                .get("stage")
                .cloned()
                .unwrap_or(Value::from(0));
            Ok(json!({"stage": stage}))
        }
        "can-consume-override" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let date_str = as_str(payload.get("date_str")).if_empty_then(&now_date());
            Ok(can_consume_override(
                &policy,
                &date_str,
                &paths.influence_budget,
            ))
        }
        "consume-override" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let date_str = as_str(payload.get("date_str")).if_empty_then(&now_date());
            let source = as_str(payload.get("source")).if_empty_then("unknown");
            consume_override(&source, &policy, &date_str, &paths.influence_budget)
        }
        "load-influence-guard" => Ok(load_influence_guard(&paths.influence_guard)),
        "save-influence-guard" => {
            let guard = payload
                .get("guard")
                .cloned()
                .unwrap_or_else(|| Value::Object(payload.clone()));
            save_influence_guard(&guard, &paths.influence_guard)
        }
        "influence-blocked" => {
            let guard = payload
                .get("guard")
                .cloned()
                .unwrap_or_else(|| load_influence_guard(&paths.influence_guard));
            let now_ts = as_str(payload.get("now_ts"));
            Ok(is_influence_blocked(
                &guard,
                if now_ts.is_empty() {
                    None
                } else {
                    Some(now_ts.as_str())
                },
            ))
        }
        "apply-influence-guard" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let report = payload
                .get("report_payload")
                .cloned()
                .or_else(|| payload.get("report").cloned())
                .unwrap_or_else(|| Value::Object(payload.clone()));
            apply_influence_guard(&report, &policy, &paths.influence_guard)
        }
        _ => Err("trit_shadow_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("trit_shadow_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("trit_shadow_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("trit_shadow_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "trit-shadow-kernel-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).unwrap();
        base.join(name)
    }

    #[test]
    fn normalize_policy_clamps_values() {
        let normalized = normalize_policy(payload_obj(&json!({
            "influence": {
                "stage": 7,
                "max_overrides_per_day": -5,
                "auto_stage": {"mode": "override"}
            },
            "trust": {"source_trust_floor": 0.2, "source_trust_ceiling": 9}
        })));
        assert_eq!(
            normalized
                .pointer("/influence/stage")
                .and_then(Value::as_i64),
            Some(3)
        );
        assert_eq!(
            normalized
                .pointer("/influence/max_overrides_per_day")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            normalized
                .pointer("/trust/source_trust_floor")
                .and_then(Value::as_f64),
            Some(0.2_f64.max(0.01))
        );
        assert_eq!(
            normalized
                .pointer("/influence/auto_stage/mode")
                .and_then(Value::as_str),
            Some("override")
        );
    }

    #[test]
    fn trust_state_round_trip_and_map() {
        let path = temp_file("trust_state.json");
        let policy = default_policy();
        let saved = save_trust_state_to_path(
            &json!({
                "default_source_trust": 1.2,
                "by_source": {
                    "policy": {"trust": 1.4, "samples": 10, "hit_rate": 0.7}
                }
            }),
            &policy,
            &path,
        )
        .unwrap();
        let loaded = load_trust_state_from_path(&policy, &path);
        assert_eq!(
            saved.pointer("/by_source/policy/trust"),
            loaded.pointer("/by_source/policy/trust")
        );
        let trust_map = build_trust_map(&loaded);
        assert_eq!(trust_map.get("policy").and_then(Value::as_f64), Some(1.4));
    }

    #[test]
    fn productivity_and_auto_stage_activate_from_histories() {
        let root = Path::new(".");
        let report_history = temp_file("reports.jsonl");
        let calibration_history = temp_file("calibration.jsonl");
        fs::write(
            &report_history,
            concat!(
                "{\"type\":\"trit_shadow_report\",\"ok\":true,\"ts\":\"2026-03-17T00:00:00Z\",\"summary\":{\"total_decisions\":30,\"divergence_rate\":0.01},\"success_criteria\":{\"pass\":true,\"checks\":{\"safety_regressions\":{\"pass\":true},\"drift_non_increasing\":{\"pass\":true}}}}\n"
            ),
        ).unwrap();
        fs::write(
            &calibration_history,
            concat!(
                "{\"type\":\"trit_shadow_replay_calibration\",\"ok\":true,\"ts\":\"2026-03-17T00:10:00Z\",\"date\":\"2026-03-17\",\"summary\":{\"total_events\":30,\"accuracy\":0.7,\"expected_calibration_error\":0.1},\"source_reliability\":[{\"source\":\"policy\",\"samples\":12,\"hit_rate\":0.7}]}\n"
            ),
        ).unwrap();
        let payload = json!({
            "paths": {
                "report_history": report_history,
                "calibration_history": calibration_history
            },
            "policy": {
                "influence": {
                    "stage": 1,
                    "activation": {
                        "enabled": true,
                        "report_window": 1,
                        "calibration_window": 1,
                        "min_decisions": 20,
                        "min_calibration_events": 20,
                        "min_source_samples": 8,
                        "min_source_hit_rate": 0.55,
                        "max_sources_below_threshold": 1,
                        "allow_if_no_source_data": false
                    },
                    "auto_stage": {
                        "enabled": true,
                        "mode": "floor",
                        "stage2": {
                            "consecutive_reports": 1,
                            "min_calibration_reports": 1,
                            "min_decisions": 20,
                            "max_divergence_rate": 0.08,
                            "min_calibration_events": 20,
                            "min_calibration_accuracy": 0.55,
                            "max_calibration_ece": 0.25,
                            "require_source_reliability": false
                        },
                        "stage3": {
                            "consecutive_reports": 2,
                            "min_calibration_reports": 2,
                            "min_decisions": 40,
                            "max_divergence_rate": 0.05,
                            "min_calibration_events": 40,
                            "min_calibration_accuracy": 0.65,
                            "max_calibration_ece": 0.2,
                            "require_source_reliability": false
                        }
                    }
                }
            }
        });
        let result = run_command(root, "evaluate-auto-stage", payload_obj(&payload)).unwrap();
        assert_eq!(result.get("stage").and_then(Value::as_i64), Some(2));
        let decision = run_command(root, "resolve-stage-decision", payload_obj(&payload)).unwrap();
        assert_eq!(decision.get("stage").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn override_budget_and_guard_flow() {
        let root = Path::new(".");
        let budget_path = temp_file("influence_budget.json");
        let guard_path = temp_file("influence_guard.json");
        let policy = json!({"influence": {"max_overrides_per_day": 2, "auto_disable_hours_on_regression": 24}});
        let consume_payload = json!({
            "policy": policy,
            "date_str": "2026-03-17",
            "source": "planner",
            "paths": {"influence_budget": budget_path}
        });
        let first = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(first.get("consumed").and_then(Value::as_bool), Some(true));
        let second = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(second.get("consumed").and_then(Value::as_bool), Some(true));
        let third = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(third.get("consumed").and_then(Value::as_bool), Some(false));

        let guard = run_command(
            root,
            "apply-influence-guard",
            payload_obj(&json!({
                "policy": {"influence": {"auto_disable_hours_on_regression": 24}},
                "paths": {"influence_guard": guard_path},
                "report_payload": {
                    "ts": "2026-03-17T12:00:00Z",
                    "summary": {
                        "status": "critical",
                        "gate": {"enabled": true, "pass": false, "reason": "divergence_rate_exceeds_limit"}
                    }
                }
            })),
        ).unwrap();
        assert_eq!(guard.get("disabled").and_then(Value::as_bool), Some(true));
        let blocked = run_command(
            root,
            "influence-blocked",
            payload_obj(&json!({
                "guard": guard,
                "now_ts": "2026-03-17T12:30:00Z"
            })),
        )
        .unwrap();
        assert_eq!(blocked.get("blocked").and_then(Value::as_bool), Some(true));
    }
}

