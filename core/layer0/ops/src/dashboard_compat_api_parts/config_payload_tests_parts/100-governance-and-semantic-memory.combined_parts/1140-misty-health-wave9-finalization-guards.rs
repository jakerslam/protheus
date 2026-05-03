// SRS: V12-MISTY-HEALTH-WAVE9-001

#[test]
fn misty_wave9_gate_choice_prefix_is_rejected_without_system_retry() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-gate-prefix-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "Yes, tool family: Conversation. Tool: Answer directly."},
            {"response": "I can answer normally and keep tool-menu choices out of visible chat."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Do you feel like you can answer normally and decide whether tools are needed? Answer naturally."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(response_text, "");
    assert!(!response_text.starts_with("Yes,"), "{response_text}");
    assert_eq!(
        response
            .payload
            .get("visible_response_source")
            .and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/fallback_source")
            .and_then(Value::as_str)
            .unwrap_or("")
            == "withheld_invalid_gate_draft"
    );
}

#[test]
fn misty_wave9_invalid_tool_choice_draft_is_not_visible_fallback() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-invalid-tool-choice-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "I would choose to run a batch query for current framework evidence."},
            {"response": "I would choose web search for this comparison."},
            {"response": "I need to perform a web search before answering."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to compare infring to top agentic frameworks in April 2026. Return a source-backed comparison."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(response.payload.get("pending_tool_request"), None);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/invalid_gate_draft_fallback_withheld")
            .and_then(Value::as_bool),
        Some(true),
        "{}",
        response.payload
    );
}

#[test]
fn misty_wave9_gate_2_tool_request_draft_is_never_visible_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-gate2-draft-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "Initial private interpretation draft."},
            {"response": "3"},
            {"response": "I would choose web search to collect current framework evidence."},
            {"response": "I will perform a web search and then compare the findings."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Compare infring to top agentic frameworks in April 2026 using web research."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/workflow_control/direct_response_path")
            .and_then(Value::as_str),
        Some("gate_2_pending_llm_tool_request"),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/fallback_source")
            .and_then(Value::as_str),
        Some("withheld_invalid_gate_draft"),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave9_web_no_findings_without_tool_receipt_becomes_pending_choice() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-web-evidence-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "No. The web search returned no findings about current OpenHands and AutoGPT status."},
            {"response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"current OpenHands and AutoGPT status\",\"aperture\":\"medium\"}."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to check current OpenHands and AutoGPT status, then summarize whether you need another tool before answering."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(response_text, "");
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/execution_claim_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/web_intent/detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave9_explicit_web_search_creates_pending_tool_request_without_system_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-explicit-web-execution-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"Use web search to find one current source about OpenHands agent framework and summarize it in one sentence.\",\"aperture\":\"medium\"}."},
            {"response": "OpenHands is an open-source agent framework for software development tasks."}
        ], "calls": []}),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({"queue": [
            {
                "tool": "batch_query",
                "payload": {
                    "ok": true,
                    "status": "ok",
                    "summary": "OpenHands is an open-source agent framework for software development tasks."
                }
            }
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to find one current source about OpenHands agent framework and summarize it in one sentence."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(response_text, "");
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
    assert!(
        response.payload.get("pending_tool_request").is_some(),
        "{}",
        response.payload
    );
    assert!(
        response
            .payload
            .pointer("/response_finalization/pending_tool_request/tool_name")
            .and_then(Value::as_str)
            == Some("web_search"),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_quality_telemetry/tool_overcall_rate")
            .and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave9_tool_synthesis_unwraps_content_type_json_fragment() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave9-json-fragment-agent","role":"assistant"}"#,
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
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "1"},
            {"response": "\"content\": \"OpenHands is an AI agent platform for software development.\", \"type\": \"platform\", \"format\": \"plain\" }"}
        ], "calls": []}),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({"queue": [
            {
                "tool": "batch_query",
                "payload": {
                    "ok": true,
                    "status": "ok",
                    "summary": "OpenHands is an AI agent platform for software development."
                }
            }
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Summarize this current-source content fragment about OpenHands in one sentence."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("OpenHands is an AI agent platform for software development.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/issue_count")
            .and_then(Value::as_u64),
        Some(0),
        "{}",
        response.payload
    );
}
