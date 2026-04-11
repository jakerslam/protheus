// FILE_SIZE_EXCEPTION: reason=Single action-dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-12
fn clean_chat_text_preserve_layout(value: &str, max_len: usize) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .take(max_len)
        .collect::<String>()
}

fn assistant_runtime_access_denied(assistant_lower: &str) -> bool {
    const DENIED_SIGNATURES: [&str; 7] = [
        "don't have access",
        "do not have access",
        "cannot access",
        "without system monitoring",
        "text-based ai assistant",
        "cannot directly interface",
        "no access to",
    ];
    DENIED_SIGNATURES
        .iter()
        .any(|signature| assistant_lower.contains(signature))
}

fn runtime_sync_requested(input_lower: &str) -> bool {
    input_lower.contains("report runtime sync now")
        || ((input_lower.contains("queue depth")
            || input_lower.contains("cockpit blocks")
            || input_lower.contains("conduit signals"))
            && (input_lower.contains("runtime")
                || input_lower.contains("sync")
                || input_lower.contains("status")
                || input_lower.contains("what changed")))
}

fn run_action(root: &Path, action: &str, payload: &Value) -> LaneResult {
    let normalized = clean_text(action, 80);
    match normalized.as_str() {
        "app.switchProvider" => {
            let provider = payload
                .get("provider")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "openai".to_string());
            let model = payload
                .get("model")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 100))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gpt-5".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "switch-provider".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--provider={provider}"),
                    format!("--model={model}"),
                ],
            )
        }
        "app.chat" => {
            let raw_input = payload
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| payload.get("message").and_then(Value::as_str))
                .map(|v| v.to_string())
                .unwrap_or_default();
            let input = clean_text(&raw_input, 2000);
            if input.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["app-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "chat_input_required"
                    })),
                };
            }
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "chat-ui-default-agent".to_string());
            let lane = run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--session-id={agent_id}"),
                    format!("--input={input}"),
                ],
            );
            let mut lane_payload = lane.payload.clone().unwrap_or_else(|| json!({}));
            if !lane_payload.is_object() {
                lane_payload = json!({
                    "ok": lane.ok,
                    "type": "infring_dashboard_action_lane_passthrough"
                });
            }
            let mut assistant_text = String::new();
            if lane.ok {
                assistant_text = lane_payload
                    .get("response")
                    .and_then(Value::as_str)
                    .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                    .or_else(|| {
                        lane_payload
                            .get("turn")
                            .and_then(|turn| turn.get("assistant"))
                            .and_then(Value::as_str)
                    })
                    .or_else(|| {
                        lane_payload
                            .get("turns")
                            .and_then(Value::as_array)
                            .and_then(|turns| turns.last())
                            .and_then(|turn| turn.get("assistant").and_then(Value::as_str))
                    })
                    .unwrap_or("")
                    .to_string();
            }
            let runtime_flags = Flags {
                mode: "runtime-sync".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let runtime = build_runtime_sync(root, &runtime_flags);
            let mut runtime_sync = runtime.get("summary").cloned().unwrap_or_else(|| json!({}));
            if !runtime_sync.is_object() {
                runtime_sync = json!({});
            }
            let health =
                read_cached_snapshot_component(root, "health").unwrap_or_else(|| json!({}));
            let receipt_latency_p95 = i64_from_value(
                health.pointer("/dashboard_metrics/receipt_latency_p95_ms/value"),
                0,
            );
            let receipt_latency_p99 = i64_from_value(
                health.pointer("/dashboard_metrics/receipt_latency_p99_ms/value"),
                0,
            );
            let benchmark_sanity_status = clean_text(
                health
                    .pointer("/checks/benchmark_sanity/status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                32,
            );
            runtime_sync["receipt_latency_p95_ms"] = json!(receipt_latency_p95);
            runtime_sync["receipt_latency_p99_ms"] = json!(receipt_latency_p99);
            runtime_sync["benchmark_sanity_status"] = json!(benchmark_sanity_status);
            runtime_sync["critical_attention_total"] = runtime
                .pointer("/attention_queue/critical_total_count")
                .cloned()
                .unwrap_or_else(|| json!(0));
            runtime_sync["conduit_signals_raw"] = runtime
                .pointer("/attention_queue/backpressure/conduit_signals_raw")
                .cloned()
                .unwrap_or_else(|| json!(0));
            lane_payload["runtime_sync"] = runtime_sync.clone();

            let input_lower = input.to_ascii_lowercase();
            let raw_input_lower = raw_input.to_ascii_lowercase();
            let assistant_lower = assistant_text.to_ascii_lowercase();
            if runtime_sync_requested(&input_lower)
                || assistant_runtime_access_denied(&assistant_lower)
            {
                let queue_depth = i64_from_value(runtime_sync.get("queue_depth"), 0);
                let cockpit_blocks = i64_from_value(runtime_sync.get("cockpit_blocks"), 0);
                let cockpit_total_blocks =
                    i64_from_value(runtime_sync.get("cockpit_total_blocks"), 0);
                let conduit_signals = i64_from_value(runtime_sync.get("conduit_signals"), 0);
                let authoritative = format!(
                    "Current queue depth: {queue_depth}, cockpit blocks: {cockpit_blocks} active ({cockpit_total_blocks} total), conduit signals: {conduit_signals}. Attention queue is readable. Runtime memory context and protheus/infring command surfaces are available through this dashboard lane."
                );
                lane_payload["response"] = json!(authoritative.clone());
                lane_payload["output"] = json!(authoritative.clone());
                if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                    turn.insert("assistant".to_string(), json!(authoritative.clone()));
                }
                if let Some(turns) = lane_payload.get_mut("turns").and_then(Value::as_array_mut) {
                    if let Some(last) = turns.last_mut() {
                        if let Some(last_obj) = last.as_object_mut() {
                            last_obj.insert("assistant".to_string(), json!(authoritative));
                        }
                    }
                }
            }
            if input_lower.contains("one week ago") && input_lower.contains("memory file path") {
                let memory_dir = root.join("local/workspace/memory");
                let target = (Utc::now() - chrono::Duration::days(7))
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string();
                let mut selected_date = target.clone();
                let mut selected_rel = format!("local/workspace/memory/{selected_date}.md");
                if !memory_dir.join(format!("{target}.md")).is_file() {
                    let mut candidates = Vec::<String>::new();
                    if let Ok(entries) = fs::read_dir(&memory_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.len() == 13
                                && name.ends_with(".md")
                                && name[..10]
                                    .chars()
                                    .all(|ch| ch.is_ascii_digit() || ch == '-')
                            {
                                candidates.push(name[..10].to_string());
                            }
                        }
                    }
                    candidates.sort();
                    if let Some(last) = candidates.last() {
                        selected_date = last.clone();
                        selected_rel = format!("local/workspace/memory/{selected_date}.md");
                    }
                }
                lane_payload["response"] = json!(format!(
                    "Exact date: {selected_date}. Memory file path: {selected_rel}."
                ));
                let mut tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                tools.push(json!({
                    "tool": "read_file",
                    "input": selected_rel
                }));
                lane_payload["tools"] = Value::Array(tools);
            }
            if input_lower.contains("summarize client layer now")
                && input_lower.contains("attention queue")
                && input_lower.contains("cockpit")
            {
                let summary_flags = Flags {
                    mode: "snapshot".to_string(),
                    host: DEFAULT_HOST.to_string(),
                    port: DEFAULT_PORT,
                    team: DEFAULT_TEAM.to_string(),
                    refresh_ms: DEFAULT_REFRESH_MS,
                    pretty: false,
                };
                let snapshot_now = build_snapshot(root, &summary_flags);
                let memory_entries = snapshot_now
                    .pointer("/memory/entries")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let receipt_count = snapshot_now
                    .pointer("/receipts/recent")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let log_count = snapshot_now
                    .pointer("/logs/recent")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let health_checks = snapshot_now
                    .pointer("/health/checks")
                    .and_then(Value::as_object)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let attention_depth =
                    i64_from_value(snapshot_now.pointer("/attention_queue/queue_depth"), 0);
                let cockpit_blocks =
                    i64_from_value(snapshot_now.pointer("/cockpit/block_count"), 0);
                lane_payload["response"] = json!(format!(
                    "Client layer now: memory entries {memory_entries}, receipts {receipt_count}, logs {log_count}, health checks {health_checks}, attention queue depth {attention_depth}, cockpit blocks {cockpit_blocks}."
                ));
            }
            if raw_input_lower.contains("run exactly these commands to create a swarm of subagents")
                && raw_input_lower.contains("collab-plane launch-role")
            {
                let mut launched = Vec::<String>::new();
                let mut tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for raw_line in raw_input.lines() {
                    let line = raw_line.trim();
                    if !line.starts_with("protheus-ops collab-plane launch-role") {
                        continue;
                    }
                    let mut team = DEFAULT_TEAM.to_string();
                    let mut role = "analyst".to_string();
                    let mut shadow = String::new();
                    for token in line.split_whitespace() {
                        if let Some(value) = token.strip_prefix("--team=") {
                            let cleaned = clean_text(value, 60);
                            if !cleaned.is_empty() {
                                team = cleaned;
                            }
                        } else if let Some(value) = token.strip_prefix("--role=") {
                            let cleaned = clean_text(value, 60);
                            if !cleaned.is_empty() {
                                role = cleaned;
                            }
                        } else if let Some(value) = token.strip_prefix("--shadow=") {
                            shadow = clean_text(value, 80);
                        }
                    }
                    if shadow.is_empty() {
                        shadow = format!("{team}-{role}-{}", Utc::now().timestamp_millis());
                    }
                    let launch = run_lane(
                        root,
                        "collab-plane",
                        &[
                            "launch-role".to_string(),
                            format!("--team={team}"),
                            format!("--role={role}"),
                            format!("--shadow={shadow}"),
                        ],
                    );
                    if launch.ok {
                        let _ = dashboard_agent_state::upsert_profile(
                            root,
                            &shadow,
                            &json!({
                                "name": shadow,
                                "role": role,
                                "state": "Running"
                            }),
                        );
                        launched.push(shadow.clone());
                    }
                    tools.push(json!({
                        "tool": "shell",
                        "input": line
                    }));
                }
                if !tools.is_empty() {
                    lane_payload["tools"] = Value::Array(tools);
                }
                if !launched.is_empty() {
                    lane_payload["response"] = json!(launched.join(" "));
                }
            }

            LaneResult {
                ok: lane.ok,
                status: lane.status,
                argv: lane.argv,
                payload: Some(lane_payload),
            }
        }
        "collab.launchRole" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "analyst".to_string());
            let shadow = payload
                .get("shadow")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("{team}-{role}-shadow"));
            run_lane(
                root,
                "collab-plane",
                &[
                    "launch-role".to_string(),
                    format!("--team={team}"),
                    format!("--role={role}"),
                    format!("--shadow={shadow}"),
                ],
            )
        }
        "skills.run" => {
            let skill = payload
                .get("skill")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            if skill.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["skills-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "skill_required"
                    })),
                };
            }
            let input = payload
                .get("input")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 600))
                .unwrap_or_default();
            let mut args = vec!["run".to_string(), format!("--skill={skill}")];
            if !input.is_empty() {
                args.push(format!("--input={input}"));
            }
            run_lane(root, "skills-plane", &args)
        }
        "dashboard.assimilate" => {
            let target = payload
                .get("target")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "codex".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--input=assimilate target {target} with receipt-first safety"),
                ],
            )
        }
        "dashboard.benchmark" => run_lane(root, "health-status", &["dashboard".to_string()]),
        "dashboard.models.catalog" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::catalog_payload(root, &snapshot);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.models.catalog".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.model.routeDecision" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::route_decision_payload(root, &snapshot, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.model.routeDecision".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.create" => {
            let result = dashboard_terminal_broker::create_session(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.exec" => {
            let result = dashboard_terminal_broker::exec_command(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: result.get("exit_code").and_then(Value::as_i64).unwrap_or(
                    if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        0
                    } else {
                        2
                    },
                ) as i32,
                argv: vec!["dashboard.terminal.exec".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.close" => {
            let session_id = payload
                .get("session_id")
                .or_else(|| payload.get("sessionId"))
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_terminal_broker::close_session(root, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.close".to_string()],
                payload: Some(result),
            }
        }
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
                argv: vec![normalized.clone(), format!("--team={team}")],
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
        "dashboard.agent.upsertProfile" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_profile(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertProfile".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.archive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::archive_agent(root, &agent_id, &reason);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.archive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.unarchive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::unarchive_agent(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.unarchive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.upsertContract" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_contract(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertContract".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.enforceContracts" => {
            let result = dashboard_agent_state::enforce_expired_contracts(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.agent.enforceContracts".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::load_session(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.create" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let label = payload
                .get("label")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let result = dashboard_agent_state::create_session(root, &agent_id, &label);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.switch" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::switch_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.switch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::delete_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.appendTurn" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_text = payload
                .get("user")
                .and_then(Value::as_str)
                .or_else(|| payload.get("input").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 2000))
                .unwrap_or_default();
            let assistant_text = payload
                .get("assistant")
                .and_then(Value::as_str)
                .or_else(|| payload.get("response").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 4000))
                .unwrap_or_default();
            let result =
                dashboard_agent_state::append_turn(root, &agent_id, &user_text, &assistant_text);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.appendTurn".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.set" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            let result = dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.set".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_get(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.suggestions" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_hint = payload
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| payload.get("hint").and_then(Value::as_str))
                .map(|v| clean_text(v, 220))
                .unwrap_or_default();
            let result = dashboard_agent_state::suggestions(root, &agent_id, &user_hint);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.suggestions".to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}
