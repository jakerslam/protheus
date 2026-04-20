    if attempt_replay_blocked {
        let mut receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "fetch_replay_guard_blocked",
            Some("web_fetch_duplicate_attempt_suppressed"),
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
        out.insert(
            "error".to_string(),
            Value::String("web_fetch_duplicate_attempt_suppressed".to_string()),
        );
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
            Value::String("replay_suppressed".to_string()),
        );
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            fetch_retry_envelope_runtime(
                "change_query_or_provider",
                "web_fetch_duplicate_attempt_suppressed",
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_fetch",
                "replay_suppressed",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(fetch_provider_chain.clone()),
                &selected_provider,
                Some("web_fetch_duplicate_attempt_suppressed"),
            ),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Repeated identical web fetch attempts were suppressed by replay guard. Adjust URL or request parameters before retrying."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
    let fetched = fetch_with_curl_retry(
        &resolved_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        allow_rfc2544_benchmark_range,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = normalize_fetch_content_type(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let fetched_body = fetched.get("body").and_then(Value::as_str).unwrap_or("");
    let content_is_textual = content_type_is_textual(&content_type);
    let (raw_content, raw_title, content_truncated, extractor) = if content_is_textual {
        extract_fetch_content_with_extractor(fetched_body, &content_type, &extract_mode, max_chars)
    } else {
        (String::new(), None, false, "binary".to_string())
    };
    let (content, wrapped_truncated, raw_length, wrapped_length) = if content_is_textual {
        wrap_web_fetch_content(&raw_content, max_chars)
    } else {
        (String::new(), false, 0, 0)
    };
    let title = wrap_web_fetch_field(raw_title.as_deref());
    let final_url = clean_text(
        fetched
            .get("effective_url")
            .and_then(Value::as_str)
            .unwrap_or(resolved_url.as_str()),
        2200,
    );
    let summary_body = extract_fetch_summary_body(&raw_content, &extract_mode);
    let summary = if content_is_textual {
        summarize_text(&summary_body, 900)
    } else if resolved_url.is_empty() {
        format!(
            "Fetched non-text content ({}).",
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    } else {
        format!(
            "Fetched non-text content from {} ({}).",
            clean_text(&final_url, 220),
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    };
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = request
        .get("materialize_artifact")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let artifact = if materialize_artifact {
        persist_artifact(root, &resolved_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && if content_is_textual {
            !content.is_empty()
        } else {
            status_code >= 200 && status_code < 400
        };
    let error_value = fetched
        .get("stderr")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 320))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            if status_code >= 400 {
                let detail = format_web_fetch_error_detail(fetched_body, &content_type, 4000);
                if detail.is_empty() {
                    String::new()
                } else {
                    wrap_web_fetch_content(&detail, 4000).0
                }
            } else {
                String::new()
            }
        });
    let receipt_reason = if matches!(
        error_value.as_str(),
        "invalid_fetch_url"
            | "blocked_hostname"
            | "blocked_private_network_target"
            | "blocked_private_network_redirect"
            | "invalid_redirect_target"
    ) {
        error_value.as_str()
    } else {
        reason.as_str()
    };
    let receipt = build_receipt(
        &raw_requested_url,
        &decision,
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        receipt_reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let mut receipt = receipt;
