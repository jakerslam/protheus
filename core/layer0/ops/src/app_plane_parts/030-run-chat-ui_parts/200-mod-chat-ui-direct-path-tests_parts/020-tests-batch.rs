#[test]
fn direct_run_chat_ui_keeps_web_tool_selection_llm_controlled_for_explicit_chili_prompt() {
    let root = tempfile::tempdir().expect("tempdir");
    write_chat_script(
        root.path(),
        &json!({
            "queue": [
                {
                    "response": "I don't have web search capabilities.",
                    "tools": []
                }
            ],
            "batch_query_queue": [
                {
                    "ok": true,
                    "type": "batch_query",
                    "status": "ok",
                    "summary": "Key findings: allrecipes.com: Best Damn Chili Recipe.",
                    "evidence_refs": [
                        {
                            "locator": "https://www.allrecipes.com/recipe/233613/best-damn-chili/"
                        }
                    ]
                }
            ]
        }),
    );

    let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=direct-web-parity".to_string(),
            "--message=well try doing a web search and returning the results. make the websearch about best chili recipes".to_string(),
            "--strict=1".to_string(),
        ]);
    let payload = run_chat_ui(root.path(), &parsed, true, "run");
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    let response = payload
        .pointer("/turn/assistant")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        response.contains("before any search tool call was recorded"),
        "{response}"
    );
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!tools.iter().any(|row| {
        clean(
            row.get("name")
                .or_else(|| row.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase()
            == "batch_query"
    }));
    let invariant = payload
        .pointer("/response_finalization/web_invariant")
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        invariant.get("classification").and_then(Value::as_str),
        Some("tool_not_invoked")
    );
    assert_eq!(
        invariant.get("tool_attempted").and_then(Value::as_bool),
        Some(false)
    );
    let summary = payload
        .pointer("/response_finalization/tool_receipt_summary")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(summary
        .to_ascii_lowercase()
        .contains("tool transaction failed"));
    let transaction = payload
        .pointer("/response_finalization/tool_transaction")
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        transaction.get("complete").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        transaction.get("status").and_then(Value::as_str),
        Some("failed")
    );
}

#[test]
fn chat_ui_finalization_fail_closes_when_tool_surface_is_unavailable() {
    let (assistant, outcome) = finalize_chat_ui_assistant_response(
        "search current top agent frameworks",
        "I'll get you an update on the current best AI agent frameworks.",
        &[json!({
            "name": "batch_query",
            "status": "error",
            "error": "web_search_tool_surface_unavailable"
        })],
    );
    assert_eq!(outcome, "tool_surface_error_fail_closed");
    assert!(assistant.is_empty(), "{assistant}");
}

#[test]
fn direct_run_chat_ui_surfaces_tool_surface_unavailable_error_and_classification() {
    let root = tempfile::tempdir().expect("tempdir");
    write_chat_script(
        root.path(),
        &json!({
            "queue": [
                {
                    "response": "working on it",
                    "tools": [
                        {
                            "name": "batch_query",
                            "status": "error",
                            "error": "web_search_tool_surface_unavailable"
                        }
                    ]
                }
            ]
        }),
    );

    let parsed = crate::parse_args(&[
        "run".to_string(),
        "--app=chat-ui".to_string(),
        "--session-id=surface-unavailable".to_string(),
        "--message=try searching for current top agent frameworks".to_string(),
        "--strict=1".to_string(),
    ]);
    let payload = run_chat_ui(root.path(), &parsed, true, "run");
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        payload.get("error").and_then(Value::as_str),
        Some("web_tool_surface_unavailable")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/classification")
            .and_then(Value::as_str),
        Some("tool_surface_unavailable")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/status")
            .and_then(Value::as_str),
        Some("failed")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str),
        Some("forced_web_tool_surface_unavailable")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/classification_guard/fail_closed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/web_tooling_fallback/reason")
            .and_then(Value::as_str),
        Some("detected_tool_surface_error")
    );
    let assistant = payload
        .pointer("/turn/assistant")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(assistant.is_empty(), "{assistant}");
}

#[test]
fn chat_ui_finalization_fail_closes_when_tool_surface_is_degraded() {
    let (assistant, outcome) = finalize_chat_ui_assistant_response(
        "search current top agent frameworks",
        "let me check that quickly",
        &[json!({
            "name": "batch_query",
            "status": "error",
            "error": "web_search_tool_surface_degraded"
        })],
    );
    assert_eq!(outcome, "tool_surface_error_fail_closed");
    assert!(assistant.is_empty(), "{assistant}");
}

#[test]
fn direct_run_chat_ui_surfaces_tool_surface_degraded_error_and_classification() {
    let root = tempfile::tempdir().expect("tempdir");
    write_chat_script(
        root.path(),
        &json!({
            "queue": [
                {
                    "response": "working on it",
                    "tools": [
                        {
                            "name": "batch_query",
                            "status": "error",
                            "error": "web_search_tool_surface_degraded"
                        }
                    ]
                }
            ]
        }),
    );

    let parsed = crate::parse_args(&[
        "run".to_string(),
        "--app=chat-ui".to_string(),
        "--session-id=surface-degraded".to_string(),
        "--message=try searching for current top agent frameworks".to_string(),
        "--strict=1".to_string(),
    ]);
    let payload = run_chat_ui(root.path(), &parsed, true, "run");
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        payload.get("error").and_then(Value::as_str),
        Some("web_tool_surface_degraded")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/classification")
            .and_then(Value::as_str),
        Some("tool_surface_degraded")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/status")
            .and_then(Value::as_str),
        Some("degraded")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str),
        Some("forced_web_tool_surface_degraded")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/classification_guard/fail_closed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/retry/recommended")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/retry/strategy")
            .and_then(Value::as_str),
        Some("retry_with_backoff")
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/retry/plan/auto")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/tool_transaction/retry/plan/attempts")
            .and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        payload
            .pointer("/web_tooling_fallback/reason")
            .and_then(Value::as_str),
        Some("detected_tool_surface_error")
    );
    let assistant = payload
        .pointer("/turn/assistant")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(assistant.is_empty(), "{assistant}");
}

#[test]
fn chat_ui_tool_surface_detector_prioritizes_unavailable_over_degraded() {
    let code = chat_ui_detect_tool_surface_error_code(&[
        json!({
            "name": "batch_query",
            "status": "error",
            "error": "web_search_tool_surface_degraded"
        }),
        json!({
            "name": "batch_query",
            "status": "error",
            "error": "web_search_tool_surface_unavailable"
        }),
    ]);
    assert_eq!(code, Some("web_tool_surface_unavailable"));
}
