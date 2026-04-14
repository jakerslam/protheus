fn finalize_agent_scope_tool_payload(
    root: &Path,
    agent_id: &str,
    tool_name: &str,
    tool_input: &Value,
    payload: &mut Value,
    nexus_connection: Option<Value>,
) {
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root, agent_id, tool_name, payload,
    );
    let audit_receipt =
        append_tool_decision_audit(root, agent_id, tool_name, tool_input, payload, "none");
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "recovery_strategy".to_string(),
            Value::String("none".to_string()),
        );
        obj.insert("recovery_attempts".to_string(), json!(0));
        obj.insert(
            "decision_audit_receipt".to_string(),
            Value::String(audit_receipt),
        );
        if let Some(meta) = nexus_connection {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
}

fn handle_agent_scope_file_read_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "POST" && segments.len() == 2 && segments[0] == "file" && segments[1] == "read" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let requested_path = clean_text(
            request
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| request.get("file_path").and_then(Value::as_str))
                .unwrap_or(""),
            4000,
        );
        if requested_path.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "path_required"}),
            });
        }
        let nexus_connection =
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "file_read",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 403,
                        payload: json!({
                            "ok": false,
                            "error": "file_read_nexus_delivery_denied",
                            "message": "File read blocked by hierarchical nexus ingress policy.",
                            "nexus_error": clean_text(&err, 240)
                        }),
                    })
                }
            };
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let target = resolve_workspace_path(&workspace_base, &requested_path);
        let Some(target_path) = target else {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
            });
        };
        if !target_path.is_file() {
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({
                    "ok": false,
                    "error": "file_not_found",
                    "file": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                }),
            });
        }
        let bytes = fs::read(&target_path).unwrap_or_default();
        let full = request
            .get("full")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allow_binary = request
            .get("allow_binary")
            .or_else(|| request.get("binary"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_bytes = if full {
            bytes.len().max(1)
        } else {
            request
                .get("max_bytes")
                .and_then(Value::as_u64)
                .unwrap_or((256 * 1024) as u64)
                .clamp(1, (8 * 1024 * 1024) as u64) as usize
        };
        let binary = bytes_look_binary(&bytes);
        let content_type = guess_mime_type_for_file(&target_path, &bytes);
        if binary && !allow_binary {
            return Some(CompatApiResponse {
                status: 415,
                payload: json!({
                    "ok": false,
                    "error": "binary_file_requires_opt_in",
                    "file": {
                        "ok": false,
                        "path": target_path.to_string_lossy().to_string(),
                        "bytes": bytes.len(),
                        "binary": true,
                        "content_type": content_type,
                        "file_name": clean_text(
                            target_path.file_name().and_then(|v| v.to_str()).unwrap_or("download.bin"),
                            180
                        )
                    }
                }),
            });
        }
        let (content, truncated) = if binary {
            (String::new(), bytes.len() > max_bytes)
        } else {
            truncate_utf8_lossy(&bytes, max_bytes)
        };
        let content_base64 = if binary {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let slice_end = bytes.len().min(max_bytes.max(1));
            STANDARD.encode(&bytes[..slice_end])
        } else {
            String::new()
        };
        let download_url = if bytes.len() <= (2 * 1024 * 1024) {
            data_url_from_bytes(&bytes, &content_type)
        } else {
            String::new()
        };
        let file_name = clean_text(
            target_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("download.txt"),
            180,
        );
        let mut payload = json!({
            "ok": true,
            "file": {
                "ok": true,
                "path": target_path.to_string_lossy().to_string(),
                "content": content,
                "content_base64": content_base64,
                "truncated": truncated,
                "bytes": bytes.len(),
                "max_bytes": max_bytes,
                "full": full,
                "binary": binary,
                "allow_binary": allow_binary,
                "download_url": download_url,
                "file_name": file_name,
                "content_type": content_type
            }
        });
        let tool_input = json!({
            "path": requested_path,
            "full": full,
            "allow_binary": allow_binary
        });
        let trace_id = crate::deterministic_receipt_hash(&json!({
            "agent_id": agent_id,
            "tool": "file_read",
            "path": requested_path
        }));
        let task_id = format!(
            "tool-file-read-{}",
            trace_id.chars().take(12).collect::<String>()
        );
        let pipeline = tooling_pipeline_execute(
            &trace_id,
            &task_id,
            "file_read",
            &tool_input,
            |_| Ok(payload.clone()),
        );
        if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            attach_tool_pipeline(&mut payload, &pipeline);
        }
        finalize_agent_scope_tool_payload(
            root,
            agent_id,
            "file_read",
            &tool_input,
            &mut payload,
            nexus_connection,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if method == "POST"
        && segments.len() == 2
        && segments[0] == "file"
        && segments[1] == "read-many"
    {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let mut paths = request
            .get("paths")
            .or_else(|| request.get("sources"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|row| row.as_str().map(|v| clean_text(v, 4000)))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>();
        if paths.is_empty() {
            let single = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("file_path").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if !single.is_empty() {
                paths.push(single);
            }
        }
        if paths.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "paths_required"}),
            });
        }
        let nexus_connection =
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "file_read_many",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 403,
                        payload: json!({
                            "ok": false,
                            "error": "file_read_many_nexus_delivery_denied",
                            "message": "File read-many blocked by hierarchical nexus ingress policy.",
                            "nexus_error": clean_text(&err, 240)
                        }),
                    })
                }
            };
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let full = request
            .get("full")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allow_binary = request
            .get("allow_binary")
            .or_else(|| request.get("binary"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_bytes = request
            .get("max_bytes")
            .and_then(Value::as_u64)
            .unwrap_or((256 * 1024) as u64)
            .clamp(1, (8 * 1024 * 1024) as u64) as usize;
        let mut files = Vec::<Value>::new();
        let mut failed = Vec::<Value>::new();
        let mut unclassified = Vec::<Value>::new();
        let mut grouped_text = Vec::<String>::new();
        let mut grouped_binary = Vec::<String>::new();
        let mut grouped_unclassified = Vec::<String>::new();
        for requested_path in &paths {
            let target = resolve_workspace_path(&workspace_base, requested_path);
            let Some(target_path) = target else {
                failed.push(json!({
                    "path": requested_path,
                    "error": "path_outside_workspace",
                    "status": 400
                }));
                grouped_unclassified.push(requested_path.clone());
                continue;
            };
            if !target_path.is_file() {
                let rendered = target_path.to_string_lossy().to_string();
                unclassified.push(json!({
                    "path": rendered,
                    "error": "file_not_found",
                    "status": 404
                }));
                grouped_unclassified.push(target_path.to_string_lossy().to_string());
                continue;
            }
            let bytes = fs::read(&target_path).unwrap_or_default();
            let file_max_bytes = if full { bytes.len().max(1) } else { max_bytes };
            let binary = bytes_look_binary(&bytes);
            let content_type = guess_mime_type_for_file(&target_path, &bytes);
            if binary && !allow_binary {
                failed.push(json!({
                    "path": target_path.to_string_lossy().to_string(),
                    "error": "binary_file_requires_opt_in",
                    "status": 415,
                    "binary": true,
                    "bytes": bytes.len(),
                    "content_type": content_type
                }));
                grouped_binary.push(target_path.to_string_lossy().to_string());
                continue;
            }
            let (content, truncated) = if binary {
                (String::new(), bytes.len() > file_max_bytes)
            } else {
                truncate_utf8_lossy(&bytes, file_max_bytes)
            };
            let content_base64 = if binary {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                let slice_end = bytes.len().min(file_max_bytes.max(1));
                STANDARD.encode(&bytes[..slice_end])
            } else {
                String::new()
            };
            let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                data_url_from_bytes(&bytes, &content_type)
            } else {
                String::new()
            };
            let file_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("download.txt"),
                180,
            );
            let rendered_path = target_path.to_string_lossy().to_string();
            if binary {
                grouped_binary.push(rendered_path.clone());
            } else {
                grouped_text.push(rendered_path.clone());
            }
            files.push(json!({
                "ok": true,
                "path": rendered_path,
                "content": content,
                "content_base64": content_base64,
                "truncated": truncated,
                "bytes": bytes.len(),
                "max_bytes": file_max_bytes,
                "full": full,
                "binary": binary,
                "allow_binary": allow_binary,
                "download_url": download_url,
                "file_name": file_name,
                "content_type": content_type
            }));
        }
        let ok = !files.is_empty();
        let status = if ok {
            200
        } else {
            failed
                .first()
                .or_else(|| unclassified.first())
                .and_then(|row| row.get("status").and_then(Value::as_u64))
                .unwrap_or(400) as u16
        };
        let mut payload = json!({
            "ok": ok,
            "type": "file_read_many",
            "files": files,
            "failed": failed,
            "unclassified": unclassified,
            "partial": ok && (!failed.is_empty() || !unclassified.is_empty()),
            "groups": {
                "text": grouped_text,
                "binary": grouped_binary,
                "unclassified": grouped_unclassified
            },
            "counts": {
                "requested": paths.len(),
                "ok": files.len(),
                "failed": failed.len(),
                "unclassified": unclassified.len(),
                "text": grouped_text.len(),
                "binary": grouped_binary.len(),
                "group_unclassified": grouped_unclassified.len()
            }
        });
        let tool_input = json!({
            "paths": paths,
            "full": full,
            "allow_binary": allow_binary
        });
        let trace_id = crate::deterministic_receipt_hash(&json!({
            "agent_id": agent_id,
            "tool": "file_read_many",
            "paths": paths
        }));
        let task_id = format!(
            "tool-file-read-many-{}",
            trace_id.chars().take(12).collect::<String>()
        );
        let pipeline = tooling_pipeline_execute(
            &trace_id,
            &task_id,
            "file_read_many",
            &tool_input,
            |_| Ok(payload.clone()),
        );
        if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            attach_tool_pipeline(&mut payload, &pipeline);
        }
        finalize_agent_scope_tool_payload(
            root,
            agent_id,
            "file_read_many",
            &tool_input,
            &mut payload,
            nexus_connection,
        );
        return Some(CompatApiResponse { status, payload });
    }
    None
}
