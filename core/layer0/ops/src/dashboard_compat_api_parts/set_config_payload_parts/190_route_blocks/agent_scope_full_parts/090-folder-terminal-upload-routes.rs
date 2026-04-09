fn handle_agent_scope_folder_terminal_upload_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    headers: &[(&str, &str)],
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "POST" && segments.len() == 2 && segments[0] == "folder" && segments[1] == "export"
    {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let requested_path = clean_text(
            request
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| request.get("folder").and_then(Value::as_str))
                .unwrap_or(""),
            4000,
        );
        if requested_path.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "path_required"}),
            });
        }
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let target = resolve_workspace_path(&workspace_base, &requested_path);
        let Some(target_path) = target else {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
            });
        };
        if !target_path.is_dir() {
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({
                    "ok": false,
                    "error": "folder_not_found",
                    "folder": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                }),
            });
        }
        let full = request
            .get("full")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_entries = if full {
            1_000_000usize
        } else {
            request
                .get("max_entries")
                .and_then(Value::as_u64)
                .unwrap_or(20_000)
                .clamp(1, 100_000) as usize
        };
        let mut lines = Vec::<String>::new();
        let root_name = clean_text(
            target_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("folder"),
            180,
        );
        lines.push(format!("[d] {root_name}"));
        let mut entries = 0usize;
        let mut truncated = false;
        for entry in WalkDir::new(&target_path)
            .follow_links(false)
            .sort_by_file_name()
        {
            let Ok(row) = entry else {
                continue;
            };
            let path = row.path();
            if path == target_path {
                continue;
            }
            entries += 1;
            if entries > max_entries {
                truncated = true;
                continue;
            }
            let rel = path.strip_prefix(&target_path).unwrap_or(path);
            let rel_name = clean_text(rel.file_name().and_then(|v| v.to_str()).unwrap_or(""), 240);
            if rel_name.is_empty() {
                continue;
            }
            let depth = rel.components().count().saturating_sub(1).min(32);
            let indent = "  ".repeat(depth + 1);
            let marker = if row.file_type().is_dir() { "[d]" } else { "-" };
            lines.push(format!("{indent}{marker} {rel_name}"));
        }
        let tree = lines.join("\n");
        let archive_name = if root_name.is_empty() {
            "folder-tree.txt".to_string()
        } else {
            format!("{root_name}-tree.txt")
        };
        let tree_bytes = tree.as_bytes().len();
        let download_url = if tree_bytes > 0 && tree_bytes <= (2 * 1024 * 1024) {
            data_url_from_bytes(tree.as_bytes(), "text/plain; charset=utf-8")
        } else {
            String::new()
        };
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "folder": {
                    "ok": true,
                    "path": target_path.to_string_lossy().to_string(),
                    "tree": tree,
                    "entries": entries,
                    "truncated": truncated,
                    "full": full,
                    "max_entries": max_entries
                },
                "archive": {
                    "ok": true,
                    "download_url": download_url,
                    "file_name": archive_name,
                    "bytes": tree_bytes
                }
            }),
        });
    }

    if method == "POST" && segments.len() == 1 && segments[0] == "terminal" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let command = clean_text(
            request
                .get("command")
                .and_then(Value::as_str)
                .or_else(|| request.get("cmd").and_then(Value::as_str))
                .unwrap_or(""),
            16_000,
        );
        if command.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "command_required"}),
            });
        }
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let requested_cwd = clean_text(
            request.get("cwd").and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        let cwd = if requested_cwd.is_empty() {
            workspace_base.clone()
        } else {
            resolve_workspace_path(&workspace_base, &requested_cwd)
                .unwrap_or(workspace_base.clone())
        };
        let session_id = format!("agent-{}", clean_agent_id(&agent_id));
        let _ = crate::dashboard_terminal_broker::create_session(
            root,
            &json!({
                "id": session_id,
                "cwd": workspace_base.to_string_lossy().to_string()
            }),
        );
        let payload = crate::dashboard_terminal_broker::exec_command(
            root,
            &json!({
                "session_id": session_id,
                "command": command,
                "cwd": cwd.to_string_lossy().to_string()
            }),
        );
        let status = match payload.get("error").and_then(Value::as_str).unwrap_or("") {
            "session_id_and_command_required" | "session_not_found" | "cwd_outside_workspace" => {
                400
            }
            _ => 200,
        };
        return Some(CompatApiResponse { status, payload });
    }
    if method == "POST" && segments.len() == 1 && segments[0] == "upload" {
        let file_name = clean_text(
            header_value(headers, "X-Filename")
                .as_deref()
                .unwrap_or("upload.bin"),
            240,
        );
        let content_type = clean_text(
            header_value(headers, "Content-Type")
                .as_deref()
                .unwrap_or("application/octet-stream"),
            120,
        );
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let uploads_dir = workspace_base.join(".infring").join("uploads");
        let _ = fs::create_dir_all(&uploads_dir);
        let file_id = format!(
            "upload-{}",
            crate::deterministic_receipt_hash(&json!({
                "agent_id": agent_id,
                "filename": file_name,
                "bytes": body.len(),
                "ts": crate::now_iso()
            }))
            .chars()
            .take(16)
            .collect::<String>()
        );
        let ext = Path::new(&file_name)
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| clean_text(v, 16))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "bin".to_string());
        let stored_name = format!("{file_id}.{ext}");
        let stored_path = uploads_dir.join(&stored_name);
        let _ = fs::write(&stored_path, body);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "file_id": file_id,
                "filename": file_name,
                "content_type": content_type,
                "bytes": body.len(),
                "stored_path": stored_path.to_string_lossy().to_string(),
                "uploaded_at": crate::now_iso()
            }),
        });
    }
    None
}
