fn api_search_serper(
    root: &Path,
    query: &str,
    summary_only: bool,
    human_approved: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    requested_timeout_ms: u64,
) -> Value {
    let requested_url = SERPER_SEARCH_URL.to_string();
    let (policy, _policy_path_value) = load_policy(root);
    let credential_source = resolve_provider_credential_source_with_env(
        &policy,
        "serperdev",
        WebProviderFamily::Search,
        |key| std::env::var(key).ok(),
    );
    let Some(api_key) = resolve_search_provider_credential(&policy, "serperdev") else {
        return json!({
            "ok": false,
            "error": "serper_api_key_missing",
            "requested_url": requested_url,
            "provider": "serperdev",
            "credential_source": credential_source,
            "docs": "https://docs.openclaw.ai/tools/web"
        });
    };
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some("policy_denied"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "provider": "serperdev",
            "receipt": receipt
        });
    }
    let timeout_ms = requested_timeout_ms.clamp(
        1_000,
        policy_eval
            .pointer("/policy/timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(search_default_timeout_ms(&policy)),
    );
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let fetched = fetch_serper_with_retry(
        &api_key,
        query,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        top_k,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_serper_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = true;
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    let receipt = build_receipt(
        &requested_url,
        "allow",
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/json".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "provider": "serperdev",
        "credential_source": credential_source,
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("serper_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}
