    if let Some(receipt_obj) = receipt.as_object_mut() {
        receipt_obj.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        receipt_obj.insert(
            "provider".to_string(),
            Value::String(selected_provider.clone()),
        );
    }
    let _ = append_jsonl(&receipts_path(root), &receipt);
    let epistemic_object = json!({
        "kind": "web_document",
        "trusted": false,
        "provenance": {
            "source": "web_conduit",
            "requested_url": raw_requested_url,
            "resolved_url": resolved_url,
            "final_url": final_url,
            "response_hash": response_hash,
            "artifact_id": artifact
                .as_ref()
                .and_then(|row| row.get("artifact_id"))
                .cloned()
                .unwrap_or(Value::Null),
            "artifact_path": artifact
                .as_ref()
                .and_then(|row| row.get("path"))
                .cloned()
                .unwrap_or(Value::Null),
            "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null)
        },
        "verity": {
            "validated": false,
            "checks": [
                "policy_gate_passed",
                "content_hash_recorded",
                "source_marked_untrusted_until_verified"
            ]
        }
    });
    let fetch_error = if fetch_ok {
        Value::Null
    } else if error_value.is_empty() {
        if tool_surface_status == "unavailable" {
            json!("web_fetch_tool_surface_unavailable")
        } else if tool_surface_status == "degraded" && !tool_surface_ready {
            json!("web_fetch_tool_surface_degraded")
        } else {
            json!("web_conduit_fetch_failed")
        }
    } else {
        json!(error_value)
    };
    let fetch_process_summary = runtime_web_process_summary(
        "web_fetch",
        "provider_fetch_result",
        true,
        &tool_execution_gate,
        &attempt_replay_guard,
        &json!(fetch_provider_chain.clone()),
        &selected_provider,
        fetch_error.as_str(),
    );
    let fetched_ssrf_guard = fetched.get("ssrf_guard").cloned().unwrap_or(ssrf_guard);
    let content_type_value = if content_type.is_empty() {
        Value::String(String::new())
    } else {
        Value::String(content_type)
    };
    let content_value = if summary_only {
        Value::String(String::new())
    } else {
        Value::String(content.clone())
    };
    let external_content = json!({
        "untrusted": true,
        "source": "web_fetch",
        "wrapped": content_is_textual,
        "provider": selected_provider.clone(),
        "provider_chain": fetch_provider_chain.clone(),
        "tool_surface_status": tool_surface_status.clone()
    });
    let mut out_obj = serde_json::Map::<String, Value>::new();
    out_obj.insert("ok".to_string(), Value::Bool(fetch_ok));
    out_obj.insert(
        "type".to_string(),
        Value::String("web_conduit_fetch".to_string()),
    );
    out_obj.insert(
        "requested_url".to_string(),
        Value::String(raw_requested_url.clone()),
    );
    out_obj.insert(
        "requested_url_input".to_string(),
        Value::String(requested_url_input.clone()),
    );
    out_obj.insert(
        "requested_url_source".to_string(),
        Value::String(requested_url_source.to_string()),
    );
    out_obj.insert(
        "requested_url_source_kind".to_string(),
        Value::String(requested_url_source_kind.to_string()),
    );
    out_obj.insert(
        "requested_url_source_confidence".to_string(),
        Value::String(requested_url_source_confidence.to_string()),
    );
    out_obj.insert(
        "requested_url_source_recovery_mode".to_string(),
        Value::String(requested_url_source_recovery_mode.to_string()),
    );
    out_obj.insert(
        "requested_url_source_lineage".to_string(),
        requested_url_source_lineage,
    );
    out_obj.insert(
        "resolved_url".to_string(),
        Value::String(resolved_url.clone()),
    );
    out_obj.insert("final_url".to_string(), Value::String(final_url.clone()));
    out_obj.insert(
        "citation_redirect_resolved".to_string(),
        Value::Bool(redirect_resolved),
    );
    out_obj.insert(
        "provider".to_string(),
        Value::String(selected_provider.clone()),
    );
    out_obj.insert(
        "provider_hint".to_string(),
        Value::String(provider_hint.clone()),
    );
    out_obj.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
    out_obj.insert("provider_resolution".to_string(), provider_resolution.clone());
    out_obj.insert(
        "tool_surface_status".to_string(),
        Value::String(tool_surface_status.clone()),
    );
    out_obj.insert(
        "tool_surface_ready".to_string(),
        Value::Bool(tool_surface_ready),
    );
    out_obj.insert(
        "tool_surface_blocking_reason".to_string(),
        Value::String(tool_surface_blocking_reason.clone()),
    );
    out_obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
    out_obj.insert("replay_policy".to_string(), replay_policy.clone());
    out_obj.insert("replay_bypass".to_string(), replay_bypass.clone());
    out_obj.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
    out_obj.insert(
        "attempt_signature".to_string(),
        Value::String(fetch_attempt_signature.clone()),
    );
    out_obj.insert("extractor".to_string(), Value::String(extractor.clone()));
    out_obj.insert(
        "fetch_url_shape_stats".to_string(),
        fetch_url_shape_stats(&raw_requested_url),
    );
    out_obj.insert(
        "fetch_url_shape_error".to_string(),
        json!(fetch_url_shape_error),
    );
    out_obj.insert(
        "fetch_url_shape_category".to_string(),
        json!(fetch_url_shape_category(fetch_url_shape_error)),
    );
    out_obj.insert(
        "fetch_url_shape_recommended_action".to_string(),
        json!(fetch_url_shape_recommended_action(fetch_url_shape_error)),
    );
    out_obj.insert(
        "fetch_url_shape_route_hint".to_string(),
        json!(fetch_url_shape_route_hint(fetch_url_shape_error)),
    );
    out_obj.insert(
        "fetch_url_shape_override_used".to_string(),
        json!(fetch_url_override_used),
    );
    out_obj.insert(
        "fetch_url_shape_override_source".to_string(),
        json!(fetch_url_override_source),
    );
    out_obj.insert(
        "fetch_url_shape".to_string(),
        fetch_url_shape_contract(
            &requested_url_input,
            &raw_requested_url,
            fetch_url_shape_error,
            fetch_url_override_used,
            fetch_url_override_source,
        ),
    );
    out_obj.insert("status_code".to_string(), json!(status_code));
    out_obj.insert("content_type".to_string(), content_type_value);
    out_obj.insert("extract_mode".to_string(), Value::String(extract_mode.clone()));
    out_obj.insert(
        "title".to_string(),
        Value::String(title.unwrap_or_default()),
    );
    out_obj.insert("summary".to_string(), Value::String(summary.clone()));
    let mut out = Value::Object(out_obj);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("content".to_string(), content_value);
        obj.insert(
            "content_truncated".to_string(),
            Value::Bool(content_truncated || wrapped_truncated),
        );
        obj.insert("raw_length".to_string(), json!(raw_length));
        obj.insert("wrapped_length".to_string(), json!(wrapped_length));
        obj.insert("external_content".to_string(), external_content);
        obj.insert("process_summary".to_string(), fetch_process_summary);
        obj.insert("cache_status".to_string(), json!("miss"));
        obj.insert(
            "retry_attempts".to_string(),
            fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        );
        obj.insert(
            "retry_used".to_string(),
            fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        );
        obj.insert(
            "redirect_count".to_string(),
            fetched.get("redirect_count").cloned().unwrap_or_else(|| json!(0)),
        );
        obj.insert(
            "user_agent".to_string(),
            fetched
                .get("user_agent")
                .cloned()
                .unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        );
        obj.insert(
            "accept_header".to_string(),
            fetched
                .get("accept_header")
                .cloned()
                .unwrap_or_else(|| json!(FETCH_MARKDOWN_ACCEPT_HEADER)),
        );
        obj.insert(
            "x_markdown_tokens".to_string(),
            fetched.get("x_markdown_tokens").cloned().unwrap_or(Value::Null),
        );
        obj.insert("response_hash".to_string(), json!(response_hash));
        obj.insert("artifact".to_string(), artifact.unwrap_or(Value::Null));
        obj.insert("policy_decision".to_string(), policy_eval);
        obj.insert("ssrf_guard".to_string(), fetched_ssrf_guard);
        obj.insert("receipt".to_string(), receipt);
        obj.insert("epistemic_object".to_string(), epistemic_object);
        obj.insert("error".to_string(), fetch_error);
    }
    if !fetch_ok {
        if let Some(obj) = out.as_object_mut() {
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            if current_error == "web_fetch_tool_surface_unavailable" {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Web fetch tool surface is currently unavailable. Retry after provider runtime is restored."
                            .to_string(),
                    ),
                );
            } else if current_error == "web_fetch_tool_surface_degraded" {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Web fetch tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                            .to_string(),
                    ),
                );
            }
        }
    }
    let cache_status = if fetch_ok { "ok" } else { "error" };
    if cache_ttl_minutes > 0 {
        if let Some(obj) = out.as_object_mut() {
            obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
        }
        store_fetch_cache(
            root,
            &fetch_cache_key,
            &out,
            cache_status,
            cache_ttl_minutes,
        );
    }
    out
