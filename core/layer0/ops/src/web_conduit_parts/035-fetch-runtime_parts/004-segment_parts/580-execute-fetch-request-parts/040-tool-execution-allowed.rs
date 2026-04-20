    if !tool_execution_allowed {
        let preflight_error = if tool_surface_status == "unavailable" {
            "web_fetch_tool_surface_unavailable"
        } else if tool_surface_status == "degraded" {
            "web_fetch_tool_surface_degraded"
        } else {
            "web_fetch_tool_execution_blocked"
        };
        let cache_skip_reason = if preflight_error == "web_fetch_tool_surface_unavailable" {
            "tool_surface_unavailable"
        } else if preflight_error == "web_fetch_tool_surface_degraded" {
            "tool_surface_degraded"
        } else {
            "tool_execution_blocked"
        };
        let mut receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "fetch_preflight_gate_blocked",
            Some(preflight_error),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(fetch_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "provider".to_string(),
                Value::String(selected_provider.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert("error".to_string(), Value::String(preflight_error.to_string()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_fetch".to_string()),
        );
        out.insert(
            "requested_url".to_string(),
            Value::String(raw_requested_url.clone()),
        );
        out.insert("resolved_url".to_string(), Value::String(resolved_url.clone()));
        out.insert(
            "citation_redirect_resolved".to_string(),
            Value::Bool(redirect_resolved),
        );
        out.insert(
            "provider".to_string(),
            Value::String(selected_provider.clone()),
        );
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert(
            "requested_url_input".to_string(),
            Value::String(requested_url_input.clone()),
        );
        out.insert(
            "requested_url_source".to_string(),
            Value::String(requested_url_source.to_string()),
        );
        out.insert(
            "requested_url_source_kind".to_string(),
            Value::String(requested_url_source_kind.to_string()),
        );
        out.insert(
            "requested_url_source_confidence".to_string(),
            Value::String(requested_url_source_confidence.to_string()),
        );
        out.insert(
            "requested_url_source_recovery_mode".to_string(),
            Value::String(requested_url_source_recovery_mode.to_string()),
        );
        out.insert(
            "requested_url_source_lineage".to_string(),
            requested_url_source_lineage.clone(),
        );
        out.insert(
            "fetch_url_shape_stats".to_string(),
            fetch_url_shape_stats(&raw_requested_url),
        );
        out.insert(
            "fetch_url_shape_error".to_string(),
            json!(fetch_url_shape_error),
        );
        out.insert(
            "fetch_url_shape_category".to_string(),
            json!(fetch_url_shape_category(fetch_url_shape_error)),
        );
        out.insert(
            "fetch_url_shape_recommended_action".to_string(),
            json!(fetch_url_shape_recommended_action(fetch_url_shape_error)),
        );
        out.insert(
            "fetch_url_shape_route_hint".to_string(),
            json!(fetch_url_shape_route_hint(fetch_url_shape_error)),
        );
        out.insert(
            "fetch_url_shape_override_used".to_string(),
            json!(fetch_url_override_used),
        );
        out.insert(
            "fetch_url_shape_override_source".to_string(),
            json!(fetch_url_override_source),
        );
        out.insert(
            "fetch_url_shape".to_string(),
            fetch_url_shape_contract(
                &requested_url_input,
                &raw_requested_url,
                fetch_url_shape_error,
                fetch_url_override_used,
                fetch_url_override_source,
            ),
        );
        out.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &fetch_provider_chain),
        );
        out.insert(
            "fetch_provider_catalog".to_string(),
            fetch_provider_catalog_snapshot(root, &policy),
        );
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert(
            "tool_surface_ready".to_string(),
            Value::Bool(tool_surface_ready),
        );
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_attempted".to_string(), Value::Bool(false));
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("meta_query_blocked".to_string(), Value::Bool(false));
        out.insert(
            "cache_status".to_string(),
            Value::String("skipped_validation".to_string()),
        );
        out.insert("cache_store_allowed".to_string(), Value::Bool(false));
        out.insert("cache_write_attempted".to_string(), Value::Bool(false));
        out.insert(
            "cache_skip_reason".to_string(),
            Value::String(cache_skip_reason.to_string()),
        );
        out.insert(
            "retry".to_string(),
            fetch_retry_envelope_runtime(
                if preflight_error == "web_fetch_tool_surface_unavailable" {
                    "restore_tool_surface_or_use_supported_provider"
                } else if preflight_error == "web_fetch_tool_surface_degraded" {
                    "stabilize_provider_runtime_and_retry"
                } else {
                    "resolve_tool_execution_gate"
                },
                preflight_error,
                "web_fetch",
                0,
            ),
        );
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_fetch",
                "preflight_blocked",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(fetch_provider_chain.clone()),
                &selected_provider,
                Some(preflight_error),
            ),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Web fetch execution was blocked by runtime tooling gate before provider calls were attempted."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
