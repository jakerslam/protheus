fn message_requests_live_web_comparison(message: &str) -> bool {
    let lowered = clean_text(message, 500).to_ascii_lowercase();
    if lowered.is_empty() || !message_requests_comparative_answer(message) {
        return false;
    }
    let mentions_external_peer = lowered.contains("openclaw")
        || lowered.contains("chatgpt")
        || lowered.contains("claude")
        || lowered.contains("codex")
        || lowered.contains("cursor")
        || lowered.contains("copilot")
        || lowered.contains("windsurf")
        || lowered.contains("perplexity");
    let asks_live_web = lowered.contains("web")
        || lowered.contains("web search")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("browse")
        || lowered.contains("online")
        || lowered.contains("latest")
        || lowered.contains("current")
        || lowered.contains("today")
        || lowered.contains("source-backed")
        || lowered.contains("with sources");
    asks_live_web && mentions_external_peer
}

fn comparative_web_query_from_message(message: &str) -> Option<String> {
    if !message_requests_live_web_comparison(message) {
        return None;
    }
    let query = clean_text(message, 600);
    if query.is_empty() { None } else { Some(query) }
}

fn comparative_no_findings_fallback(message: &str) -> String {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    let asks_rank = lowered.contains("rank") || lowered.contains("ranking");
    let asks_structured_compare = lowered.contains("compare")
        || lowered.contains("comparison")
        || lowered.contains("vs")
        || lowered.contains("versus");
    if asks_rank || asks_structured_compare {
        return "Live web retrieval was low-signal in this turn (search-engine chrome without extractable findings). Provisional comparison: Infring is strongest in identity persistence, memory continuity, and integrated tool orchestration; top peers are currently stronger on tool/search failure recovery and handoff consistency. Ask me to rerun `batch_query` with named competitors and I will return a source-backed ranked table.".to_string();
    }
    "Live web retrieval was low-signal in this turn, so here is the stable comparison: Infring is strongest in identity persistence, memory continuity, and integrated tool orchestration, while mature peers are still stronger on failure recovery and handoff consistency. If you want live sourcing, I can rerun with `batch_query` and a narrower competitor set.".to_string()
}

fn comparative_natural_web_intent_from_message(message: &str) -> Option<(String, Value)> {
    comparative_web_query_from_message(message).map(|query| {
        (
            "batch_query".to_string(),
            json!({"source": "web", "query": query, "aperture": "medium"}),
        )
    })
}

