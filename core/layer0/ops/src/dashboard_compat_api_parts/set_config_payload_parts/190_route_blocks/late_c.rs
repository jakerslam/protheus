fn finalize_global_post_tool_payload(
    root: &Path,
    tool_name: &str,
    tool_input: &Value,
    payload: &mut Value,
    nexus_connection: Option<Value>,
) {
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root,
        "dashboard-api",
        tool_name,
        payload,
    );
    let audit_receipt = append_tool_decision_audit(
        root,
        "dashboard-api",
        tool_name,
        tool_input,
        payload,
        "none",
    );
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

fn handle_global_post_delete_routes(
    root: &Path,
    method: &str,
    _path: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    if method == "POST" {
        if path_only == "/api/system/restart" {
            let payload = crate::dashboard_release_update::dispatch_system_action(root, "restart");
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    500
                },
                payload,
            });
        }
        if path_only == "/api/system/shutdown" {
            let payload = crate::dashboard_release_update::dispatch_system_action(root, "shutdown");
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    500
                },
                payload,
            });
        }
        if path_only == "/api/system/update" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = if request.get("apply").and_then(Value::as_bool).unwrap_or(true) {
                crate::dashboard_release_update::dispatch_update_apply(root)
            } else {
                crate::dashboard_release_update::check_update(root)
            };
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    if payload.get("queued").and_then(Value::as_bool).unwrap_or(false) {
                        202
                    } else {
                        200
                    }
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/update/apply" {
            let payload = crate::dashboard_release_update::dispatch_update_apply(root);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    if payload.get("queued").and_then(Value::as_bool).unwrap_or(false) {
                        202
                    } else {
                        200
                    }
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/config/set" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = set_config_payload(root, snapshot, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/receipts/lineage" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let task_id = clean_text(
                request
                    .get("task_id")
                    .or_else(|| request.get("taskId"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            if task_id.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "error": "task_id_required"
                    }),
                });
            }
            let trace_id = clean_text(
                request
                    .get("trace_id")
                    .or_else(|| request.get("traceId"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            let trace_opt = if trace_id.is_empty() {
                None
            } else {
                Some(trace_id.as_str())
            };
            let limit = request
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(4000)
                .clamp(1, 50_000);
            let scan_root = clean_text(
                request
                    .get("scan_root")
                    .or_else(|| request.get("scanRoot"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                500,
            );
            let scan_root_path = if scan_root.is_empty() {
                None
            } else {
                let candidate = PathBuf::from(scan_root);
                Some(if candidate.is_absolute() {
                    candidate
                } else {
                    root.join(candidate)
                })
            };
            let sources = clean_text(
                request
                    .get("sources")
                    .or_else(|| request.get("sources_csv"))
                    .or_else(|| request.get("sourcesCsv"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                4_000,
            );
            let sources_opt = if sources.is_empty() {
                None
            } else {
                Some(sources.as_str())
            };
            let payload = match crate::action_receipts_kernel::query_task_lineage(
                root,
                &task_id,
                trace_opt,
                limit,
                scan_root_path.as_deref(),
                sources_opt,
            ) {
                Ok(out) => out,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 400,
                        payload: json!({
                            "ok": false,
                            "error": clean_text(&err, 240)
                        }),
                    })
                }
            };
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }
        if path_only == "/api/web/fetch" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "web_fetch",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "web_fetch_nexus_delivery_denied",
                                "message": "Web fetch blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "web_fetch",
                "request": request,
                "route": "api_web_fetch_post"
            }));
            let task_id = format!(
                "tool-web-fetch-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "web_fetch",
                &request,
                |normalized_args| Ok(crate::web_conduit::api_fetch(root, normalized_args)),
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            finalize_global_post_tool_payload(
                root,
                "web_fetch",
                &request,
                &mut payload,
                nexus_connection,
            );
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/web/media-host" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = crate::web_conduit::api_media_host(root, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/web/search" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "web_search",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "web_search_nexus_delivery_denied",
                                "message": "Web search blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "web_search",
                "request": request,
                "route": "api_web_search_post"
            }));
            let task_id = format!(
                "tool-web-search-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "web_search",
                &request,
                |normalized_args| Ok(crate::web_conduit::api_search(root, normalized_args)),
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            finalize_global_post_tool_payload(
                root,
                "web_search",
                &request,
                &mut payload,
                nexus_connection,
            );
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/batch-query" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "batch_query",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "batch_query_nexus_delivery_denied",
                                "message": "Batch query blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "batch_query",
                "request": request,
                "route": "api_batch_query_post"
            }));
            let task_id = format!(
                "tool-batch-query-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "batch_query",
                &request,
                |normalized_args| {
                    Ok(crate::batch_query_primitive::api_batch_query(
                        root,
                        normalized_args,
                    ))
                },
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"status":"blocked","error":"tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            finalize_global_post_tool_payload(
                root,
                "batch_query",
                &request,
                &mut payload,
                nexus_connection,
            );
            return Some(CompatApiResponse {
                status: if payload.get("status").and_then(Value::as_str) == Some("blocked") {
                    400
                } else {
                    200
                },
                payload,
            });
        }
        if path_only == "/api/route/auto" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}
