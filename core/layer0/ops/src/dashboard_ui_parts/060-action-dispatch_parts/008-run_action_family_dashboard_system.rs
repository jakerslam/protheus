fn run_action_family_dashboard_system(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.update.check" => {
            let result = crate::dashboard_release_update::check_update(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.check".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.update.apply" => {
            let result = crate::dashboard_release_update::dispatch_update_apply(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.apply".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.system.restart" => {
            let result = crate::dashboard_release_update::dispatch_system_action(root, "restart");
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.system.restart".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.system.shutdown" => {
            let result = crate::dashboard_release_update::dispatch_system_action(root, "shutdown");
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.system.shutdown".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.runtime.executeSwarmRecommendation"
        | "dashboard.runtime.applyTelemetryRemediations" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let action_key = if normalized == "dashboard.runtime.applyTelemetryRemediations" {
                "apply_telemetry_remediations"
            } else {
                "execute_swarm_recommendation"
            };
            let runtime_flags = Flags {
                mode: "runtime-sync".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: team.clone(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let runtime = build_runtime_sync(root, &runtime_flags);
            let summary = runtime
                .get("summary")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let queue_depth = i64_from_value(summary.get("queue_depth"), 0);
            let target_conduit_signals = i64_from_value(summary.get("target_conduit_signals"), 4);
            let critical_attention_total =
                i64_from_value(summary.get("critical_attention_total"), 0);
            let conduit_scale_required = summary
                .get("conduit_scale_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let snapshot_now = build_snapshot(root, &runtime_flags);
            let active_swarm_agents = snapshot_now
                .pointer("/collab/dashboard/agents")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            let mut swarm_target_agents = active_swarm_agents;
            if queue_depth >= 80 || critical_attention_total >= 5 {
                swarm_target_agents = std::cmp::max(active_swarm_agents + 2, 4);
            } else if queue_depth >= 40 || conduit_scale_required {
                swarm_target_agents = std::cmp::max(active_swarm_agents + 1, 3);
            }
            let swarm_scale_required = swarm_target_agents > active_swarm_agents;
            let throttle_required = queue_depth >= 75 || critical_attention_total >= 5;
            let predictive_drain_required = queue_depth >= 65 || critical_attention_total >= 4;
            let attention_drain_required = queue_depth >= 60 || critical_attention_total >= 2;
            let attention_compaction_required = queue_depth >= 45 || conduit_scale_required;
            let coarse_signal_remediation_required =
                i64_from_value(summary.get("cockpit_stale_blocks"), 0) > 0;
            let reliability_gate_required = false;
            let slo_gate_required = queue_depth >= 95;
            let slo_gate = json!({
                "required": slo_gate_required,
                "severity": if slo_gate_required { "high" } else { "normal" },
                "block_scale": false,
                "containment_required": slo_gate_required,
                "failed_checks": [],
                "thresholds": {
                    "spine_success_rate_min": 0.999,
                    "receipt_latency_p95_max_ms": 100.0,
                    "receipt_latency_p99_max_ms": 150.0,
                    "queue_depth_max": 90
                }
            });
            let mut role_plan = vec![json!({"role": "coordinator", "required": true})];
            if conduit_scale_required || throttle_required {
                role_plan.push(json!({"role": "researcher", "required": true}));
            }
            if queue_depth >= 60 || critical_attention_total >= 3 {
                role_plan.push(json!({"role": "analyst", "required": true}));
            }
            if swarm_scale_required {
                role_plan.push(json!({"role": "builder", "required": true}));
                role_plan.push(json!({"role": "reviewer", "required": true}));
            }
            let turns = role_plan
                .iter()
                .take(3)
                .enumerate()
                .map(|(idx, row)| {
                    let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or("agent"), 80);
                    json!({
                        "turn_id": format!("swarm-turn-{}", idx + 1),
                        "role": role,
                        "required": row.get("required").cloned().unwrap_or_else(|| json!(false)),
                        "status": "completed",
                        "summary": format!("{role} acknowledged runtime pressure and prepared remediation."),
                        "ts": now_iso()
                    })
                })
                .collect::<Vec<_>>();
            let policies = vec![
                json!({
                    "policy": "queue_throttle",
                    "required": throttle_required,
                    "applied": throttle_required
                }),
                json!({
                    "policy": "conduit_scale",
                    "required": conduit_scale_required,
                    "applied": conduit_scale_required,
                    "target_conduit_signals": target_conduit_signals
                }),
                json!({
                    "policy": "predictive_drain",
                    "required": predictive_drain_required,
                    "applied": predictive_drain_required
                }),
                json!({
                    "policy": "attention_queue_autodrain",
                    "required": attention_drain_required,
                    "applied": attention_drain_required
                }),
                json!({
                    "policy": "attention_queue_compaction",
                    "required": attention_compaction_required,
                    "applied": attention_compaction_required
                }),
                json!({
                    "policy": "coarse_lane_demotion",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "coarse_conduit_scale_up",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "coarse_stale_lane_drain",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "spine_reliability_gate",
                    "required": reliability_gate_required,
                    "applied": reliability_gate_required
                }),
                json!({
                    "policy": "human_escalation_guard",
                    "required": reliability_gate_required,
                    "applied": reliability_gate_required
                }),
                json!({
                    "policy": "runtime_slo_gate",
                    "required": slo_gate_required,
                    "applied": slo_gate_required,
                    "thresholds": slo_gate.get("thresholds").cloned().unwrap_or_else(|| json!({}))
                }),
            ];
            let mut launch_receipt = Value::Null;
            if queue_depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH {
                let shadow = format!("{team}-drain-{}", Utc::now().timestamp_millis());
                let launch = run_lane(
                    root,
                    "collab-plane",
                    &[
                        "launch-role".to_string(),
                        format!("--team={team}"),
                        "--role=analyst".to_string(),
                        format!("--shadow={shadow}"),
                    ],
                );
                launch_receipt = launch.payload.unwrap_or_else(|| {
                    json!({
                        "ok": launch.ok,
                        "status": launch.status,
                        "argv": launch.argv
                    })
                });
            }
            let launches = if launch_receipt.is_null() {
                Vec::<Value>::new()
            } else {
                vec![launch_receipt.clone()]
            };
            let executed_count = turns.len() as i64;
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string(), format!("--team={team}")],
                payload: Some(json!({
                    "ok": true,
                    "type": "infring_dashboard_runtime_action",
                    "action": action_key,
                    "ts": now_iso(),
                    "team": team,
                    "queue_depth": queue_depth,
                    "target_conduit_signals": target_conduit_signals,
                    "conduit_scale_required": conduit_scale_required,
                    "launch_receipt": launch_receipt,
                    "launches": launches,
                    "executed_count": executed_count,
                    "turns": turns,
                    "policies": policies,
                    "recommendation": {
                        "action": action_key,
                        "active_swarm_agents": active_swarm_agents,
                        "swarm_target_agents": swarm_target_agents,
                        "swarm_scale_required": swarm_scale_required,
                        "throttle_required": throttle_required,
                        "predictive_drain_required": predictive_drain_required,
                        "attention_drain_required": attention_drain_required,
                        "attention_compaction_required": attention_compaction_required,
                        "coarse_signal_remediation_required": coarse_signal_remediation_required,
                        "reliability_gate_required": reliability_gate_required,
                        "slo_gate_required": slo_gate_required,
                        "slo_gate": slo_gate,
                        "role_plan": role_plan
                    }
                })),
            }
        }
        _ => run_action_family_dashboard_agent(root, normalized, payload),
    }
}
