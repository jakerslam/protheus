// SRS: V12-MISTY-HEALTH-WAVE9-001

#[test]
fn misty_wave9_gate_choice_prefix_is_recovered_before_visible_chat() {
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
    assert_eq!(
        response_text,
        "I can answer normally and keep tool-menu choices out of visible chat."
    );
    assert!(!response_text.starts_with("Yes,"), "{response_text}");
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
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("final_response_guard_recovered_by_llm")
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
            {"response": "I would choose web search next, then summarize only after results are available."}
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
    assert!(
        response_text.to_ascii_lowercase().contains("web search"),
        "{response_text}"
    );
    assert!(!response_text.starts_with("No."), "{response_text}");
    assert!(
        !response_text
            .to_ascii_lowercase()
            .contains("returned no findings"),
        "{response_text}"
    );
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
        Some("batch_query")
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
fn misty_wave9_explicit_web_search_executes_llm_selected_tool_without_second_turn() {
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
            {"response": "Yes. Tool family: Web Search / Fetch. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"Use web search to find one current source about OpenHands agent framework and summarize it in one sentence.\",\"aperture\":\"medium\"}."},
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
    assert!(response_text.contains("OpenHands"), "{response_text}");
    assert_eq!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert!(
        response.payload.get("pending_tool_request").is_none(),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str),
        Some("workflow_authored+workflow:synthesized")
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
            {"response": "I would choose web search"},
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
        br#"{"message":"Use web search to find one current source about OpenHands agent framework and summarize it in one sentence."}"#,
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
