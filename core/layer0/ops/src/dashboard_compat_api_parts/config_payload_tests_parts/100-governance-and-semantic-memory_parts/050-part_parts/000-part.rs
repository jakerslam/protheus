fn assert_selected_workflow_and_visible_response(response: &CompatApiResponse, expected: &str) {
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/response")
            .and_then(Value::as_str),
        Some(expected)
    );
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some(expected)
    );
}

#[test]
fn workflow_response_contract_persists_selected_workflow_for_direct_success() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-direct-agent","role":"assistant"}"#,
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
                {"response": "Workflow-authored direct response from the final stage."}
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
    assert_selected_workflow_and_visible_response(
        &response,
        "Workflow-authored direct response from the final stage.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
}

#[test]
fn workflow_response_contract_persists_selected_workflow_for_tool_success() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-tool-agent","role":"researcher"}"#,
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
                    "response": "The fetched results in this run highlighted LangGraph, OpenAI Agents SDK, and AutoGen."
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
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced in the fetched results."
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
    assert_selected_workflow_and_visible_response(
        &response,
        "The fetched results in this run highlighted LangGraph, OpenAI Agents SDK, and AutoGen.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
}

#[test]
fn workflow_executes_latent_web_candidate_when_initial_draft_skips_tool_call() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-latent-web-agent","role":"researcher"}"#,
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
                    "response": "I attempted to perform a web search, but the search function isn't currently operational in this session."
                },
                {
                    "response": "I reran this through live retrieval and found that LangGraph, OpenAI Agents SDK, and CrewAI are consistently cited as top frameworks."
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
                        "summary": "LangGraph, OpenAI Agents SDK, and CrewAI surfaced across current framework coverage."
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
        br#"{"message":"try doing text random web search and returning a summury of you findings"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_selected_workflow_and_visible_response(
        &response,
        "I reran this through live retrieval and found that LangGraph, OpenAI Agents SDK, and CrewAI are consistently cited as top frameworks.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert!(response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("name").and_then(Value::as_str) == Some("batch_query"))
        })
        .unwrap_or(false));
}

#[test]
fn workflow_retry_debug_prompt_executes_latent_web_candidate_from_failed_draft() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-retry-debug-agent","role":"researcher"}"#,
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
                    "response": "I attempted to run a web search as requested, but the system rejected the function call format. The workflow events show my draft response was flagged invalid with reason \"ack_only\"."
                },
                {
                    "response": "I reran live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen."
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
        br#"{"message":"try again and output the exact system results so we can debug"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_selected_workflow_and_visible_response(
        &response,
        "I reran live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
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
        .unwrap_or("");
    assert!(
        !response_text
            .to_ascii_lowercase()
            .contains("rejected the function call format"),
        "{response_text}"
    );
}

#[test]
fn workflow_retry_executes_latent_web_candidate_after_speculative_web_block_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-speculative-web-retry-agent","role":"researcher"}"#,
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
                    "response": "I understand you're looking for a comparison between this platform and OpenClaw, but I'm currently unable to access web search functionality to gather the necessary information. The system is blocking tool execution attempts, which prevents me from retrieving current details. Based on the system behavior I'm observing, likely reasons include Configuration Restrictions, Authentication Issues, Rate Limiting, or intentional sandboxed design."
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
        br#"{"message":"try doing text random web search and returning a summury of you findings"}"#,
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
    let has_batch_query_tool = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("name").and_then(Value::as_str) == Some("batch_query"))
        })
        .unwrap_or(false);
    assert!(has_batch_query_tool);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.contains("configuration restrictions"), "{response_text}");
    assert!(
        !response_text.contains("blocking tool execution attempts"),
        "{response_text}"
    );
}

#[test]
