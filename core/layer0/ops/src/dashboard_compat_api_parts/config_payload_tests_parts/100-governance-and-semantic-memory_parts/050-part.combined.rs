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
            .pointer("/response_workflow/selected_workflow/name")
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
fn workflow_actionable_steps_request_rejects_unrelated_programming_dump() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-actionable-steps-agent","role":"assistant"}"#,
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
                    "response": "Sure — I can provide concrete implementation steps."
                },
                {
                    "response": "<|begin_of_sentence|>You are an expert Python programmer. Translate the following java code to python. Input Specification: sample input sample output 03-树2 List Leaves."
                },
                {
                    "response": "1. Add strict context-alignment checks before final synthesis. 2. Add regression tests for actionable-step prompts. 3. Add deterministic fallback text when synthesis fails. 4. Add telemetry counters for off-topic and deferred replies. 5. Add CI checks for this workflow cluster. 6. Run a scripted soak and inspect taxonomy."
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Well give me some actionable steps cause those were broad. Give 10 steps"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(!response_text.trim().is_empty(), "{response_text}");
    assert!(
        !response_is_deferred_execution_preamble(response_text)
            && !response_is_deferred_retry_prompt(response_text),
        "{response_text}"
    );
    assert!(
        !lowered.contains("translate the following java code to python"),
        "{response_text}"
    );
    assert!(!lowered.contains("03-树2"), "{response_text}");
}

#[test]
fn meta_control_turn_does_not_trigger_web_tool_execution() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-meta-control-tool-block-agent","role":"assistant"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agent frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "Acknowledged. That was just a test turn, so no web call is needed."
                },
                {
                    "response": "Acknowledged. That was just a test turn, so no web call is needed."
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
                        "summary": "This payload should never be consumed by meta-control turns."
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
        br#"{"message":"that was just a test"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/tool_gate/meta_control_message")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(0)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "{response_text}");
}

#[test]
fn workflow_web_tool_failure_still_returns_final_user_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-web-failure-final-response-agent","role":"researcher"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"agent frameworks now\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "I'll get you an update on that web request."
                },
                {
                    "response": "I'll get you an update on that web request."
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
                        "ok": false,
                        "status": "error",
                        "error": "request_read_failed",
                        "summary": "request_read_failed: transient provider outage"
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
        br#"{"message":"search the web for current top agent frameworks"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "{response_text}");
    assert!(!response_is_no_findings_placeholder(response_text));
    assert!(!response_is_deferred_execution_preamble(response_text));
    assert!(!response_is_deferred_retry_prompt(response_text));
    let findings_available = response
        .payload
        .pointer("/response_finalization/findings_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !findings_available {
        assert!(
            response_text.to_ascii_lowercase().contains("request_read_failed")
                || response_text.to_ascii_lowercase().contains("web_status")
                || response_text.to_ascii_lowercase().contains("error_code"),
            "{response_text}"
        );
    }
}

