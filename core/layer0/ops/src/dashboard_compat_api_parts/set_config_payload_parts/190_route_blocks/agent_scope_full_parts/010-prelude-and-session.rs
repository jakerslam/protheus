fn handle_agent_scope_full(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
    if let Some((requested_agent_id, segments)) = parse_agent_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        let lineage_message_route =
            method == "POST" && segments.len() == 1 && segments[0] == "message";
        if !requester_agent.is_empty()
            && method != "GET"
            && requester_agent != agent_id
            && !(if lineage_message_route {
                actor_can_message_target(root, snapshot, &requester_agent, &agent_id)
            } else {
                actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
            })
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": if lineage_message_route {
                        "agent_message_forbidden"
                    } else {
                        "agent_manage_forbidden"
                    },
                    "actor_agent_id": requester_agent,
                    "target_agent_id": agent_id
                }),
            });
        }
        let existing = agent_row_by_id(root, snapshot, &agent_id);
        let is_archived =
            crate::dashboard_agent_state::archived_agent_ids(root).contains(&agent_id);
        if method == "GET" && segments.is_empty() {
            if let Some(row) = existing {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: row,
                });
            }
            if is_archived {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: archived_agent_stub(root, &agent_id),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "DELETE" && segments.is_empty() {
            if existing.is_none() {
                if is_archived {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "type": "dashboard_agent_archive",
                            "id": agent_id,
                            "agent_id": agent_id,
                            "state": "inactive",
                            "archived": true
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_archive_reason = clean_text(
                request.get("reason").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let archive_reason = if requested_archive_reason.is_empty() {
                "user_archive".to_string()
            } else {
                requested_archive_reason
            };
            let requested_termination_reason = clean_text(
                request
                    .get("termination_reason")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let archive_reason_lower = archive_reason.to_ascii_lowercase();
            let termination_reason = if requested_termination_reason == "parent_archived"
                || archive_reason_lower == "archived by parent agent"
                || archive_reason_lower == "parent_archived"
                || archive_reason_lower == "parent_archive"
            {
                "parent_archived"
            } else {
                "user_archived"
            };
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"state": "Archived", "updated_at": crate::now_iso()}),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": termination_reason,
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = crate::dashboard_agent_state::archive_agent(root, &agent_id, &archive_reason);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "type": "dashboard_agent_archive",
                    "id": agent_id,
                    "agent_id": agent_id,
                    "state": "inactive",
                    "archived": true,
                    "reason": archive_reason
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "stop" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "stopped",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_stop", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "start" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "state": "Running",
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "active",
                    "termination_reason": "",
                    "terminated_at": "",
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_start", "agent_id": agent_id}),
            });
        }

        if existing.is_none() {
            if is_archived && method == "POST" && segments.len() == 1 && segments[0] == "message" {
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_inactive",
                        "agent_id": agent_id,
                        "state": "inactive",
                        "archived": true
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "session" {
            return Some(CompatApiResponse {
                status: 200,
                payload: session_payload(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "reset"
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: reset_active_session(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "compact"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: compact_active_session(root, &agent_id, &request),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "sessions" {
            let payload = session_payload(root, &agent_id);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "active_session_id": payload.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
                    "sessions": payload.get("sessions").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "sessions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let label = clean_text(
                request
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::create_session(root, &agent_id, &label),
            });
        }

        if method == "POST"
            && segments.len() == 3
            && segments[0] == "sessions"
            && segments[2] == "switch"
        {
            let session_id = decode_path_segment(&segments[1]);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::switch_session(root, &agent_id, &session_id),
            });
        }

        if method == "DELETE" && segments.len() == 1 && segments[0] == "history" {
            let mut state = load_session_state(root, &agent_id);
            if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
                for row in rows.iter_mut() {
                    row["messages"] = Value::Array(Vec::new());
                    row["updated_at"] = Value::String(crate::now_iso());
                }
            }
            save_session_state(root, &agent_id, &state);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_history_cleared", "agent_id": agent_id}),
            });
        }

        if let Some(response) = handle_agent_scope_message_route(
            root, method, &segments, body, path, snapshot, &agent_id, &existing,
        ) {
            return Some(response);
        }
        if let Some(response) = handle_agent_scope_suggestions_command_config_routes(
            root, method, &segments, body, snapshot, &agent_id, &existing,
        ) {
            return Some(response);
        }
        if let Some(response) = handle_agent_scope_eval_feedback_report_issue_routes(
            root, method, &segments, body, &agent_id,
        ) {
            return Some(response);
        }
        if let Some(response) =
            handle_agent_scope_file_read_routes(root, method, &segments, body, &agent_id, &existing)
        {
            return Some(response);
        }
        if let Some(response) = handle_agent_scope_folder_terminal_upload_routes(
            root, method, &segments, body, headers, &agent_id, &existing,
        ) {
            return Some(response);
        }
        if let Some(response) = handle_agent_scope_files_tools_clone_avatar_routes(
            root, method, path, &segments, body, &agent_id, &existing,
        ) {
            return Some(response);
        }
    }
    None
}
