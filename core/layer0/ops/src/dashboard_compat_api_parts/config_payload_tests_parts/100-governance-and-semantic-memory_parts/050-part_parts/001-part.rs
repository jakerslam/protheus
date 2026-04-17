fn workflow_retry_executes_latent_web_candidate_after_ack_only_format_block_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-ack-only-format-retry-agent","role":"researcher"}"#,
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
                    "response": "I attempted the web search again with the exact function call format, but the system rejected it with an \"ack_only\" response, meaning it recognized the function but didn't execute it. The platform is actively blocking external tool execution. The rejection appears to be a deliberate system policy rather than a technical failure."
                },
                {
                    "response": "I reran this through live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen."
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
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced in this rerun."
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
        br#"{"message":"Test the web capability again and summarize findings."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_selected_workflow_and_visible_response(
        &response,
        "I reran this through live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.contains("ack_only"), "{response_text}");
    assert!(
        !response_text.contains("function call format"),
        "{response_text}"
    );
    assert!(
        !response_text.contains("didn't execute"),
        "{response_text}"
    );
}

#[test]
fn workflow_retry_executes_latent_web_candidate_after_security_controls_block_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-security-controls-web-retry-agent","role":"researcher"}"#,
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
                    "response": "I attempted the web search and fetch operations again using the exact function call format, but the system continues to block external tool execution. The platform recognizes the function requests, but consistently prevents actual execution, likely due to the security controls you've implemented. The system requires proper authorization before executing external calls. Since you're the security team, you might want to check API gateway configurations, external service allowlists, execution policy settings, and sandbox-vs-production mode differences."
                },
                {
                    "response": "I reran live retrieval and found current web-tooling coverage with source-backed results for LangGraph, OpenAI Agents SDK, and CrewAI."
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
                        "summary": "LangGraph, OpenAI Agents SDK, and CrewAI surfaced in this rerun."
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
        br#"{"message":"try the web tooling again"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert!(response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("name").and_then(Value::as_str) == Some("batch_query"))
        })
        .unwrap_or(false));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.is_empty(), "{response_text}");
    assert!(
        response_text.contains("langgraph")
            || response_text.contains("openai agents sdk")
            || response_text.contains("crewai"),
        "{response_text}"
    );
    assert!(!response_text.contains("security controls"), "{response_text}");
    assert!(
        !response_text.contains("requires proper authorization"),
        "{response_text}"
    );
    assert!(!response_text.contains("api gateway"), "{response_text}");
    assert!(!response_text.contains("allowlists"), "{response_text}");
}

#[test]
fn workflow_response_contract_strips_follow_up_tool_markup_from_final_reply() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-tool-tail-agent","role":"researcher"}"#,
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
                    "response": "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names.\n\n<function=web_search>{\"query\":\"LangChain AutoGPT BabyAGI AI agent frameworks comparison\"}</function>"
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
                        "status": "no_results",
                        "summary": "Web retrieval ran, but did not return enough catalog-style framework evidence in this turn."
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
            .pointer("/response_workflow/selected/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response_text.contains(
            "My search for \"top AI agentic frameworks\" didn't return specific framework listings."
        ),
        "{response_text}"
    );
    let lowered = response_text.to_ascii_lowercase();
    assert!(!lowered.contains("<function="), "{response_text}");
    assert!(
        !lowered.contains("let me try a more targeted approach"),
        "{response_text}"
    );
    assert!(
        !lowered.contains("well-known framework names"),
        "{response_text}"
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
fn workflow_system_fallback_requires_final_stage_failure() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-failure-agent","role":"assistant"}"#,
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
                {"response": "Initial direct draft."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Say hello and confirm workflow ownership."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesis_failed")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected explicit fallback reply");
    assert!(
        response_text.to_ascii_lowercase().contains("response-synthesis failure")
            || response_text.to_ascii_lowercase().contains("retry"),
        "{response_text}"
    );
}

#[test]
fn workflow_repair_does_not_resurrect_prior_speculative_web_blocker_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-speculative-blocker-resurrection-guard-agent","role":"assistant"}"#,
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
                    "response": "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic. The system flagged this as an invalid response attempt rather than processing the queries."
                },
                {
                    "response": "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic. The system flagged this as an invalid response attempt rather than processing the queries."
                }
            ],
            "calls": []
        }),
    );
    let seed = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"try web search and report exactly what happened"}"#,
        &snapshot,
    )
    .expect("seed message response");
    assert_eq!(seed.status, 200);
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "I'll attempt the web search again to test current behavior."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"try again"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !response_text.contains("blocked the function calls from executing entirely"),
        "{response_text}"
    );
    assert!(
        !response_text.contains("invalid response attempt"),
        "{response_text}"
    );
    let finalization_outcome = response
        .payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !finalization_outcome.contains("repaired_with_latest_assistant"),
        "{finalization_outcome}"
    );
}

fn latest_persisted_assistant_text_for_test(root: &Path, agent_id: &str) -> String {
    let state = load_session_state(root, agent_id);
    let active_session_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for session in sessions {
        let session_id = clean_text(
            session.get("session_id").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if session_id != active_session_id {
            continue;
        }
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in messages.into_iter().rev() {
            if row.get("role").and_then(Value::as_str) != Some("assistant") {
                continue;
            }
            let text = clean_text(
                row.get("text")
                    .or_else(|| row.get("content"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                8_000,
            );
            if !text.is_empty() {
                return text;
            }
        }
    }
    String::new()
}

fn normalize_test_text_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
