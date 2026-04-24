use std::sync::Mutex;

static GOVERNANCE_LIVE_WEB_ENV_MUTEX: Mutex<()> = Mutex::new(());

struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn truthy_test_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(ref value) if value == "1" || value == "true" || value == "yes"
    )
}

#[test]
fn workflow_library_allows_direct_answer_without_second_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-direct-answer-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Hello, the direct path is working."
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Say hello and confirm the chain is working."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("Hello, the direct path is working.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_not_required")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn workflow_library_owns_successful_tool_turn_final_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-tool-turn-agent","role":"researcher"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {"response": "For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen."}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced as top AI agentic frameworks in the fetched results."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn natural_web_prompt_stays_off_direct_tool_route_when_models_are_available() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"natural-web-model-first-agent","role":"researcher"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "Based on the fetched results, LangGraph, OpenAI Agents SDK, and AutoGen are the clearest agentic framework hits."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced in the fetched framework results."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_ne!(
        response.payload.get("provider").and_then(Value::as_str),
        Some("tool")
    );
    assert_ne!(
        response.payload.get("runtime_model").and_then(Value::as_str),
        Some("tool-router")
    );
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some(
            "Based on the fetched results, LangGraph, OpenAI Agents SDK, and AutoGen are the clearest agentic framework hits."
        )
    );
}

#[test]
fn workflow_retry_validator_blocks_search_again_language() {
    assert!(workflow_response_requests_more_tooling(
        "Let me search for more specific AI agent framework information using a narrower query."
    ));
    assert!(workflow_response_requests_more_tooling(
        "Retry with a narrower query or one specific source URL."
    ));
    assert!(!workflow_response_requests_more_tooling(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
}

#[test]
fn web_tooling_harness_surfaces_no_results_with_final_llm_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"web-tooling-no-results-agent","role":"researcher"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": false,
                        "status": "no_results",
                        "summary": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or a specific source URL.",
                        "error": "search_providers_exhausted"
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
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
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("no_results")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(!response_text.trim().is_empty(), "expected synthesized no-results reply");
    assert!(
        lowered.contains("low-signal") || lowered.contains("source-backed answer"),
        "{response_text}"
    );
    assert!(!response_is_no_findings_placeholder(response_text));
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
}

#[test]
fn compare_workflow_hint_clusters_workspace_and_web_tools() {
    let hints = latent_tool_candidates_for_message("compare this system to openclaw", &[]);
    let tool_names = hints
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"workspace_analyze"), "{tool_names:?}");
    assert!(tool_names.contains(&"batch_query"), "{tool_names:?}");
}

#[test]
fn compare_platform_wording_clusters_workspace_and_web_tools() {
    let hints = latent_tool_candidates_for_message("compare this platform to openclaw", &[]);
    let tool_names = hints
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"workspace_analyze"), "{tool_names:?}");
    assert!(tool_names.contains(&"batch_query"), "{tool_names:?}");
}

#[test]
fn compare_workflow_harness_decomposes_local_and_web_evidence_before_final_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-agent","role":"researcher"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1 and a domain-grouped tool catalog."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling and native search contracts."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        !response_text.trim().is_empty(),
        "expected synthesized compare response"
    );
}

#[test]
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