fn execute_tool_call_by_name(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    tool_name: &str,
    input: &Value,
) -> Value {
    let normalized = normalize_tool_name(tool_name);
    let resolved = resolve_tool_name_fallback(&normalized, input);
    let actor = clean_agent_id(actor_agent_id);
    if actor.is_empty() {
        return json!({
            "ok": false,
            "error": "actor_agent_required"
        });
    }
    if let Some(gate_payload) =
        enforce_tool_capability_tier(root, snapshot, &actor, &resolved, input)
    {
        return gate_payload;
    }
    let headers = vec![("X-Actor-Agent-Id", actor.as_str())];
    let route_with_body = |method: &str, path: &str, body: &Value| -> Value {
        let body_bytes = serde_json::to_vec(body).unwrap_or_default();
        handle_with_headers(root, method, path, &body_bytes, &headers, snapshot)
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
    };
    let route_without_body = |method: &str, path: &str| -> Value {
        handle_with_headers(root, method, path, &[], &headers, snapshot)
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
    };
    match resolved.as_str() {
        "tool_capabilities" | "capabilities" | "capability_status" | "tools_status" => {
            let mut payload = route_without_body("GET", "/api/capabilities/status");
            if let Some(obj) = payload.as_object_mut() {
                let read_surfaces = vec![
                    json!({"name":"workspace_analyze","route":"terminal_exec(read-only alias)","default_enabled":true}),
                    json!({"name":"file_read","route":"agent file read","default_enabled":true}),
                    json!({"name":"file_read_many","route":"agent file read-many","default_enabled":true}),
                    json!({"name":"folder_export","route":"agent folder export","default_enabled":true}),
                    json!({"name":"terminal_exec","route":"agent terminal","default_enabled":true}),
                ];
                obj.insert("command_surface".to_string(), json!("governed_tool_router"));
                obj.insert("read_surfaces".to_string(), Value::Array(read_surfaces));
                obj.insert(
                    "explicit_tool_commands".to_string(),
                    Value::Array(
                        EXPLICIT_SUPPORTED_TOOL_COMMANDS
                            .iter()
                            .map(|value| Value::String((*value).to_string()))
                            .collect::<Vec<_>>(),
                    ),
                );
            }
            payload
        }
        "file_read" | "read_file" | "file" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/file/read");
            route_with_body("POST", &path, &body)
        }
        "file_read_many" | "read_files" | "files_read" | "batch_file_read" => {
            let body = if input.is_object() {
                input.clone()
            } else if let Some(value) = input.as_array() {
                json!({"paths": value})
            } else {
                let raw = clean_text(input.as_str().unwrap_or(""), 12000);
                let paths = raw
                    .split(|ch: char| ch == '\n' || ch == ',' || ch == ';')
                    .map(str::trim)
                    .filter(|row| !row.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                json!({"paths": paths})
            };
            let path = format!("/api/agents/{actor}/file/read-many");
            route_with_body("POST", &path, &body)
        }
        "folder_export" | "list_folder" | "folder_tree" | "folder" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/folder/export");
            route_with_body("POST", &path, &body)
        }
        "workspace_analyze" | "workspace_scan" | "analyze_workspace" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or("."), 4000)})
            };
            let path = format!("/api/agents/{actor}/folder/export");
            route_with_body("POST", &path, &body)
        }
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => {
            let mut body = if input.is_object() {
                input.clone()
            } else {
                json!({"command": clean_text(input.as_str().unwrap_or(""), 12000)})
            };
            let current_command = clean_text(
                body.get("command")
                    .or_else(|| body.get("cmd"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                12_000,
            );
            if current_command.is_empty() {
                if let Some(fallback_command) = terminal_alias_command_for_tool(&normalized, input)
                {
                    body["command"] = Value::String(fallback_command);
                }
            }
            let has_command = !clean_text(
                body.get("command")
                    .or_else(|| body.get("cmd"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                12_000,
            )
            .is_empty();
            if !has_command {
                return json!({
                    "ok": false,
                    "error": "command_required",
                    "tool": resolved,
                    "next_step": "Provide `command` in the terminal tool input."
                });
            }
            let path = format!("/api/agents/{actor}/terminal");
            route_with_body("POST", &path, &body)
        }
        "web_fetch" | "browse" | "web_conduit_fetch" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"url": clean_text(input.as_str().unwrap_or(""), 2200)})
            };
            route_with_body("POST", "/api/web/fetch", &body)
        }
        "batch_query" | "batch-query" | "web_search" | "search_web" | "search" | "web_query" => {
            let mut body = if input.is_object() {
                input.clone()
            } else {
                json!({"query": clean_text(input.as_str().unwrap_or(""), 600)})
            };
            if body
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                body["source"] = json!("web");
            }
            if body
                .get("aperture")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                body["aperture"] = json!("medium");
            }
            route_with_body("POST", "/api/batch-query", &body)
        }
        "cron_list" | "schedule_list" | "cron_jobs" => {
            route_without_body("GET", "/api/cron/jobs")
        }
        "cron_schedule" | "schedule_task" | "cron_create" => {
            let interval_minutes =
                parse_non_negative_i64(input.get("interval_minutes"), 60).clamp(1, 10_080);
            let default_name = format!("{}-{}m-checkin", actor, interval_minutes);
            let job_name = clean_text(
                input
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(default_name.as_str()),
                180,
            );
            let action_message = clean_text(
                input
                    .get("message")
                    .or_else(|| input.get("task"))
                    .or_else(|| input.get("objective"))
                    .and_then(Value::as_str)
                    .unwrap_or("Scheduled follow-up check."),
                2_000,
            );
            let mut request_body = json!({
                "name": if job_name.is_empty() { default_name } else { job_name },
                "agent_id": actor,
                "enabled": input.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                "schedule": {
                    "kind": "every",
                    "every_secs": interval_minutes.saturating_mul(60)
                },
                "action": {
                    "kind": "agent_turn",
                    "message": if action_message.is_empty() {
                        "Scheduled follow-up check."
                    } else {
                        action_message.as_str()
                    }
                }
            });
            if let Some(custom_schedule) = input.get("schedule").cloned() {
                request_body["schedule"] = custom_schedule;
            }
            route_with_body("POST", "/api/cron/jobs", &request_body)
        }
        "cron_cancel" | "cron_delete" | "schedule_cancel" => {
            let job_id = clean_text(
                input
                    .get("job_id")
                    .or_else(|| input.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return json!({"ok": false, "error": "job_id_required"});
            }
            let path = format!("/api/cron/jobs/{job_id}");
            route_without_body("DELETE", &path)
        }
        "cron_run" | "schedule_run" | "cron_trigger" => {
            let job_id = clean_text(
                input
                    .get("job_id")
                    .or_else(|| input.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return json!({"ok": false, "error": "job_id_required"});
            }
            let path = format!("/api/schedules/{job_id}/run");
            route_without_body("POST", &path)
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            execute_spawn_subagents_tool(root, snapshot, &actor, input, &headers)
        }
        "session_rollback_last_turn" | "undo_last_turn" | "rewind_turn" => {
            rollback_last_turn(root, &actor)
        }
        "memory_kv_get" => {
            let key = clean_text(input.get("key").and_then(Value::as_str).unwrap_or(""), 180);
            if key.is_empty() {
                return json!({"ok": false, "error": "memory_key_required"});
            }
            crate::dashboard_agent_state::memory_kv_get(root, &actor, &key)
        }
        "memory_kv_set" => {
            let key = clean_text(input.get("key").and_then(Value::as_str).unwrap_or(""), 180);
            if key.is_empty() {
                return json!({"ok": false, "error": "memory_key_required"});
            }
            let value = input.get("value").cloned().unwrap_or(Value::Null);
            crate::dashboard_agent_state::memory_kv_set(root, &actor, &key, &value)
        }
        "memory_kv_list" | "memory_kv_pairs" => {
            crate::dashboard_agent_state::memory_kv_pairs(root, &actor)
        }
        "memory_semantic_query" | "memory_query" => {
            let query = clean_text(
                input
                    .get("query")
                    .or_else(|| input.get("q"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            );
            let limit = input
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(8)
                .clamp(1, 25);
            crate::dashboard_agent_state::memory_kv_semantic_query(root, &actor, &query, limit)
        }
        "agent_action" | "manage_agent" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let target = clean_agent_id(
                input
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or(actor.as_str()),
            );
            if target.is_empty() || action.is_empty() {
                return json!({"ok": false, "error": "agent_action_and_target_required"});
            }
            let parent_archive_override = parent_can_archive_descendant_without_signoff(
                root,
                snapshot,
                &actor,
                &normalized,
                input,
            );
            let (method, path, body) = match action.as_str() {
                "start" => ("POST", format!("/api/agents/{target}/start"), json!({})),
                "stop" => ("POST", format!("/api/agents/{target}/stop"), json!({})),
                "archive" | "delete" => (
                    "DELETE",
                    format!("/api/agents/{target}"),
                    if parent_archive_override {
                        json!({
                            "reason": "Archived by parent agent",
                            "termination_reason": "parent_archived"
                        })
                    } else {
                        json!({})
                    },
                ),
                "clone" => (
                    "POST",
                    format!("/api/agents/{target}/clone"),
                    json!({"new_name": input.get("new_name").cloned().unwrap_or(Value::Null)}),
                ),
                "message" => (
                    "POST",
                    format!("/api/agents/{target}/message"),
                    json!({"message": clean_text(input.get("message").and_then(Value::as_str).unwrap_or(""), 8000)}),
                ),
                "spawn" | "spawn_subagent" => (
                    "POST",
                    "/api/agents".to_string(),
                    json!({
                        "name": clean_text(input.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                        "role": clean_text(input.get("role").and_then(Value::as_str).unwrap_or("analyst"), 60),
                        "parent_agent_id": target,
                        "contract": {
                            "owner": clean_text(input.get("owner").and_then(Value::as_str).unwrap_or("manage_agent_spawn"), 80),
                            "mission": clean_text(input.get("mission").and_then(Value::as_str).unwrap_or("Assist parent mission"), 200),
                            "termination_condition": "task_or_timeout",
                            "expiry_seconds": input.get("expiry_seconds").and_then(Value::as_i64).unwrap_or(3600).clamp(60, 172_800),
                            "auto_terminate_allowed": input.get("auto_terminate_allowed").and_then(Value::as_bool).unwrap_or(true),
                            "idle_terminate_allowed": input.get("idle_terminate_allowed").and_then(Value::as_bool).unwrap_or(true)
                        }
                    }),
                ),
                _ => {
                    return json!({
                        "ok": false,
                        "error": "unsupported_agent_action",
                        "action": action
                    })
                }
            };
            route_with_body(method, &path, &body)
        }
        "tool_command_router" => {
            let mut out = if input.is_object() {
                input.clone()
            } else {
                json!({})
            };
            if out.get("ok").is_none() {
                out["ok"] = Value::Bool(false);
            }
            if out.get("error").and_then(Value::as_str).unwrap_or("").is_empty() {
                out["error"] = json!("invalid_tool_command");
            }
            if out.get("message").and_then(Value::as_str).unwrap_or("").is_empty() {
                out["message"] =
                    json!("Invalid `tool::` command. Use `tool::<command>:::<params>`.");
            }
            out
        }
        "tabs_list" | "list_tabs" => {
            let _ = existing;
            json!({
                "ok": true,
                "tabs": [
                    "agents",
                    "chat",
                    "channels",
                    "plugins",
                    "sessions",
                    "approvals",
                    "workflows",
                    "scheduler",
                    "settings",
                    "network",
                    "security",
                    "usage",
                    "comms"
                ]
            })
        }
        _ => json!({
            "ok": false,
            "error": "unsupported_tool",
            "tool": tool_name,
            "resolved_tool": resolved
        }),
    }
}
