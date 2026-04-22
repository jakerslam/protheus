pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    let strict = bool_flag(parsed.flags.get("strict"), policy.strict_default);
    let apply = bool_flag(parsed.flags.get("apply"), false);

    match cmd.as_str() {
        "status" => {
            let lane = parsed
                .flags
                .get("lane")
                .map(String::as_str)
                .unwrap_or("V6-F100-012");
            let out = status(&policy, lane);
            println!(
                "{}",
                serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
            );
            0
        }
        "run" => {
            let Some(lane_raw) = parsed.flags.get("lane").map(|v| v.trim().to_string()) else {
                let out = cli_error(argv, "missing_lane", 2);
                println!(
                    "{}",
                    serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
                );
                return 2;
            };
            let lane = lane_raw
                .to_ascii_uppercase()
                .replace('_', "-")
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '.')
                .collect::<String>();
            let mut lane_payload = run_lane(root, &policy, &lane, apply);
            lane_payload["ts"] = Value::String(now_iso());
            lane_payload["strict"] = Value::Bool(strict);
            lane_payload["policy_path"] =
                Value::String(policy.policy_path.to_string_lossy().to_string());
            lane_payload["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&lane_payload));

            if let Err(err) = persist_lane(&policy, &lane, &lane_payload) {
                let out = cli_error(argv, &format!("persist_lane_failed:{err}"), 1);
                println!(
                    "{}",
                    serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
                );
                return 1;
            }

            let mut receipt = json!({
                "ok": lane_payload.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "f100_readiness_program_run",
                "lane_program": LANE_ID,
                "lane": lane,
                "strict": strict,
                "apply": apply,
                "ts": now_iso(),
                "result": lane_payload
            });
            receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
            let _ = write_text_atomic(
                &policy.latest_path,
                &(serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".to_string())
                    + "\n"),
            );
            let _ = append_jsonl(&policy.history_path, &receipt);
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
            if strict && !receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                1
            } else {
                0
            }
        }
        "run-all" => {
            let mut lane_results = Vec::new();
            let mut all_ok = true;
            let mut persist_errors = Vec::<Value>::new();
            for lane in EXECUTABLE_LANES {
                let mut lane_payload = run_lane(root, &policy, lane, apply);
                lane_payload["ts"] = Value::String(now_iso());
                lane_payload["strict"] = Value::Bool(strict);
                lane_payload["policy_path"] =
                    Value::String(policy.policy_path.to_string_lossy().to_string());
                lane_payload["receipt_hash"] =
                    Value::String(crate::deterministic_receipt_hash(&lane_payload));
                if let Err(err) = persist_lane(&policy, lane, &lane_payload) {
                    persist_errors.push(json!({
                        "lane": lane,
                        "error": err
                    }));
                    all_ok = false;
                }
                all_ok &= lane_payload
                    .get("ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                lane_results.push(lane_payload);
            }

            let mut receipt = json!({
                "ok": all_ok,
                "type": "f100_readiness_program_run_all",
                "lane_program": LANE_ID,
                "strict": strict,
                "apply": apply,
                "ts": now_iso(),
                "lanes": lane_results,
                "persist_errors": persist_errors
            });
            receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
            let _ = write_text_atomic(
                &policy.latest_path,
                &(serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".to_string())
                    + "\n"),
            );
            let _ = append_jsonl(&policy.history_path, &receipt);
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
            if strict && !all_ok {
                1
            } else {
                0
            }
        }
        _ => {
            usage();
            let out = cli_error(argv, "unknown_command", 2);
            println!(
                "{}",
                serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
            );
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_text(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(path, body).expect("write");
    }

    fn setup_policy(root: &Path) {
        write_text(
            &root.join("client/runtime/config/f100_readiness_program_policy.json"),
            &json!({
                "strict_default": true,
                "outputs": {
                    "state_root": "local/state/ops/f100_readiness_program",
                    "latest_path": "local/state/ops/f100_readiness_program/latest.json",
                    "history_path": "local/state/ops/f100_readiness_program/history.jsonl"
                },
                "lanes": {
                    "V6-F100-005": {
                        "profile_path": "client/runtime/config/one_million_performance_profile.json"
                    },
                    "V7-F100-005": {
                        "profile_path": "client/runtime/config/one_million_performance_profile.json"
                    },
                    "V7-F100-006": {
                        "contract_path": "client/runtime/config/multi_tenant_isolation_contract.json",
                        "adversarial_path": "local/state/security/multi_tenant_isolation_adversarial/latest.json"
                    },
                    "V7-F100-007": {
                        "registry_path": "client/runtime/config/api_cli_contract_registry.json",
                        "required_deprecation_days": 90
                    },
                    "V7-F100-008": {
                        "incident_policy_path": "client/runtime/config/oncall_incident_policy.json",
                        "gameday_path": "local/state/ops/oncall_gameday/latest.json",
                        "target_mtta_minutes": 5,
                        "target_mttr_minutes": 30,
                        "required_docs": [
                            "docs/observability/runbooks/INCIDENT_COMMAND.md",
                            "docs/observability/runbooks/POSTMORTEM_TEMPLATE.md"
                        ]
                    }
                }
            })
            .to_string(),
        );
    }

    #[test]
    fn million_user_lane_passes_with_budgeted_profile() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        write_text(
            &tmp.path()
                .join("client/runtime/config/one_million_performance_profile.json"),
            &json!({
                "budgets": {
                    "p95_ms": 250,
                    "p99_ms": 500,
                    "error_rate": 0.01,
                    "saturation_pct": 80,
                    "cost_per_request_usd": 0.05
                },
                "observed": {
                    "p95_ms": 200,
                    "p99_ms": 300,
                    "error_rate": 0.005,
                    "saturation_pct": 72,
                    "cost_per_request_usd": 0.02
                }
            })
            .to_string(),
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V6-F100-005".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn scorecard_lane_needs_two_cycles() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        let _ = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V6-F100-012".to_string(),
                "--strict=0".to_string(),
            ],
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V6-F100-012".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn v7_million_user_lane_uses_v7_policy() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        write_text(
            &tmp.path()
                .join("client/runtime/config/one_million_performance_profile.json"),
            &json!({
                "budgets": {
                    "p95_ms": 250,
                    "p99_ms": 500,
                    "error_rate": 0.01,
                    "saturation_pct": 80,
                    "cost_per_request_usd": 0.05
                },
                "observed": {
                    "p95_ms": 120,
                    "p99_ms": 220,
                    "error_rate": 0.001,
                    "saturation_pct": 60,
                    "cost_per_request_usd": 0.01
                }
            })
            .to_string(),
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V7-F100-005".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn v7_multi_tenant_lane_uses_v7_contracts() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        write_text(
            &tmp.path()
                .join("client/runtime/config/multi_tenant_isolation_contract.json"),
            &json!({"contract":"ok"}).to_string(),
        );
        write_text(
            &tmp.path()
                .join("local/state/security/multi_tenant_isolation_adversarial/latest.json"),
            &json!({
                "cross_tenant_leaks": 0,
                "delete_export_pass": true,
                "classification_enforced": true
            })
            .to_string(),
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V7-F100-006".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn v7_interface_lifecycle_lane_uses_v7_registry() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        write_text(
            &tmp.path()
                .join("client/runtime/config/api_cli_contract_registry.json"),
            &json!({
                "api_contracts": [
                    {"name":"agents-v1","version":"1.2.0","status":"stable"}
                ],
                "cli_contracts": [
                    {"name":"shell-v1","version":"2.0.0","status":"stable"}
                ]
            })
            .to_string(),
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V7-F100-007".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn v7_oncall_lane_uses_v7_sla_paths() {
        let tmp = tempdir().expect("tmp");
        setup_policy(tmp.path());
        write_text(
            &tmp.path()
                .join("client/runtime/config/oncall_incident_policy.json"),
            &json!({"policy":"v1"}).to_string(),
        );
        write_text(
            &tmp.path()
                .join("local/state/ops/oncall_gameday/latest.json"),
            &json!({"mtta_minutes":4.0,"mttr_minutes":20.0}).to_string(),
        );
        write_text(
            &tmp.path()
                .join("docs/observability/runbooks/INCIDENT_COMMAND.md"),
            "runbook\n",
        );
        write_text(
            &tmp.path()
                .join("docs/observability/runbooks/POSTMORTEM_TEMPLATE.md"),
            "runbook\n",
        );
        let code = run(
            tmp.path(),
            &[
                "run".to_string(),
                "--lane=V7-F100-008".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 0);
    }
}
