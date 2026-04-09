// FILE_SIZE_EXCEPTION: reason=Function-scale route/tool block retained atomically during staged decomposition; owner=jay; expires=2026-04-22
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
    match resolved.as_str() {
        "file_read" | "read_file" | "file" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/file/read");
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "folder_export" | "list_folder" | "folder_tree" | "folder" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/folder/export");
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "web_fetch" | "browse" | "web_conduit_fetch" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"url": clean_text(input.as_str().unwrap_or(""), 2200)})
            };
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/web/fetch",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/batch-query",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "cron_list" | "schedule_list" | "cron_jobs" => {
            handle_with_headers(root, "GET", "/api/cron/jobs", &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            let body_bytes = serde_json::to_vec(&request_body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/cron/jobs",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            handle_with_headers(root, "DELETE", &path, &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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
            handle_with_headers(root, "POST", &path, &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let spawn_policy = spawn_guard_policy(root);
            let max_per_spawn = spawn_policy
                .get("max_per_spawn")
                .and_then(Value::as_i64)
                .unwrap_or(8)
                .clamp(1, 64) as usize;
            let max_descendants_per_parent = spawn_policy
                .get("max_descendants_per_parent")
                .and_then(Value::as_i64)
                .unwrap_or(24)
                .clamp(1, 4096) as usize;
            let depth_limit = spawn_policy
                .get("max_depth")
                .and_then(Value::as_i64)
                .unwrap_or(4)
                .clamp(1, 32) as usize;
            let per_child_budget_default = spawn_policy
                .get("per_child_budget_default")
                .and_then(Value::as_i64)
                .unwrap_or(800)
                .clamp(64, 200_000);
            let per_child_budget_max = spawn_policy
                .get("per_child_budget_max")
                .and_then(Value::as_i64)
                .unwrap_or(5000)
                .clamp(per_child_budget_default, 2_000_000);
            let spawn_budget_cap = spawn_policy
                .get("spawn_budget_cap")
                .and_then(Value::as_i64)
                .unwrap_or(per_child_budget_max.saturating_mul(max_per_spawn as i64))
                .clamp(per_child_budget_max, 20_000_000);

            let requested_count_raw = input
                .get("count")
                .or_else(|| input.get("team_size"))
                .or_else(|| input.get("agents"))
                .and_then(Value::as_i64)
                .unwrap_or(3);
            let requested_count_raw_pos = requested_count_raw.max(1) as usize;
            let requested_count = requested_count_raw_pos.min(max_per_spawn);
            let expiry_seconds = input
                .get("expiry_seconds")
                .or_else(|| input.get("lifespan_sec"))
                .and_then(Value::as_i64)
                .unwrap_or(3600)
                .clamp(60, 172_800);
            let budget_tokens_requested_raw = input
                .get("budget_tokens")
                .or_else(|| input.get("token_budget"))
                .and_then(Value::as_i64)
                .unwrap_or(per_child_budget_default);
            let budget_tokens = budget_tokens_requested_raw.clamp(64, per_child_budget_max);
            let budget_tokens_for_capacity =
                budget_tokens_requested_raw.clamp(64, spawn_budget_cap);
            let objective = clean_text(
                input
                    .get("objective")
                    .or_else(|| input.get("task"))
                    .or_else(|| input.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Parallel child task requested by parent directive."),
                800,
            );
            let merge_strategy = match clean_text(
                input
                    .get("merge_strategy")
                    .or_else(|| input.get("merge"))
                    .and_then(Value::as_str)
                    .unwrap_or("reduce"),
                40,
            )
            .to_ascii_lowercase()
            .as_str()
            {
                "voting" | "vote" => "voting",
                "concat" | "concatenate" => "concatenate",
                _ => "reduce",
            }
            .to_string();
            let mut role_plan = input
                .get("roles")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| clean_text(row, 60))
                        .filter(|row| !row.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let role_hint = clean_text(
                input
                    .get("role")
                    .or_else(|| input.get("default_role"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                60,
            );
            if !role_hint.is_empty() && role_plan.is_empty() {
                role_plan.push(role_hint);
            }
            if role_plan.is_empty() {
                role_plan = vec![
                    "analyst".to_string(),
                    "researcher".to_string(),
                    "builder".to_string(),
                    "reviewer".to_string(),
                ];
            }
            let base_name = clean_text(
                input
                    .get("base_name")
                    .or_else(|| input.get("name_prefix"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            let parent_map = agent_parent_map(root, snapshot);
            let current_depth = agent_depth_from_parent_map(&parent_map, &actor);
            if current_depth + 1 > depth_limit {
                return json!({
                    "ok": false,
                    "error": "spawn_depth_limit_exceeded",
                    "parent_agent_id": actor,
                    "current_depth": current_depth,
                    "max_depth": depth_limit
                });
            }
            let existing_descendants = descendant_count(&parent_map, &actor);
            if existing_descendants >= max_descendants_per_parent {
                return json!({
                    "ok": false,
                    "error": "spawn_descendant_limit_exceeded",
                    "parent_agent_id": actor,
                    "existing_descendants": existing_descendants,
                    "max_descendants_per_parent": max_descendants_per_parent
                });
            }
            let remaining_capacity =
                max_descendants_per_parent.saturating_sub(existing_descendants);
            let budget_limited_count =
                ((spawn_budget_cap / budget_tokens_for_capacity.max(1)) as usize).max(1);
            let effective_count = requested_count
                .min(remaining_capacity.max(1))
                .min(budget_limited_count.max(1));
            if effective_count == 0 {
                return json!({
                    "ok": false,
                    "error": "spawn_budget_exceeded",
                    "parent_agent_id": actor,
                    "spawn_budget_cap": spawn_budget_cap,
                    "requested_budget_tokens": budget_tokens
                });
            }
            let context_slice = subagent_context_slice(root, &actor, &objective);
            let directive_receipt = crate::deterministic_receipt_hash(&json!({
                "type": "agent_spawn_directive",
                "actor_agent_id": actor,
                "requested_count_raw": requested_count_raw,
                "requested_count": requested_count,
                "effective_count": effective_count,
                "objective": objective,
                "merge_strategy": merge_strategy,
                "budget_tokens": budget_tokens,
                "budget_tokens_requested_raw": budget_tokens_requested_raw,
                "budget_tokens_for_capacity": budget_tokens_for_capacity,
                "requested_at": crate::now_iso()
            }));

            let mut created = Vec::<Value>::new();
            let mut errors = Vec::<Value>::new();
            for idx in 0..effective_count {
                let role = role_plan
                    .get(idx % role_plan.len())
                    .cloned()
                    .unwrap_or_else(|| "analyst".to_string());
                let mut request_body = json!({
                    "role": role,
                    "parent_agent_id": actor,
                    "contract": {
                        "owner": "descendant_auto_spawn",
                        "mission": if objective.is_empty() {
                            format!("Parallel subtask for parent {}", actor)
                        } else {
                            format!("Parallel subtask for parent {}: {}", actor, objective)
                        },
                        "termination_condition": "task_or_timeout",
                        "expiry_seconds": expiry_seconds,
                        "auto_terminate_allowed": true,
                        "budget_tokens": budget_tokens,
                        "merge_strategy": merge_strategy,
                        "context_slice": context_slice,
                        "source_user_directive": objective,
                        "source_user_directive_receipt": directive_receipt,
                        "spawn_guard": {
                            "max_depth": depth_limit,
                            "max_descendants_per_parent": max_descendants_per_parent,
                            "spawn_budget_cap": spawn_budget_cap
                        }
                    }
                });
                if !base_name.is_empty() {
                    request_body["name"] = json!(format!("{base_name}-{}", idx + 1));
                }
                let body_bytes = serde_json::to_vec(&request_body).unwrap_or_default();
                let spawned = handle_with_headers(
                    root,
                    "POST",
                    "/api/agents",
                    &body_bytes,
                    &headers,
                    snapshot,
                )
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}));
                if spawned.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    created.push(json!({
                        "agent_id": clean_agent_id(
                            spawned
                                .get("agent_id")
                                .or_else(|| spawned.get("id"))
                                .and_then(Value::as_str)
                                .unwrap_or("")
                        ),
                        "name": clean_text(spawned.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                        "role": role
                    }));
                } else {
                    errors.push(json!({
                        "role": role,
                        "error": clean_text(spawned.get("error").and_then(Value::as_str).unwrap_or("spawn_failed"), 160)
                    }));
                }
            }
            let mut out = json!({
                "ok": !created.is_empty(),
                "type": "spawn_subagents",
                "parent_agent_id": actor,
                "requested_count_raw": requested_count_raw,
                "requested_count": requested_count,
                "effective_count": effective_count,
                "created_count": created.len(),
                "failed_count": errors.len(),
                "directive": {
                    "objective": objective,
                    "receipt": directive_receipt,
                    "merge_strategy": merge_strategy,
                    "budget_tokens": budget_tokens
                },
                "circuit_breakers": {
                    "max_depth": depth_limit,
                    "current_depth": current_depth,
                    "existing_descendants": existing_descendants,
                    "max_descendants_per_parent": max_descendants_per_parent,
                    "spawn_budget_cap": spawn_budget_cap,
                    "remaining_capacity": remaining_capacity,
                    "degraded": effective_count < requested_count_raw_pos
                },
                "children": created,
                "errors": errors
            });
            out["receipt_hash"] = json!(crate::deterministic_receipt_hash(&out));
            out
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
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, method, &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
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

