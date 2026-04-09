fn handle_agent_scope_files_tools_clone_avatar_routes(
    root: &Path,
    method: &str,
    path: &str,
    segments: &[String],
    body: &[u8],
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "GET" && segments.len() == 1 && segments[0] == "files" {
        let dir = agent_files_dir(root, &agent_id);
        let mut rows = Vec::<Value>::new();
        let defaults = vec!["SOUL.md".to_string(), "SYSTEM.md".to_string()];
        for name in defaults {
            let path = dir.join(&name);
            rows.push(json!({
                "name": name,
                "exists": path.exists(),
                "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
            }));
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = clean_text(path.file_name().and_then(|v| v.to_str()).unwrap_or(""), 180);
                if name.is_empty() {
                    continue;
                }
                if rows
                    .iter()
                    .any(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str()))
                {
                    continue;
                }
                rows.push(json!({
                    "name": name,
                    "exists": true,
                    "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                }));
            }
        }
        rows.sort_by(|a, b| {
            clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 180).cmp(&clean_text(
                b.get("name").and_then(Value::as_str).unwrap_or(""),
                180,
            ))
        });
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": agent_id, "files": rows}),
        });
    }

    if (method == "GET" || method == "PUT") && segments.len() >= 2 && segments[0] == "files" {
        let file_name = decode_path_segment(&segments[1..].join("/"));
        if file_name.is_empty() || file_name.contains("..") {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "invalid_file_name"}),
            });
        }
        let path = agent_files_dir(root, &agent_id).join(&file_name);
        if method == "GET" {
            if !path.exists() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "file_not_found"}),
                });
            }
            let content = fs::read_to_string(&path).unwrap_or_default();
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "name": file_name, "content": content}),
            });
        }
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let content = request
            .get("content")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, content);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": agent_id, "name": file_name}),
        });
    }

    if method == "GET" && segments.len() == 1 && segments[0] == "tools" {
        let payload = read_json_loose(&agent_tools_path(root, &agent_id))
            .unwrap_or_else(|| json!({"tool_allowlist": [], "tool_blocklist": []}));
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    if method == "PUT" && segments.len() == 1 && segments[0] == "tools" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let payload = json!({
            "tool_allowlist": request.get("tool_allowlist").cloned().unwrap_or_else(|| json!([])),
            "tool_blocklist": request.get("tool_blocklist").cloned().unwrap_or_else(|| json!([]))
        });
        write_json_pretty(&agent_tools_path(root, &agent_id), &payload);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": agent_id, "tool_filters": payload}),
        });
    }

    if method == "POST" && segments.len() == 1 && segments[0] == "clone" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let source = existing.clone().unwrap_or_else(|| json!({}));
        let requested_new_name = clean_text(
            request
                .get("new_name")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let source_role = clean_text(
            source
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("analyst"),
            60,
        );
        let resolved_requested_name = if requested_new_name.is_empty() {
            dashboard_compat_api_agent_identity::resolve_agent_name(root, "", &source_role)
        } else {
            requested_new_name.clone()
        };
        let new_id_seed = if resolved_requested_name.is_empty() {
            "agent".to_string()
        } else {
            resolved_requested_name.clone()
        };
        let new_id = make_agent_id(root, &new_id_seed);
        let new_name = if resolved_requested_name.is_empty() {
            dashboard_compat_api_agent_identity::default_agent_name(&new_id)
        } else {
            resolved_requested_name
        };
        let mut profile_patch = source.clone();
        profile_patch["name"] = Value::String(new_name.clone());
        profile_patch["agent_id"] = Value::String(new_id.clone());
        profile_patch["parent_agent_id"] = Value::String(agent_id.to_string());
        profile_patch["state"] = Value::String("Running".to_string());
        if requested_new_name.is_empty() {
            profile_patch["identity"] = dashboard_compat_api_agent_identity::resolve_agent_identity(
                root,
                &json!({}),
                &source_role,
            );
        }
        profile_patch["created_at"] = Value::String(crate::now_iso());
        profile_patch["updated_at"] = Value::String(crate::now_iso());
        let _ = update_profile_patch(root, &new_id, &profile_patch);
        let _ = upsert_contract_patch(
            root,
            &new_id,
            &json!({
                "status": "active",
                "created_at": crate::now_iso(),
                "updated_at": crate::now_iso(),
                "owner": "dashboard_clone",
                "mission": format!("Assist with assigned mission for {}.", new_id),
                "parent_agent_id": agent_id,
                "termination_condition": "task_or_timeout",
                "expiry_seconds": 3600,
                "auto_terminate_allowed": false,
                "idle_terminate_allowed": false
            }),
        );
        append_turn_message(root, &new_id, "", "");
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": new_id, "name": new_name}),
        });
    }

    if method == "POST" && segments.len() == 1 && segments[0] == "avatar" {
        let content_type = clean_text(
            query_value(path, "content_type").as_deref().unwrap_or(""),
            120,
        );
        let inferred = if content_type.is_empty() {
            "image/png".to_string()
        } else {
            content_type
        };
        let encoded = {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            STANDARD.encode(body)
        };
        let avatar_url = format!("data:{};base64,{}", inferred, encoded);
        let _ = update_profile_patch(root, &agent_id, &json!({"avatar_url": avatar_url}));
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": agent_id, "avatar_url": avatar_url}),
        });
    }
    None
}
