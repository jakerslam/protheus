fn browser_materialization_config_from_policy(policy: &Value) -> Value {
    policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn browser_materialization_request_field_list(
    config: &Value,
    pointer: &str,
    fallback: &[&str],
) -> Vec<String> {
    config
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|row| row.to_string()).collect())
}

fn browser_materialization_first_denied_request_field(
    request: &Value,
    denied_fields: &[String],
) -> Option<String> {
    let obj = request.as_object()?;
    denied_fields
        .iter()
        .find(|field| obj.contains_key(field.as_str()))
        .cloned()
}

fn browser_materialization_safety_projection(url: &str, ssrf_guard: &Value) -> Value {
    json!({
        "version": "browser_materialization_url_safety_v1",
        "url": clean_text(url, 2200),
        "ok": ssrf_guard.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "status": ssrf_guard
            .get("url_safety_status")
            .and_then(Value::as_str)
            .unwrap_or("invalid_url"),
        "host": ssrf_guard.get("host").cloned().unwrap_or(Value::Null),
        "error": ssrf_guard.get("error").cloned().unwrap_or(Value::Null),
        "resolved_ip_addrs": ssrf_guard
            .get("resolved_ip_addrs")
            .cloned()
            .unwrap_or_else(|| json!([]))
    })
}

fn browser_materialization_output_contract_projection(config: &Value) -> Value {
    let output_contract = config
        .get("output_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialized_page_contract_v1",
        "schema_ref": "web_research.browser_materialized_page.v1",
        "fields": output_contract
            .get("fields")
            .cloned()
            .unwrap_or_else(|| json!([
                "source_url",
                "final_url",
                "status_code",
                "title",
                "main_text_or_markdown",
                "links_summary",
                "blocker_classification",
                "extraction_confidence",
                "artifact_ref"
            ])),
        "chat_visible": output_contract
            .get("chat_visible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "raw_payload_chat_visible": false
    })
}

fn browser_materialization_evidence_handoff_projection(config: &Value) -> Value {
    let handoff = config
        .get("evidence_handoff")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialized_page_evidence_handoff_v1",
        "target_lane": handoff
            .get("target_lane")
            .and_then(Value::as_str)
            .unwrap_or("candidate_enrichment"),
        "promotion_requires": handoff
            .get("promotion_requires")
            .cloned()
            .unwrap_or_else(|| json!([
                "safe_final_url",
                "substantive_main_text",
                "query_relevance",
                "not_blocker_shell"
            ])),
        "confidence_values": handoff
            .get("confidence_values")
            .cloned()
            .unwrap_or_else(|| json!(["usable", "low_confidence_raw", "rejected"])),
        "evidence_candidate_state": "not_created",
        "raw_payload_chat_visible": handoff
            .get("raw_payload_chat_visible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "browser_success_is_not_source_truth_without_packaging": true
    })
}

fn browser_materialization_artifact_quarantine_projection() -> Value {
    json!({
        "version": "browser_materialization_artifact_quarantine_v1",
        "state": "not_created",
        "artifact_ref": Value::Null,
        "raw_payload_chat_visible": false,
        "browser_trace_chat_visible": false
    })
}

fn browser_materialization_fail_closed(
    error: &str,
    reason: &str,
    url: &str,
    config: &Value,
    runtime_metadata: &Value,
    url_safety: Value,
) -> Value {
    json!({
        "ok": false,
        "type": "web_conduit_browser_materialization",
        "capability": "browser_materialize_page",
        "error": error,
        "reason": clean_text(reason, 240),
        "url": clean_text(url, 2200),
        "tool_execution_attempted": false,
        "browser_launch_attempted": false,
        "raw_payload_chat_visible": false,
        "chat_visible": false,
        "materialized_page": Value::Null,
        "evidence_candidate": Value::Null,
        "artifact_ref": Value::Null,
        "materialized_page_contract": browser_materialization_output_contract_projection(config),
        "evidence_handoff_contract": browser_materialization_evidence_handoff_projection(config),
        "artifact_quarantine": browser_materialization_artifact_quarantine_projection(),
        "url_safety": url_safety,
        "profile_compilation": runtime_metadata
            .pointer("/profile_compilation")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "readiness_lifecycle": runtime_metadata
            .pointer("/capability_contract/readiness_lifecycle")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "execution_gate": runtime_metadata
            .get("execution_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "capability_contract_ref": "core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json#browser_materialization_capability_contract",
        "decision_authority": "web_conduit_policy_and_tool_cd"
    })
}

pub fn api_browser_materialize_page(root: &Path, request: &Value) -> Value {
    let (policy, _) = load_policy(root);
    let config = browser_materialization_config_from_policy(&policy);
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let runtime_metadata = runtime_web_tools_metadata
        .get("browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let url = clean_text(request.get("url").and_then(Value::as_str).unwrap_or(""), 2200);
    let admission_ref = clean_text(
        request
            .get("admission_ref")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let blank_safety = json!({
        "version": "browser_materialization_url_safety_v1",
        "url": url,
        "ok": false,
        "status": "not_evaluated",
        "host": Value::Null,
        "error": Value::Null,
        "resolved_ip_addrs": []
    });
    if url.is_empty() {
        return browser_materialization_fail_closed(
            "missing_required_field",
            "Browser materialization requires a URL.",
            "",
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }
    if admission_ref.is_empty() {
        return browser_materialization_fail_closed(
            "missing_required_field",
            "Browser materialization requires an admission_ref capability handle.",
            &url,
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }

    let denied_fields = browser_materialization_request_field_list(
        &config,
        "/request_contract/denied_fields",
        &[
            "browser_args",
            "launch_args",
            "cdp_command",
            "user_script",
            "proxy",
            "proxy_url",
            "proxy_credentials",
            "session_id",
            "storage_state",
            "local_file",
        ],
    );
    if let Some(field) = browser_materialization_first_denied_request_field(request, &denied_fields)
    {
        return browser_materialization_fail_closed(
            "unsafe_caller_control_rejected",
            &format!("Caller-supplied browser control field rejected: {field}."),
            &url,
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }

    let ssrf_guard = evaluate_fetch_ssrf_guard(&url, false, None);
    let url_safety = browser_materialization_safety_projection(&url, &ssrf_guard);
    if !ssrf_guard
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return browser_materialization_fail_closed(
            "url_safety_blocked",
            "Browser materialization rejected the URL before adapter execution.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    let enabled = config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !enabled {
        return browser_materialization_fail_closed(
            "capability_not_enabled",
            "Browser materialization is declared but not enabled by policy.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    let adapter_ready = config
        .get("adapter_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !adapter_ready {
        return browser_materialization_fail_closed(
            "adapter_not_ready",
            "Browser materialization is enabled, but no admitted adapter is ready.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    browser_materialization_fail_closed(
        "browser_adapter_stub_only",
        "Browser materialization adapter boundary exists, but live execution is not implemented in this primitive yet.",
        &url,
        &config,
        &runtime_metadata,
        url_safety,
    )
}
