fn fetch_with_curl_retry(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    allow_rfc2544_benchmark_range: bool,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "fetch_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current = fetch_with_curl(
            url,
            timeout_ms,
            max_response_bytes,
            ua,
            allow_rfc2544_benchmark_range,
        );
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn fetch_serper_with_retry(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    top_k: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "serper_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current =
            fetch_serper_with_curl(api_key, query, timeout_ms, max_response_bytes, ua, top_k);
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn build_receipt(
    requested_url: &str,
    policy_decision: &str,
    response_hash: Option<&str>,
    status_code: i64,
    policy_reason: &str,
    error: Option<&str>,
) -> Value {
    let timestamp = crate::now_iso();
    let mut row = json!({
        "type": "web_conduit_receipt",
        "timestamp": timestamp,
        "requested_url": clean_text(requested_url, 2200),
        "domain": extract_domain(requested_url),
        "policy_decision": clean_text(policy_decision, 40),
        "policy_reason": clean_text(policy_reason, 160),
        "status_code": status_code,
        "response_hash": response_hash.unwrap_or(""),
        "error": clean_text(error.unwrap_or(""), 320)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    row
}

pub fn api_status(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let recent = read_recent_receipts(root, 12);
    let denied = recent
        .iter()
        .filter(|row| row.get("policy_decision").and_then(Value::as_str) == Some("deny"))
        .count();
    let last = recent.first().cloned().unwrap_or(Value::Null);
    let default_search_provider_chain =
        crate::web_conduit_provider_runtime::resolved_search_provider_chain(
            "",
            &json!({}),
            &policy,
        );
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_provider_catalog = provider_catalog_snapshot(root, &policy);
    let fetch_provider_catalog = fetch_provider_catalog_snapshot(root, &policy);
    let search_request_contract = search_provider_request_contract(&policy);
    let mut tool_catalog = web_tool_catalog_snapshot(&policy);
    append_web_media_tool_entry(&mut tool_catalog, &policy);
    append_web_image_tool_entry(&mut tool_catalog, root, &policy);
    let search_provider_registration_contract = search_provider_registration_contract(&policy);
    let fetch_provider_registration_contract = fetch_provider_registration_contract(&policy);
    let public_artifact_contracts = web_provider_public_artifact_contracts();
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let image_tool_runtime = runtime_web_tools_metadata
        .get("image_tool")
        .cloned()
        .unwrap_or(Value::Null);
    let native_codex_web_search = native_codex_public_contract(root, &policy);
    let media_generation_action_contracts = media_generate_action_contracts();
    json!({
        "ok": true,
        "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "policy": policy,
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "provider_catalog": search_provider_catalog.clone(),
        "search_provider_catalog": search_provider_catalog,
        "fetch_provider_catalog": fetch_provider_catalog,
        "search_request_contract": search_request_contract,
        "search_provider_registration_contract": search_provider_registration_contract,
        "fetch_provider_registration_contract": fetch_provider_registration_contract,
        "media_request_contract": web_media_request_contract(),
        "image_tool_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy),
        "image_tool_runtime": image_tool_runtime,
        "public_artifact_contracts": public_artifact_contracts,
        "runtime_web_tools_state_path": runtime_web_tools_state_path(root).display().to_string(),
        "runtime_web_tools_metadata": runtime_web_tools_metadata,
        "native_codex_web_search": native_codex_web_search,
        "media_generation_action_contracts": media_generation_action_contracts,
        "tool_catalog": tool_catalog,
        "receipts_total": receipt_count(root),
        "recent_denied": denied,
        "recent_receipts": recent,
        "last_receipt": last
    })
}

pub fn api_providers(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let default_search_provider_chain =
        crate::web_conduit_provider_runtime::resolved_search_provider_chain(
            "",
            &json!({}),
            &policy,
        );
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_providers = provider_catalog_snapshot(root, &policy);
    let fetch_providers = fetch_provider_catalog_snapshot(root, &policy);
    let mut tool_catalog = web_tool_catalog_snapshot(&policy);
    append_web_media_tool_entry(&mut tool_catalog, &policy);
    append_web_image_tool_entry(&mut tool_catalog, root, &policy);
    let search_provider_registration_contract = search_provider_registration_contract(&policy);
    let fetch_provider_registration_contract = fetch_provider_registration_contract(&policy);
    let public_artifact_contracts = web_provider_public_artifact_contracts();
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let image_tool_runtime = runtime_web_tools_metadata
        .get("image_tool")
        .cloned()
        .unwrap_or(Value::Null);
    let native_codex_web_search = native_codex_public_contract(root, &policy);
    let media_generation_action_contracts = media_generate_action_contracts();
    json!({
        "ok": true,
        "type": "web_conduit_providers",
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "search_request_contract": search_provider_request_contract(&policy),
        "search_provider_registration_contract": search_provider_registration_contract,
        "fetch_provider_registration_contract": fetch_provider_registration_contract,
        "media_request_contract": web_media_request_contract(),
        "image_tool_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy),
        "image_tool_runtime": image_tool_runtime,
        "public_artifact_contracts": public_artifact_contracts,
        "runtime_web_tools_state_path": runtime_web_tools_state_path(root).display().to_string(),
        "runtime_web_tools_metadata": runtime_web_tools_metadata,
        "native_codex_web_search": native_codex_web_search,
        "media_generation_action_contracts": media_generation_action_contracts,
        "tool_catalog": tool_catalog,
        "providers": search_providers.clone(),
        "search_providers": search_providers,
        "fetch_providers": fetch_providers
    })
}

pub fn api_receipts(root: &Path, limit: usize) -> Value {
    json!({
        "ok": true,
        "receipts": read_recent_receipts(root, limit.clamp(1, 200))
    })
}

pub fn api_fetch(root: &Path, request: &Value) -> Value {
    execute_fetch_request(root, request)
}
