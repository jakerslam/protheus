fn web_tooling_live_smoke_uses_real_model_provider_when_enabled() {
    if !truthy_test_env("INFRING_LIVE_WEB_TOOLING_SMOKE") {
        return;
    }
    let _env_lock = GOVERNANCE_LIVE_WEB_ENV_MUTEX.lock().expect("lock");
    let model_ref = std::env::var("INFRING_LIVE_WEB_TOOLING_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "openai/gpt-5".to_string());
    if model_ref.starts_with("openai/")
        && std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return;
    }
    let query = std::env::var("INFRING_LIVE_WEB_TOOLING_QUERY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "top AI agentic frameworks".to_string());
    let use_real_retrieval = truthy_test_env("INFRING_LIVE_WEB_TOOLING_USE_REAL_RETRIEVAL");
    let fixture_guard = if use_real_retrieval {
        None
    } else {
        Some(ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&json!({
                query.clone(): {
                    "ok": true,
                    "summary": "LangGraph, OpenAI Agents SDK, and AutoGen are commonly cited as top AI agentic frameworks.",
                    "requested_url": "https://example.com/frameworks",
                    "status_code": 200
                }
            }))
            .expect("encode fixture"),
        ))
    };

    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"live-web-smoke-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());

    let set_model_payload = serde_json::to_vec(&json!({"model": model_ref})).expect("serialize model");
    let set_model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        &snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);

    let message_payload = serde_json::to_vec(&json!({
        "message": format!("Try to web search \"{query}\" and return the results")
    }))
    .expect("serialize message");
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        &message_payload,
        &snapshot,
    )
    .expect("message response");
    drop(fixture_guard);

    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(!response_text.trim().is_empty(), "expected live synthesized response");
    assert!(!response_is_no_findings_placeholder(response_text));
    if !use_real_retrieval {
        assert!(
            lowered.contains("langgraph")
                || lowered.contains("openai agents sdk")
                || lowered.contains("autogen"),
            "{response_text}"
        );
    }
}

// Decomposed for backend file-size/cohesion remediation; behavior preserved via ordered includes.

#[test]