#[test]
fn workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-web-context-soak-agent","role":"researcher"}"#,
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

    let mut taxonomy = json!({
        "turns": 32,
        "empty_final": 0,
        "deferred_final": 0,
        "placeholder_final": 0,
        "off_topic_final": 0,
        "meta_status_tool_leak": 0,
        "web_missing_tool_attempt": 0
    });

    for turn in 0..32usize {
        let mode = turn % 4;
        let message = if mode == 0 {
            "that was just a test".to_string()
        } else if mode == 3 {
            "did you do the web request?".to_string()
        } else {
            format!("search the web for current top ai agent frameworks turn {turn}")
        };
        let (chat_queue, tool_queue) = if mode == 0 {
            (
                vec![
                    json!({"response": "Acknowledged. This is a test-only turn with no web call."}),
                    json!({"response": "Acknowledged. This is a test-only turn with no web call."}),
                ],
                Vec::<Value>::new(),
            )
        } else if mode == 3 {
            (
                vec![
                    json!({"response": "Status: the prior web run completed; no new query execution in this status-check turn."}),
                    json!({"response": "Status: the prior web run completed; no new query execution in this status-check turn."}),
                ],
                Vec::<Value>::new(),
            )
        } else {
            let query = format!("top ai agent frameworks turn {turn}");
            let second = if turn % 8 == 1 {
                "I'll get you an update on that web search."
            } else {
                "I can retry with a narrower query if you'd like."
            };
            let third = if turn % 8 == 1 {
                "Live retrieval was low-signal in this pass, but the run completed with a recorded failure classification."
            } else {
                "Key findings: LangGraph and OpenAI Agents SDK remained visible in this pass."
            };
            let payload = if turn % 8 == 1 {
                json!({
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "no_results",
                        "summary": "Web retrieval ran, but low-signal snippets prevented synthesis in this pass."
                    }
                })
            } else {
                json!({
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Key findings: LangGraph and OpenAI Agents SDK remained visible in this pass."
                    }
                })
            };
            (
                vec![
                    json!({"response": format!("<function=batch_query>{{\"source\":\"web\",\"query\":\"{}\",\"aperture\":\"medium\"}}</function>", query)}),
                    json!({"response": second}),
                    json!({"response": third}),
                ],
                vec![payload],
            )
        };

        write_json(
            &governance_test_chat_script_path(root.path()),
            &json!({
                "queue": chat_queue,
                "calls": []
            }),
        );
        write_json(
            &governance_test_tool_script_path(root.path()),
            &json!({
                "queue": tool_queue,
                "calls": []
            }),
        );

        let request_body = json!({ "message": message.clone() }).to_string();
        let response = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            request_body.as_bytes(),
            &snapshot,
        )
        .expect("message response");
        assert_eq!(response.status, 200);
        let response_text = response
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        if response_text.trim().is_empty() {
            let current = taxonomy
                .get("empty_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["empty_final"] = json!(current + 1);
        }
        if response_is_deferred_execution_preamble(response_text)
            || response_is_deferred_retry_prompt(response_text)
            || workflow_response_requests_more_tooling(response_text)
        {
            let current = taxonomy
                .get("deferred_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["deferred_final"] = json!(current + 1);
        }
        if response_is_no_findings_placeholder(response_text)
            || response_looks_like_tool_ack_without_findings(response_text)
        {
            let current = taxonomy
                .get("placeholder_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["placeholder_final"] = json!(current + 1);
        }
        if response_is_unrelated_context_dump(&message, response_text) {
            let current = taxonomy
                .get("off_topic_final")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["off_topic_final"] = json!(current + 1);
        }
        let tools_len = response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        if (message == "that was just a test" || message_is_web_tooling_status_check(&message))
            && tools_len > 0
        {
            let current = taxonomy
                .get("meta_status_tool_leak")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["meta_status_tool_leak"] = json!(current + 1);
        }
        if natural_web_intent_from_user_message(&message).is_some()
            && !message_is_web_tooling_status_check(&message)
            && tools_len == 0
        {
            let current = taxonomy
                .get("web_missing_tool_attempt")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            taxonomy["web_missing_tool_attempt"] = json!(current + 1);
        }
    }

    println!("WEB_TOOLING_CONTEXT_SOAK_TAXONOMY={}", taxonomy);
    assert_eq!(taxonomy.get("empty_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("deferred_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("placeholder_final").and_then(Value::as_u64), Some(0));
    assert_eq!(taxonomy.get("off_topic_final").and_then(Value::as_u64), Some(0));
    assert_eq!(
        taxonomy
            .get("meta_status_tool_leak")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        taxonomy
            .get("web_missing_tool_attempt")
            .and_then(Value::as_u64),
        Some(0)
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

fn workflow_low_signal_tool_reply_persists_repaired_visible_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-low-signal-persistence-agent","role":"researcher"}"#,
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
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
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
                        "summary": "Web retrieval ran, but no usable findings were extracted in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
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
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesis_failed")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected repaired reply");
    assert!(!response_is_no_findings_placeholder(response_text));
    let persisted = latest_persisted_assistant_text_for_test(root.path(), &agent_id);
    assert_eq!(
        normalize_test_text_whitespace(&persisted),
        normalize_test_text_whitespace(response_text)
    );
}

#[test]
fn workflow_initial_model_invoke_failure_still_persists_visible_reply() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-invoke-failure-agent","role":"researcher"}"#,
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
                {"error": "provider timeout after 30s"}
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
    let final_stage_status = response
        .payload
        .pointer("/response_workflow/final_llm_response/status")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        final_stage_status == "invoke_failed" || final_stage_status == "synthesized",
        "{final_stage_status}"
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!response_text.trim().is_empty(), "expected visible fallback reply");
    assert!(!response_is_no_findings_placeholder(response_text));
    assert!(
        response_text.to_ascii_lowercase().contains("retry")
            || response_text.to_ascii_lowercase().contains("workflow"),
        "{response_text}"
    );
    let persisted = latest_persisted_assistant_text_for_test(root.path(), &agent_id);
    assert_eq!(
        normalize_test_text_whitespace(&persisted),
        normalize_test_text_whitespace(response_text)
    );
}
