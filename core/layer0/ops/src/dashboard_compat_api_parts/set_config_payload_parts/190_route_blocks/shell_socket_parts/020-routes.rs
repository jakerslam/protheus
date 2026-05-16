fn handle_shell_socket_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
    request_host: &str,
) -> Option<CompatApiResponse> {
    if !path_only.starts_with(SHELL_SOCKET_PREFIX) {
        return None;
    }
    let parts = shell_socket_path_parts(path_only);
    if method == "GET" && parts == ["runtime-status"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_runtime_status(root, snapshot, request_host) });
    }
    if method == "GET" && parts == ["agents"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_agent_roster(root, path, snapshot) });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "sessions" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_session_list(root, &parts[1], path) });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "sessions" && parts[2] == "messages" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_message_window(root, &parts[1], path) });
    }
    if method == "GET" && parts.len() >= 2 && parts[0] == "details" {
        let detail_ref = parts[1..].join("/");
        return Some(match shell_socket_detail_projection(root, &detail_ref, path) {
            Some(payload) => CompatApiResponse { status: 200, payload },
            None => CompatApiResponse {
                status: 404,
                payload: json!({
                    "detail_id": clean_text(&detail_ref, 180),
                    "detail_kind": "unknown",
                    "requested_view": query_value(path, "view").unwrap_or_else(|| "summary".to_string()),
                    "detail_projection": json!({}),
                    "size_bound": json!({"max_response_bytes": 65536}),
                    "next_cursor": Value::Null,
                    "receipt_ref": shell_socket_receipt_ref("get_message_detail", &json!({"detail_ref": detail_ref})),
                    "correlation_id": "shell_socket.message_detail"
                }),
            },
        });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "sessions" && parts[2] == "events" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_event_projection(root, &parts[1]) });
    }
    if method == "GET" && parts == ["search"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_search(root, path) });
    }
    if method == "GET" && parts == ["models"] {
        let mut payload = crate::dashboard_model_catalog::catalog_payload(root, snapshot);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("next_cursor".to_string(), Value::Null);
            obj.insert("detail_refs".to_string(), json!({"models": "model_catalog:default"}));
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("list_models", &json!({"path": path}))),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.model_catalog"));
        }
        return Some(CompatApiResponse { status: 200, payload });
    }
    if method == "POST" && parts == ["models", "discover"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let input = clean_text(
            request
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| request.get("api_key").and_then(Value::as_str))
                .unwrap_or(""),
            4096,
        );
        let mut payload = crate::dashboard_provider_runtime::discover_models(root, &input);
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if let Some(obj) = payload.as_object_mut() {
            let input_kind = obj.get("input_kind").cloned().unwrap_or(Value::Null);
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("discover_models", &json!({"input_kind": input_kind}))),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.model_discovery"));
        }
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload,
        });
    }
    if method == "POST" && parts == ["input"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").or_else(|| request.get("target_agent_id")).and_then(Value::as_str).unwrap_or(""));
        let message = clean_chat_text(
            request.get("message").or_else(|| request.get("text")).or_else(|| request.get("input")).and_then(Value::as_str).unwrap_or(""),
            24_000,
        );
        if agent_id.is_empty() || message.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_input", false, "agent_id_and_message_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/message");
        let legacy_body = serde_json::to_vec(&json!({"message": message})).unwrap_or_default();
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, &legacy_body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_input", legacy));
    }
    if method == "POST" && parts == ["issues"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_issue", false, "agent_id_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/eval-feedback/report-issue");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_issue", legacy));
    }
    if method == "POST" && parts == ["session-index", "rebuild"] {
        let payload = rebuild_indexed_session_states(
            root,
            &serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({})),
        );
        let accepted = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "rebuild_session_indexes",
                accepted,
                if accepted {
                    "accepted"
                } else {
                    "session_index_rebuild_failed"
                },
                &payload,
            ),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "approvals" && parts[2] == "decision" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let decision = clean_text(
            request
                .get("decision")
                .or_else(|| request.get("action"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        );
        let reason = clean_text(
            request
                .get("reason")
                .or_else(|| request.get("deny_reason"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            400,
        );
        let authority = crate::approval_gate_kernel::apply_approval_decision(
            root,
            &parts[1],
            &decision,
            if reason.is_empty() { None } else { Some(&reason) },
        );
        let accepted = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let reason_code = if accepted {
            "accepted"
        } else {
            authority
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("approval_gateway_rejected")
        };
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "submit_approval_decision",
                accepted,
                reason_code,
                &json!({"approval_id": parts[1], "request": request, "authority": authority}),
            ),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "model" {
        let legacy_path = format!("/api/agents/{}/model", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "PUT", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("set_model", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "git-tree" {
        let legacy_path = format!("/api/agents/{}/git-tree/switch", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("set_git_tree", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "fresh-session" {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/reset");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("fresh_session", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "compact-session" {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/compact");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("compact_session", legacy));
    }
    if method == "POST"
        && parts.len() == 4
        && parts[0] == "agents"
        && parts[2] == "session-index"
        && parts[3] == "rebuild"
    {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/index/rebuild");
        let legacy = handle_agent_scope_routes(
            root,
            "POST",
            &legacy_path,
            &legacy_path,
            body,
            headers,
            snapshot,
            requester_agent,
        )?;
        return Some(shell_socket_ack_from_legacy("rebuild_session_index", legacy));
    }
    if method == "POST" && parts == ["terminal", "commands"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_terminal_command", false, "agent_id_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/terminal");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_terminal_command", legacy));
    }
    Some(CompatApiResponse { status: 404, payload: json!({"error": "shell_socket_route_not_found"}) })
}
