fn fetch_early_validation_payload(
    error: &str,
    requested_url: &str,
    provider_hint: &str,
    cache_status: &str,
    cache_skip_reason: &str,
    validation_route: &str,
    summary: Option<&str>,
    override_hint: Option<&str>,
    requested_provider: Option<&str>,
    provider_catalog: Option<Value>,
    receipt: Value,
) -> Value {
    let early_gate = json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": validation_route,
        "source": "early_validation"
    });
    let early_replay_guard = json!({
        "blocked": false,
        "reason": "not_evaluated"
    });
    let mut out_obj = serde_json::Map::<String, Value>::new();
    out_obj.insert("ok".to_string(), json!(false));
    out_obj.insert("error".to_string(), json!(error));
    out_obj.insert("type".to_string(), json!("web_conduit_fetch"));
    out_obj.insert(
        "requested_url".to_string(),
        Value::String(clean_text(requested_url, 2_200)),
    );
    out_obj.insert(
        "requested_url_cache_key".to_string(),
        Value::String(fetch_normalize_cache_key(requested_url)),
    );
    out_obj.insert("resolved_url".to_string(), Value::String(String::new()));
    out_obj.insert("citation_redirect_resolved".to_string(), json!(false));
    out_obj.insert("provider".to_string(), json!("none"));
    out_obj.insert(
        "provider_hint".to_string(),
        Value::String(clean_text(provider_hint, 40).to_ascii_lowercase()),
    );
    out_obj.insert("provider_chain".to_string(), json!([]));
    out_obj.insert(
        "provider_resolution".to_string(),
        json!({
            "status": "not_evaluated",
            "reason": validation_route,
            "source": "early_validation",
            "tool_surface_health": {
                "status": "not_evaluated",
                "selected_provider_ready": false,
                "blocking_reason": "early_validation"
            }
        }),
    );
    out_obj.insert("tool_surface_status".to_string(), json!("not_evaluated"));
    out_obj.insert("tool_surface_ready".to_string(), json!(false));
    out_obj.insert(
        "tool_surface_blocking_reason".to_string(),
        json!("early_validation"),
    );
    out_obj.insert("tool_execution_attempted".to_string(), json!(false));
    out_obj.insert(
        "tool_execution_gate".to_string(),
        json!({
            "should_execute": false,
            "reason": validation_route,
            "source": "early_validation"
        }),
    );
    out_obj.insert(
        "meta_query_blocked".to_string(),
        json!(validation_route == "meta_query_blocked"),
    );
    out_obj.insert("cache_status".to_string(), json!(cache_status));
    out_obj.insert("cache_store_allowed".to_string(), json!(false));
    out_obj.insert("cache_write_attempted".to_string(), json!(false));
    out_obj.insert("cache_skip_reason".to_string(), json!(cache_skip_reason));
    out_obj.insert(
        "retry".to_string(),
        fetch_retry_envelope_for_validation(error, validation_route),
    );
    out_obj.insert(
        "process_summary".to_string(),
        runtime_web_process_summary(
            "web_fetch",
            validation_route,
            false,
            &early_gate,
            &early_replay_guard,
            &json!([]),
            "none",
            Some(error),
        ),
    );
    if let Some(text) = summary {
        out_obj.insert("summary".to_string(), json!(clean_text(text, 900)));
    }
    if let Some(text) = override_hint {
        out_obj.insert("override_hint".to_string(), json!(clean_text(text, 120)));
    }
    if let Some(text) = requested_provider {
        let cleaned = clean_text(text, 120);
        if !cleaned.is_empty() {
            out_obj.insert("requested_provider".to_string(), json!(cleaned));
        }
    }
    if let Some(catalog) = provider_catalog {
        out_obj.insert("fetch_provider_catalog".to_string(), catalog);
    }
    out_obj.insert("receipt".to_string(), receipt);
    Value::Object(out_obj)
}
