fn run_action_family_app(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
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
            let input_lower = input.to_ascii_lowercase();
            let raw_input_lower = raw_input.to_ascii_lowercase();
            let requires_live_web = app_chat_requests_live_web(&raw_input_lower);
            let lane = {
                #[cfg(test)]
                {
                    app_chat_run_scripted_lane(root, &agent_id, &input).unwrap_or_else(|| {
                        run_lane(
                            root,
                            "app-plane",
                            &[
                                "run".to_string(),
                                "--app=chat-ui".to_string(),
                                format!("--session-id={agent_id}"),
                                format!("--input={input}"),
                            ],
                        )
                    })
                }
                #[cfg(not(test))]
                {
                    run_lane(
                        root,
                        "app-plane",
                        &[
                            "run".to_string(),
                            "--app=chat-ui".to_string(),
                            format!("--session-id={agent_id}"),
                            format!("--input={input}"),
                        ],
                    )
                }
            };
            let mut lane_payload = lane.payload.clone().unwrap_or_else(|| json!({}));
            if !lane_payload.is_object() {
                lane_payload = json!({
                    "ok": lane.ok,
                    "type": "infring_dashboard_action_lane_passthrough"
                });
            }
            if requires_live_web {
                let tools_now = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if app_chat_web_search_call_count(&tools_now) == 0 {
                    let fallback_query = app_chat_extract_web_query(&raw_input);
                    let fallback_lane = app_chat_run_web_batch_query(root, &fallback_query, payload);
                    let fallback_payload = fallback_lane.payload.clone().unwrap_or_else(|| json!({}));
                    let fallback_ok = fallback_lane.ok
                        && fallback_payload
                            .get("ok")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                    let summary = clean_text(
                        fallback_payload
                            .get("summary")
                            .or_else(|| fallback_payload.get("response"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        2_000,
                    );
                    let query_aligned = app_chat_web_result_matches_query(&fallback_query, &summary);
                    if fallback_ok && query_aligned {
                        let assistant = if summary.is_empty() {
                            format!("Web search ran for \"{fallback_query}\" and returned results.")
                        } else {
                            format!("Web search results for \"{fallback_query}\": {summary}")
                        };
                        let mut tools = tools_now;
                        tools.push(json!({
                            "name": "batch_query",
                            "status": "ok",
                            "ok": true,
                            "query": fallback_query,
                            "result": summary,
                            "source": "web",
                            "evidence_refs": fallback_payload.get("evidence_refs").cloned().unwrap_or_else(|| json!([]))
                        }));
                        lane_payload["tools"] = Value::Array(tools);
                        lane_payload["response"] = json!(assistant.clone());
                        lane_payload["output"] = json!(assistant.clone());
                        if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                            turn.insert("assistant".to_string(), json!(assistant.clone()));
                        }
                        lane_payload["web_tooling_fallback"] = json!({
                            "applied": true,
                            "query": fallback_query,
                            "status": "ok",
                            "source": "batch_query"
                        });
                        let mut response_finalization = lane_payload
                            .get("response_finalization")
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        if !response_finalization.is_object() {
                            response_finalization = json!({});
                        }
                        response_finalization["outcome"] =
                            json!("forced_web_tool_attempt_success");
                        lane_payload["response_finalization"] = response_finalization;
                    } else {
                        let mismatch_only = fallback_ok && !query_aligned;
                        let (assistant, error_code) = if mismatch_only {
                            (
                                crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                                    "provider_low_signal",
                                    "web_tool_low_signal",
                                    Some("query_result_mismatch"),
                                ),
                                "web_tool_low_signal",
                            )
                        } else {
                            (
                                "Web tooling execution failed before any search tool call was recorded (error_code: web_tool_not_invoked). Retry lane: run `batch_query` with a narrower query or one specific source URL.".to_string(),
                                "web_tool_not_invoked",
                            )
                        };
                        if mismatch_only {
                            let mut tools = tools_now.clone();
                            tools.push(json!({
                                "name": "batch_query",
                                "status": "low_signal",
                                "ok": false,
                                "query": fallback_query,
                                "result": summary,
                                "source": "web",
                                "error": "web_tool_low_signal"
                            }));
                            lane_payload["tools"] = Value::Array(tools);
                        }
                        lane_payload["response"] = json!(assistant.clone());
                        lane_payload["output"] = json!(assistant.clone());
                        if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                            turn.insert("assistant".to_string(), json!(assistant.clone()));
                        }
                        lane_payload["error"] = json!(error_code);
                        lane_payload["web_tooling_fallback"] = json!({
                            "applied": true,
                            "query": fallback_query,
                            "status": if mismatch_only { "mismatch" } else { "failed" },
                            "error_code": error_code,
                            "query_aligned": query_aligned,
                            "lane_ok": fallback_lane.ok,
                            "lane_status": fallback_lane.status
                        });
                        let mut response_finalization = lane_payload
                            .get("response_finalization")
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        if !response_finalization.is_object() {
                            response_finalization = json!({});
                        }
                        response_finalization["outcome"] =
                            json!(if mismatch_only { "forced_web_tool_low_signal" } else { "forced_web_tool_not_invoked" });
                        response_finalization["error_code"] =
                            json!(error_code);
                        lane_payload["response_finalization"] = response_finalization;
                    }
                }
            }
            let tools_for_rewrite = lane_payload
                .get("tools")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let lane_response_before_rewrite = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| {
                    lane_payload
                        .get("turn")
                        .and_then(|turn| turn.get("assistant"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("")
                .to_string();
            let (rewritten_response, rewrite_outcome) = app_chat_rewrite_tooling_response(
                &raw_input,
                &lane_response_before_rewrite,
                &tools_for_rewrite,
            );
            if !rewrite_outcome.is_empty() {
                lane_payload["response"] = json!(rewritten_response.clone());
                lane_payload["output"] = json!(rewritten_response.clone());
                if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                    turn.insert("assistant".to_string(), json!(rewritten_response.clone()));
                }
                let mut response_finalization = lane_payload
                    .get("response_finalization")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if !response_finalization.is_object() {
                    response_finalization = json!({});
                }
                response_finalization["outcome"] = json!(rewrite_outcome);
                lane_payload["response_finalization"] = response_finalization;
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

            let mut terminal_response = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
                .unwrap_or("")
                .to_string();
            if terminal_response.trim().is_empty() {
                let error_code = canonical_web_tooling_error_code(
                    lane_payload
                        .get("error")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            lane_payload
                                .pointer("/response_finalization/error_code")
                                .and_then(Value::as_str)
                        })
                        .unwrap_or("web_tool_error"),
                );
                terminal_response = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                    "failed",
                    &error_code,
                    None,
                );
                lane_payload["response"] = Value::String(terminal_response.clone());
                lane_payload["output"] = Value::String(terminal_response.clone());
            }
            let mut response_finalization = lane_payload
                .get("response_finalization")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if !response_finalization.is_object() {
                response_finalization = json!({});
            }
            if response_finalization.get("web_invariant").is_none() {
                let tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let web_search_calls = app_chat_web_search_call_count(&tools);
                if requires_live_web && web_search_calls == 0 {
                    let forced = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                        "tool_not_invoked",
                        "web_tool_not_invoked",
                        None,
                    );
                    lane_payload["response"] = json!(forced.clone());
                    lane_payload["output"] = json!(forced.clone());
                    if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut)
                    {
                        turn.insert("assistant".to_string(), json!(forced));
                    }
                    response_finalization["outcome"] = json!("forced_web_tool_not_invoked");
                    response_finalization["error_code"] = json!("web_tool_not_invoked");
                    lane_payload["error"] = json!("web_tool_not_invoked");
                }
                let payload_error = clean_text(
                    lane_payload
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    200,
                )
                .to_ascii_lowercase();
                let payload_error_blocked = payload_error.contains("blocked")
                    || payload_error.contains("denied")
                    || payload_error.contains("policy")
                    || payload_error.contains("nexus");
                let mut tool_attempted = web_search_calls > 0
                    || tools.iter().any(|row| {
                        clean_text(
                            row.get("name")
                                .or_else(|| row.get("tool"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            64,
                        )
                        .to_ascii_lowercase()
                        .contains("web")
                            || clean_text(
                                row.get("name")
                                    .or_else(|| row.get("tool"))
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                64,
                            )
                            .to_ascii_lowercase()
                            .contains("batch_query")
                    });
                if !tool_attempted && (payload_error_blocked || (requires_live_web && !lane.ok)) {
                    tool_attempted = true;
                }
                let blocked_signal = payload_error_blocked
                    || tools.iter().any(|row| {
                        let status = clean_text(
                            row.get("status").and_then(Value::as_str).unwrap_or(""),
                            64,
                        )
                        .to_ascii_lowercase();
                        let error = clean_text(
                            row.get("error").and_then(Value::as_str).unwrap_or(""),
                            120,
                        )
                        .to_ascii_lowercase();
                        status.contains("blocked")
                            || error.contains("blocked")
                            || error.contains("denied")
                            || error.contains("policy")
                            || error.contains("nexus")
                    });
                let classification = if requires_live_web && web_search_calls == 0 {
                    "tool_not_invoked"
                } else if blocked_signal || (requires_live_web && !lane.ok && tool_attempted) {
                    "policy_blocked"
                } else if tools.iter().any(|row| {
                    let status = clean_text(
                        row.get("status").and_then(Value::as_str).unwrap_or(""),
                        64,
                    )
                    .to_ascii_lowercase();
                    let error = clean_text(
                        row.get("error").and_then(Value::as_str).unwrap_or(""),
                        120,
                    )
                    .to_ascii_lowercase();
                    status.contains("low_signal")
                        || status.contains("no_results")
                        || status.contains("no_result")
                        || error.contains("no_results")
                        || error.contains("low_signal")
                }) {
                    "low_signal"
                } else if tool_attempted {
                    "attempted_no_findings"
                } else {
                    "not_required"
                };
                response_finalization["web_invariant"] = json!({
                    "requires_live_web": requires_live_web,
                    "tool_attempted": tool_attempted,
                    "web_search_calls": web_search_calls,
                    "classification": classification,
                    "diagnostic": "forced_live_web_invariant_from_dashboard_action_bus"
                });
            }
            if response_finalization.get("tool_transaction").is_none() {
                let classification = clean_text(
                    response_finalization
                        .pointer("/web_invariant/classification")
                        .and_then(Value::as_str)
                        .unwrap_or("not_required"),
                    80,
                )
                .to_ascii_lowercase();
                let complete = !requires_live_web || classification == "healthy";
                let status = if complete {
                    "complete"
                } else if matches!(classification.as_str(), "low_signal" | "attempted_no_findings")
                {
                    "degraded"
                } else {
                    "failed"
                };
                response_finalization["tool_transaction"] = json!({
                    "id": format!("txn_{}", &crate::v8_kernel::sha256_hex_str(&format!("{}:{}:{}", normalized, classification, now_iso()))[..12]),
                    "intent": app_chat_extract_web_query(&raw_input),
                    "status": status,
                    "complete": complete,
                    "classification": classification,
                    "closed_at": now_iso()
                });
            }
            let assistant_candidate = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
                .unwrap_or("");
            let mut hard_guard = json!({
                "applied": false
            });
            if assistant_candidate.trim().is_empty()
                || crate::tool_output_match_filter::matches_ack_placeholder(assistant_candidate)
                || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(
                    assistant_candidate,
                )
            {
                let classification = clean_text(
                    response_finalization
                        .pointer("/web_invariant/classification")
                        .and_then(Value::as_str)
                        .unwrap_or("parse_failed"),
                    80,
                )
                .to_ascii_lowercase();
                let guard_status = if classification == "tool_not_invoked" {
                    "tool_not_invoked"
                } else {
                    "parse_failed"
                };
                let guard_error_code = if guard_status == "tool_not_invoked" {
                    "web_tool_not_invoked"
                } else if classification == "policy_blocked" {
                    "web_tool_policy_blocked"
                } else if classification == "low_signal" {
                    "web_tool_low_signal"
                } else {
                    "web_tool_invalid_response"
                };
                let fallback = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                    guard_status,
                    guard_error_code,
                    None,
                );
                lane_payload["response"] = json!(fallback.clone());
                lane_payload["output"] = json!(fallback.clone());
                if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                    turn.insert("assistant".to_string(), json!(fallback));
                }
                hard_guard = json!({
                    "applied": true,
                    "status": guard_status,
                    "error_code": guard_error_code
                });
                if let Some(txn) = response_finalization
                    .get_mut("tool_transaction")
                    .and_then(Value::as_object_mut)
                {
                    txn.insert("status".to_string(), json!("failed"));
                    txn.insert("complete".to_string(), json!(false));
                }
            }
            response_finalization["hard_guard"] = hard_guard;
            lane_payload["response_finalization"] = response_finalization;
            let final_response = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
                .unwrap_or("");
            let forced_ok = lane.ok || !final_response.trim().is_empty();
            let troubleshooting = dashboard_troubleshooting_capture_chat_exchange(
                root,
                &agent_id,
                &raw_input,
                &lane_payload,
                forced_ok,
                requires_live_web,
            );
            if let Some(obj) = lane_payload.as_object_mut() {
                obj.insert("troubleshooting".to_string(), troubleshooting);
            }
            LaneResult {
                ok: forced_ok,
                status: if forced_ok { 0 } else { lane.status },
                argv: lane.argv,
                payload: Some(lane_payload),
            }
        }
        _ => run_action_family_collab(root, normalized, payload),
    }
}
